use super::{
    ConversionEvents, ConversionStats, ConvertError, ConvertResult, PackageSelection,
    TauriConversionEvents, convert_volume, duration_ms, resolve_edit_pages,
};
use crate::events::{ConversionCompleteEvent, VolumeCompleteEvent, VolumeStartEvent};
use crate::state::{ConvState, ConvertOptions, VolumeEdit};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tauri::{AppHandle, State};
use thasia_core::prelude::{ParsedImage, VolumeNaming, apply_naming, sanitize_filename_component};
use thasia_processor::{EncodeOptions, TransformOptions};
use thasia_source::LocalSource;

pub(crate) async fn run_tauri_conversion(
    options: ConvertOptions,
    edits: Vec<VolumeEdit>,
    state: State<'_, RwLock<ConvState>>,
    app: AppHandle,
) -> ConvertResult<()> {
    let events = TauriConversionEvents::new(&app);
    run_conversion(options, edits, &state, &events).await
}

async fn run_conversion(
    options: ConvertOptions,
    edits: Vec<VolumeEdit>,
    state: &State<'_, RwLock<ConvState>>,
    events: &dyn ConversionEvents,
) -> ConvertResult<()> {
    let start = Instant::now();
    let snapshot = conversion_state_snapshot(state)?;

    snapshot.cancel.store(false, Ordering::SeqCst);

    let scan_map: BTreeMap<u32, Vec<ParsedImage>> = snapshot.scan_result.into_iter().collect();
    let groups: Vec<(u32, Vec<ParsedImage>)> = edits
        .iter()
        .map(|edit| resolve_edit_pages(edit, &scan_map).map(|pages| (edit.volume_num, pages)))
        .collect::<Result<_, _>>()?;

    let naming = VolumeNaming {
        name: &options.output_name,
        separator: &options.volume_separator,
        hide_single_volume: options.hide_single_volume,
    };
    let plans = apply_naming(groups, &naming);
    let total_volumes = plans.len() as u32;

    let out_root = output_root(&options)?;
    create_output_dir(&out_root).await?;

    let encode_opts = encode_options(&options);
    let package = PackageSelection::from(&options);

    let mut successful = 0u32;
    let mut failed = 0u32;
    let mut conversion_stats = ConversionStats::default();
    let mut outputs = Vec::new();

    for plan in plans {
        if snapshot.cancel.load(Ordering::SeqCst) {
            break;
        }

        let vol_num = plan.volume_num;
        events.volume_started(VolumeStartEvent {
            volume_num: vol_num,
            volume_name: plan.display_name.clone(),
            total_volumes,
        });

        let result = convert_volume(
            plan,
            &out_root,
            package,
            encode_opts,
            snapshot.source.clone(),
            snapshot.cancel.clone(),
            events,
        )
        .await;

        let (success, error, output_path) = match result {
            Ok(result) => {
                successful += 1;
                conversion_stats.merge(&result.stats);
                outputs.push(result.output.clone());
                (true, None, Some(result.output.path))
            }
            Err(error) => {
                failed += 1;
                tracing::error!("Volume {} failed: {error}", vol_num);
                (false, Some(error.to_string()), None)
            }
        };

        events.volume_completed(VolumeCompleteEvent {
            volume_num: vol_num,
            success,
            error,
            output_path,
        });
    }

    events.conversion_completed(ConversionCompleteEvent {
        successful,
        failed,
        duration_secs: start.elapsed().as_secs_f64(),
        total_pages: conversion_stats.pages,
        input_bytes: conversion_stats.input_bytes,
        output_bytes: conversion_stats.output_bytes,
        passthrough_pages: conversion_stats.passthrough_pages,
        encoded_pages: conversion_stats.encoded_pages,
        fetch_ms: duration_ms(conversion_stats.fetch_time),
        decode_ms: duration_ms(conversion_stats.decode_time),
        transform_ms: duration_ms(conversion_stats.transform_time),
        encode_ms: duration_ms(conversion_stats.encode_time),
        outputs,
    });

    Ok(())
}

fn conversion_state_snapshot(
    state: &State<'_, RwLock<ConvState>>,
) -> ConvertResult<ConversionStateSnapshot> {
    let state = state.read()?;
    Ok(ConversionStateSnapshot {
        scan_result: state
            .scan_result
            .clone()
            .ok_or(ConvertError::MissingScanResult)?,
        source: state.source.clone().ok_or(ConvertError::MissingSource)?,
        cancel: state.cancel.clone(),
    })
}

fn output_root(options: &ConvertOptions) -> ConvertResult<PathBuf> {
    if options.create_directory {
        Ok(PathBuf::from(&options.output_dir)
            .join(sanitize_filename_component(&options.output_name)?))
    } else {
        Ok(PathBuf::from(&options.output_dir))
    }
}

async fn create_output_dir(path: &std::path::Path) -> ConvertResult<()> {
    tokio::fs::create_dir_all(path)
        .await
        .map_err(|source| ConvertError::CreateOutputDir {
            path: path.to_path_buf(),
            source,
        })
}

fn encode_options(options: &ConvertOptions) -> EncodeOptions {
    EncodeOptions {
        format: options.image_format,
        force_reencode: options.force_reencode,
        transform: TransformOptions {
            max_width: options.max_width,
            clean_tones: options.clean_tones,
            color_enhance: options.color_enhance,
            sharpen: options.sharpen,
            direction: options.direction,
            auto_crop: options.auto_crop,
            crop_padding: options.crop_padding,
            moire_reduction: options.moire_reduction,
            eink_dither: options.eink_dither,
            split_double_page: options.split_double_page,
        },
    }
}

struct ConversionStateSnapshot {
    scan_result: Vec<(u32, Vec<ParsedImage>)>,
    source: Arc<LocalSource>,
    cancel: Arc<AtomicBool>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use thasia_core::prelude::{
        BundleMode, ColorEnhanceMode, Direction, ImageFormat, OutputFormat, SharpenMode,
    };

    fn options() -> ConvertOptions {
        ConvertOptions {
            output_dir: "/tmp/out".into(),
            output_name: "Series 01".into(),
            create_directory: true,
            image_format: ImageFormat::Avif,
            max_width: Some(1920),
            force_reencode: true,
            clean_tones: true,
            color_enhance: ColorEnhanceMode::Balanced,
            sharpen: SharpenMode::Mild,
            output_format: OutputFormat::Cbz,
            direction: Direction::Ltr,
            bundle: BundleMode::Auto,
            volume_separator: " - ".into(),
            hide_single_volume: false,
        }
    }

    #[test]
    fn output_root_creates_sanitized_child_directory() {
        let root = output_root(&options()).expect("valid output root");
        assert!(root.ends_with("Series 01"));
    }

    #[test]
    fn output_root_rejects_invalid_child_directory_name() {
        let mut options = options();
        options.output_name = "Series: 01".into();
        assert!(output_root(&options).is_err());
    }

    #[test]
    fn output_root_uses_output_dir_directly_when_disabled() {
        let mut options = options();
        options.create_directory = false;
        let root = output_root(&options).expect("valid output root");
        assert_eq!(root, PathBuf::from("/tmp/out"));
    }

    #[test]
    fn encode_options_preserve_frontend_transform_contract() {
        let encode = encode_options(&options());
        assert_eq!(encode.format, ImageFormat::Avif);
        assert!(encode.force_reencode);
        assert_eq!(encode.transform.max_width, Some(1920));
        assert!(encode.transform.clean_tones);
        assert_eq!(encode.transform.color_enhance, ColorEnhanceMode::Balanced);
        assert_eq!(encode.transform.sharpen, SharpenMode::Mild);
    }
}
