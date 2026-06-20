use crate::{
    encode::{convert_to_avif, convert_to_webp},
    error::{ProcessorError, Result as ProcessorResult},
    transform::{TransformPipeline, maybe_split_double_page},
};
use std::time::{Duration, Instant};
use thasia_core::prelude::{ImageFormat, ProcessedImage, ProcessingStats};
use thasia_source::prelude::FetchedImage;

pub(super) fn encode_images(
    raw: super::RawImage,
    opts: super::EncodeOptions,
) -> Vec<ProcessorResult<ProcessedImage>> {
    let fetch_duration = raw.fetch_duration;
    let original_ext = raw.original_ext.clone();
    let base_parsed = raw.parsed;

    if !opts.transform.split_double_page || opts.format == ImageFormat::Original {
        return vec![
            encode_image(raw.bytes, &original_ext, opts, fetch_duration).map(|enc| ProcessedImage {
                parsed_data: base_parsed,
                image_data: enc.data,
                ext: enc.ext,
                width: enc.width,
                height: enc.height,
                stats: enc.stats,
            }),
        ];
    }

    let decode_start = Instant::now();
    let img = match decode_image(raw.bytes.as_ref(), &original_ext) {
        Ok(img) => img,
        Err(e) => return vec![Err(e)],
    };
    let decode_ms = super::duration_ms(decode_start.elapsed());
    drop(raw.bytes);

    let split_start = Instant::now();
    let halves = maybe_split_double_page(img, opts.transform.direction);
    let split_ms = super::duration_ms(split_start.elapsed());

    if halves.len() == 1 {
        let img = halves.into_iter().next().unwrap();
        let result = encode_decoded_image(img, opts, fetch_duration, decode_ms, split_ms)
            .map(|enc| ProcessedImage {
                parsed_data: base_parsed,
                image_data: enc.data,
                ext: enc.ext,
                width: enc.width,
                height: enc.height,
                stats: enc.stats,
            });
        return vec![result];
    }

    halves
        .into_iter()
        .enumerate()
        .map(|(i, half)| {
            let mut parsed = base_parsed.clone();
            if i == 1 {
                parsed.page_number += 0.5;
            }
            encode_decoded_image(half, opts, fetch_duration, decode_ms, split_ms).map(|enc| {
                ProcessedImage {
                    parsed_data: parsed,
                    image_data: enc.data,
                    ext: enc.ext,
                    width: enc.width,
                    height: enc.height,
                    stats: enc.stats,
                }
            })
        })
        .collect()
}

fn encode_image(
    bytes: FetchedImage,
    original_ext: &str,
    opts: super::EncodeOptions,
    fetch_duration: Duration,
) -> ProcessorResult<super::EncodedImage> {
    let total_start = Instant::now();
    let input_bytes = bytes.len() as u64;

    if opts.format == ImageFormat::Original {
        let (w, h) = super::image_dimensions_best_effort(bytes.as_ref());
        let bytes = bytes.into_vec();
        let output_bytes = bytes.len() as u64;
        return Ok(super::EncodedImage {
            data: bytes,
            ext: original_ext.to_string(),
            width: w,
            height: h,
            stats: ProcessingStats {
                input_bytes,
                output_bytes,
                passthrough: true,
                fetch_ms: super::duration_ms(fetch_duration),
                total_ms: super::duration_ms(total_start.elapsed()),
                ..ProcessingStats::default()
            },
        });
    }

    if super::can_passthrough(original_ext, opts) {
        let (w, h) = super::image_dimensions_best_effort(bytes.as_ref());
        let bytes = bytes.into_vec();
        let output_bytes = bytes.len() as u64;
        return Ok(super::EncodedImage {
            data: bytes,
            ext: super::target_extension(opts.format).to_string(),
            width: w,
            height: h,
            stats: ProcessingStats {
                input_bytes,
                output_bytes,
                passthrough: true,
                fetch_ms: super::duration_ms(fetch_duration),
                total_ms: super::duration_ms(total_start.elapsed()),
                ..ProcessingStats::default()
            },
        });
    }

    let decode_start = Instant::now();
    let img = decode_image(bytes.as_ref(), original_ext)?;
    let decode_ms = super::duration_ms(decode_start.elapsed());
    drop(bytes);

    encode_decoded_image(img, opts, fetch_duration, decode_ms, 0.0).map(|mut enc| {
        enc.stats.input_bytes = input_bytes;
        enc
    })
}

