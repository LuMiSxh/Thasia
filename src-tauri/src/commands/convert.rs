use crate::app_error::CommandResult;
use crate::conversion::{
    ConversionStats, ConvertError, ConvertResult, PackageSelection, convert_volume, duration_ms,
    resolve_edit_pages,
};
use crate::events::{ConversionCompleteEvent, VolumeCompleteEvent, VolumeStartEvent};
use crate::state::{ConvState, ConvertOptions, PipelinePlan, VolumeEdit};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use tauri::{AppHandle, State};
use tauri_specta::Event;
use thasia_core::prelude::{ParsedImage, VolumeNaming, apply_naming, sanitize_filename_component};
use thasia_processor::{EncodeOptions, TransformOptions};
use thasia_source::LocalSource;

#[tauri::command]
#[specta::specta]
pub fn build_pipeline_plan(
    options: ConvertOptions,
    edits: Vec<VolumeEdit>,
) -> CommandResult<PipelinePlan> {
    Ok(crate::pipeline_plan::build(options, edits))
}

#[tauri::command]
#[specta::specta]
pub async fn convert(
    options: ConvertOptions,
    edits: Vec<VolumeEdit>,
    state: State<'_, RwLock<ConvState>>,
    app: AppHandle,
) -> CommandResult<()> {
    convert_inner(options, edits, state, app).await?;
    Ok(())
}

async fn convert_inner(
    options: ConvertOptions,
    edits: Vec<VolumeEdit>,
    state: State<'_, RwLock<ConvState>>,
    app: AppHandle,
) -> ConvertResult<()> {
    let start = Instant::now();

    let snapshot = conversion_state_snapshot(&state)?;

    // Reset cancellation for this run.
    snapshot.cancel.store(false, Ordering::SeqCst);

    // Build lookup: scan_volume_num → pages
    let scan_map: BTreeMap<u32, Vec<ParsedImage>> = snapshot.scan_result.into_iter().collect();

    // Resolve each VolumeEdit into a concrete (volume_num, Vec<ParsedImage>) group.
    let groups: Vec<(u32, Vec<ParsedImage>)> = edits
        .iter()
        .map(|edit| resolve_edit_pages(edit, &scan_map).map(|pages| (edit.volume_num, pages)))
        .collect::<Result<_, _>>()?;

    // Build the conversion plan (apply naming).
    let naming = VolumeNaming {
        name: &options.output_name,
        separator: &options.volume_separator,
        hide_single_volume: options.hide_single_volume,
    };
    let plans = apply_naming(groups, &naming);
    let total_volumes = plans.len() as u32;

    let out_root = if options.create_directory {
        PathBuf::from(&options.output_dir).join(sanitize_filename_component(&options.output_name)?)
    } else {
        PathBuf::from(&options.output_dir)
    };
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
        VolumeStartEvent {
            volume_num: vol_num,
            volume_name: plan.display_name.clone(),
            total_volumes,
        }
        .emit(&app)
        .ok();

        let result = convert_volume(
            plan,
            &out_root,
            package,
            encode_opts,
            snapshot.source.clone(),
            snapshot.cancel.clone(),
            &app,
        )
        .await;

        let (success, error, output_path) = match result {
            Ok(result) => {
                successful += 1;
                conversion_stats.merge(&result.stats);
                outputs.push(result.output.clone());
                (true, None, Some(result.output.path))
            }
            Err(e) => {
                failed += 1;
                tracing::error!("Volume {} failed: {e}", vol_num);
                (false, Some(e.to_string()), None)
            }
        };
        let _ = VolumeCompleteEvent {
            volume_num: vol_num,
            success,
            error,
            output_path,
        }
        .emit(&app);
    }

    ConversionCompleteEvent {
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
    }
    .emit(&app)
    .ok();

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
        },
    }
}

struct ConversionStateSnapshot {
    scan_result: Vec<(u32, Vec<ParsedImage>)>,
    source: Arc<LocalSource>,
    cancel: Arc<AtomicBool>,
}
