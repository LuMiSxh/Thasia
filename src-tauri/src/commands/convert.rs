use crate::events::{
    ConversionCompleteEvent, ConversionOutput, ImageProgressEvent, VolumeCompleteEvent,
    VolumeStartEvent,
};
use crate::state::{ConvState, ConvertOptions, PageEditSource, PipelinePlan, VolumeEdit};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, State};
use tauri_specta::Event;
use thasia_core::prelude::{
    ChapterIdentifier, DiscoveredImage, OutputFormat, ParsedImage, ProcessedImage, ThasiaError,
    VolumeNaming, VolumePlan, apply_naming, sanitize_filename_component,
};
use thasia_packager::{CbzGenerator, EpubGenerator, Generator, RawGenerator};
use thasia_processor::{EncodeOptions, start_pipeline_with_cancel};
use thasia_source::LocalSource;
use thiserror::Error;

const MAX_CUSTOM_IMAGE_BYTES: u64 = 512 * 1024 * 1024;

type ConvertResult<T> = Result<T, ConvertError>;

#[derive(Debug, Error)]
enum ConvertError {
    #[error("No scan result - run scan_source first")]
    MissingScanResult,
    #[error("Source missing - re-run scan_source")]
    MissingSource,
    #[error("Conversion was cancelled")]
    Cancelled,
    #[error("Output path was not initialized")]
    MissingOutputPath,
    #[error("State lock failed: {0}")]
    StateLock(String),
    #[error("Failed to create output directory {path:?}: {source}")]
    CreateOutputDir {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Invalid output name: {0}")]
    InvalidOutputName(#[from] ThasiaError),
    #[error("Custom page is not a supported image: {path:?}")]
    UnsupportedCustomImage { path: PathBuf },
    #[error("Custom page is not readable ({path:?}): {source}")]
    CustomImageMetadata {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Custom page is not a file: {path:?}")]
    CustomImageNotFile { path: PathBuf },
    #[error("Custom page is too large ({bytes} bytes): {path:?}")]
    CustomImageTooLarge { path: PathBuf, bytes: u64 },
    #[error("Package error: {0}")]
    Package(ThasiaError),
    #[error("Image pipeline error: {0}")]
    Pipeline(ThasiaError),
}

#[tauri::command]
#[specta::specta]
pub fn build_pipeline_plan(
    options: ConvertOptions,
    edits: Vec<VolumeEdit>,
) -> Result<PipelinePlan, String> {
    Ok(crate::pipeline_plan::build(options, edits))
}

#[tauri::command]
#[specta::specta]
pub async fn convert(
    options: ConvertOptions,
    edits: Vec<VolumeEdit>,
    state: State<'_, RwLock<ConvState>>,
    app: AppHandle,
) -> Result<(), String> {
    convert_inner(options, edits, state, app)
        .await
        .map_err(|e| e.to_string())
}

async fn convert_inner(
    options: ConvertOptions,
    edits: Vec<VolumeEdit>,
    state: State<'_, RwLock<ConvState>>,
    app: AppHandle,
) -> ConvertResult<()> {
    let start = Instant::now();

    let snapshot = conversion_state_snapshot(&state)?;

    // Reset cancellation for this run.
    snapshot.cancel.store(false, Ordering::SeqCst);

    // Build lookup: scan_volume_num → pages
    let scan_map: BTreeMap<u32, Vec<ParsedImage>> = snapshot.scan_result.into_iter().collect();

    // Resolve each VolumeEdit into a concrete (volume_num, Vec<ParsedImage>) group.
    let groups: Vec<(u32, Vec<ParsedImage>)> = edits
        .iter()
        .map(|edit| resolve_edit_pages(edit, &scan_map).map(|pages| (edit.volume_num, pages)))
        .collect::<Result<_, _>>()?;

    // Build the conversion plan (apply naming).
    let naming = VolumeNaming {
        name: &options.output_name,
        separator: &options.volume_separator,
        hide_single_volume: options.hide_single_volume,
    };
    let plans = apply_naming(groups, &naming);
    let total_volumes = plans.len() as u32;

    let out_root = if options.create_directory {
        PathBuf::from(&options.output_dir).join(sanitize_filename_component(&options.output_name)?)
    } else {
        PathBuf::from(&options.output_dir)
    };
    tokio::fs::create_dir_all(&out_root)
        .await
        .map_err(|source| ConvertError::CreateOutputDir {
            path: out_root.clone(),
            source,
        })?;

    let encode_opts = EncodeOptions {
        format: options.image_format,
        max_width: options.max_width,
        force_reencode: options.force_reencode,
        clean_tones: options.clean_tones,
        color_enhance: options.color_enhance,
        sharpen: options.sharpen,
    };

    let mut successful = 0u32;
    let mut failed = 0u32;
    let mut conversion_stats = ConversionStats::default();
    let mut outputs = Vec::new();

    for plan in plans {
        if snapshot.cancel.load(Ordering::SeqCst) {
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
            snapshot.source.clone(),
            snapshot.cancel.clone(),
            &app,
        )
        .await;

        let (success, error, output_path) = match result {
            Ok(result) => {
                successful += 1;
                conversion_stats.merge(&result.stats);
                outputs.push(result.output.clone());
                (true, None, Some(result.output.path))
            }
            Err(e) => {
                failed += 1;
                tracing::error!("Volume {} failed: {e}", vol_num);
                (false, Some(e.to_string()), None)
            }
        };
        let _ = VolumeCompleteEvent {
            volume_num: vol_num,
            success,
            error,
            output_path,
        }
        .emit(&app);
    }

    ConversionCompleteEvent {
        successful,
        failed,
        duration_secs: start.elapsed().as_secs_f64(),
        total_pages: conversion_stats.pages,
        input_bytes: conversion_stats.input_bytes,
        output_bytes: conversion_stats.output_bytes,
        passthrough_pages: conversion_stats.passthrough_pages,
        encoded_pages: conversion_stats.encoded_pages,
        fetch_ms: duration_ms(conversion_stats.fetch_time),
        decode_ms: duration_ms(conversion_stats.decode_time),
        transform_ms: duration_ms(conversion_stats.transform_time),
        encode_ms: duration_ms(conversion_stats.encode_time),
        outputs,
    }
    .emit(&app)
    .ok();

    Ok(())
}

fn conversion_state_snapshot(
    state: &State<'_, RwLock<ConvState>>,
) -> ConvertResult<ConversionStateSnapshot> {
    let state = state
        .read()
        .map_err(|e| ConvertError::StateLock(e.to_string()))?;
    Ok(ConversionStateSnapshot {
        scan_result: state
            .scan_result
            .clone()
            .ok_or(ConvertError::MissingScanResult)?,
        source: state.source.clone().ok_or(ConvertError::MissingSource)?,
        cancel: state.cancel.clone(),
    })
}

/// Materialize the final ordered page list for one VolumeEdit by looking each
/// PageEditEntry up in the scan_map (or constructing a custom-path entry).
fn resolve_edit_pages(
    edit: &VolumeEdit,
    scan_map: &BTreeMap<u32, Vec<ParsedImage>>,
) -> ConvertResult<Vec<ParsedImage>> {
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
                validate_custom_image_path(&path)?;
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
    Ok(final_pages)
}

fn validate_custom_image_path(path: &Path) -> ConvertResult<()> {
    if !LocalSource::is_supported_image_path(path) {
        return Err(ConvertError::UnsupportedCustomImage {
            path: path.to_path_buf(),
        });
    }
    let metadata = std::fs::metadata(path).map_err(|source| ConvertError::CustomImageMetadata {
        path: path.to_path_buf(),
        source,
    })?;
    if !metadata.is_file() {
        return Err(ConvertError::CustomImageNotFile {
            path: path.to_path_buf(),
        });
    }
    if metadata.len() > MAX_CUSTOM_IMAGE_BYTES {
        return Err(ConvertError::CustomImageTooLarge {
            path: path.to_path_buf(),
            bytes: metadata.len(),
        });
    }
    Ok(())
}

#[derive(Debug, Default, Clone)]
struct ConversionStats {
    pages: u32,
    input_bytes: u64,
    output_bytes: u64,
    passthrough_pages: u32,
    encoded_pages: u32,
    fetch_time: Duration,
    decode_time: Duration,
    transform_time: Duration,
    encode_time: Duration,
}

#[derive(Debug, Clone)]
struct VolumeConversionResult {
    stats: ConversionStats,
    output: ConversionOutput,
}

struct ConversionStateSnapshot {
    scan_result: Vec<(u32, Vec<ParsedImage>)>,
    source: Arc<LocalSource>,
    cancel: Arc<AtomicBool>,
}

impl ConversionStats {
    fn add_image(&mut self, img: &ProcessedImage) {
        self.pages += 1;
        self.input_bytes += img.stats.input_bytes;
        self.output_bytes += img.stats.output_bytes;
        if img.stats.passthrough {
            self.passthrough_pages += 1;
        } else {
            self.encoded_pages += 1;
        }
        self.fetch_time += duration_from_ms(img.stats.fetch_ms);
        self.decode_time += duration_from_ms(img.stats.decode_ms);
        self.transform_time += duration_from_ms(img.stats.transform_ms);
        self.encode_time += duration_from_ms(img.stats.encode_ms);
    }

    fn merge(&mut self, other: &Self) {
        self.pages += other.pages;
        self.input_bytes += other.input_bytes;
        self.output_bytes += other.output_bytes;
        self.passthrough_pages += other.passthrough_pages;
        self.encoded_pages += other.encoded_pages;
        self.fetch_time += other.fetch_time;
        self.decode_time += other.decode_time;
        self.transform_time += other.transform_time;
        self.encode_time += other.encode_time;
    }
}

fn duration_from_ms(ms: f64) -> Duration {
    Duration::from_secs_f64((ms / 1000.0).max(0.0))
}

fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn estimated_remaining(elapsed: Duration, current: u32, total: u32) -> Option<f64> {
    if current == 0 || total <= current {
        return None;
    }
    let pages_per_sec = current as f64 / elapsed.as_secs_f64().max(0.001);
    Some((total - current) as f64 / pages_per_sec)
}

async fn convert_volume(
    plan: VolumePlan,
    out_root: &Path,
    options: &ConvertOptions,
    encode_opts: EncodeOptions,
    source: Arc<LocalSource>,
    cancel: Arc<AtomicBool>,
    app: &AppHandle,
) -> ConvertResult<VolumeConversionResult> {
    let started = Instant::now();
    let vol_num = plan.volume_num;
    let total = plan.pages.len() as u32;
    let display_name = plan.display_name;
    let mut stats = ConversionStats::default();

    let (tx_parsed, rx_parsed) = tokio::sync::mpsc::channel(256);

    // Feed pages into the pipeline without blocking the encoding loop.
    let mut pages = plan.pages;
    for (i, page) in pages.iter_mut().enumerate() {
        page.page_number = i as f32;
    }
    tokio::spawn(async move {
        for parsed in pages {
            if tx_parsed.send(parsed).await.is_err() {
                break;
            }
        }
    });

    let mut rx_processed =
        start_pipeline_with_cancel(source, rx_parsed, encode_opts, cancel.clone()).await;

    let mut pkg: Box<dyn Generator> = match options.output_format {
        OutputFormat::Cbz => Box::new(CbzGenerator::new()),
        OutputFormat::Epub => Box::new(EpubGenerator::new().with_direction(options.direction)),
        OutputFormat::Raw => Box::new(RawGenerator::new()),
    };

    pkg.init(out_root, &display_name)
        .await
        .map_err(ConvertError::Package)?;
    let output_path = pkg.output_path().ok_or(ConvertError::MissingOutputPath)?;

    let mut pending: BTreeMap<u32, ProcessedImage> = BTreeMap::new();
    let mut next_page = 0u32;
    let mut current = 0u32;
    while let Some(result) = rx_processed.recv().await {
        if cancel.load(Ordering::SeqCst) {
            return Err(ConvertError::Cancelled);
        }
        let img = result.map_err(ConvertError::Pipeline)?;
        current += 1;
        stats.add_image(&img);
        let elapsed = started.elapsed();
        let pages_per_sec = current as f64 / elapsed.as_secs_f64().max(0.001);
        ImageProgressEvent {
            volume_num: vol_num,
            current,
            total,
            elapsed_secs: elapsed.as_secs_f64(),
            pages_per_sec,
            estimated_remaining_secs: estimated_remaining(elapsed, current, total),
            input_bytes: stats.input_bytes,
            output_bytes: stats.output_bytes,
            passthrough_pages: stats.passthrough_pages,
            encoded_pages: stats.encoded_pages,
            fetch_ms: duration_ms(stats.fetch_time),
            decode_ms: duration_ms(stats.decode_time),
            transform_ms: duration_ms(stats.transform_time),
            encode_ms: duration_ms(stats.encode_time),
        }
        .emit(app)
        .ok();
        pending.insert(page_index(&img), img);
        while let Some(img) = pending.remove(&next_page) {
            pkg.add_page(img).await.map_err(ConvertError::Package)?;
            next_page += 1;
        }
    }
    for (_, img) in pending {
        pkg.add_page(img).await.map_err(ConvertError::Package)?;
    }

    pkg.finalize().await.map_err(ConvertError::Package)?;
    Ok(VolumeConversionResult {
        stats,
        output: ConversionOutput {
            volume_num: vol_num,
            volume_name: display_name,
            path: output_path.to_string_lossy().into_owned(),
        },
    })
}

fn page_index(img: &ProcessedImage) -> u32 {
    img.parsed_data.page_number as u32
}
