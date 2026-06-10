use crate::{
    encode::{convert_to_avif, convert_to_webp},
    error::{ProcessorError, Result as ProcessorResult},
    retry::with_retries,
    transform::{TransformOptions, TransformPipeline},
};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use thasia_core::prelude::{ImageFormat, ParsedImage, ProcessedImage, ProcessingStats};
use thasia_source::prelude::{FetchedImage, Source};
use tokio::sync::mpsc;
use tracing::warn;

#[derive(Debug, Clone, Copy)]
pub struct EncodeOptions {
    pub format: ImageFormat,
    pub force_reencode: bool,
    pub transform: TransformOptions,
}

struct RawImage {
    parsed: ParsedImage,
    bytes: FetchedImage,
    original_ext: String,
    fetch_duration: Duration,
}

struct EncodedImage {
    data: Vec<u8>,
    ext: String,
    width: u32,
    height: u32,
    stats: ProcessingStats,
}

/// Two-stage pipeline:
///
/// Stage 1 — async fetch (tokio, bounded concurrency via semaphore)
///   Parsed images arrive → fetch raw bytes with retry → send to raw channel.
///
/// Stage 2 — Rayon encode (dedicated std thread + rayon::scope)
///   Receives (ParsedImage, raw bytes) → encodes in parallel across all CPU cores
///   using Rayon's work-stealing, with no tokio overhead on the hot path.
///
/// This matches Palaxy's approach: N cores are always busy encoding; no thread
/// thrashing from spawning one blocking task per image.
pub async fn start_pipeline_with_cancel<S: Source + 'static>(
    source: Arc<S>,
    parsed_rx: mpsc::Receiver<ParsedImage>,
    options: EncodeOptions,
    cancel: Arc<AtomicBool>,
) -> mpsc::Receiver<ProcessorResult<ProcessedImage>> {
    start_pipeline_inner(source, parsed_rx, options, Some(cancel)).await
}

