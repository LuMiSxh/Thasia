use image::DynamicImage;

pub(super) fn resize_max_width(img: &mut DynamicImage, max_width: u32) {
    let (width, height) = (img.width(), img.height());
    if width <= max_width {
        return;
    }
    let new_height = ((height as f64 * max_width as f64 / width as f64).round() as u32).max(1);

    // Precompute 256-entry LUTs so only 256 powf calls happen (not one per pixel).
    let to_linear = build_lut(super::srgb_to_linear);
    let to_srgb = build_lut(super::linear_to_srgb);

    apply_lut(img, &to_linear, false);
    *img = img.resize_exact(max_width, new_height, image::imageops::FilterType::Lanczos3);
    apply_lut(img, &to_srgb, false);
}

fn build_lut(f: fn(f32) -> f32) -> [u8; 256] {
    let mut lut = [0u8; 256];
    for (i, entry) in lut.iter_mut().enumerate() {
        *entry = (f(i as f32 / 255.0) * 255.0).round() as u8;
    }
    lut
}

fn apply_lut(img: &mut DynamicImage, lut: &[u8; 256], skip_alpha: bool) {
    match img {
        DynamicImage::ImageRgb8(buf) => {
            for c in buf.as_flat_samples_mut().samples.iter_mut() {
                *c = lut[*c as usize];
            }
        }
        DynamicImage::ImageRgba8(buf) => {
            for px in buf.as_flat_samples_mut().samples.chunks_exact_mut(4) {
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
