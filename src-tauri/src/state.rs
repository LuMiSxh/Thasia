use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use thasia_core::{
    BundleMode,
    models::{Direction, ImageFormat, OutputFormat, ParsedImage},
};
use thasia_source::suwayomi::{SuwayomiClient, SuwayomiInstaller, SuwayomiManager};
use tokio::sync::{Mutex, RwLock as AsyncRwLock};
use tokio::task::JoinHandle;

pub const DEFAULT_EXTENSION_REPO: &str =
    "https://raw.githubusercontent.com/keiyoushi/extensions/repo/index.min.json";

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

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverySettings {
    pub enabled: bool,
    pub installed_version: Option<String>,
    pub auto_start: bool,
    pub last_update_check: Option<String>,
    #[serde(default)]
    pub download_dir: Option<String>,
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

pub struct DiscoveryState {
    pub settings_path: PathBuf,
    pub settings: AsyncRwLock<DiscoverySettings>,
    pub installer: Arc<SuwayomiInstaller>,
    pub manager: Arc<SuwayomiManager>,
    pub client: Arc<AsyncRwLock<Option<Arc<SuwayomiClient>>>>,
    pub install_lock: Mutex<()>,
    pub monitor: Mutex<Option<JoinHandle<()>>>,
}

impl DiscoveryState {
    pub fn new(app_data_dir: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(&app_data_dir).map_err(|e| e.to_string())?;
        let settings_path = app_data_dir.join("discovery.json");
        let settings = load_discovery_settings_file(&settings_path);
        let installer = Arc::new(SuwayomiInstaller::new(app_data_dir).map_err(|e| e.to_string())?);
        let manager = Arc::new(SuwayomiManager::new(installer.clone()));
        Ok(Self {
            settings_path,
            settings: AsyncRwLock::new(settings),
            installer,
            manager,
            client: Arc::new(AsyncRwLock::new(None)),
            install_lock: Mutex::new(()),
            monitor: Mutex::new(None),
        })
    }

    pub async fn persist_settings(&self, settings: &DiscoverySettings) -> Result<(), String> {
        let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
        if let Some(parent) = self.settings_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| e.to_string())?;
        }
        tokio::fs::write(&self.settings_path, json)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn refresh_installed_version(&self) -> Result<DiscoverySettings, String> {
        let installed_version = self.installer.installed_version().await;
        let mut settings = self.settings.write().await;
        settings.installed_version = installed_version;
        if settings.installed_version.is_none() {
            settings.enabled = false;
        }
        let out = settings.clone();
        self.persist_settings(&out).await?;
        Ok(out)
    }

    pub async fn prepare_suwayomi_config(&self) -> Result<(), String> {
        let repos = {
            let settings = self.settings.read().await;
            settings.extension_repos.clone()
        };
        write_server_conf(self.installer.data_dir().join("server.conf"), &repos).await
    }
}

fn load_discovery_settings_file(path: &std::path::Path) -> DiscoverySettings {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<DiscoverySettings>(&raw).ok())
        .unwrap_or_default()
}

fn default_extension_repos() -> Vec<String> {
    vec![DEFAULT_EXTENSION_REPO.to_string()]
}

async fn write_server_conf(path: PathBuf, repos: &[String]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| e.to_string())?;
    }
    let repos = repos
        .iter()
        .map(|repo| repo.trim())
        .filter(|repo| !repo.is_empty())
        .map(|repo| format!("\"{}\"", repo.replace('\\', "\\\\").replace('"', "\\\"")))
        .collect::<Vec<_>>()
        .join(", ");
    // KCEF (Chromium-based Cloudflare bypass) is enabled. Suwayomi initializes
    // it on first source request either way; explicitly enabling it lets it
    // download at startup and gives Cloudflare-protected sources a chance to
    // work. The dock icon stays hidden via `apple.awt.UIElement=true` in the
    // JVM args (see manager.rs).
    let content = format!(
        r#"server {{
  ip = "127.0.0.1"
  webUIEnabled = false
  initialOpenInBrowserEnabled = false
  systemTrayEnabled = false
  downloadAsCbz = true
  autoDownloadNewChapters = false
  updateMangas = false
  globalUpdateInterval = 0
  extensionRepos = [{repos}]
}}

suwayomi {{
  server {{
    kcefEnabled = true
    downloadKcefAtStartup = true
  }}
}}
"#
    );
    tokio::fs::write(path, content)
        .await
        .map_err(|e| e.to_string())
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
