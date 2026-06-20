use crate::encode::grayscale::ImageTone;
use image::DynamicImage;
use thasia_core::models::ColorEnhanceMode;

pub(super) fn normalize_color(img: &mut DynamicImage) {
    match img {
        DynamicImage::ImageRgb8(_) | DynamicImage::ImageRgba8(_) | DynamicImage::ImageLuma8(_) => {}
        _ if img.color().has_alpha() => *img = DynamicImage::ImageRgba8(img.to_rgba8()),
        _ => *img = DynamicImage::ImageRgb8(img.to_rgb8()),
    }
}

pub(super) fn drop_opaque_alpha(img: &mut DynamicImage) {
    let Some(rgb) = ({
        let Some(rgba) = img.as_rgba8() else { return };
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

pub(super) fn enhance_color(img: &mut DynamicImage, mode: ColorEnhanceMode, tone: ImageTone) {
    if mode == ColorEnhanceMode::Off || tone != ImageTone::Color {
        return;
    }
    let (contrast, saturation, brightness) = match mode {
        ColorEnhanceMode::Off => return,
        ColorEnhanceMode::Mild => (1.04, 1.06, 0),
        ColorEnhanceMode::Balanced => (1.08, 1.12, 2),
        ColorEnhanceMode::Strong => (1.14, 1.20, 3),
    };
    // Precompute sRGB->linear for all 256 byte values to avoid powf(2.4) per channel per pixel.
    let linear: [f32; 256] = std::array::from_fn(|i| super::srgb_to_linear(i as f32 / 255.0));
    if let Some(rgb) = img.as_mut_rgb8() {
        for px in rgb.pixels_mut() {
            let [r, g, b] = px.0;
            px.0 = enhance_channels(r, g, b, contrast, saturation, brightness, &linear);
        }
    } else if let Some(rgba) = img.as_mut_rgba8() {
        for px in rgba.pixels_mut() {
            let [r, g, b, a] = px.0;
            let [r, g, b] = enhance_channels(r, g, b, contrast, saturation, brightness, &linear);
            px.0 = [r, g, b, a];
        }
    }
}

#[inline(always)]
fn enhance_channels(
    r: u8,
    g: u8,
    b: u8,
    contrast: f32,
    saturation: f32,
    brightness: i16,
    linear: &[f32; 256],
) -> [u8; 3] {
    let (l, a, b_ok) = to_oklab(r, g, b, linear);
    let l = ((l - 0.5) * contrast + 0.5 + brightness as f32 / 255.0).clamp(0.0, 1.0);
    from_oklab(l, a * saturation, b_ok * saturation)
}

#[inline(always)]
#[allow(clippy::excessive_precision)]
fn to_oklab(r: u8, g: u8, b: u8, linear: &[f32; 256]) -> (f32, f32, f32) {
    let r = linear[r as usize];
    let g = linear[g as usize];
    let b = linear[b as usize];

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
#[allow(clippy::excessive_precision)]
fn from_oklab(l: f32, a: f32, b: f32) -> [u8; 3] {
    let l_ = l + 0.3963377774 * a + 0.2158037573 * b;
    let m_ = l - 0.1055613458 * a - 0.0638541728 * b;
    let s_ = l - 0.0894841775 * a - 1.2914855480 * b;

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    let r = super::linear_to_srgb(4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s);
    let g = super::linear_to_srgb(-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s);
    let b = super::linear_to_srgb(-0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s);

    [
        (r.clamp(0.0, 1.0) * 255.0).round() as u8,
        (g.clamp(0.0, 1.0) * 255.0).round() as u8,
        (b.clamp(0.0, 1.0) * 255.0).round() as u8,
    ]
}
