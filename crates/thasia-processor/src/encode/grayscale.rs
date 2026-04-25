//! Fast grayscale detection for manga content.
//! Ported from the Palaxy project's common::image_utils module.

use image::{DynamicImage, GenericImageView};

const SAMPLE_COUNT: u64 = 500;
const RGB_THRESHOLD: u8 = 3;
const GRAY_THRESHOLD: f32 = 0.95;

/// Detects if an image is predominantly grayscale using distributed sampling.
/// This approach samples ~500 evenly distributed pixels in cache-friendly
/// row-major order without downscaling.
pub fn is_grayscale(img: &DynamicImage) -> bool {
    // Native grayscale formats — fast path
    if matches!(
        img,
        DynamicImage::ImageLuma8(_) | DynamicImage::ImageLuma16(_)
    ) {
        return true;
    }

    let width = img.width() as u64;
    let height = img.height() as u64;
    let pixel_count = width * height;
    let step = (pixel_count / SAMPLE_COUNT).max(1);

    let mut gray_count = 0u32;
    let mut total = 0u32;
    let mut i = 0u64;

    while i < pixel_count {
        let x = (i % width) as u32;
        let y = (i / width) as u32;
        let channels = img.get_pixel(x, y).0;

        if channels.len() >= 3 {
            let (r, g, b) = (channels[0], channels[1], channels[2]);
            if r.abs_diff(g) <= RGB_THRESHOLD
                && g.abs_diff(b) <= RGB_THRESHOLD
                && b.abs_diff(r) <= RGB_THRESHOLD
            {
                gray_count += 1;
            }
            total += 1;
        }

        i += step;
    }

    total > 0 && (gray_count as f32 / total as f32) > GRAY_THRESHOLD
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, ImageBuffer, Rgb};

    #[test]
    fn test_native_luma_is_grayscale() {
        let img: image::GrayImage = ImageBuffer::new(10, 10);
        let dyn_img = DynamicImage::ImageLuma8(img);
        assert!(is_grayscale(&dyn_img));
    }

    #[test]
    fn test_gray_rgb_image_is_grayscale() {
        // All pixels R=G=B=128
        let img: image::RgbImage =
            ImageBuffer::from_fn(100, 100, |_, _| Rgb([128u8, 128u8, 128u8]));
        let dyn_img = DynamicImage::ImageRgb8(img);
        assert!(is_grayscale(&dyn_img));
    }

    #[test]
    fn test_color_image_is_not_grayscale() {
        // Red image
        let img: image::RgbImage = ImageBuffer::from_fn(100, 100, |_, _| Rgb([255u8, 0u8, 0u8]));
        let dyn_img = DynamicImage::ImageRgb8(img);
        assert!(!is_grayscale(&dyn_img));
    }
}
