use crate::state::{
    ConvertOptions, PageEditSource, PipelineCostClass, PipelinePlan, PipelineStage, PipelineStep,
    PipelineStepEffects, VolumeEdit,
};
use thasia_core::{
    OutputFormat,
    models::{ColorEnhanceMode, ImageFormat, SharpenMode},
};

pub fn build(options: ConvertOptions, edits: Vec<VolumeEdit>) -> PipelinePlan {
    let total_pages: u32 = edits.iter().map(|edit| edit.pages.len() as u32).sum();
    let excluded_pages = edits
        .iter()
        .flat_map(|edit| &edit.pages)
        .filter(|page| page.excluded)
        .count() as u32;
    let included_pages = total_pages.saturating_sub(excluded_pages);
    let added_pages = edits
        .iter()
        .flat_map(|edit| &edit.pages)
        .filter(|page| matches!(page.source, PageEditSource::Custom { .. }) && !page.excluded)
        .count() as u32;

    PipelinePlan {
        total_pages,
        included_pages,
        excluded_pages,
        added_pages,
        volumes: edits.len() as u32,
        image_format: options.image_format,
        output_format: options.output_format,
        stages: stages(&options),
    }
}

fn stages(options: &ConvertOptions) -> Vec<PipelineStage> {
    let reencodes = options.image_format != ImageFormat::Original;
    vec![
        PipelineStage {
            id: "input".into(),
            label: "Input".into(),
            enabled: true,
            steps: vec![step(
                "read-source",
                "Read selected pages",
                "source",
                true,
                true,
                PipelineCostClass::Cheap,
                PipelineStepEffects::default(),
            )],
        },
        decode_stage(options, reencodes),
        transform_stage(options, reencodes),
        encode_stage(options, reencodes),
        package_stage(options),
    ]
}

fn decode_stage(options: &ConvertOptions, reencodes: bool) -> PipelineStage {
    PipelineStage {
        id: "decode".into(),
        label: "Decode".into(),
        enabled: reencodes,
        steps: vec![
            step(
                "passthrough",
                "Pass through matching files",
                "decode",
                reencodes
                    && !options.force_reencode
                    && options.max_width.is_none()
                    && !options.clean_tones
                    && options.color_enhance == ColorEnhanceMode::Off
                    && options.sharpen == SharpenMode::Off,
                true,
                PipelineCostClass::Cheap,
                PipelineStepEffects {
                    passthrough: true,
                    ..PipelineStepEffects::default()
                },
            ),
            step(
                "decode-image",
                "Decode when needed",
                "decode",
                reencodes,
                true,
                PipelineCostClass::Medium,
                PipelineStepEffects {
                    pixels: true,
                    ..PipelineStepEffects::default()
                },
            ),
        ],
    }
}

fn transform_stage(options: &ConvertOptions, reencodes: bool) -> PipelineStage {
    PipelineStage {
        id: "transform".into(),
        label: "Transform".into(),
        enabled: reencodes,
        steps: vec![
            step(
                "normalize-color",
                "Normalize color buffers",
                "cleanup",
                reencodes,
                true,
                PipelineCostClass::Cheap,
                PipelineStepEffects {
                    pixels: true,
                    ..PipelineStepEffects::default()
                },
            ),
            step(
                "drop-opaque-alpha",
                "Drop unused alpha",
                "cleanup",
                reencodes,
                true,
                PipelineCostClass::Cheap,
                PipelineStepEffects {
                    alpha: true,
                    ..PipelineStepEffects::default()
                },
            ),
            step(
                "resize-max-width",
                "Downscale max width",
                "geometry",
                reencodes && options.max_width.is_some(),
                false,
                PipelineCostClass::Medium,
                PipelineStepEffects {
                    dimensions: true,
                    pixels: true,
                    passthrough: true,
                    ..PipelineStepEffects::default()
                },
            ),
            step(
                "clean-scan-tones",
                "Clean scan tones",
                "cleanup",
                reencodes && options.clean_tones,
                false,
                PipelineCostClass::Medium,
                PipelineStepEffects {
                    pixels: true,
                    passthrough: true,
                    ..PipelineStepEffects::default()
                },
            ),
            step(
                "enhance-color",
                color_enhance_label(options.color_enhance),
                "enhance",
                reencodes && options.color_enhance != ColorEnhanceMode::Off,
                false,
                PipelineCostClass::Medium,
                PipelineStepEffects {
                    pixels: true,
                    passthrough: true,
                    ..PipelineStepEffects::default()
                },
            ),
            step(
                "sharpen",
                sharpen_label(options.sharpen),
                "enhance",
                reencodes && options.sharpen != SharpenMode::Off,
                false,
                PipelineCostClass::Medium,
                PipelineStepEffects {
                    pixels: true,
                    passthrough: true,
                    ..PipelineStepEffects::default()
                },
            ),
        ],
    }
}

fn encode_stage(options: &ConvertOptions, reencodes: bool) -> PipelineStage {
    PipelineStage {
        id: "encode".into(),
        label: "Encode".into(),
        enabled: true,
        steps: vec![
            step(
                "strip-metadata",
                "Strip metadata through encode",
                "encode",
                reencodes,
                true,
                PipelineCostClass::Cheap,
                PipelineStepEffects {
                    metadata: true,
                    ..PipelineStepEffects::default()
                },
            ),
            step(
                "encode-output",
                encode_label(options),
                "encode",
                true,
                true,
                if options.image_format == ImageFormat::Avif {
                    PipelineCostClass::Expensive
                } else {
                    PipelineCostClass::Medium
                },
                PipelineStepEffects {
                    pixels: reencodes,
                    ..PipelineStepEffects::default()
                },
            ),
        ],
    }
}

fn package_stage(options: &ConvertOptions) -> PipelineStage {
    PipelineStage {
        id: "package".into(),
        label: "Package".into(),
        enabled: true,
        steps: vec![step(
            "write-container",
            package_label(options),
            "package",
            true,
            true,
            PipelineCostClass::Medium,
            PipelineStepEffects::default(),
        )],
    }
}

fn step(
    id: &str,
    label: impl Into<String>,
    category: &str,
    enabled: bool,
    default_enabled: bool,
    cost: PipelineCostClass,
    effects: PipelineStepEffects,
) -> PipelineStep {
    PipelineStep {
        id: id.into(),
        label: label.into(),
        category: category.into(),
        enabled,
        default_enabled,
        exclusive_group: None,
        conflicts: Vec::new(),
        effects,
        cost,
    }
}

fn encode_label(options: &ConvertOptions) -> &'static str {
    match options.image_format {
        ImageFormat::Avif => "Encode AVIF",
        ImageFormat::Webp => "Encode WebP",
        ImageFormat::Original => "Keep original files",
    }
}

fn package_label(options: &ConvertOptions) -> &'static str {
    match options.output_format {
        OutputFormat::Cbz => "Write CBZ",
        OutputFormat::Epub => "Write EPUB",
        OutputFormat::Raw => "Write raw folder",
    }
}

fn color_enhance_label(mode: ColorEnhanceMode) -> &'static str {
    match mode {
        ColorEnhanceMode::Off => "Enhance color",
        ColorEnhanceMode::Mild => "Enhance color mildly",
        ColorEnhanceMode::Balanced => "Enhance color balanced",
        ColorEnhanceMode::Strong => "Enhance color strongly",
    }
}

fn sharpen_label(mode: SharpenMode) -> &'static str {
    match mode {
        SharpenMode::Off => "Sharpen",
        SharpenMode::Mild => "Sharpen mildly",
    }
}
