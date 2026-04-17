use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
#[cfg_attr(feature = "tauri", derive(specta::Type))]
pub struct ChapterIdentifier {
    pub volume: Option<u32>,
    pub chapter: Option<f32>, // Float supports things like "Chapter 10.5"
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "tauri", derive(specta::Type))]
pub struct DiscoveredImage {
    pub absolute_path: PathBuf,
    pub relative_path: String, // e.g. "Season 1/Ch 04/001.jpg"
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "tauri", derive(specta::Type))]
pub struct ParsedImage {
    pub source: DiscoveredImage,
    pub identifier: ChapterIdentifier,
    pub page_number: u32,
    pub is_cover: bool,
}

#[cfg_attr(feature = "tauri", derive(specta::Type))]
pub struct ProcessedImage {
    pub parsed_data: ParsedImage,
    pub image_data: Vec<u8>,
    pub ext: String,
    pub width: u32,
    pub height: u32,
}

/// Target image encoding format.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
#[cfg_attr(feature = "tauri", derive(specta::Type))]
pub enum ImageFormat {
    /// AVIF (AV1 Image Format) — best compression, slowest encoding.
    #[default]
    Avif,
    /// WebP — good compression, faster than AVIF.
    Webp,
    /// Keep original format without re-encoding.
    Original,
}

/// Output container format.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
#[cfg_attr(feature = "tauri", derive(specta::Type))]
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
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
#[cfg_attr(feature = "tauri", derive(specta::Type))]
pub enum Direction {
    /// Left-to-right (Western comics, manhwa).
    #[default]
    Ltr,
    /// Right-to-left (manga, manhua).
    Rtl,
}
