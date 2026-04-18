use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::Arc;
use thasia_core::models::{Direction, ImageFormat, OutputFormat, ParsedImage};

/// Full runtime state — held in Tauri managed state, never serialized to frontend.
pub struct ConvState {
    /// Grouped scan result: (volume_num, ordered pages). Populated by scan_source.
    pub scan_result: Option<Vec<(u32, Vec<ParsedImage>)>>,
    /// Keeps the TempDir alive for ZIP/CBZ extractions.
    pub source: Option<Arc<thasia_source::LocalSource>>,
}

impl Default for ConvState {
    fn default() -> Self {
        Self { scan_result: None, source: None }
    }
}

/// Sent from frontend to the convert command.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ConvertOptions {
    pub output_dir: String,
    pub output_name: String,
    pub create_directory: bool,
    pub image_format: ImageFormat,
    pub max_width: Option<u32>,
    pub output_format: OutputFormat,
    pub direction: Direction,
    pub bundle: BundleMode,
    pub volume_separator: String,
    pub hide_single_volume: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BundleMode {
    Auto,
    Flatten,
}

/// Metadata for a single scanned page — sent to JS. No image bytes.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct PageMeta {
    pub volume_num: u32,
    pub page_index: u32,
    /// `thasia://image?path={url_encoded_absolute_path}`
    pub url: String,
    pub file_name: String,
}

/// One volume's page metadata — returned by scan_source.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct VolumeMeta {
    pub volume_num: u32,
    pub pages: Vec<PageMeta>,
}

/// One page entry from the page editor — sent from JS to convert.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct PageEditEntry {
    /// Index into the original scan_result pages for this volume.
    /// None if this is a user-added custom image.
    pub original_page_index: Option<u32>,
    /// Which scan volume to look up original_page_index in.
    /// Defaults to the parent VolumeEdit.volume_num; set explicitly when a page
    /// has been moved from a different scan volume (e.g. after manual merge).
    pub source_volume_num: Option<u32>,
    /// Set for user-added custom images; None for scanned pages.
    pub custom_path: Option<String>,
    pub excluded: bool,
}

/// The page editor's output for one volume — sent to convert.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct VolumeEdit {
    pub volume_num: u32,
    /// Pages in final display order (after drag reorder + excludes + additions).
    pub pages: Vec<PageEditEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_mode_serializes_snake_case() {
        let json = serde_json::to_string(&BundleMode::Flatten).unwrap();
        assert_eq!(json, r#""flatten""#);
        let json = serde_json::to_string(&BundleMode::Auto).unwrap();
        assert_eq!(json, r#""auto""#);
    }

    #[test]
    fn page_edit_entry_custom_image() {
        let entry = PageEditEntry {
            original_page_index: None,
            custom_path: Some("/tmp/cover.png".into()),
            excluded: false,
        };
        assert!(entry.original_page_index.is_none());
        assert!(entry.custom_path.is_some());
    }

    #[test]
    fn conv_state_default_is_empty() {
        let state = ConvState::default();
        assert!(state.scan_result.is_none());
        assert!(state.source.is_none());
    }
}
