//! AVIF encoding with 5-tier adaptive quality tuning.
//! Ported from the Palaxy project.

use super::constants::*;
use super::grayscale::is_grayscale;
use thasia_core::ThasiaError;
use tracing::trace;

pub fn auto_tune_avif(img: &image::DynamicImage, gray: bool) -> (f32, u8) {
    let pixels = img.width() as u64 * img.height() as u64;

    let (mut quality, mut speed) = if pixels < AVIF_TINY_THRESHOLD {
        (AVIF_QUALITY_TINY, AVIF_SPEED_TINY)
    } else if pixels < AVIF_SMALL_THRESHOLD {
        (AVIF_QUALITY_SMALL, AVIF_SPEED_SMALL)
    } else if pixels < AVIF_MEDIUM_THRESHOLD {
        (AVIF_QUALITY_MEDIUM, AVIF_SPEED_MEDIUM)
    } else if pixels < AVIF_LARGE_THRESHOLD {
        (AVIF_QUALITY_LARGE, AVIF_SPEED_LARGE)
    } else {
        (AVIF_QUALITY_HUGE, AVIF_SPEED_HUGE)
    };

    if gray {
        speed = speed.saturating_sub(1).max(6);
        quality = (quality - AVIF_GRAYSCALE_QUALITY_REDUCTION).max(55.0);
    }

    (quality, speed)
}

pub fn convert_to_avif(img: &image::DynamicImage) -> Result<Vec<u8>, ThasiaError> {
    let width = img.width() as usize;
    let height = img.height() as usize;
    let gray = is_grayscale(img);
    let (quality, speed) = auto_tune_avif(img, gray);

    trace!(
        "AVIF encode: {}x{} quality={} speed={} gray={}",
        width,
        height,
        quality,
        speed,
        gray
    );

    // Single thread per encode: rayon handles parallelism across images.
    let encoder = ravif::Encoder::new()
        .with_quality(quality)
        .with_speed(speed)
        .with_alpha_quality(AVIF_ALPHA_QUALITY)
        .with_num_threads(Some(1));

    if gray {
        encode_gray(&encoder, img, width, height)
    } else {
        encode_color(&encoder, img, width, height)
    }
}

/// Grayscale path: feeds luma as Y with neutral chroma (Cb=Cr=128).
/// The constant chroma planes compress to near-zero, yielding much smaller files.
fn encode_gray(
    encoder: &ravif::Encoder,
    img: &image::DynamicImage,
    width: usize,
    height: usize,
) -> Result<Vec<u8>, ThasiaError> {
    let luma = img.to_luma8();
    let planes = luma.as_raw().iter().map(|&y| [y, 128u8, 128u8]);

    let encoded = encoder
        .encode_raw_planes_8_bit(
            width,
            height,
            planes,
            None::<[u8; 0]>,
            rav1e::prelude::PixelRange::Full,
            ravif::MatrixCoefficients::BT601,
        )
        .map_err(|e| ThasiaError::Fatal(format!("AVIF gray encode failed: {e}")))?;

    Ok(encoded.avif_file)
}

fn encode_color(
    encoder: &ravif::Encoder,
    img: &image::DynamicImage,
    width: usize,
    height: usize,
) -> Result<Vec<u8>, ThasiaError> {
    use ravif::{Img, RGB8};

    let rgb_cow = if let Some(r) = img.as_rgb8() {
        std::borrow::Cow::Borrowed(r)
    } else {
        std::borrow::Cow::Owned(img.to_rgb8())
    };

    let rgb_slice: &[RGB8] = unsafe {
        std::slice::from_raw_parts(rgb_cow.as_raw().as_ptr() as *const RGB8, width * height)
    };

    let encoded = encoder
        .encode_rgb(Img::new(rgb_slice, width, height))
        .map_err(|e| ThasiaError::Fatal(format!("AVIF color encode failed: {e}")))?;

    Ok(encoded.avif_file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, ImageBuffer, Rgb};

    #[test]
    fn test_auto_tune_quality_tiers() {
        // 100x100 = 10_000 pixels → tiny tier
        let tiny = ImageBuffer::from_fn(100, 100, |_, _| Rgb([128u8, 128u8, 128u8]));
        let (q, s) = auto_tune_avif(&DynamicImage::ImageRgb8(tiny), false);
        assert_eq!(q, AVIF_QUALITY_TINY);
        assert_eq!(s, AVIF_SPEED_TINY);

        // 1200x1200 = 1_440_000 pixels → medium tier
        let medium = ImageBuffer::from_fn(1200, 1200, |_, _| Rgb([128u8, 128u8, 128u8]));
        let (q, _) = auto_tune_avif(&DynamicImage::ImageRgb8(medium), false);
        assert_eq!(q, AVIF_QUALITY_MEDIUM);
    }

    #[test]
    fn test_grayscale_reduces_quality() {
        let img = ImageBuffer::from_fn(100, 100, |_, _| Rgb([128u8, 128u8, 128u8]));
        let (q_color, _) = auto_tune_avif(&DynamicImage::ImageRgb8(img.clone()), false);
        let (q_gray, _) = auto_tune_avif(&DynamicImage::ImageRgb8(img), true);
        assert!(q_gray < q_color);
    }
}
