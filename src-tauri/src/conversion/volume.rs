use super::{ConvertError, ConvertResult};
use crate::events::{ConversionOutput, ImageProgressEvent};
use crate::state::ConvertOptions;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::AppHandle;
use tauri_specta::Event;
use thasia_core::prelude::{OutputFormat, ProcessedImage, VolumePlan};
use thasia_packager::{CbzGenerator, EpubGenerator, Generator, RawGenerator};
use thasia_processor::{EncodeOptions, start_pipeline_with_cancel};
use thasia_source::LocalSource;

#[derive(Debug, Default, Clone)]
pub(crate) struct ConversionStats {
    pub(crate) pages: u32,
    pub(crate) input_bytes: u64,
    pub(crate) output_bytes: u64,
    pub(crate) passthrough_pages: u32,
    pub(crate) encoded_pages: u32,
    pub(crate) fetch_time: Duration,
    pub(crate) decode_time: Duration,
    pub(crate) transform_time: Duration,
    pub(crate) encode_time: Duration,
}

#[derive(Debug, Clone)]
pub(crate) struct VolumeConversionResult {
    pub(crate) stats: ConversionStats,
    pub(crate) output: ConversionOutput,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PackageSelection {
    output_format: OutputFormat,
    direction: thasia_core::prelude::Direction,
}

impl From<&ConvertOptions> for PackageSelection {
    fn from(options: &ConvertOptions) -> Self {
        Self {
            output_format: options.output_format,
            direction: options.direction,
        }
    }
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

    pub(crate) fn merge(&mut self, other: &Self) {
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

pub(crate) async fn convert_volume(
    plan: VolumePlan,
    out_root: &Path,
    package: PackageSelection,
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
    let mut pkg = package_generator(package);

    pkg.init(out_root, &display_name).await?;
    let output_path = pkg.output_path().ok_or(ConvertError::MissingOutputPath)?;

    let mut pending = BTreeMap::new();
    let mut next_page = 0u32;
    let mut current = 0u32;
    while let Some(result) = rx_processed.recv().await {
        if cancel.load(Ordering::SeqCst) {
            return Err(ConvertError::Cancelled);
        }
        let img = result?;
        current += 1;
        stats.add_image(&img);
        emit_image_progress(app, vol_num, current, total, started.elapsed(), &stats);
        pending.insert(page_index(&img), img);
        while let Some(img) = pending.remove(&next_page) {
            pkg.add_page(img).await?;
            next_page += 1;
        }
    }
    if cancel.load(Ordering::SeqCst) {
        return Err(ConvertError::Cancelled);
    }
    if current != total {
        return Err(ConvertError::IncompleteVolume {
            volume_num: vol_num,
            expected: total,
            actual: current,
        });
    }
    for (_, img) in pending {
        pkg.add_page(img).await?;
    }

    pkg.finalize().await?;
    Ok(VolumeConversionResult {
        stats,
        output: ConversionOutput {
            volume_num: vol_num,
            volume_name: display_name,
            path: output_path.to_string_lossy().into_owned(),
        },
    })
}

pub(crate) fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn package_generator(package: PackageSelection) -> Box<dyn Generator> {
    match package.output_format {
        OutputFormat::Cbz => Box::new(CbzGenerator::new()),
        OutputFormat::Epub => Box::new(EpubGenerator::new().with_direction(package.direction)),
        OutputFormat::Raw => Box::new(RawGenerator::new()),
    }
}

fn emit_image_progress(
    app: &AppHandle,
    volume_num: u32,
    current: u32,
    total: u32,
    elapsed: Duration,
    stats: &ConversionStats,
) {
    let pages_per_sec = current as f64 / elapsed.as_secs_f64().max(0.001);
    ImageProgressEvent {
        volume_num,
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
}

fn duration_from_ms(ms: f64) -> Duration {
    Duration::from_secs_f64((ms / 1000.0).max(0.0))
}

fn estimated_remaining(elapsed: Duration, current: u32, total: u32) -> Option<f64> {
    if current == 0 || total <= current {
        return None;
    }
    let pages_per_sec = current as f64 / elapsed.as_secs_f64().max(0.001);
    Some((total - current) as f64 / pages_per_sec)
}

fn page_index(img: &ProcessedImage) -> u32 {
    img.parsed_data.page_number as u32
}
