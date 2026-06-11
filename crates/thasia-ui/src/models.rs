use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thasia_core::{
    BundleMode,
    models::{ColorEnhanceMode, Direction, ImageFormat, OutputFormat, ParsedImage, SharpenMode},
};

pub type ScanGroups = Vec<(u32, Vec<ParsedImage>)>;

#[derive(Debug, Clone)]
pub struct PageMeta {
    pub volume_num: u32,
    pub page_index: u32,
    pub path: PathBuf,
    pub file_name: String,
}

#[derive(Debug, Clone)]
pub struct VolumeMeta {
    pub volume_num: u32,
    pub source_volume_num: u32,
    pub pages: Vec<PageMeta>,
}

#[derive(Debug, Clone)]
pub struct ConvertOptions {
    pub output_dir: PathBuf,
    pub output_name: String,
    pub create_directory: bool,
    pub image_format: ImageFormat,
    pub max_width: Option<u32>,
    pub force_reencode: bool,
    pub clean_tones: bool,
    pub color_enhance: ColorEnhanceMode,
    pub sharpen: SharpenMode,
    pub auto_crop: bool,
    pub crop_padding: u32,
    pub moire_reduction: bool,
    pub eink_dither: bool,
    pub split_double_page: bool,
    pub output_format: OutputFormat,
    pub direction: Direction,
    pub bundle: BundleMode,
    pub volume_separator: String,
    pub hide_single_volume: bool,
}

impl Default for ConvertOptions {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::new(),
            output_name: "output".into(),
            create_directory: false,
            image_format: ImageFormat::Avif,
            max_width: None,
            force_reencode: false,
            clean_tones: false,
            color_enhance: ColorEnhanceMode::Off,
            sharpen: SharpenMode::Off,
            auto_crop: false,
            crop_padding: 0,
            moire_reduction: false,
            eink_dither: false,
            split_double_page: false,
            output_format: OutputFormat::Cbz,
            direction: Direction::Ltr,
            bundle: BundleMode::Auto,
            volume_separator: " - ".into(),
            hide_single_volume: false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum PageEditSource {
    Original {
        page_index: u32,
        source_volume_num: Option<u32>,
    },
    Custom {
        path: PathBuf,
    },
}

#[derive(Debug, Clone)]
pub struct PageEditEntry {
    pub source: PageEditSource,
    pub excluded: bool,
}

#[derive(Debug, Clone)]
pub struct VolumeEdit {
    pub volume_num: u32,
    pub pages: Vec<PageEditEntry>,
}

#[derive(Debug, Clone)]
pub enum PipelineCostClass {
    Cheap,
    Medium,
    Expensive,
}

#[derive(Debug, Clone, Default)]
pub struct PipelineStepEffects {
    pub dimensions: bool,
    pub pixels: bool,
    pub alpha: bool,
    pub metadata: bool,
    pub passthrough: bool,
}

#[derive(Debug, Clone)]
pub struct PipelineStep {
    pub id: String,
    pub label: String,
    pub category: String,
    pub enabled: bool,
    pub default_enabled: bool,
    pub exclusive_group: Option<String>,
    pub conflicts: Vec<String>,
    pub effects: PipelineStepEffects,
    pub cost: PipelineCostClass,
}

#[derive(Debug, Clone)]
pub struct PipelineStage {
    pub id: String,
    pub label: String,
    pub enabled: bool,
    pub steps: Vec<PipelineStep>,
}

#[derive(Debug, Clone)]
pub struct PipelinePlan {
    pub total_pages: u32,
    pub included_pages: u32,
    pub excluded_pages: u32,
    pub added_pages: u32,
    pub volumes: u32,
    pub image_format: ImageFormat,
    pub output_format: OutputFormat,
    pub stages: Vec<PipelineStage>,
}

pub const DEFAULT_EXTENSION_REPO: &str =
    "https://raw.githubusercontent.com/keiyoushi/extensions/repo/index.min.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverySettings {
    pub enabled: bool,
    pub installed_version: Option<String>,
    pub auto_start: bool,
    pub last_update_check: Option<String>,
    pub download_dir: Option<PathBuf>,
    #[serde(default = "default_extension_repos")]
    pub extension_repos: Vec<String>,
}

impl Default for DiscoverySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            installed_version: None,
            auto_start: false,
            last_update_check: None,
            download_dir: None,
            extension_repos: default_extension_repos(),
        }
    }
}

fn default_extension_repos() -> Vec<String> {
    vec![DEFAULT_EXTENSION_REPO.to_string()]
}

#[derive(Debug, Clone)]
pub struct ConversionOutput {
    pub volume_num: u32,
    pub volume_name: String,
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct ConversionSummary {
    pub successful: u32,
    pub failed: u32,
    pub duration_secs: f64,
    pub total_pages: u32,
    pub input_bytes: u64,
    pub output_bytes: u64,
    pub passthrough_pages: u32,
    pub encoded_pages: u32,
    pub fetch_ms: f64,
    pub decode_ms: f64,
    pub transform_ms: f64,
    pub encode_ms: f64,
    pub outputs: Vec<ConversionOutput>,
}

#[derive(Debug, Clone)]
pub enum ConversionEvent {
    VolumeStarted {
        volume_num: u32,
        volume_name: String,
        total_volumes: u32,
    },
    ImageProgress {
        volume_num: u32,
        current: u32,
        total: u32,
        elapsed_secs: f64,
        pages_per_sec: f64,
        estimated_remaining_secs: Option<f64>,
        input_bytes: u64,
        output_bytes: u64,
        passthrough_pages: u32,
        encoded_pages: u32,
        fetch_ms: f64,
        decode_ms: f64,
        transform_ms: f64,
        encode_ms: f64,
    },
    VolumeCompleted {
        volume_num: u32,
        success: bool,
        error: Option<String>,
        output_path: Option<String>,
    },
}
