use crate::{
    encode::{convert_to_avif, convert_to_webp},
    retry::with_retries,
};
use std::sync::Arc;
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
    mut parsed_rx: mpsc::Receiver<ParsedImage>,
    options: EncodeOptions,
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
    tokio::spawn(async move {
        while let Some(parsed) = parsed_rx.recv().await {
            let permit = fetch_sem
                .clone()
                .acquire_owned()
                .await
                .expect("fetch semaphore was unexpectedly closed");
            let source = source.clone();
            let raw_tx = raw_tx.clone();

            tokio::spawn(async move {
                let _permit = permit;
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
    std::thread::spawn(move || {
        let mut raw_rx = raw_rx;

        // Bound in-flight rayon tasks so the closure queue never accumulates
        // thousands of raw image byte buffers when the result consumer is slow.
        // We use a sync_channel as a counting semaphore: pre-fill N tokens,
        // take one before spawning, return one after encoding (before blocking
        // on result_tx so we don't deadlock when result_tx is at capacity).
        let max_in_flight = num_cores * 4;
        let (slot_tx, slot_rx) = std::sync::mpsc::sync_channel::<()>(max_in_flight);
        for _ in 0..max_in_flight {
            let _ = slot_tx.send(());
        }

        rayon::scope(move |s| {
            while let Some((parsed, bytes, ext)) = raw_rx.blocking_recv() {
                slot_rx.recv().ok(); // acquire a slot; blocks when all are taken
                let result_tx = result_tx_enc.clone();
                let slot_tx = slot_tx.clone();
                s.spawn(move |_| {
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

fn encode_image(
    bytes: Vec<u8>,
    original_ext: &str,
    opts: EncodeOptions,
) -> thasia_core::Result<(Vec<u8>, String, u32, u32)> {
    if opts.format == ImageFormat::Original {
        // Header-only dimension read — orders of magnitude faster than a full
        // decode, which is critical for the "no re-encoding" fast path.
        let (w, h) = image::ImageReader::new(std::io::Cursor::new(&bytes))
            .with_guessed_format()
            .map_err(|e| ThasiaError::Fatal(format!("Failed to probe image: {e}")))?
            .into_dimensions()
            .map_err(|e| ThasiaError::Fatal(format!("Failed to read dimensions: {e}")))?;
        return Ok((bytes, original_ext.to_string(), w, h));
    }

    let mut img = image::load_from_memory(&bytes)
        .map_err(|e| ThasiaError::Fatal(format!("Failed to load image: {e}")))?;
    // Drop the compressed bytes now — AVIF/WebP encoding is the hot path and
    // holding the JPEG buffer alongside the decoded pixel buffer wastes memory.
    drop(bytes);

    if let Some(max_w) = opts.max_width {
        let (w, h) = (img.width(), img.height());
        if w > max_w {
            let scale = max_w as f64 / w as f64;
            img = img.resize(
                max_w,
                (h as f64 * scale) as u32,
                image::imageops::FilterType::Lanczos3,
            );
        }
    }

    let (w, h) = (img.width(), img.height());

    let (data, ext) = match opts.format {
        ImageFormat::Avif => (convert_to_avif(&img)?, "avif".to_string()),
        ImageFormat::Webp => (convert_to_webp(&img)?, "webp".to_string()),
        ImageFormat::Original => unreachable!(),
    };

    Ok((data, ext, w, h))
}
