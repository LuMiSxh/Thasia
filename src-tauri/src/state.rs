use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use thasia_core::{
    BundleMode,
    models::{Direction, ImageFormat, OutputFormat, ParsedImage},
};

/// One scan-volume group: (sequential scan index, ordered pages).
pub type ScanGroups = Vec<(u32, Vec<ParsedImage>)>;

/// Full runtime state — held in Tauri managed state, never serialized to frontend.
#[derive(Debug, Default)]
pub struct ConvState {
    /// Grouped scan result keyed by sequential scan index. Populated by scan_source.
    pub scan_result: Option<ScanGroups>,
    /// Keeps the TempDir alive for ZIP/CBZ extractions.
    pub source: Option<Arc<thasia_source::LocalSource>>,
    /// Cooperative cancellation flag — set by `cancel_conversion`, polled
    /// between volumes (and at safe checkpoints) by `convert`.
    pub cancel: Arc<AtomicBool>,
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
    /// Sequential scan index — unique key used for page lookup in convert.
    pub volume_num: u32,
    /// Actual parsed volume number from the source folder names — used by the
    /// frontend to compute the default chapter→volume grouping.
    pub source_volume_num: u32,
    pub pages: Vec<PageMeta>,
}

/// Where a single page in the editor comes from. Tagged enum on the wire:
/// `{ "kind": "original", "page_index": 3, "source_volume_num": 1 }` or
/// `{ "kind": "custom",   "path": "/abs/path.png" }`.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PageEditSource {
    Original {
        /// Index into the scan_result pages for `source_volume_num`.
        page_index: u32,
        /// Which scan volume to look up `page_index` in. None = parent VolumeEdit's volume.
        source_volume_num: Option<u32>,
    },
    Custom {
        path: String,
    },
}

/// One page entry from the page editor — sent from JS to convert.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct PageEditEntry {
    pub source: PageEditSource,
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
    fn page_edit_entry_original_serializes_tagged() {
        let entry = PageEditEntry {
            source: PageEditSource::Original {
                page_index: 3,
                source_volume_num: Some(2),
            },
            excluded: false,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains(r#""kind":"original""#));
        assert!(json.contains(r#""page_index":3"#));
    }

    #[test]
    fn page_edit_entry_custom_serializes_tagged() {
        let entry = PageEditEntry {
            source: PageEditSource::Custom {
                path: "/tmp/cover.png".into(),
            },
            excluded: false,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains(r#""kind":"custom""#));
        assert!(json.contains(r#""path":"/tmp/cover.png""#));
    }

    #[test]
    fn conv_state_default_is_empty() {
        let state = ConvState::default();
        assert!(state.scan_result.is_none());
        assert!(state.source.is_none());
    }
}
