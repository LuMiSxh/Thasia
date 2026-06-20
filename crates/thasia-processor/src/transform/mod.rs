use crate::encode::grayscale::{ImageTone, classify_image_tone};
use image::DynamicImage;
use thasia_core::models::{ColorEnhanceMode, Direction, SharpenMode};

mod color;
mod filters;
mod resize;
mod tones;

const DOUBLE_PAGE_RATIO: f32 = 1.2;

const DEFAULT_STEPS: &[TransformStep] = &[
    TransformStep::NormalizeColor,
    TransformStep::DropOpaqueAlpha,
];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TransformOptions {
    pub max_width: Option<u32>,
    pub clean_tones: bool,
    pub color_enhance: ColorEnhanceMode,
    pub sharpen: SharpenMode,
    pub split_double_page: bool,
    pub direction: Direction,
    pub auto_crop: bool,
    /// White-border padding (pixels) re-added after auto-crop.
    pub crop_padding: u32,
    pub moire_reduction: bool,
    pub eink_dither: bool,
}

impl TransformOptions {
    pub fn disables_passthrough(self) -> bool {
        self.max_width.is_some()
            || self.clean_tones
            || self.color_enhance != ColorEnhanceMode::Off
            || self.sharpen != SharpenMode::Off
            || self.split_double_page
            || self.auto_crop
            || self.moire_reduction
            || self.eink_dither
    }
}

