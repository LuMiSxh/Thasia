use image::DynamicImage;
use crate::encode::grayscale::ImageTone;

pub(super) fn clean_scan_tones(img: &mut DynamicImage, tone: ImageTone) {
    if tone == ImageTone::Color {
        return;
    }
    let bp = black_threshold(tone) as f32;
    let wp = white_threshold(tone) as f32;
    let lut: [u8; 256] = std::array::from_fn(|i| smoothstep_u8(i as u8, bp, wp));

    if let Some(rgb) = img.as_mut_rgb8() {
        for px in rgb.pixels_mut() {
            let [r, g, b] = px.0;
            if !is_neutral(r, g, b) {
                continue;
            }
            let v = lut[super::luma_approx(r, g, b) as usize];
            px.0 = [v, v, v];
        }
    } else if let Some(luma) = img.as_mut_luma8() {
        for px in luma.pixels_mut() {
            px.0[0] = lut[px.0[0] as usize];
        }
    }
}

#[inline(always)]
fn smoothstep_u8(luma: u8, bp: f32, wp: f32) -> u8 {
    let t = ((luma as f32 - bp) / (wp - bp)).clamp(0.0, 1.0);
    (t * t * (3.0 - 2.0 * t) * 255.0).round() as u8
}

#[inline(always)]
fn is_neutral(r: u8, g: u8, b: u8) -> bool {
    r.abs_diff(g) <= 4 && g.abs_diff(b) <= 4 && b.abs_diff(r) <= 4
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
