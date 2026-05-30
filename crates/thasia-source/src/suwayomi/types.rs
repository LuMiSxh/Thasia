use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ReleaseInfo {
    pub version: String,
    pub asset_name: String,
    pub download_url: String,
    pub checksum_url: Option<String>,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct InstalledInfo {
    pub version: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct UpdateInfo {
    pub current_version: Option<String>,
    pub latest_version: String,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Default)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum RuntimeState {
    NotInstalled,
    #[default]
    NotRunning,
    Starting,
    Ready {
        port: u16,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum InstallProgress {
    Downloading { bytes: u64, total: Option<u64> },
    Verifying,
    Extracting,
    Complete { version: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ExtensionInfo {
    pub pkg_name: String,
    pub name: String,
    pub lang: Option<String>,
    pub version_name: Option<String>,
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SourceInfo {
    pub id: String,
    pub name: String,
    pub lang: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SearchResult {
    pub id: i64,
    pub title: String,
    pub thumbnail_url: Option<String>,
    pub initialized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SearchPage {
    pub results: Vec<SearchResult>,
    pub has_next_page: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct MangaDetail {
    pub id: i64,
    pub title: String,
    pub author: Option<String>,
    pub artist: Option<String>,
    pub description: Option<String>,
    pub thumbnail_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ChapterMeta {
    pub id: i64,
    pub name: String,
    pub chapter_number: f32,
    /// Volume parsed from the chapter name (Suwayomi does not expose a volume field).
    pub volume_number: Option<u32>,
    pub scanlator: Option<String>,
    pub downloaded: bool,
}
