use super::{
    page_edits::resolve_edit_pages,
    volume::{ConversionStats, PackageSelection, convert_volume, duration_ms},
};
use crate::{
    error::{AppError, AppResult},
    models::{ConversionEvent, ConversionOutput, ConversionSummary, ConvertOptions, VolumeEdit},
    state::SharedConvState,
};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, atomic::Ordering},
    time::Instant,
};
use thasia_core::prelude::{ParsedImage, VolumeNaming, apply_naming, sanitize_filename_component};
use thasia_processor::{EncodeOptions, TransformOptions};

pub async fn run_conversion(
    options: ConvertOptions,
    edits: Vec<VolumeEdit>,
    state: SharedConvState,
    on_event: Arc<dyn Fn(ConversionEvent) + Send + Sync>,
) -> AppResult<ConversionSummary> {
    let started = Instant::now();
    let snapshot = {
        let state = state.read().map_err(|error| error.to_string())?;
        (
            state
                .scan_result
                .clone()
                .ok_or(AppError::MissingScanResult)?,
            state.source.clone().ok_or(AppError::MissingSource)?,
            state.cancel.clone(),
        )
    };
    snapshot.2.store(false, Ordering::SeqCst);

    let scan_map: BTreeMap<u32, Vec<ParsedImage>> = snapshot.0.into_iter().collect();
    let groups = edits
        .iter()
        .map(|edit| resolve_edit_pages(edit, &scan_map).map(|pages| (edit.volume_num, pages)))
        .collect::<AppResult<Vec<_>>>()?;
    let naming = VolumeNaming {
        name: &options.output_name,
        separator: &options.volume_separator,
        hide_single_volume: options.hide_single_volume,
    };
    let plans = apply_naming(groups, &naming);
    let total_volumes = plans.len() as u32;
    let output_root = output_root(&options)?;
    tokio::fs::create_dir_all(&output_root)
        .await
        .map_err(|source| AppError::CreateOutputDir {
            path: output_root.clone(),
            source,
        })?;

    let package = PackageSelection::from(&options);
    let encode = encode_options(&options);
    let mut successful = 0;
    let mut failed = 0;
    let mut stats = ConversionStats::default();
    let mut outputs: Vec<ConversionOutput> = Vec::new();

    for plan in plans {
        if snapshot.2.load(Ordering::SeqCst) {
            break;
        }
        let volume_num = plan.volume_num;
        on_event(ConversionEvent::VolumeStarted {
            volume_num,
            volume_name: plan.display_name.clone(),
            total_volumes,
        });
        let result = convert_volume(
            plan,
            &output_root,
            package,
            encode,
            snapshot.1.clone(),
            snapshot.2.clone(),
            on_event.as_ref(),
        )
        .await;
        match result {
            Ok(result) => {
                successful += 1;
                stats.merge(&result.stats);
                let output_path = result.output.path.clone();
                outputs.push(result.output);
                on_event(ConversionEvent::VolumeCompleted {
                    volume_num,
                    success: true,
                    error: None,
                    output_path: Some(output_path),
                });
            }
            Err(error) => {
                failed += 1;
                on_event(ConversionEvent::VolumeCompleted {
                    volume_num,
                    success: false,
                    error: Some(error.to_string()),
                    output_path: None,
                });
            }
        }
    }

    Ok(ConversionSummary {
        successful,
        failed,
        duration_secs: started.elapsed().as_secs_f64(),
        total_pages: stats.pages,
        input_bytes: stats.input_bytes,
        output_bytes: stats.output_bytes,
        passthrough_pages: stats.passthrough_pages,
        encoded_pages: stats.encoded_pages,
        fetch_ms: duration_ms(stats.fetch_time),
        decode_ms: duration_ms(stats.decode_time),
        transform_ms: duration_ms(stats.transform_time),
        encode_ms: duration_ms(stats.encode_time),
        outputs,
    })
}

pub fn cancel(state: &SharedConvState) {
    if let Ok(state) = state.read() {
        state.cancel.store(true, Ordering::SeqCst);
    }
}

fn output_root(options: &ConvertOptions) -> AppResult<PathBuf> {
    if options.create_directory {
        Ok(options
            .output_dir
            .join(sanitize_filename_component(&options.output_name)?))
    } else {
        Ok(options.output_dir.clone())
    }
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
