use crate::encode::grayscale::{ImageTone, classify_image_tone};
use image::DynamicImage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformStep {
    NormalizeColor,
    DropOpaqueAlpha,
    CleanScanTones,
    ResizeMaxWidth(u32),
}

const DEFAULT_STEPS: &[TransformStep] = &[
    TransformStep::NormalizeColor,
    TransformStep::DropOpaqueAlpha,
];

#[derive(Debug, Clone, Copy)]
pub struct TransformPipeline {
    max_width: Option<u32>,
    clean_tones: bool,
}

impl TransformPipeline {
    pub fn new(max_width: Option<u32>, clean_tones: bool) -> Self {
        Self {
            max_width,
            clean_tones,
        }
    }

    pub fn apply(&self, img: &mut DynamicImage) {
        for step in DEFAULT_STEPS {
            step.apply(img);
        }
        if self.clean_tones {
            TransformStep::CleanScanTones.apply(img);
        }
        if let Some(max_width) = self.max_width {
            TransformStep::ResizeMaxWidth(max_width).apply(img);
        }
    }
}

impl TransformStep {
    fn apply(self, img: &mut DynamicImage) {
        match self {
            TransformStep::NormalizeColor => normalize_color(img),
            TransformStep::DropOpaqueAlpha => drop_opaque_alpha(img),
            TransformStep::CleanScanTones => clean_scan_tones(img),
            TransformStep::ResizeMaxWidth(max_width) => resize_max_width(img, max_width),
        }
    }
}

fn normalize_color(img: &mut DynamicImage) {
    match img {
        DynamicImage::ImageRgb8(_) | DynamicImage::ImageRgba8(_) | DynamicImage::ImageLuma8(_) => {}
        _ if img.color().has_alpha() => {
            *img = DynamicImage::ImageRgba8(img.to_rgba8());
        }
        _ => {
            *img = DynamicImage::ImageRgb8(img.to_rgb8());
        }
    }
}

fn resize_max_width(img: &mut DynamicImage, max_width: u32) {
    let (width, height) = (img.width(), img.height());
    if width <= max_width {
        return;
    }

    let scale = max_width as f64 / width as f64;
    *img = img.resize(
        max_width,
        (height as f64 * scale) as u32,
        image::imageops::FilterType::Triangle,
    );
}

fn drop_opaque_alpha(img: &mut DynamicImage) {
    let rgb = {
        let Some(rgba) = img.as_rgba8() else {
            return;
        };
        if !rgba.as_raw().chunks_exact(4).all(|px| px[3] == 255) {
            return;
        }
        DynamicImage::ImageRgba8(rgba.clone()).to_rgb8()
    };
    *img = DynamicImage::ImageRgb8(rgb);
}

fn clean_scan_tones(img: &mut DynamicImage) {
    let tone = classify_image_tone(img);
    if tone == ImageTone::Color {
        return;
    }

    if let Some(rgb) = img.as_mut_rgb8() {
        clean_rgb_tones(rgb, tone);
    } else if let Some(luma) = img.as_mut_luma8() {
        clean_luma_tones(luma, tone);
    }
}

fn clean_rgb_tones(img: &mut image::RgbImage, tone: ImageTone) {
    let white_threshold = white_threshold(tone);
    let black_threshold = black_threshold(tone);
    for px in img.pixels_mut() {
        let [r, g, b] = px.0;
        if !is_neutral(r, g, b) {
            continue;
        }
        let luma = luma_approx(r, g, b);
        if luma >= white_threshold {
            px.0 = [255, 255, 255];
        } else if luma <= black_threshold {
            px.0 = [0, 0, 0];
        }
    }
}

fn clean_luma_tones(img: &mut image::GrayImage, tone: ImageTone) {
    let white_threshold = white_threshold(tone);
    let black_threshold = black_threshold(tone);
    for px in img.pixels_mut() {
        if px.0[0] >= white_threshold {
            px.0[0] = 255;
        } else if px.0[0] <= black_threshold {
            px.0[0] = 0;
        }
    }
}

#[inline(always)]
fn is_neutral(r: u8, g: u8, b: u8) -> bool {
    r.abs_diff(g) <= 4 && g.abs_diff(b) <= 4 && b.abs_diff(r) <= 4
}

