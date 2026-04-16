use crate::{
    encode::{convert_to_avif, convert_to_webp},
    retry::with_retries,
};
use std::sync::Arc;
use thasia_core::{
    models::{ParsedImage, ProcessedImage},
    ThasiaError,
};
use thasia_source::Source;
use tokio::sync::mpsc;
use tracing::{error, warn};

pub struct EncodeOptions {
    /// "avif", "webp", or "original"
    pub format: String,
    /// Optional max width; images wider than this are downscaled preserving aspect ratio.
    pub max_width: Option<u32>,
}

pub async fn start_pipeline<S: Source + Send + Sync + 'static>(
    source: Arc<S>,
    mut parsed_rx: mpsc::Receiver<ParsedImage>,
    options: Arc<EncodeOptions>,
) -> mpsc::Receiver<thasia_core::Result<ProcessedImage>> {
    let (tx, rx) = mpsc::channel(64);

    tokio::spawn(async move {
        while let Some(parsed) = parsed_rx.recv().await {
            let tx = tx.clone();
            let source = source.clone();
            let opts = options.clone();

            tokio::spawn(async move {
                let path = parsed.source.relative_path.clone();

                // Level 1: Fetch with retries
                let raw_bytes = match with_retries(&format!("fetch:{path}"), || async {
                    source
                        .fetch(&parsed.source)
                        .await
                        .map_err(backoff::Error::transient)
                })
                .await
                {
                    Ok(b) => b,
                    Err(e) => {
                        warn!("Skipping page (fetch failed): {}", e);
                        return;
                    }
                };

                // Level 2: Encode with retries (CPU-bound, offloaded via spawn_blocking)
                let process_res = with_retries(&format!("encode:{path}"), || async {
                    let bytes = raw_bytes.clone();
                    let opts = opts.clone();

                    tokio::task::spawn_blocking(move || encode_image(bytes, &opts))
                        .await
                        .unwrap()
                        .map_err(backoff::Error::transient)
                })
                .await;

                match process_res {
                    Ok((image_data, ext, width, height)) => {
                        let _ = tx
                            .send(Ok(ProcessedImage {
                                parsed_data: parsed,
                                image_data,
                                ext,
                                width,
                                height,
                            }))
                            .await;
                    }
                    Err(e) => {
                        error!("Failed to encode {}: {}", path, e);
                    }
                }
            });
        }
    });

    rx
}

fn encode_image(
    bytes: Vec<u8>,
    opts: &EncodeOptions,
) -> thasia_core::Result<(Vec<u8>, String, u32, u32)> {
    let mut img = image::load_from_memory(&bytes)
        .map_err(|e| ThasiaError::Fatal(format!("Failed to load image: {e}")))?;

    // Downscale if needed
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

    let (data, ext) = match opts.format.as_str() {
        "avif" => (convert_to_avif(&img)?, "avif".to_string()),
        "webp" => (convert_to_webp(&img)?, "webp".to_string()),
        _ => {
            // Original passthrough
            (bytes, "jpg".to_string())
        }
    };

    Ok((data, ext, w, h))
}
