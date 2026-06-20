use image::DynamicImage;

const BILATERAL_RADIUS: i32 = 2;
const BILATERAL_SIGMA_SPACE: f32 = 1.5;
const BILATERAL_SIGMA_RANGE: f32 = 30.0;

const CROP_BG_LUMA: u8 = 235;
const CROP_BG_ROW_RATIO: f32 = 0.97;

pub(super) fn moire_reduction(img: &mut DynamicImage) {
    let (w, h) = (img.width(), img.height());
    if w < 3 || h < 3 {
        return;
    }
    let r = BILATERAL_RADIUS;
    let diam = (2 * r + 1) as usize;
    let mut spatial = Vec::with_capacity(diam * diam);
    for dy in -r..=r {
        for dx in -r..=r {
            let d2 = (dx * dx + dy * dy) as f32;
            spatial.push((-d2 / (2.0 * BILATERAL_SIGMA_SPACE * BILATERAL_SIGMA_SPACE)).exp());
        }
    }
    let range_coeff = -1.0 / (2.0 * BILATERAL_SIGMA_RANGE * BILATERAL_SIGMA_RANGE);

    // Pixel diffs are bounded [-255, 255], so precompute all 511 range weights
    // to avoid calling exp() in the innermost pixel loop.
    let mut range_lut = [0.0f32; 511];
    for (i, entry) in range_lut.iter_mut().enumerate() {
        let diff = i as i32 - 255;
        *entry = ((diff * diff) as f32 * range_coeff).exp();
    }

    match img {
        DynamicImage::ImageRgb8(buf) => bilateral_rgb(buf, w, h, r, &spatial, &range_lut),
        DynamicImage::ImageLuma8(buf) => bilateral_luma(buf, w, h, r, &spatial, &range_lut),
        _ => {}
    }
}

fn bilateral_rgb(
    buf: &mut image::RgbImage,
    w: u32,
    h: u32,
    r: i32,
    spatial: &[f32],
    range_lut: &[f32; 511],
) {
    let input = buf.clone();
    let raw_in = input.as_raw();
    let data = buf.as_flat_samples_mut().samples;
    for y in 0..h {
        for x in 0..w {
            let cb = (y as usize * w as usize + x as usize) * 3;
            let cl = ((raw_in[cb] as u16 * 77 + raw_in[cb + 1] as u16 * 150 + raw_in[cb + 2] as u16 * 29) >> 8) as i32;
            let (mut sw, mut sr, mut sg, mut sb) = (0.0f32, 0.0, 0.0, 0.0);
            let mut ki = 0;
            for dy in -r..=r {
                for dx in -r..=r {
                    let nx = (x as i32 + dx).clamp(0, w as i32 - 1) as u32;
                    let ny = (y as i32 + dy).clamp(0, h as i32 - 1) as u32;
                    let base = (ny as usize * w as usize + nx as usize) * 3;
                    let nl = ((raw_in[base] as u16 * 77 + raw_in[base + 1] as u16 * 150 + raw_in[base + 2] as u16 * 29) >> 8) as i32;
                    let wt = spatial[ki] * range_lut[(nl - cl + 255) as usize];
                    sr += wt * raw_in[base] as f32;
                    sg += wt * raw_in[base + 1] as f32;
                    sb += wt * raw_in[base + 2] as f32;
                    sw += wt;
                    ki += 1;
                }
            }
            data[cb] = (sr / sw).round() as u8;
            data[cb + 1] = (sg / sw).round() as u8;
            data[cb + 2] = (sb / sw).round() as u8;
        }
    }
}

fn bilateral_luma(
    buf: &mut image::GrayImage,
    w: u32,
    h: u32,
    r: i32,
    spatial: &[f32],
    range_lut: &[f32; 511],
) {
    let input = buf.clone();
    let raw = buf.as_flat_samples_mut().samples;
    for y in 0..h {
        for x in 0..w {
            let center = input.get_pixel(x, y).0[0] as i32;
            let (mut sw, mut sv) = (0.0f32, 0.0f32);
            let mut ki = 0;
            for dy in -r..=r {
                for dx in -r..=r {
                    let nx = (x as i32 + dx).clamp(0, w as i32 - 1) as u32;
                    let ny = (y as i32 + dy).clamp(0, h as i32 - 1) as u32;
                    let nv = input.get_pixel(nx, ny).0[0] as i32;
                    let wt = spatial[ki] * range_lut[(nv - center + 255) as usize];
                    sv += wt * nv as f32;
                    sw += wt;
                    ki += 1;
                }
            }
            raw[(y * w + x) as usize] = (sv / sw).round() as u8;
        }
    }
}

pub(super) fn auto_crop(img: &mut DynamicImage, padding: u32) {
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return;
    }

    // Borrow luma data without allocating when the image is already grayscale.
    // The block scope ensures the borrow ends before we mutate img below.
    let (top, bottom, left, right) = {
        let luma_owned;
        let luma: &image::GrayImage = if let Some(l) = img.as_luma8() {
            l
        } else {
            luma_owned = img.to_luma8();
            &luma_owned
        };
        let raw = luma.as_raw();

        let row_bg = |y: u32| -> bool {
            let start = (y * w) as usize;
            raw[start..start + w as usize]
                .iter()
                .filter(|&&p| p >= CROP_BG_LUMA)
                .count() as f32
                / w as f32
                >= CROP_BG_ROW_RATIO
        };
        let col_bg = |x: u32| -> bool {
            (0..h)
                .filter(|&y| raw[(y * w + x) as usize] >= CROP_BG_LUMA)
                .count() as f32
                / h as f32
                >= CROP_BG_ROW_RATIO
        };

        let top = (0..h).find(|&y| !row_bg(y)).unwrap_or(0);
        let bottom = (0..h).rev().find(|&y| !row_bg(y)).map(|y| y + 1).unwrap_or(h);
        let left = (0..w).find(|&x| !col_bg(x)).unwrap_or(0);
        let right = (0..w).rev().find(|&x| !col_bg(x)).map(|x| x + 1).unwrap_or(w);

        (top, bottom, left, right)
    };

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
    *img = img.crop_imm(x, y, crop_w, crop_h);
}

pub(super) fn eink_dither(img: &mut DynamicImage) {
    let (w, h) = (img.width(), img.height());
    let mut gray = img.to_luma8();
    let raw = gray.as_flat_samples_mut().samples;
    let mut buf: Vec<i16> = raw.iter().map(|&v| v as i16).collect();

    const LEVELS: i16 = 16;
    const STEP: i16 = 255 / (LEVELS - 1); // 17

    for y in 0..h as usize {
        for x in 0..w as usize {
            let old = buf[y * w as usize + x].clamp(0, 255);
            let new = ((old + STEP / 2) / STEP * STEP).clamp(0, 255);
            let err = old - new;
            buf[y * w as usize + x] = new;
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
