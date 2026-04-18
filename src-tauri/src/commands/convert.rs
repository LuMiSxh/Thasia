use crate::events::{
    ConversionCompleteEvent, ImageProgressEvent, VolumeCompleteEvent, VolumeStartEvent,
};
use crate::state::{ConvState, ConvertOptions, VolumeEdit};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tauri::{AppHandle, State};
use tauri_specta::Event;
use thasia_core::models::{ChapterIdentifier, DiscoveredImage, OutputFormat, ParsedImage};
use thasia_packager::{CbzGenerator, EpubGenerator, Generator, RawGenerator};
use thasia_processor::{start_pipeline, EncodeOptions};
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

    // Pull scan result and source out of state
    let (scan_result, source) = {
        let s = state.read().map_err(|e| e.to_string())?;
        let scan_result = s
            .scan_result
            .clone()
            .ok_or_else(|| "No scan result — run scan_source first".to_string())?;
        // Arc<LocalSource> is cheap to clone — keeps TempDir alive across volumes.
        let source = s.source.clone();
        (scan_result, source)
    };

    // Build lookup: volume_num → pages
    let scan_map: BTreeMap<u32, Vec<ParsedImage>> = scan_result.into_iter().collect();

    let total_volumes = edits.len() as u32;
    let mut volumes: Vec<(u32, Vec<ParsedImage>, String)> = Vec::new();

    for edit in &edits {
        let original_pages = scan_map.get(&edit.volume_num).cloned().unwrap_or_default();
        let mut final_pages: Vec<ParsedImage> = Vec::new();

        for entry in &edit.pages {
            if entry.excluded {
                continue;
            }

            if let Some(idx) = entry.original_page_index {
                // source_volume_num lets pages moved from another scan volume be looked up correctly.
                let src_vol = entry.source_volume_num.unwrap_or(edit.volume_num);
                let page = if src_vol == edit.volume_num {
                    original_pages.get(idx as usize)
                } else {
                    scan_map.get(&src_vol).and_then(|v| v.get(idx as usize))
                };
                if let Some(p) = page {
                    final_pages.push(p.clone());
                }
            } else if let Some(ref cp) = entry.custom_path {
                let path = PathBuf::from(cp);
                let rel = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                let is_cover = final_pages.is_empty();
                let page_number = final_pages.len() as u32;
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

        let vol_name = if total_volumes == 1 && options.hide_single_volume {
            options.output_name.clone()
        } else {
            format!(
                "{}{}{}",
                options.output_name, options.volume_separator, edit.volume_num
            )
        };

        volumes.push((edit.volume_num, final_pages, vol_name));
    }

    let out_root = if options.create_directory {
        PathBuf::from(&options.output_dir).join(&options.output_name)
    } else {
        PathBuf::from(&options.output_dir)
    };
    tokio::fs::create_dir_all(&out_root)
        .await
        .map_err(|e| e.to_string())?;

    let encode_opts = Arc::new(EncodeOptions {
        format: options.image_format.clone(),
        max_width: options.max_width,
    });

    // Use the stored source if available; otherwise create a minimal LocalSource.
    // LocalSource::fetch reads absolute_path directly, so the root only matters
    // for discover(), which is not called here.
    let arc_source: Arc<LocalSource> = source
        .unwrap_or_else(|| Arc::new(LocalSource::new(out_root.clone())));

    let mut successful = 0u32;
    let mut failed = 0u32;

    for (vol_num, pages, vol_name) in volumes {
        VolumeStartEvent {
            volume_num: vol_num,
            volume_name: vol_name.clone(),
            total_volumes,
        }
        .emit(&app)
        .ok();

        let result = convert_volume(
            pages,
            vol_name.clone(),
            &out_root,
            &options,
            encode_opts.clone(),
            arc_source.clone(),
            vol_num,
            &app,
        )
        .await;

        match result {
            Ok(()) => {
                successful += 1;
                VolumeCompleteEvent {
                    volume_num: vol_num,
                    success: true,
                    error: None,
                }
                .emit(&app)
                .ok();
            }
            Err(e) => {
                failed += 1;
                tracing::error!("Volume {} failed: {e}", vol_num);
                VolumeCompleteEvent {
                    volume_num: vol_num,
                    success: false,
                    error: Some(e),
                }
                .emit(&app)
                .ok();
            }
        }
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

async fn convert_volume(
    pages: Vec<ParsedImage>,
    vol_name: String,
    out_root: &Path,
    options: &ConvertOptions,
    encode_opts: Arc<EncodeOptions>,
    source: Arc<LocalSource>,
    vol_num: u32,
    app: &AppHandle,
) -> Result<(), String> {
    let total = pages.len() as u32;

    let (tx_parsed, rx_parsed) = tokio::sync::mpsc::channel(256);

    // Feed pages into the pipeline without blocking the encoding loop.
    tokio::spawn(async move {
        for parsed in pages {
            let _ = tx_parsed.send(parsed).await;
        }
    });

    let mut rx_processed = start_pipeline(source, rx_parsed, encode_opts).await;

    let mut gen: Box<dyn Generator + Send> = match options.output_format {
        OutputFormat::Cbz => Box::new(CbzGenerator::new()),
        OutputFormat::Epub => {
            Box::new(EpubGenerator::new().with_direction(options.direction.clone()))
        }
        OutputFormat::Raw => Box::new(RawGenerator::new()),
    };

    gen.init(out_root, &vol_name)
        .await
        .map_err(|e| e.to_string())?;

    // Emit progress after each image is encoded and written — tracks real work.
    let mut current = 0u32;
    while let Some(result) = rx_processed.recv().await {
        let img = result.map_err(|e| e.to_string())?;
        gen.add_page(img).await.map_err(|e| e.to_string())?;
        current += 1;
        ImageProgressEvent { volume_num: vol_num, current, total }
            .emit(app)
            .ok();
    }

    gen.finalize().await.map_err(|e| e.to_string())?;
    Ok(())
}
