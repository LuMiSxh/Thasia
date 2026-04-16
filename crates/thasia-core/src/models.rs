use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct ChapterIdentifier {
    pub volume: Option<u32>,
    pub chapter: Option<f32>, // Float supports things like "Chapter 10.5"
}

#[derive(Debug, Clone)]
pub struct DiscoveredImage {
    pub absolute_path: PathBuf,
    pub relative_path: String, // e.g. "Season 1/Ch 04/001.jpg"
}

#[derive(Debug, Clone)]
pub struct ParsedImage {
    pub source: DiscoveredImage,
    pub identifier: ChapterIdentifier,
    pub page_number: u32,
    pub is_cover: bool,
}

pub struct ProcessedImage {
    pub parsed_data: ParsedImage,
    pub image_data: Vec<u8>,
    pub ext: String, // "avif", "webp", "jpg"
    pub width: u32,
    pub height: u32,
}
