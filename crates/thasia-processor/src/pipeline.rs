use crate::{
    encode::{convert_to_avif, convert_to_webp},
    retry::with_retries,
    transform::TransformPipeline,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use thasia_core::{
    ThasiaError,
    models::{ImageFormat, ParsedImage, ProcessedImage},
};
use thasia_source::Source;
use tokio::sync::mpsc;
use tracing::warn;

#[derive(Debug, Clone, Copy)]
pub struct EncodeOptions {
    pub format: ImageFormat,
    pub max_width: Option<u32>,
    pub force_reencode: bool,
    pub clean_tones: bool,
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
pub async fn start_pipeline<S: Source + 'static>(
    source: Arc<S>,
    parsed_rx: mpsc::Receiver<ParsedImage>,
    options: EncodeOptions,
) -> mpsc::Receiver<thasia_core::Result<ProcessedImage>> {
    start_pipeline_inner(source, parsed_rx, options, None).await
}

pub async fn start_pipeline_with_cancel<S: Source + 'static>(
    source: Arc<S>,
    parsed_rx: mpsc::Receiver<ParsedImage>,
    options: EncodeOptions,
    cancel: Arc<AtomicBool>,
) -> mpsc::Receiver<thasia_core::Result<ProcessedImage>> {
    start_pipeline_inner(source, parsed_rx, options, Some(cancel)).await
}

