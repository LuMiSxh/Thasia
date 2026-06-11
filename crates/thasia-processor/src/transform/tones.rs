use image::DynamicImage;
use crate::encode::grayscale::{ImageTone, classify_image_tone};

pub(super) fn clean_scan_tones(img: &mut DynamicImage) {
    let tone = classify_image_tone(img);
    if tone == ImageTone::Color {
        return;
    }
    if let Some(rgb) = img.as_mut_rgb8() {
        let bp = black_threshold(tone) as f32;
        let wp = white_threshold(tone) as f32;
        for px in rgb.pixels_mut() {
            let [r, g, b] = px.0;
            if !is_neutral(r, g, b) {
                continue;
            }
            let v = smoothstep_u8(luma_approx(r, g, b), bp, wp);
            px.0 = [v, v, v];
        }
    } else if let Some(luma) = img.as_mut_luma8() {
        let bp = black_threshold(tone) as f32;
        let wp = white_threshold(tone) as f32;
        for px in luma.pixels_mut() {
            px.0[0] = smoothstep_u8(px.0[0], bp, wp);
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