fn encode_decoded_image(
    mut img: image::DynamicImage,
    opts: super::EncodeOptions,
    fetch_duration: Duration,
    prior_decode_ms: f64,
    prior_transform_ms: f64,
) -> ProcessorResult<super::EncodedImage> {
    let total_start = Instant::now();

    let transform_start = Instant::now();
    let tone = TransformPipeline::new(opts.transform).apply(&mut img);
    let transform_ms = prior_transform_ms + super::duration_ms(transform_start.elapsed());

    let (w, h) = (img.width(), img.height());

    let encode_start = Instant::now();
    let (data, ext) = match opts.format {
        ImageFormat::Avif => (convert_to_avif(&img, tone)?, "avif".to_string()),
        ImageFormat::Webp => (convert_to_webp(&img, tone)?, "webp".to_string()),
        ImageFormat::Original => {
            return Err(ProcessorError::UnsupportedFormat(
                "original output can only be used via passthrough",
            ));
        }
    };
    let encode_ms = super::duration_ms(encode_start.elapsed());
    let output_bytes = data.len() as u64;

    Ok(super::EncodedImage {
        data,
        ext,
        width: w,
        height: h,
        stats: ProcessingStats {
            input_bytes: 0,
            output_bytes,
            passthrough: false,
            fetch_ms: super::duration_ms(fetch_duration),
            decode_ms: prior_decode_ms,
            transform_ms,
            encode_ms,
            total_ms: super::duration_ms(total_start.elapsed()),
        },
    })
}

pub(super) fn decode_image(
    bytes: &[u8],
    original_ext: &str,
) -> ProcessorResult<image::DynamicImage> {
    if super::normalize_ext(original_ext) == "avif" {
        return decode_avif(bytes);
    }
    image::load_from_memory(bytes).map_err(ProcessorError::decode)
}

fn decode_avif(bytes: &[u8]) -> ProcessorResult<image::DynamicImage> {
    use avif_decode::{Decoder, Image};
    use image::{DynamicImage, ImageBuffer};

    let decoded = Decoder::from_avif(bytes)
        .and_then(Decoder::to_image)
        .map_err(ProcessorError::avif_decode)?;

    match decoded {
        Image::Rgb8(img) => {
            let (pixels, width, height) = img.into_contiguous_buf();
            let raw = pixels
                .into_iter()
                .flat_map(|px| [px.r, px.g, px.b])
                .collect::<Vec<_>>();
            ImageBuffer::from_vec(width as u32, height as u32, raw)
                .map(DynamicImage::ImageRgb8)
                .ok_or(ProcessorError::InvalidBuffer("AVIF RGB"))
        }
        Image::Rgba8(img) => {
            let (pixels, width, height) = img.into_contiguous_buf();
            let raw = pixels
                .into_iter()
                .flat_map(|px| [px.r, px.g, px.b, px.a])
                .collect::<Vec<_>>();
            ImageBuffer::from_vec(width as u32, height as u32, raw)
                .map(DynamicImage::ImageRgba8)
                .ok_or(ProcessorError::InvalidBuffer("AVIF RGBA"))
        }
        Image::Gray8(img) => {
            let (pixels, width, height) = img.into_contiguous_buf();
            let raw = pixels.into_iter().map(|px| px.value()).collect::<Vec<_>>();
            ImageBuffer::from_vec(width as u32, height as u32, raw)
                .map(DynamicImage::ImageLuma8)
                .ok_or(ProcessorError::InvalidBuffer("AVIF gray"))
        }
        Image::Rgb16(img) => {
            let (pixels, width, height) = img.into_contiguous_buf();
            let raw = pixels
                .into_iter()
                .flat_map(|px| [px.r, px.g, px.b])
                .collect::<Vec<_>>();
            ImageBuffer::from_vec(width as u32, height as u32, raw)
                .map(DynamicImage::ImageRgb16)
                .ok_or(ProcessorError::InvalidBuffer("AVIF RGB16"))
        }
        Image::Rgba16(img) => {
            let (pixels, width, height) = img.into_contiguous_buf();
            let raw = pixels
                .into_iter()
                .flat_map(|px| [px.r, px.g, px.b, px.a])
                .collect::<Vec<_>>();
            ImageBuffer::from_vec(width as u32, height as u32, raw)
                .map(DynamicImage::ImageRgba16)
                .ok_or(ProcessorError::InvalidBuffer("AVIF RGBA16"))
        }
        Image::Gray16(img) => {
            let (pixels, width, height) = img.into_contiguous_buf();
            let raw = pixels.into_iter().map(|px| px.value()).collect::<Vec<_>>();
            ImageBuffer::from_vec(width as u32, height as u32, raw)
                .map(DynamicImage::ImageLuma16)
                .ok_or(ProcessorError::InvalidBuffer("AVIF gray16"))
        }
    }
}
