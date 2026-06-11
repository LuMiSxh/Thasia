use directories::ProjectDirs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub settings: PathBuf,
    pub discovery_settings: PathBuf,
    pub window_state: PathBuf,
    pub logs: PathBuf,
    pub suwayomi: PathBuf,
    pub assets: PathBuf,
}

impl AppPaths {
    pub fn resolve() -> std::io::Result<Self> {
        let dirs = ProjectDirs::from("com", "lumisxh", "Thasia")
            .ok_or_else(|| std::io::Error::other("platform directories unavailable"))?;
        let config_dir = dirs.config_dir().to_path_buf();
        let data_dir = dirs.data_dir().to_path_buf();
        let cache_dir = dirs.cache_dir().to_path_buf();
        let logs = data_dir.join("logs");
        let suwayomi = data_dir.join("suwayomi");
        for dir in [&config_dir, &data_dir, &cache_dir, &logs, &suwayomi] {
            std::fs::create_dir_all(dir)?;
        }
        Ok(Self {
            settings: config_dir.join("settings.json"),
            discovery_settings: config_dir.join("discovery.json"),
            window_state: config_dir.join("window-state.json"),
            config_dir,
            data_dir,
            cache_dir,
            logs,
            suwayomi,
            assets: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join("assets"),
        })
    }
}