#[inline(always)]
fn luma_approx(r: u8, g: u8, b: u8) -> u8 {
    ((r as u16 * 77 + g as u16 * 150 + b as u16 * 29) >> 8) as u8
}

fn white_threshold(tone: ImageTone) -> u8 {
    match tone {
        ImageTone::LineArt => 238,
        ImageTone::Grayscale => 246,
        ImageTone::Color => 255,
    }
}

fn black_threshold(tone: ImageTone) -> u8 {
    match tone {
        ImageTone::LineArt => 24,
        ImageTone::Grayscale => 8,
        ImageTone::Color => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Luma, Rgb, Rgba};

    #[test]
    fn resize_step_only_downscales_wide_images() {
        let img = ImageBuffer::from_fn(200, 100, |_, _| Rgb([128u8, 128u8, 128u8]));
        let mut img = DynamicImage::ImageRgb8(img);
        TransformStep::ResizeMaxWidth(100).apply(&mut img);
        assert_eq!(img.width(), 100);
        assert_eq!(img.height(), 50);
    }

    #[test]
    fn normalize_keeps_luma8_fast_path() {
        let img = ImageBuffer::from_fn(10, 10, |_, _| Luma([128u8]));
        let mut img = DynamicImage::ImageLuma8(img);
        TransformStep::NormalizeColor.apply(&mut img);
        assert!(matches!(img, DynamicImage::ImageLuma8(_)));
    }

    #[test]
    fn drops_alpha_when_all_pixels_are_opaque() {
        let img = ImageBuffer::from_fn(10, 10, |_, _| Rgba([1u8, 2u8, 3u8, 255u8]));
        let mut img = DynamicImage::ImageRgba8(img);
        TransformStep::DropOpaqueAlpha.apply(&mut img);
        assert!(matches!(img, DynamicImage::ImageRgb8(_)));
    }

    #[test]
    fn keeps_alpha_when_any_pixel_is_transparent() {
        let img = ImageBuffer::from_fn(10, 10, |x, _| {
            if x == 0 {
                Rgba([1u8, 2u8, 3u8, 128u8])
            } else {
                Rgba([1u8, 2u8, 3u8, 255u8])
            }
        });
        let mut img = DynamicImage::ImageRgba8(img);
        TransformStep::DropOpaqueAlpha.apply(&mut img);
        assert!(matches!(img, DynamicImage::ImageRgba8(_)));
    }

    #[test]
    fn clean_scan_tones_does_not_crop() {
        let img = ImageBuffer::from_fn(100, 100, |x, y| {
            if (20..80).contains(&x) && (20..80).contains(&y) {
                Rgb([0u8, 0u8, 0u8])
            } else {
                Rgb([255u8, 255u8, 255u8])
            }
        });
        let mut img = DynamicImage::ImageRgb8(img);
        TransformStep::CleanScanTones.apply(&mut img);
        assert_eq!(img.width(), 100);
        assert_eq!(img.height(), 100);
    }

    #[test]
    fn clean_scan_tones_maps_near_white_to_white() {
        let img = ImageBuffer::from_fn(20, 20, |x, _| {
            if x == 0 {
                Rgb([0u8, 0u8, 0u8])
            } else {
                Rgb([248u8, 248u8, 248u8])
            }
        });
        let mut img = DynamicImage::ImageRgb8(img);
        TransformStep::CleanScanTones.apply(&mut img);
        let rgb = img.as_rgb8().unwrap();
        assert!(rgb.pixels().any(|px| px.0 == [255, 255, 255]));
    }

    #[test]
    fn clean_scan_tones_maps_near_black_to_black() {
        let img = ImageBuffer::from_fn(20, 20, |x, _| {
            if x == 0 {
                Rgb([255u8, 255u8, 255u8])
            } else {
                Rgb([18u8, 18u8, 18u8])
            }
        });
        let mut img = DynamicImage::ImageRgb8(img);
        TransformStep::CleanScanTones.apply(&mut img);
        let rgb = img.as_rgb8().unwrap();
        assert!(rgb.pixels().any(|px| px.0 == [0, 0, 0]));
    }
}