/// Splits a landscape double-page spread into two images in logical reading order.
/// Returns a single-element vec for portrait images.
pub fn maybe_split_double_page(img: DynamicImage, direction: Direction) -> Vec<DynamicImage> {
    let (w, h) = (img.width(), img.height());
    if h == 0 || (w as f32 / h as f32) <= DOUBLE_PAGE_RATIO {
        return vec![img];
    }
    let half = w / 2;
    let left = img.crop_imm(0, 0, half, h);
    let right = img.crop_imm(half, 0, w - half, h);
    match direction {
        Direction::Rtl => vec![right, left],
        Direction::Ltr => vec![left, right],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformStep {
    NormalizeColor,
    DropOpaqueAlpha,
    Sharpen(SharpenMode),
    ResizeMaxWidth(u32),
    AutoCrop(u32),
    MoireReduction,
    EinkDither,
}

#[derive(Debug, Clone, Copy)]
pub struct TransformPipeline {
    options: TransformOptions,
}

impl TransformPipeline {
    pub fn new(options: TransformOptions) -> Self {
        Self { options }
    }

    /// Applies all configured transforms and returns the image tone, classified
    /// once after normalization so downstream encode steps don't re-scan.
    pub fn apply(&self, img: &mut DynamicImage) -> ImageTone {
        for step in DEFAULT_STEPS {
            step.apply(img);
        }

        // Classify once here; passed to transforms that need it and returned
        // to the caller so the encoder can skip its own classification pass.
        let tone = classify_image_tone(img);

        if self.options.moire_reduction {
            TransformStep::MoireReduction.apply(img);
        }
        if let Some(max_width) = self.options.max_width {
            TransformStep::ResizeMaxWidth(max_width).apply(img);
        }
        if self.options.clean_tones {
            tones::clean_scan_tones(img, tone);
        }
        if self.options.color_enhance != ColorEnhanceMode::Off {
            color::enhance_color(img, self.options.color_enhance, tone);
        }
        if self.options.sharpen != SharpenMode::Off {
            TransformStep::Sharpen(self.options.sharpen).apply(img);
        }
        if self.options.auto_crop {
            TransformStep::AutoCrop(self.options.crop_padding).apply(img);
        }
        if self.options.eink_dither {
            TransformStep::EinkDither.apply(img);
            // eink_dither always produces ImageLuma8 with 16-level quantization.
            return ImageTone::LineArt;
        }

        tone
    }
}

impl TransformStep {
    fn apply(self, img: &mut DynamicImage) {
        match self {
            TransformStep::NormalizeColor => color::normalize_color(img),
            TransformStep::DropOpaqueAlpha => color::drop_opaque_alpha(img),
            TransformStep::Sharpen(mode) => sharpen(img, mode),
            TransformStep::ResizeMaxWidth(max_width) => resize::resize_max_width(img, max_width),
            TransformStep::AutoCrop(padding) => filters::auto_crop(img, padding),
            TransformStep::MoireReduction => filters::moire_reduction(img),
            TransformStep::EinkDither => filters::eink_dither(img),
        }
    }
}

fn sharpen(img: &mut DynamicImage, mode: SharpenMode) {
    let (sigma, threshold) = match mode {
        SharpenMode::Off => return,
        SharpenMode::Mild => (0.85, 4),
    };
    *img = img.unsharpen(sigma, threshold);
}

/// BT.601 luma approximation — integer-only, shared across transform submodules.
#[inline(always)]
pub(crate) fn luma_approx(r: u8, g: u8, b: u8) -> u8 {
    ((r as u16 * 77 + g as u16 * 150 + b as u16 * 29) >> 8) as u8
}

// Shared gamma primitives used by both color (Oklab) and resize (LUT builder).
// Defined here so child modules can access them via `super::`.
#[inline(always)]
fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

#[inline(always)]
fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, ImageBuffer, Luma, Rgb, Rgba};
    use thasia_core::models::Direction;

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
        tones::clean_scan_tones(&mut img, crate::encode::grayscale::ImageTone::LineArt);
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
        tones::clean_scan_tones(&mut img, crate::encode::grayscale::ImageTone::Grayscale);
        assert!(
            img.as_rgb8()
                .unwrap()
                .pixels()
                .any(|px| px.0 == [255, 255, 255])
        );
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
        // 95% B&W pixels → LineArt (bp=24), so luma=18 clamps to 0
        tones::clean_scan_tones(&mut img, crate::encode::grayscale::ImageTone::LineArt);
        assert!(img.as_rgb8().unwrap().pixels().any(|px| px.0 == [0, 0, 0]));
    }

    #[test]
    fn color_enhance_lifts_washed_out_color() {
        let img = ImageBuffer::from_fn(20, 20, |x, _| {
            if x % 2 == 0 {
                Rgb([150u8, 120u8, 100u8])
            } else {
                Rgb([110u8, 135u8, 150u8])
            }
        });
        let mut img = DynamicImage::ImageRgb8(img);
        color::enhance_color(&mut img, ColorEnhanceMode::Balanced, crate::encode::grayscale::ImageTone::Color);
        let first = img.as_rgb8().unwrap().get_pixel(0, 0).0;
        assert!(first[0] > 150, "red channel should increase");
        assert!(
            (first[0] as i16 - first[2] as i16) > 50,
            "R-B spread should increase"
        );
    }

    #[test]
    fn color_enhance_skips_grayscale_pages() {
        let img = ImageBuffer::from_fn(20, 20, |_, _| Rgb([120u8, 120u8, 120u8]));
        let mut img = DynamicImage::ImageRgb8(img);
        color::enhance_color(&mut img, ColorEnhanceMode::Strong, crate::encode::grayscale::ImageTone::Grayscale);
        assert_eq!(img.as_rgb8().unwrap().get_pixel(0, 0).0, [120, 120, 120]);
    }

    #[test]
    fn sharpen_keeps_dimensions() {
        let img = ImageBuffer::from_fn(30, 40, |x, _| {
            if x < 15 {
                Rgb([40u8, 40u8, 40u8])
            } else {
                Rgb([220u8, 220u8, 220u8])
            }
        });
        let mut img = DynamicImage::ImageRgb8(img);
        TransformStep::Sharpen(SharpenMode::Mild).apply(&mut img);
        assert_eq!(img.width(), 30);
        assert_eq!(img.height(), 40);
    }

    #[test]
    fn double_page_split_ltr_order() {
        let img = ImageBuffer::from_fn(200, 100, |x, _| {
            if x < 100 {
                Rgb([255u8, 0, 0])
            } else {
                Rgb([0u8, 0, 255])
            }
        });
        let halves = maybe_split_double_page(DynamicImage::ImageRgb8(img), Direction::Ltr);
        assert_eq!(halves.len(), 2);
        assert_eq!(halves[0].as_rgb8().unwrap().get_pixel(0, 0).0, [255, 0, 0]);
        assert_eq!(halves[1].as_rgb8().unwrap().get_pixel(0, 0).0, [0, 0, 255]);
    }

    #[test]
    fn double_page_split_rtl_order() {
        let img = ImageBuffer::from_fn(200, 100, |x, _| {
            if x < 100 {
                Rgb([255u8, 0, 0])
            } else {
                Rgb([0u8, 0, 255])
            }
        });
        let halves = maybe_split_double_page(DynamicImage::ImageRgb8(img), Direction::Rtl);
        assert_eq!(halves[0].as_rgb8().unwrap().get_pixel(0, 0).0, [0, 0, 255]);
        assert_eq!(halves[1].as_rgb8().unwrap().get_pixel(0, 0).0, [255, 0, 0]);
    }

    #[test]
    fn portrait_image_not_split() {
        let img = ImageBuffer::from_fn(100, 200, |_, _| Rgb([128u8, 128, 128]));
        let result = maybe_split_double_page(DynamicImage::ImageRgb8(img), Direction::Ltr);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].width(), 100);
    }
}
