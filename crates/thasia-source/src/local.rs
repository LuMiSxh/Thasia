use crate::Source;
use async_trait::async_trait;
use std::path::PathBuf;
use thasia_core::{models::DiscoveredImage, Result, ThasiaError};
use tokio::sync::mpsc;
use walkdir::WalkDir;

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "avif"];

pub struct LocalSource {
    root: PathBuf,
    _temp_dir_handle: Option<tempfile::TempDir>,
}

impl LocalSource {
    pub fn new(root: PathBuf) -> Self {
        Self { root, _temp_dir_handle: None }
    }

    /// Use this when source is a ZIP/CBZ that was extracted to a temp dir.
    /// The TempDir handle is kept alive to prevent early cleanup.
    pub fn from_temp_dir(temp: tempfile::TempDir) -> Self {
        let root = temp.path().to_path_buf();
        Self { root, _temp_dir_handle: Some(temp) }
    }
}

#[async_trait]
impl Source for LocalSource {
    async fn discover(&self) -> Result<mpsc::Receiver<DiscoveredImage>> {
        let (tx, rx) = mpsc::channel(5000);
        let root = self.root.clone();

        tokio::task::spawn_blocking(move || {
            let mut entries: Vec<_> = WalkDir::new(&root)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_type().is_file()
                        && e.path()
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .map(|ext| IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
                            .unwrap_or(false)
                })
                .collect();

            // Natural sort: ensures page ordering is consistent
            entries.sort_by(|a, b| {
                natord::compare(
                    &a.path().to_string_lossy(),
                    &b.path().to_string_lossy(),
                )
            });

            for entry in entries {
                let relative = entry
                    .path()
                    .strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                let img = DiscoveredImage {
                    absolute_path: entry.path().to_path_buf(),
                    relative_path: relative,
                };
                if tx.blocking_send(img).is_err() {
                    break;
                }
            }
        });

        Ok(rx)
    }

    async fn fetch(&self, img: &DiscoveredImage) -> Result<Vec<u8>> {
        tokio::fs::read(&img.absolute_path).await.map_err(ThasiaError::Io)
    }
}
