use crate::events::{
    ConversionCompleteEvent, ImageProgressEvent, VolumeCompleteEvent, VolumeStartEvent,
};
use crate::state::{ConvState, ConvertOptions, PageEditSource, VolumeEdit};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use tauri::{AppHandle, State};
use tauri_specta::Event;
use thasia_core::{
    OutputFormat, VolumeNaming, VolumePlan, apply_naming,
    models::{ChapterIdentifier, DiscoveredImage, ParsedImage, ProcessedImage},
};
use thasia_packager::{CbzGenerator, EpubGenerator, Generator, RawGenerator};
use thasia_processor::{EncodeOptions, start_pipeline};
use thasia_source::LocalSource;

#[tauri::command]
#[specta::specta]
pub async fn convert(
    options: ConvertOptions,
    edits: Vec<VolumeEdit>,
    state: State<'_, RwLock<ConvState>>,
    app: AppHandle,
) -> Result<(), String> {
    let start = Instant::now();

    // Pull scan result, source, and cancel handle out of state in a single read.
    let (scan_result, source, cancel) = {
        let s = state.read().map_err(|e| e.to_string())?;
        let scan_result = s
            .scan_result
            .clone()
            .ok_or_else(|| "No scan result — run scan_source first".to_string())?;
        let source = s
            .source
            .clone()
            .ok_or_else(|| "Source missing — re-run scan_source".to_string())?;
        let cancel = s.cancel.clone();
        (scan_result, source, cancel)
    };

    // Reset cancellation for this run.
    cancel.store(false, Ordering::SeqCst);

    // Build lookup: scan_volume_num → pages
    let scan_map: BTreeMap<u32, Vec<ParsedImage>> = scan_result.into_iter().collect();

    // Resolve each VolumeEdit into a concrete (volume_num, Vec<ParsedImage>) group.
    let groups: Vec<(u32, Vec<ParsedImage>)> = edits
        .iter()
        .map(|edit| (edit.volume_num, resolve_edit_pages(edit, &scan_map)))
        .collect();

    // Build the conversion plan (apply naming).
    let naming = VolumeNaming {
        name: &options.output_name,
        separator: &options.volume_separator,
        hide_single_volume: options.hide_single_volume,
    };
    let plans = apply_naming(groups, &naming);
    let total_volumes = plans.len() as u32;

    let out_root = if options.create_directory {
        PathBuf::from(&options.output_dir).join(&options.output_name)
    } else {
        PathBuf::from(&options.output_dir)
    };
    tokio::fs::create_dir_all(&out_root)
        .await
        .map_err(|e| e.to_string())?;

    let encode_opts = EncodeOptions {
        format: options.image_format,
        max_width: options.max_width,
    };

    let mut successful = 0u32;
    let mut failed = 0u32;

    for plan in plans {
        if cancel.load(Ordering::SeqCst) {
            break;
        }

        let vol_num = plan.volume_num;
        VolumeStartEvent {
            volume_num: vol_num,
            volume_name: plan.display_name.clone(),
            total_volumes,
        }
        .emit(&app)
        .ok();

        let result = convert_volume(
            plan,
            &out_root,
            &options,
            encode_opts,
            source.clone(),
            cancel.clone(),
            &app,
        )
        .await;

        let (success, error) = match result {
            Ok(()) => {
                successful += 1;
                (true, None)
            }
            Err(e) => {
                failed += 1;
                tracing::error!("Volume {} failed: {e}", vol_num);
                (false, Some(e))
            }
        };
        let _ = VolumeCompleteEvent {
            volume_num: vol_num,
            success,
            error,
        }
        .emit(&app);
    }

    ConversionCompleteEvent {
        successful,
        failed,
        duration_secs: start.elapsed().as_secs_f64(),
    }
    .emit(&app)
    .ok();

    Ok(())
}

