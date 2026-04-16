//! WebP encoding with 5-tier adaptive quality tuning.
//! Ported from the Palaxy project.

use super::constants::*;
use super::grayscale::is_grayscale;
use thasia_core::ThasiaError;
use tracing::trace;

pub fn auto_tune_webp(img: &image::DynamicImage, gray: bool) -> f32 {
    let pixels = img.width() as u64 * img.height() as u64;

    let mut quality = if pixels < WEBP_TINY_THRESHOLD {
        WEBP_QUALITY_TINY
    } else if pixels < WEBP_SMALL_THRESHOLD {
        WEBP_QUALITY_SMALL
    } else if pixels < WEBP_MEDIUM_THRESHOLD {
        WEBP_QUALITY_MEDIUM
    } else if pixels < WEBP_LARGE_THRESHOLD {
        WEBP_QUALITY_LARGE
    } else {
        WEBP_QUALITY_HUGE
    };

    if gray {
        quality = (quality - WEBP_GRAYSCALE_QUALITY_REDUCTION).max(60.0);
    }

    quality
}

pub fn convert_to_webp(img: &image::DynamicImage) -> Result<Vec<u8>, ThasiaError> {
    use ::webp::Encoder;

    let gray = is_grayscale(img);
    let quality = auto_tune_webp(img, gray);
    let (w, h) = (img.width(), img.height());

    trace!("WebP encode: {}x{} quality={} gray={}", w, h, quality, gray);

    let encoded = if img.color().has_alpha() {
        let rgba = img.to_rgba8();
        Encoder::from_rgba(&rgba, w, h).encode(quality)
    } else {
        let rgb_cow = if let Some(r) = img.as_rgb8() {
            std::borrow::Cow::Borrowed(r)
        } else {
            std::borrow::Cow::Owned(img.to_rgb8())
        };
        Encoder::from_rgb(&rgb_cow, w, h).encode(quality)
    };

    Ok(encoded.to_vec())
}
