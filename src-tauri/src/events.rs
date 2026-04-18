use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;

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
}

/// Emitted when a volume finishes (success or failure).
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct VolumeCompleteEvent {
    pub volume_num: u32,
    pub success: bool,
    pub error: Option<String>,
}

/// Emitted when all volumes are done.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct ConversionCompleteEvent {
    pub successful: u32,
    pub failed: u32,
    pub duration_secs: f64,
}