/// Materialize the final ordered page list for one VolumeEdit by looking each
/// PageEditEntry up in the scan_map (or constructing a custom-path entry).
fn resolve_edit_pages(
    edit: &VolumeEdit,
    scan_map: &BTreeMap<u32, Vec<ParsedImage>>,
) -> Vec<ParsedImage> {
    let mut final_pages: Vec<ParsedImage> = Vec::new();

    for entry in &edit.pages {
        if entry.excluded {
            continue;
        }

        match &entry.source {
            PageEditSource::Original {
                page_index,
                source_volume_num,
            } => {
                let src_vol = source_volume_num.unwrap_or(edit.volume_num);
                if let Some(page) = scan_map
                    .get(&src_vol)
                    .and_then(|v| v.get(*page_index as usize))
                {
                    final_pages.push(page.clone());
                }
            }
            PageEditSource::Custom { path } => {
                let path = PathBuf::from(path);
                let rel = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                let is_cover = final_pages.is_empty();
                let page_number = final_pages.len() as f32;
                final_pages.push(ParsedImage {
                    source: DiscoveredImage {
                        absolute_path: path,
                        relative_path: rel,
                    },
                    identifier: ChapterIdentifier {
                        volume: Some(edit.volume_num),
                        chapter: None,
                    },
                    page_number,
                    is_cover,
                });
            }
        }
    }

    // Assign sequential per-volume page numbers so filenames are unique and
    // the sort in convert_volume produces the correct output order regardless
    // of Rayon's non-deterministic completion order.
    for (i, page) in final_pages.iter_mut().enumerate() {
        page.page_number = i as f32;
    }
    final_pages
}

async fn convert_volume(
    plan: VolumePlan,
    out_root: &Path,
    options: &ConvertOptions,
    encode_opts: EncodeOptions,
    source: Arc<LocalSource>,
    cancel: Arc<AtomicBool>,
    app: &AppHandle,
) -> Result<(), String> {
    let vol_num = plan.volume_num;
    let total = plan.pages.len() as u32;

    let (tx_parsed, rx_parsed) = tokio::sync::mpsc::channel(256);

    // Feed pages into the pipeline without blocking the encoding loop.
    let pages = plan.pages;
    tokio::spawn(async move {
        for parsed in pages {
            if tx_parsed.send(parsed).await.is_err() {
                break;
            }
        }
    });

    let mut rx_processed = start_pipeline(source, rx_parsed, encode_opts).await;

    let mut pkg: Box<dyn Generator> = match options.output_format {
        OutputFormat::Cbz => Box::new(CbzGenerator::new()),
        OutputFormat::Epub => Box::new(EpubGenerator::new().with_direction(options.direction)),
        OutputFormat::Raw => Box::new(RawGenerator::new()),
    };

    pkg.init(out_root, &plan.display_name)
        .await
        .map_err(|e| e.to_string())?;

    // Collect all encoded images before writing — Rayon processes pages in
    // parallel and returns them in completion order (non-deterministic), so we
    // sort by page_number (assigned sequentially in resolve_edit_pages) before
    // writing so CBZ entries land in the right order on disk.
    let mut all_images: Vec<ProcessedImage> = Vec::with_capacity(total as usize);
    let mut current = 0u32;
    while let Some(result) = rx_processed.recv().await {
        if cancel.load(Ordering::SeqCst) {
            return Err("Cancelled".to_string());
        }
        let img = result.map_err(|e| e.to_string())?;
        current += 1;
        ImageProgressEvent {
            volume_num: vol_num,
            current,
            total,
        }
        .emit(app)
        .ok();
        all_images.push(img);
    }
    all_images.sort_by(|a, b| {
        a.parsed_data
            .page_number
            .total_cmp(&b.parsed_data.page_number)
    });
    for img in all_images {
        pkg.add_page(img).await.map_err(|e| e.to_string())?;
    }

    pkg.finalize().await.map_err(|e| e.to_string())?;
    Ok(())
}
