use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;
use thasia_source::suwayomi::{InstallProgress, RuntimeState};

/// Emitted each time one image finishes scanning (for large archives).
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct ScanProgressEvent {
    pub current: u32,
    pub total: u32,
}

/// Emitted when a volume starts converting.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct VolumeStartEvent {
    pub volume_num: u32,
    pub volume_name: String,
    pub total_volumes: u32,
}

/// Emitted after each image is encoded and written.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct ImageProgressEvent {
    pub volume_num: u32,
    pub current: u32,
    pub total: u32,
    pub elapsed_secs: f64,
    pub pages_per_sec: f64,
    pub estimated_remaining_secs: Option<f64>,
    pub input_bytes: u64,
    pub output_bytes: u64,
    pub passthrough_pages: u32,
    pub encoded_pages: u32,
    pub fetch_ms: f64,
    pub decode_ms: f64,
    pub transform_ms: f64,
    pub encode_ms: f64,
}

/// Emitted when a volume finishes (success or failure).
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct VolumeCompleteEvent {
    pub volume_num: u32,
    pub success: bool,
    pub error: Option<String>,
    pub output_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ConversionOutput {
    pub volume_num: u32,
    pub volume_name: String,
    pub path: String,
}

/// Emitted when all volumes are done.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct ConversionCompleteEvent {
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

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct SuwayomiStateChangedEvent {
    pub state: RuntimeState,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct SuwayomiInstallProgressEvent {
    pub progress: InstallProgress,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct DownloadStartEvent {
    pub series_title: String,
    pub total_chapters: u32,
}

/// Phase of an in-progress chapter download batch.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ChapterDownloadPhase {
    /// Waiting for the next chapter to finish downloading.
    Downloading,
    /// A single chapter finished downloading.
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct ChapterDownloadEvent {
    pub current_chapter: String,
    pub current: u32,
    pub total: u32,
    pub phase: ChapterDownloadPhase,
    pub tick: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct DownloadCompleteEvent {
    pub success: bool,
    pub error: Option<String>,
    pub output_dir: Option<String>,
}
