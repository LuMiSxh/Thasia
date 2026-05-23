//! Fast grayscale detection for manga content.
//! Ported from the Palaxy project's common::image_utils module.

use image::{DynamicImage, GenericImageView};

const SAMPLE_COUNT: u64 = 500;
const RGB_THRESHOLD: u8 = 3;
const GRAY_THRESHOLD: f32 = 0.95;

#[inline(always)]
fn is_gray_pixel(r: u8, g: u8, b: u8) -> bool {
    r.abs_diff(g) <= RGB_THRESHOLD
        && g.abs_diff(b) <= RGB_THRESHOLD
        && b.abs_diff(r) <= RGB_THRESHOLD
}

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
    if pixel_count == 0 {
        return false;
    }
    let step = (pixel_count / SAMPLE_COUNT).max(1);

    // Fast paths: index directly into the raw byte slice. Avoids virtual
    // dispatch through DynamicImage::get_pixel for the common RGB8/RGBA8 cases.
    if let Some(rgb) = img.as_rgb8() {
        let raw = rgb.as_raw();
        let mut gray_count = 0u32;
        let mut total = 0u32;
        let mut i = 0u64;
        while i < pixel_count {
            let base = (i as usize) * 3;
            if is_gray_pixel(raw[base], raw[base + 1], raw[base + 2]) {
                gray_count += 1;
            }
            total += 1;
            i += step;
        }
        return total > 0 && (gray_count as f32 / total as f32) > GRAY_THRESHOLD;
    }

    if let Some(rgba) = img.as_rgba8() {
        let raw = rgba.as_raw();
        let mut gray_count = 0u32;
        let mut total = 0u32;
        let mut i = 0u64;
        while i < pixel_count {
            let base = (i as usize) * 4;
            if is_gray_pixel(raw[base], raw[base + 1], raw[base + 2]) {
                gray_count += 1;
            }
            total += 1;
            i += step;
        }
        return total > 0 && (gray_count as f32 / total as f32) > GRAY_THRESHOLD;
    }

    // Slow path: 16-bit / float images — virtual dispatch through DynamicImage.
    let mut gray_count = 0u32;
    let mut total = 0u32;
    let mut i = 0u64;

    while i < pixel_count {
        let x = (i % width) as u32;
        let y = (i / width) as u32;
        let channels = img.get_pixel(x, y).0;

        if channels.len() >= 3 {
            if is_gray_pixel(channels[0], channels[1], channels[2]) {
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
    use image::{DynamicImage, ImageBuffer, Rgb, Rgba};

    #[test]
    fn test_native_luma_is_grayscale() {
        let img: image::GrayImage = ImageBuffer::new(10, 10);
        let dyn_img = DynamicImage::ImageLuma8(img);
        assert!(is_grayscale(&dyn_img));
    }

    #[test]
    fn test_gray_rgb_image_is_grayscale() {
        let img: image::RgbImage =
            ImageBuffer::from_fn(100, 100, |_, _| Rgb([128u8, 128u8, 128u8]));
        let dyn_img = DynamicImage::ImageRgb8(img);
        assert!(is_grayscale(&dyn_img));
    }

    #[test]
    fn test_gray_rgba_image_is_grayscale() {
        let img: image::RgbaImage =
            ImageBuffer::from_fn(100, 100, |_, _| Rgba([200u8, 200u8, 200u8, 255u8]));
        let dyn_img = DynamicImage::ImageRgba8(img);
        assert!(is_grayscale(&dyn_img));
    }

    #[test]
    fn test_color_image_is_not_grayscale() {
        let img: image::RgbImage = ImageBuffer::from_fn(100, 100, |_, _| Rgb([255u8, 0u8, 0u8]));
        let dyn_img = DynamicImage::ImageRgb8(img);
        assert!(!is_grayscale(&dyn_img));
    }
}
