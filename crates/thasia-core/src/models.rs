use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
#[cfg_attr(
    feature = "tauri",
    derive(serde::Serialize, serde::Deserialize, specta::Type)
)]
pub struct ChapterIdentifier {
    pub volume: Option<u32>,
    pub chapter: Option<f32>, // Float supports things like "Chapter 10.5"
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "tauri",
    derive(serde::Serialize, serde::Deserialize, specta::Type)
)]
pub struct DiscoveredImage {
    pub absolute_path: PathBuf,
    pub relative_path: String, // e.g. "Season 1/Ch 04/001.jpg"
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "tauri",
    derive(serde::Serialize, serde::Deserialize, specta::Type)
)]
pub struct ParsedImage {
    pub source: DiscoveredImage,
    pub identifier: ChapterIdentifier,
    /// Sort key within (volume, chapter). Float so insert pages (e.g. `p35.5`,
    /// `p35.6` between `p35` and `p36`) and double-page spreads work naturally.
    /// Sort with `f32::total_cmp` for a strict total order.
    pub page_number: f32,
    pub is_cover: bool,
}

#[cfg_attr(
    feature = "tauri",
    derive(serde::Serialize, serde::Deserialize, specta::Type)
)]
pub struct ProcessedImage {
    pub parsed_data: ParsedImage,
    pub image_data: Vec<u8>,
    pub ext: String,
    pub width: u32,
    pub height: u32,
    pub stats: ProcessingStats,
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(
    feature = "tauri",
    derive(serde::Serialize, serde::Deserialize, specta::Type)
)]
pub struct ProcessingStats {
    pub input_bytes: u64,
    pub output_bytes: u64,
    pub passthrough: bool,
    pub fetch_ms: f64,
    pub decode_ms: f64,
    pub transform_ms: f64,
    pub encode_ms: f64,
    pub total_ms: f64,
}

/// Target image encoding format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "tauri",
    derive(serde::Serialize, serde::Deserialize, specta::Type)
)]
pub enum ImageFormat {
    /// AVIF (AV1 Image Format) — best compression, slowest encoding.
    #[default]
    Avif,
    /// WebP — good compression, faster than AVIF.
    Webp,
    /// Keep original format without re-encoding.
    Original,
}

/// Optional color enhancement for washed-out color pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "tauri",
    derive(serde::Serialize, serde::Deserialize, specta::Type)
)]
pub enum ColorEnhanceMode {
    /// Leave colors untouched.
    #[default]
    Off,
    /// Subtle contrast and saturation lift.
    Mild,
    /// Moderate contrast and saturation lift.
    Balanced,
    /// Stronger lift for very faded scans.
    Strong,
}

/// Optional sharpening for soft scans.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "tauri",
    derive(serde::Serialize, serde::Deserialize, specta::Type)
)]
pub enum SharpenMode {
    /// Leave sharpness untouched.
    #[default]
    Off,
    /// Mild unsharp-mask pass.
    Mild,
}

/// Output container format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "tauri",
    derive(serde::Serialize, serde::Deserialize, specta::Type)
)]
pub enum OutputFormat {
    /// Comic Book ZIP — widely supported by comic readers.
    #[default]
    Cbz,
    /// EPUB 3 fixed-layout — e-readers and reading apps.
    Epub,
    /// Raw directory — one flat folder per volume.
    Raw,
}

/// EPUB reading direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "tauri",
    derive(serde::Serialize, serde::Deserialize, specta::Type)
)]
pub enum Direction {
    /// Left-to-right (Western comics, manhwa).
    #[default]
    Ltr,
    /// Right-to-left (manga, manhua).
    Rtl,
}