async fn start_pipeline_inner<S: Source + 'static>(
    source: Arc<S>,
    mut parsed_rx: mpsc::Receiver<ParsedImage>,
    options: EncodeOptions,
    cancel: Option<Arc<AtomicBool>>,
) -> mpsc::Receiver<ProcessorResult<ProcessedImage>> {
    let (result_tx, result_rx) = mpsc::channel(64);

    let num_cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    // Bounded channel: async fetch → sync Rayon encode.
    // Buffer = num_cores * 2 lets the fetch stage stay slightly ahead of encoding.
    let (raw_tx, raw_rx) = mpsc::channel::<RawImage>(num_cores * 2);

    // ── Stage 1: async fetch ──────────────────────────────────────────────────
    // Source advertises its preferred concurrency; local FS sources stay small
    // to avoid disk thrashing while archive/network sources can ask for more.
    let fetch_concurrency = source.fetch_concurrency_hint();
    let fetch_sem = Arc::new(tokio::sync::Semaphore::new(fetch_concurrency));
    let cancel_fetch = cancel.clone();
    tokio::spawn(async move {
        while let Some(parsed) = parsed_rx.recv().await {
            if is_cancelled(&cancel_fetch) {
                break;
            }
            let Ok(permit) = fetch_sem.clone().acquire_owned().await else {
                break;
            };
            let source = source.clone();
            let raw_tx = raw_tx.clone();
            let cancel = cancel_fetch.clone();

            tokio::spawn(async move {
                let _permit = permit;
                if is_cancelled(&cancel) {
                    return;
                }
                let ext = parsed
                    .source
                    .absolute_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("jpg")
                    .to_lowercase();

                let label = || parsed.source.relative_path.clone();
                let fetch_start = Instant::now();
                match with_retries(&label(), || async {
                    source
                        .fetch(&parsed.source)
                        .await
                        .map_err(backoff::Error::transient)
                })
                .await
                {
                    Ok(bytes) => {
                        if is_cancelled(&cancel) {
                            return;
                        }
                        let _ = raw_tx
                            .send(RawImage {
                                parsed,
                                bytes,
                                original_ext: ext,
                                fetch_duration: fetch_start.elapsed(),
                            })
                            .await;
                    }
                    Err(e) => warn!("Skipping {} (fetch failed): {}", label(), e),
                }
            });
        }
        // raw_tx (outer clone) drops here; channel closes when all spawned fetch
        // tasks finish and drop their own clones.
    });

    // ── Stage 2: Rayon encode ─────────────────────────────────────────────────
    // Run on a plain std thread so the coordinator doesn't occupy a Rayon worker.
    let result_tx_enc = result_tx;
    let opts = options;
    let cancel_encode = cancel;
    std::thread::spawn(move || {
        let mut raw_rx = raw_rx;

        // Bound in-flight rayon tasks so the closure queue never accumulates
        // thousands of raw image byte buffers when the result consumer is slow.
        // We use a sync_channel as a counting semaphore: pre-fill N tokens,
        // take one before spawning, return one after encoding (before blocking
        // on result_tx so we don't deadlock when result_tx is at capacity).
        let max_in_flight = encode_in_flight_limit(num_cores);
        let (slot_tx, slot_rx) = std::sync::mpsc::sync_channel::<()>(max_in_flight);
        for _ in 0..max_in_flight {
            let _ = slot_tx.send(());
        }

        rayon::scope(move |s| {
            while let Some(raw) = raw_rx.blocking_recv() {
                if is_cancelled(&cancel_encode) {
                    break;
                }
                slot_rx.recv().ok(); // acquire a slot; blocks when all are taken
                let result_tx = result_tx_enc.clone();
                let slot_tx = slot_tx.clone();
                let cancel = cancel_encode.clone();
                s.spawn(move |_| {
                    if is_cancelled(&cancel) {
                        let _ = slot_tx.send(());
                        return;
                    }
                    let result: ProcessorResult<ProcessedImage> =
                        encode_image(raw.bytes, &raw.original_ext, opts, raw.fetch_duration).map(
                            |encoded| ProcessedImage {
                                parsed_data: raw.parsed,
                                image_data: encoded.data,
                                ext: encoded.ext,
                                width: encoded.width,
                                height: encoded.height,
                                stats: encoded.stats,
                            },
                        );
                    // Release slot before blocking_send so the coordinator can
                    // keep the encode pool fed even if result_tx is temporarily full.
                    let _ = slot_tx.send(());
                    result_tx.blocking_send(result).ok();
                });
            }
            // scope waits here for all in-flight Rayon tasks to finish
        });
    });

    result_rx
}

#[inline]
fn is_cancelled(cancel: &Option<Arc<AtomicBool>>) -> bool {
    cancel
        .as_ref()
        .map(|cancel| cancel.load(Ordering::Relaxed))
        .unwrap_or(false)
}

fn encode_image(
    bytes: FetchedImage,
    original_ext: &str,
    opts: EncodeOptions,
    fetch_duration: Duration,
) -> ProcessorResult<EncodedImage> {
    let total_start = Instant::now();
    let input_bytes = bytes.len() as u64;
    if opts.format == ImageFormat::Original {
        // Header-only dimension read — orders of magnitude faster than a full
        // decode, which is critical for the "no re-encoding" fast path. Keep
        // this best-effort so supported source extensions can still pass
        // through even when the `image` crate cannot decode that format.
        let (w, h) = image_dimensions_best_effort(bytes.as_ref());
        let bytes = bytes.into_vec();
        let output_bytes = bytes.len() as u64;
        return Ok(EncodedImage {
            data: bytes,
            ext: original_ext.to_string(),
            width: w,
            height: h,
            stats: ProcessingStats {
                input_bytes,
                output_bytes,
                passthrough: true,
                fetch_ms: duration_ms(fetch_duration),
                total_ms: duration_ms(total_start.elapsed()),
                ..ProcessingStats::default()
            },
        });
    }

    if can_passthrough(original_ext, opts) {
        let (w, h) = image_dimensions_best_effort(bytes.as_ref());
        let bytes = bytes.into_vec();
        let output_bytes = bytes.len() as u64;
        return Ok(EncodedImage {
            data: bytes,
            ext: target_extension(opts.format).to_string(),
            width: w,
            height: h,
            stats: ProcessingStats {
                input_bytes,
                output_bytes,
                passthrough: true,
                fetch_ms: duration_ms(fetch_duration),
                total_ms: duration_ms(total_start.elapsed()),
                ..ProcessingStats::default()
            },
        });
    }

    let decode_start = Instant::now();
    let mut img = decode_image(bytes.as_ref(), original_ext)?;
    let decode_ms = duration_ms(decode_start.elapsed());
    // Drop the compressed bytes now — AVIF/WebP encoding is the hot path and
    // holding the JPEG buffer alongside the decoded pixel buffer wastes memory.
    drop(bytes);

    let transform_start = Instant::now();
    TransformPipeline::new(opts.transform).apply(&mut img);
    let transform_ms = duration_ms(transform_start.elapsed());

    let (w, h) = (img.width(), img.height());

    let encode_start = Instant::now();
    let (data, ext) = match opts.format {
        ImageFormat::Avif => (convert_to_avif(&img)?, "avif".to_string()),
        ImageFormat::Webp => (convert_to_webp(&img)?, "webp".to_string()),
        ImageFormat::Original => {
            return Err(ProcessorError::UnsupportedFormat(
                "original output can only be used via passthrough",
            ));
        }
    };
    let encode_ms = duration_ms(encode_start.elapsed());
    let output_bytes = data.len() as u64;

    Ok(EncodedImage {
        data,
        ext,
        width: w,
        height: h,
        stats: ProcessingStats {
            input_bytes,
            output_bytes,
            passthrough: false,
            fetch_ms: duration_ms(fetch_duration),
            decode_ms,
            transform_ms,
            encode_ms,
            total_ms: duration_ms(total_start.elapsed()),
        },
    })
}

fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn can_passthrough(original_ext: &str, opts: EncodeOptions) -> bool {
    if opts.force_reencode || opts.transform.disables_passthrough() {
        return false;
    }
    normalize_ext(original_ext) == target_extension(opts.format)
}

fn target_extension(format: ImageFormat) -> &'static str {
    match format {
        ImageFormat::Avif => "avif",
        ImageFormat::Webp => "webp",
        ImageFormat::Original => "",
    }
}

fn normalize_ext(ext: &str) -> &str {
    match ext.trim_start_matches('.').to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" => "jpg",
        "avif" => "avif",
        "webp" => "webp",
        "png" => "png",
        _ => "",
    }
}

fn image_dimensions_best_effort(bytes: &[u8]) -> (u32, u32) {
    image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()
        .and_then(|reader| reader.into_dimensions().ok())
        .unwrap_or((0, 0))
}

fn encode_in_flight_limit(num_cores: usize) -> usize {
    num_cores.saturating_mul(4).max(1)
}

fn decode_image(bytes: &[u8], original_ext: &str) -> ProcessorResult<image::DynamicImage> {
    if normalize_ext(original_ext) == "avif" {
        return decode_avif(bytes);
    }

    image::load_from_memory(bytes).map_err(ProcessorError::decode)
}

fn decode_avif(bytes: &[u8]) -> ProcessorResult<image::DynamicImage> {
    use avif_decode::{Decoder, Image};
    use image::{DynamicImage, ImageBuffer};

    let decoded = Decoder::from_avif(bytes)
        .and_then(Decoder::to_image)
        .map_err(ProcessorError::avif_decode)?;

    match decoded {
        Image::Rgb8(img) => {
            let (pixels, width, height) = img.into_contiguous_buf();
            let raw = pixels
                .into_iter()
                .flat_map(|px| [px.r, px.g, px.b])
                .collect::<Vec<_>>();
            ImageBuffer::from_vec(width as u32, height as u32, raw)
                .map(DynamicImage::ImageRgb8)
                .ok_or(ProcessorError::InvalidBuffer("AVIF RGB"))
        }
        Image::Rgba8(img) => {
            let (pixels, width, height) = img.into_contiguous_buf();
            let raw = pixels
                .into_iter()
                .flat_map(|px| [px.r, px.g, px.b, px.a])
                .collect::<Vec<_>>();
            ImageBuffer::from_vec(width as u32, height as u32, raw)
                .map(DynamicImage::ImageRgba8)
                .ok_or(ProcessorError::InvalidBuffer("AVIF RGBA"))
        }
        Image::Gray8(img) => {
            let (pixels, width, height) = img.into_contiguous_buf();
            let raw = pixels.into_iter().map(|px| px.value()).collect::<Vec<_>>();
            ImageBuffer::from_vec(width as u32, height as u32, raw)
                .map(DynamicImage::ImageLuma8)
                .ok_or(ProcessorError::InvalidBuffer("AVIF gray"))
        }
        Image::Rgb16(img) => {
            let (pixels, width, height) = img.into_contiguous_buf();
            let raw = pixels
                .into_iter()
                .flat_map(|px| [px.r, px.g, px.b])
                .collect::<Vec<_>>();
            ImageBuffer::from_vec(width as u32, height as u32, raw)
                .map(DynamicImage::ImageRgb16)
                .ok_or(ProcessorError::InvalidBuffer("AVIF RGB16"))
        }
        Image::Rgba16(img) => {
            let (pixels, width, height) = img.into_contiguous_buf();
            let raw = pixels
                .into_iter()
                .flat_map(|px| [px.r, px.g, px.b, px.a])
                .collect::<Vec<_>>();
            ImageBuffer::from_vec(width as u32, height as u32, raw)
                .map(DynamicImage::ImageRgba16)
                .ok_or(ProcessorError::InvalidBuffer("AVIF RGBA16"))
        }
        Image::Gray16(img) => {
            let (pixels, width, height) = img.into_contiguous_buf();
            let raw = pixels.into_iter().map(|px| px.value()).collect::<Vec<_>>();
            ImageBuffer::from_vec(width as u32, height as u32, raw)
                .map(DynamicImage::ImageLuma16)
                .ok_or(ProcessorError::InvalidBuffer("AVIF gray16"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use thasia_core::prelude::{ColorEnhanceMode, SharpenMode};

    fn encode_options(format: ImageFormat) -> EncodeOptions {
        EncodeOptions {
            format,
            force_reencode: false,
            transform: TransformOptions::default(),
        }
    }

    #[test]
    fn passthrough_same_target_format_when_not_forced() {
        let opts = encode_options(ImageFormat::Avif);
        assert!(can_passthrough("avif", opts));
    }

    #[test]
    fn force_reencode_disables_passthrough() {
        let opts = EncodeOptions {
            force_reencode: true,
            ..encode_options(ImageFormat::Avif)
        };
        assert!(!can_passthrough("avif", opts));
    }

    #[test]
    fn resize_disables_passthrough() {
        let opts = EncodeOptions {
            transform: TransformOptions {
                max_width: Some(1920),
                ..TransformOptions::default()
            },
            ..encode_options(ImageFormat::Webp)
        };
        assert!(!can_passthrough("webp", opts));
    }

    #[test]
    fn cleanup_disables_passthrough() {
        let opts = EncodeOptions {
            transform: TransformOptions {
                clean_tones: true,
                ..TransformOptions::default()
            },
            ..encode_options(ImageFormat::Avif)
        };
        assert!(!can_passthrough("avif", opts));
    }

    #[test]
    fn color_enhance_disables_passthrough() {
        let opts = EncodeOptions {
            transform: TransformOptions {
                color_enhance: ColorEnhanceMode::Mild,
                ..TransformOptions::default()
            },
            ..encode_options(ImageFormat::Avif)
        };
        assert!(!can_passthrough("avif", opts));
    }

    #[test]
    fn sharpen_disables_passthrough() {
        let opts = EncodeOptions {
            transform: TransformOptions {
                sharpen: SharpenMode::Mild,
                ..TransformOptions::default()
            },
            ..encode_options(ImageFormat::Webp)
        };
        assert!(!can_passthrough("webp", opts));
    }

    #[test]
    fn transform_options_identify_passthrough_compatibility() {
        assert!(!TransformOptions::default().disables_passthrough());
        assert!(
            TransformOptions {
                sharpen: SharpenMode::Mild,
                ..TransformOptions::default()
            }
            .disables_passthrough()
        );
    }

    #[test]
    fn in_flight_limit_scales_with_cores() {
        assert_eq!(encode_in_flight_limit(1), 4);
        assert_eq!(encode_in_flight_limit(4), 16);
    }

    #[test]
    fn in_flight_limit_never_zero() {
        assert_eq!(encode_in_flight_limit(0), 1);
    }
}
