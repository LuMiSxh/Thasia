use crate::encode::grayscale::{ImageTone, classify_image_tone};
use image::DynamicImage;
use thasia_core::models::{ColorEnhanceMode, Direction, SharpenMode};

const DOUBLE_PAGE_RATIO: f32 = 1.2;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TransformOptions {
    pub max_width: Option<u32>,
    pub clean_tones: bool,
    pub color_enhance: ColorEnhanceMode,
    pub sharpen: SharpenMode,
    pub split_double_page: bool,
    pub direction: Direction,
    pub auto_crop: bool,
    /// Padding added around the cropped area in pixels.
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

/// Returns 2 images (in logical reading order) if the image is a landscape
/// double-page spread, otherwise returns the original image in a 1-element vec.
pub fn maybe_split_double_page(img: DynamicImage, direction: Direction) -> Vec<DynamicImage> {
    let (w, h) = (img.width(), img.height());
    if h == 0 || (w as f32 / h as f32) <= DOUBLE_PAGE_RATIO {
        return vec![img];
    }

    let half = w / 2;
    use image::GenericImageView;
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
    CleanScanTones,
    EnhanceColor(ColorEnhanceMode),
    Sharpen(SharpenMode),
    ResizeMaxWidth(u32),
    AutoCrop(u32),
    MoireReduction,
    EinkDither,
}

const DEFAULT_STEPS: &[TransformStep] = &[
    TransformStep::NormalizeColor,
    TransformStep::DropOpaqueAlpha,
];

/// Kernel radius for the bilateral screentone smoother.
const BILATERAL_RADIUS: i32 = 2;
/// Spatial Gaussian sigma.
const BILATERAL_SIGMA_SPACE: f32 = 1.5;
/// Range Gaussian sigma (luma units 0-255).
const BILATERAL_SIGMA_RANGE: f32 = 30.0;

#[derive(Debug, Clone, Copy)]
pub struct TransformPipeline {
    options: TransformOptions,
}

impl TransformPipeline {
    pub fn new(options: TransformOptions) -> Self {
        Self { options }
    }

    pub fn apply(&self, img: &mut DynamicImage) {
        for step in DEFAULT_STEPS {
            step.apply(img);
        }
        if self.options.moire_reduction {
            TransformStep::MoireReduction.apply(img);
        }
        if let Some(max_width) = self.options.max_width {
            TransformStep::ResizeMaxWidth(max_width).apply(img);
        }
        if self.options.clean_tones {
            TransformStep::CleanScanTones.apply(img);
        }
        if self.options.color_enhance != ColorEnhanceMode::Off {
            TransformStep::EnhanceColor(self.options.color_enhance).apply(img);
        }
        if self.options.sharpen != SharpenMode::Off {
            TransformStep::Sharpen(self.options.sharpen).apply(img);
        }
        if self.options.auto_crop {
            TransformStep::AutoCrop(self.options.crop_padding).apply(img);
        }
        if self.options.eink_dither {
            TransformStep::EinkDither.apply(img);
        }
    }
}

impl TransformStep {
    fn apply(self, img: &mut DynamicImage) {
        match self {
            TransformStep::NormalizeColor => normalize_color(img),
            TransformStep::DropOpaqueAlpha => drop_opaque_alpha(img),
            TransformStep::CleanScanTones => clean_scan_tones(img),
            TransformStep::EnhanceColor(mode) => enhance_color(img, mode),
            TransformStep::Sharpen(mode) => sharpen(img, mode),
            TransformStep::ResizeMaxWidth(max_width) => resize_max_width(img, max_width),
            TransformStep::AutoCrop(padding) => auto_crop(img, padding),
            TransformStep::MoireReduction => moire_reduction(img),
            TransformStep::EinkDither => eink_dither(img),
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

    let new_height = ((height as f64 * max_width as f64 / width as f64).round() as u32).max(1);

    // Precompute 256-entry LUTs — one powf call per distinct byte value instead
    // of one per pixel. LLVM can auto-vectorize the subsequent table-lookup loops.
    let to_linear = build_lut(srgb_to_linear);
    let to_srgb = build_lut(linear_to_srgb);

    // Linearize in-place (no alloc), resize in linear light, re-encode in-place.
    apply_lut_inplace(img, &to_linear, false);
    *img = img.resize_exact(max_width, new_height, image::imageops::FilterType::Lanczos3);
    apply_lut_inplace(img, &to_srgb, false);
}

/// Builds a 256-entry lookup table by evaluating `f` for each possible u8 value.
fn build_lut(f: fn(f32) -> f32) -> [u8; 256] {
    let mut lut = [0u8; 256];
    for (i, entry) in lut.iter_mut().enumerate() {
        *entry = (f(i as f32 / 255.0) * 255.0).round() as u8;
    }
    lut
}

/// Applies `lut` to every color channel in-place. When `skip_alpha` is true the
/// alpha byte (4th channel in RGBA) is left untouched.
fn apply_lut_inplace(img: &mut DynamicImage, lut: &[u8; 256], skip_alpha: bool) {
    match img {
        DynamicImage::ImageRgb8(buf) => {
            for c in buf.as_flat_samples_mut().samples.iter_mut() {
                *c = lut[*c as usize];
            }
        }
        DynamicImage::ImageRgba8(buf) => {
            let raw = buf.as_flat_samples_mut().samples;
            for px in raw.chunks_exact_mut(4) {
                px[0] = lut[px[0] as usize];
                px[1] = lut[px[1] as usize];
                px[2] = lut[px[2] as usize];
                if !skip_alpha {
                    px[3] = lut[px[3] as usize];
                }
            }
        }
        DynamicImage::ImageLuma8(buf) => {
            for c in buf.as_flat_samples_mut().samples.iter_mut() {
                *c = lut[*c as usize];
            }
        }
        _ => {}
    }
}

fn drop_opaque_alpha(img: &mut DynamicImage) {
    let Some(rgb) = ({
        let Some(rgba) = img.as_rgba8() else {
            return;
        };
        if !rgba.as_raw().chunks_exact(4).all(|px| px[3] == 255) {
            return;
        }
        let (width, height) = rgba.dimensions();
        let mut raw = Vec::with_capacity(rgba.as_raw().len() / 4 * 3);
        for px in rgba.as_raw().chunks_exact(4) {
            raw.extend_from_slice(&px[..3]);
        }
        image::RgbImage::from_raw(width, height, raw)
    }) else {
        return;
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

fn enhance_color(img: &mut DynamicImage, mode: ColorEnhanceMode) {
    if mode == ColorEnhanceMode::Off || classify_image_tone(img) != ImageTone::Color {
        return;
    }

    let (contrast, saturation, brightness) = match mode {
        ColorEnhanceMode::Off => return,
        ColorEnhanceMode::Mild => (1.04, 1.06, 0),
        ColorEnhanceMode::Balanced => (1.08, 1.12, 2),
        ColorEnhanceMode::Strong => (1.14, 1.20, 3),
    };

    if let Some(rgb) = img.as_mut_rgb8() {
        enhance_rgb(rgb, contrast, saturation, brightness);
    } else if let Some(rgba) = img.as_mut_rgba8() {
        enhance_rgba(rgba, contrast, saturation, brightness);
    }
}

fn sharpen(img: &mut DynamicImage, mode: SharpenMode) {
    let (sigma, threshold) = match mode {
        SharpenMode::Off => return,
        SharpenMode::Mild => (0.85, 4),
    };
    *img = img.unsharpen(sigma, threshold);
}

// Brightness threshold: pixels with luma >= this are considered background.
const CROP_BG_LUMA: u8 = 235;
// A row/column is background if this fraction of its pixels are bright.
const CROP_BG_ROW_RATIO: f32 = 0.97;

/// Quantises to 16 grayscale levels and applies Floyd-Steinberg error diffusion.
fn eink_dither(img: &mut DynamicImage) {
    let (w, h) = (img.width(), img.height());
    let mut gray = img.to_luma8();
    let raw = gray.as_flat_samples_mut().samples;

    // Work in i16 to hold diffused error without clamping mid-way.
    let mut buf: Vec<i16> = raw.iter().map(|&v| v as i16).collect();

    const LEVELS: i16 = 16;
    const STEP: i16 = 255 / (LEVELS - 1); // 17

    for y in 0..h as usize {
        for x in 0..w as usize {
            let old = buf[y * w as usize + x].clamp(0, 255);
            let new = ((old + STEP / 2) / STEP * STEP).clamp(0, 255);
            let err = old - new;
            buf[y * w as usize + x] = new;

            // Floyd-Steinberg diffusion kernel:
            //          * 7/16
            //    3/16  5/16  1/16
            if x + 1 < w as usize {
                buf[y * w as usize + x + 1] += err * 7 / 16;
            }
            if y + 1 < h as usize {
                if x > 0 {
                    buf[(y + 1) * w as usize + x - 1] += err * 3 / 16;
                }
                buf[(y + 1) * w as usize + x] += err * 5 / 16;
                if x + 1 < w as usize {
                    buf[(y + 1) * w as usize + x + 1] += err / 16;
                }
            }
        }
    }

    for (dst, src) in raw.iter_mut().zip(buf.iter()) {
        *dst = (*src).clamp(0, 255) as u8;
    }

    *img = DynamicImage::ImageLuma8(gray);
}

fn moire_reduction(img: &mut DynamicImage) {
    // Bilateral filter on the luma channel: smooths screentone high-frequency
    // patterns while preserving ink edges (large luma gradients are down-weighted).
    let (w, h) = (img.width(), img.height());
    if w < 3 || h < 3 {
        return;
    }

    // Precompute spatial kernel weights once.
    let r = BILATERAL_RADIUS;
    let diam = (2 * r + 1) as usize;
    let mut spatial: Vec<f32> = Vec::with_capacity(diam * diam);
    for dy in -r..=r {
        for dx in -r..=r {
            let d2 = (dx * dx + dy * dy) as f32;
            spatial.push((-d2 / (2.0 * BILATERAL_SIGMA_SPACE * BILATERAL_SIGMA_SPACE)).exp());
        }
    }

    let range_coeff = -1.0 / (2.0 * BILATERAL_SIGMA_RANGE * BILATERAL_SIGMA_RANGE);

    match img {
        DynamicImage::ImageRgb8(buf) => {
            let input = buf.clone();
            let raw = buf.as_flat_samples_mut();
            let (stride, data) = (w as usize * 3, raw.samples);
            for y in 0..h {
                for x in 0..w {
                    let center_luma = pixel_luma_rgb(&input, x, y);
                    let mut sum_w = 0.0f32;
                    let mut sum_r = 0.0f32;
                    let mut sum_g = 0.0f32;
                    let mut sum_b = 0.0f32;
                    let mut ki = 0;
                    for dy in -r..=r {
                        for dx in -r..=r {
                            let nx = (x as i32 + dx).clamp(0, w as i32 - 1) as u32;
                            let ny = (y as i32 + dy).clamp(0, h as i32 - 1) as u32;
                            let n_luma = pixel_luma_rgb(&input, nx, ny);
                            let range_w = ((n_luma as f32 - center_luma as f32).powi(2) * range_coeff).exp();
                            let w_total = spatial[ki] * range_w;
                            let base = (ny as usize * (w as usize) + nx as usize) * 3;
                            let nr = input.as_raw()[base] as f32;
                            let ng = input.as_raw()[base + 1] as f32;
                            let nb = input.as_raw()[base + 2] as f32;
                            sum_r += w_total * nr;
                            sum_g += w_total * ng;
                            sum_b += w_total * nb;
                            sum_w += w_total;
                            ki += 1;
                        }
                    }
                    let base = (y as usize * w as usize + x as usize) * 3;
                    data[base]     = (sum_r / sum_w).round() as u8;
                    data[base + 1] = (sum_g / sum_w).round() as u8;
                    data[base + 2] = (sum_b / sum_w).round() as u8;
                    let _ = stride; // used implicitly via base indexing
                }
            }
        }
        DynamicImage::ImageLuma8(buf) => {
            let input = buf.clone();
            let raw = buf.as_flat_samples_mut().samples;
            for y in 0..h {
                for x in 0..w {
                    let center = input.get_pixel(x, y).0[0] as f32;
                    let mut sum_w = 0.0f32;
                    let mut sum_v = 0.0f32;
                    let mut ki = 0;
                    for dy in -r..=r {
                        for dx in -r..=r {
                            let nx = (x as i32 + dx).clamp(0, w as i32 - 1) as u32;
                            let ny = (y as i32 + dy).clamp(0, h as i32 - 1) as u32;
                            let nv = input.get_pixel(nx, ny).0[0] as f32;
                            let range_w = ((nv - center).powi(2) * range_coeff).exp();
                            let w_total = spatial[ki] * range_w;
                            sum_v += w_total * nv;
                            sum_w += w_total;
                            ki += 1;
                        }
                    }
                    raw[(y * w + x) as usize] = (sum_v / sum_w).round() as u8;
                }
            }
        }
        _ => {}
    }
}

#[inline(always)]
fn pixel_luma_rgb(img: &image::RgbImage, x: u32, y: u32) -> u8 {
    let p = img.get_pixel(x, y).0;
    luma_approx(p[0], p[1], p[2])
}

fn auto_crop(img: &mut DynamicImage, padding: u32) {
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return;
    }

    let luma = img.to_luma8();
    let raw = luma.as_raw();

    let row_bg = |y: u32| -> bool {
        let start = (y * w) as usize;
        let row = &raw[start..start + w as usize];
        let bright = row.iter().filter(|&&p| p >= CROP_BG_LUMA).count();
        bright as f32 / w as f32 >= CROP_BG_ROW_RATIO
    };
    let col_bg = |x: u32| -> bool {
        let bright = (0..h)
            .filter(|&y| raw[(y * w + x) as usize] >= CROP_BG_LUMA)
            .count();
        bright as f32 / h as f32 >= CROP_BG_ROW_RATIO
    };

    let top = (0..h).find(|&y| !row_bg(y)).unwrap_or(0);
    let bottom = (0..h).rev().find(|&y| !row_bg(y)).map(|y| y + 1).unwrap_or(h);
    let left = (0..w).find(|&x| !col_bg(x)).unwrap_or(0);
    let right = (0..w).rev().find(|&x| !col_bg(x)).map(|x| x + 1).unwrap_or(w);

    if top >= bottom || left >= right {
        return;
    }

    let x = left.saturating_sub(padding);
    let y = top.saturating_sub(padding);
    let crop_w = (right + padding).min(w) - x;
    let crop_h = (bottom + padding).min(h) - y;

    if crop_w == w && crop_h == h {
        return;
    }

    use image::GenericImageView;
    *img = img.crop_imm(x, y, crop_w, crop_h);
}

fn clean_rgb_tones(img: &mut image::RgbImage, tone: ImageTone) {
    let bp = black_threshold(tone) as f32;
    let wp = white_threshold(tone) as f32;
    for px in img.pixels_mut() {
        let [r, g, b] = px.0;
        if !is_neutral(r, g, b) {
            continue;
        }
        let luma = luma_approx(r, g, b);
        let v = smoothstep_u8(luma, bp, wp);
        px.0 = [v, v, v];
    }
}

fn clean_luma_tones(img: &mut image::GrayImage, tone: ImageTone) {
    let bp = black_threshold(tone) as f32;
    let wp = white_threshold(tone) as f32;
    for px in img.pixels_mut() {
        px.0[0] = smoothstep_u8(px.0[0], bp, wp);
    }
}

#[inline(always)]
fn smoothstep_u8(luma: u8, bp: f32, wp: f32) -> u8 {
    let t = ((luma as f32 - bp) / (wp - bp)).clamp(0.0, 1.0);
    (t * t * (3.0 - 2.0 * t) * 255.0).round() as u8
}

fn enhance_rgb(img: &mut image::RgbImage, contrast: f32, saturation: f32, brightness: i16) {
    for px in img.pixels_mut() {
        let [r, g, b] = px.0;
        px.0 = enhance_rgb_channels(r, g, b, contrast, saturation, brightness);
    }
}

fn enhance_rgba(img: &mut image::RgbaImage, contrast: f32, saturation: f32, brightness: i16) {
    for px in img.pixels_mut() {
        let [r, g, b, a] = px.0;
        let [r, g, b] = enhance_rgb_channels(r, g, b, contrast, saturation, brightness);
        px.0 = [r, g, b, a];
    }
}

#[inline(always)]
fn enhance_rgb_channels(
    r: u8,
    g: u8,
    b: u8,
    contrast: f32,
    saturation: f32,
    brightness: i16,
) -> [u8; 3] {
    let (l, a, b_ok) = srgb_u8_to_oklab(r, g, b);

    // Scale chroma (a, b axes) without touching the hue angle.
    let a = a * saturation;
    let b_ok = b_ok * saturation;

    // Apply lightness contrast and brightness in the L channel.
    let l = ((l - 0.5) * contrast + 0.5 + brightness as f32 / 255.0).clamp(0.0, 1.0);

    oklab_to_srgb_u8(l, a, b_ok)
}

// ── Oklab math ────────────────────────────────────────────────────────────────

#[inline(always)]
fn srgb_u8_to_oklab(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r = srgb_to_linear(r as f32 / 255.0);
    let g = srgb_to_linear(g as f32 / 255.0);
    let b = srgb_to_linear(b as f32 / 255.0);

    let l = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b).cbrt();
    let m = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b).cbrt();
    let s = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b).cbrt();

    (
        0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
    )
}

#[inline(always)]
fn oklab_to_srgb_u8(l: f32, a: f32, b: f32) -> [u8; 3] {
    let l_ = l + 0.3963377774 * a + 0.2158037573 * b;
    let m_ = l - 0.1055613458 * a - 0.0638541728 * b;
    let s_ = l - 0.0894841775 * a - 1.2914855480 * b;

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    let r = linear_to_srgb(4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s);
    let g = linear_to_srgb(-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s);
    let b = linear_to_srgb(-0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s);

    [
        (r.clamp(0.0, 1.0) * 255.0).round() as u8,
        (g.clamp(0.0, 1.0) * 255.0).round() as u8,
        (b.clamp(0.0, 1.0) * 255.0).round() as u8,
    ]
}

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

    #[test]
    fn color_enhance_lifts_washed_out_color() {
        // Warm pixel [150, 120, 100]: red is dominant, blue is lowest.
        // Oklab chroma scaling should increase color saturation: the spread
        // (R - B) must grow and R must increase.
        let img = ImageBuffer::from_fn(20, 20, |x, _| {
            if x % 2 == 0 {
                Rgb([150u8, 120u8, 100u8])
            } else {
                Rgb([110u8, 135u8, 150u8])
            }
        });
        let mut img = DynamicImage::ImageRgb8(img);
        TransformStep::EnhanceColor(ColorEnhanceMode::Balanced).apply(&mut img);
        let first = img.as_rgb8().unwrap().get_pixel(0, 0).0;
        assert!(first[0] > 150, "red channel should increase");
        assert!(
            (first[0] as i16 - first[2] as i16) > 50,
            "R-B spread should increase (saturation lifted)"
        );
    }

    #[test]
    fn color_enhance_skips_grayscale_pages() {
        let img = ImageBuffer::from_fn(20, 20, |_, _| Rgb([120u8, 120u8, 120u8]));
        let mut img = DynamicImage::ImageRgb8(img);
        TransformStep::EnhanceColor(ColorEnhanceMode::Strong).apply(&mut img);
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
            if x < 100 { Rgb([255u8, 0, 0]) } else { Rgb([0u8, 0, 255]) }
        });
        let halves = maybe_split_double_page(DynamicImage::ImageRgb8(img), Direction::Ltr);
        assert_eq!(halves.len(), 2);
        assert_eq!(halves[0].as_rgb8().unwrap().get_pixel(0, 0).0, [255, 0, 0]);
        assert_eq!(halves[1].as_rgb8().unwrap().get_pixel(0, 0).0, [0, 0, 255]);
    }

    #[test]
    fn double_page_split_rtl_order() {
        let img = ImageBuffer::from_fn(200, 100, |x, _| {
            if x < 100 { Rgb([255u8, 0, 0]) } else { Rgb([0u8, 0, 255]) }
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
