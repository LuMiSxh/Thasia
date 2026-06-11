use crate::{error::AppResult, models::DiscoverySettings, util::paths::AppPaths};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppSettings {
    pub default_output_dir: Option<PathBuf>,
    pub show_key_hints: bool,
    pub theme: ThemePreference,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            default_output_dir: None,
            show_key_hints: true,
            theme: ThemePreference::System,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreference {
    Light,
    Dark,
    #[default]
    System,
}

pub fn load_app(paths: &AppPaths) -> AppSettings {
    load_json(&paths.settings)
}

pub fn save_app(paths: &AppPaths, settings: &AppSettings) -> AppResult<()> {
    save_json(&paths.settings, settings)
}

pub fn load_discovery(paths: &AppPaths) -> DiscoverySettings {
    load_json(&paths.discovery_settings)
}

pub fn save_discovery(paths: &AppPaths, settings: &DiscoverySettings) -> AppResult<()> {
    save_json(&paths.discovery_settings, settings)
}

fn load_json<T: serde::de::DeserializeOwned + Default>(path: &std::path::Path) -> T {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn save_json(path: &std::path::Path, value: &impl Serialize) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}
