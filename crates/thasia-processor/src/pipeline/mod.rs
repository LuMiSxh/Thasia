mod encode;

use crate::{error::Result as ProcessorResult, retry::with_retries, transform::TransformOptions};
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
    });

    // ── Stage 2: Rayon encode ─────────────────────────────────────────────────
    let result_tx_enc = result_tx;
    let opts = options;
    let cancel_encode = cancel;
    std::thread::spawn(move || {
        let mut raw_rx = raw_rx;

        // Counting semaphore via sync_channel: N tokens pre-filled, take one
        // before spawning, return one after encoding to avoid accumulating huge
        // byte buffers when the result consumer is slow.
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
                slot_rx.recv().ok();
                let result_tx = result_tx_enc.clone();
                let slot_tx = slot_tx.clone();
                let cancel = cancel_encode.clone();
                s.spawn(move |_| {
                    if is_cancelled(&cancel) {
                        let _ = slot_tx.send(());
                        return;
                    }
                    let results = encode::encode_images(raw, opts);
                    let _ = slot_tx.send(());
                    for result in results {
                        result_tx.blocking_send(result).ok();
                    }
                });
            }
        });
    });

    result_rx
}

#[inline]
fn is_cancelled(cancel: &Option<Arc<AtomicBool>>) -> bool {
    cancel
        .as_ref()
        .map(|c| c.load(Ordering::Relaxed))
        .unwrap_or(false)
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