async fn start_pipeline_inner<S: Source + 'static>(
    source: Arc<S>,
    mut parsed_rx: mpsc::Receiver<ParsedImage>,
    options: EncodeOptions,
    cancel: Option<Arc<AtomicBool>>,
) -> mpsc::Receiver<thasia_core::Result<ProcessedImage>> {
    let (result_tx, result_rx) = mpsc::channel(64);

    let num_cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    // Bounded channel: async fetch → sync Rayon encode.
    // Buffer = num_cores * 2 lets the fetch stage stay slightly ahead of encoding.
    let (raw_tx, raw_rx) = mpsc::channel::<(ParsedImage, Vec<u8>, String)>(num_cores * 2);

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
            let permit = fetch_sem
                .clone()
                .acquire_owned()
                .await
                .expect("fetch semaphore was unexpectedly closed");
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
                        let _ = raw_tx.send((parsed, bytes, ext)).await;
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
            while let Some((parsed, bytes, ext)) = raw_rx.blocking_recv() {
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
                    let result = encode_image(bytes, &ext, opts).map(|(data, enc_ext, w, h)| {
                        ProcessedImage {
                            parsed_data: parsed,
                            image_data: data,
                            ext: enc_ext,
                            width: w,
                            height: h,
                        }
                    });
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
    bytes: Vec<u8>,
    original_ext: &str,
    opts: EncodeOptions,
) -> thasia_core::Result<(Vec<u8>, String, u32, u32)> {
    if opts.format == ImageFormat::Original {
        // Header-only dimension read — orders of magnitude faster than a full
        // decode, which is critical for the "no re-encoding" fast path. Keep
        // this best-effort so supported source extensions can still pass
        // through even when the `image` crate cannot decode that format.
        let (w, h) = image_dimensions_best_effort(&bytes);
        return Ok((bytes, original_ext.to_string(), w, h));
    }

    if can_passthrough(original_ext, opts) {
        let (w, h) = image_dimensions_best_effort(&bytes);
        return Ok((bytes, target_extension(opts.format).to_string(), w, h));
    }

    let mut img = decode_image(&bytes, original_ext)?;
    // Drop the compressed bytes now — AVIF/WebP encoding is the hot path and
    // holding the JPEG buffer alongside the decoded pixel buffer wastes memory.
    drop(bytes);

    TransformPipeline::new(opts.max_width, opts.clean_tones).apply(&mut img);

    let (w, h) = (img.width(), img.height());

    let (data, ext) = match opts.format {
        ImageFormat::Avif => (convert_to_avif(&img)?, "avif".to_string()),
        ImageFormat::Webp => (convert_to_webp(&img)?, "webp".to_string()),
        ImageFormat::Original => unreachable!(),
    };

    Ok((data, ext, w, h))
}

fn can_passthrough(original_ext: &str, opts: EncodeOptions) -> bool {
    if opts.force_reencode || opts.max_width.is_some() || opts.clean_tones {
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

fn decode_image(bytes: &[u8], original_ext: &str) -> thasia_core::Result<image::DynamicImage> {
    if normalize_ext(original_ext) == "avif" {
        return decode_avif(bytes);
    }

    image::load_from_memory(bytes)
        .map_err(|e| ThasiaError::Fatal(format!("Failed to load image: {e}")))
}

fn decode_avif(bytes: &[u8]) -> thasia_core::Result<image::DynamicImage> {
    use avif_decode::{Decoder, Image};
    use image::{DynamicImage, ImageBuffer};

    let decoded = Decoder::from_avif(bytes)
        .and_then(Decoder::to_image)
        .map_err(|e| ThasiaError::Fatal(format!("AVIF decode failed: {e}")))?;

    match decoded {
        Image::Rgb8(img) => {
            let (pixels, width, height) = img.into_contiguous_buf();
            let raw = pixels
                .into_iter()
                .flat_map(|px| [px.r, px.g, px.b])
                .collect::<Vec<_>>();
            ImageBuffer::from_vec(width as u32, height as u32, raw)
                .map(DynamicImage::ImageRgb8)
                .ok_or_else(|| ThasiaError::Fatal("AVIF RGB buffer had invalid length".into()))
        }
        Image::Rgba8(img) => {
            let (pixels, width, height) = img.into_contiguous_buf();
            let raw = pixels
                .into_iter()
                .flat_map(|px| [px.r, px.g, px.b, px.a])
                .collect::<Vec<_>>();
            ImageBuffer::from_vec(width as u32, height as u32, raw)
                .map(DynamicImage::ImageRgba8)
                .ok_or_else(|| ThasiaError::Fatal("AVIF RGBA buffer had invalid length".into()))
        }
        Image::Gray8(img) => {
            let (pixels, width, height) = img.into_contiguous_buf();
            let raw = pixels.into_iter().map(|px| px.value()).collect::<Vec<_>>();
            ImageBuffer::from_vec(width as u32, height as u32, raw)
                .map(DynamicImage::ImageLuma8)
                .ok_or_else(|| ThasiaError::Fatal("AVIF gray buffer had invalid length".into()))
        }
        Image::Rgb16(img) => {
            let (pixels, width, height) = img.into_contiguous_buf();
            let raw = pixels
                .into_iter()
                .flat_map(|px| [px.r, px.g, px.b])
                .collect::<Vec<_>>();
            ImageBuffer::from_vec(width as u32, height as u32, raw)
                .map(DynamicImage::ImageRgb16)
                .ok_or_else(|| ThasiaError::Fatal("AVIF RGB16 buffer had invalid length".into()))
        }
        Image::Rgba16(img) => {
            let (pixels, width, height) = img.into_contiguous_buf();
            let raw = pixels
                .into_iter()
                .flat_map(|px| [px.r, px.g, px.b, px.a])
                .collect::<Vec<_>>();
            ImageBuffer::from_vec(width as u32, height as u32, raw)
                .map(DynamicImage::ImageRgba16)
                .ok_or_else(|| ThasiaError::Fatal("AVIF RGBA16 buffer had invalid length".into()))
        }
        Image::Gray16(img) => {
            let (pixels, width, height) = img.into_contiguous_buf();
            let raw = pixels.into_iter().map(|px| px.value()).collect::<Vec<_>>();
            ImageBuffer::from_vec(width as u32, height as u32, raw)
                .map(DynamicImage::ImageLuma16)
                .ok_or_else(|| ThasiaError::Fatal("AVIF gray16 buffer had invalid length".into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passthrough_same_target_format_when_not_forced() {
        let opts = EncodeOptions {
            format: ImageFormat::Avif,
            max_width: None,
            force_reencode: false,
            clean_tones: false,
        };
        assert!(can_passthrough("avif", opts));
    }

    #[test]
    fn force_reencode_disables_passthrough() {
        let opts = EncodeOptions {
            format: ImageFormat::Avif,
            max_width: None,
            force_reencode: true,
            clean_tones: false,
        };
        assert!(!can_passthrough("avif", opts));
    }

    #[test]
    fn resize_disables_passthrough() {
        let opts = EncodeOptions {
            format: ImageFormat::Webp,
            max_width: Some(1920),
            force_reencode: false,
            clean_tones: false,
        };
        assert!(!can_passthrough("webp", opts));
    }

    #[test]
    fn cleanup_disables_passthrough() {
        let opts = EncodeOptions {
            format: ImageFormat::Avif,
            max_width: None,
            force_reencode: false,
            clean_tones: true,
        };
        assert!(!can_passthrough("avif", opts));
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
