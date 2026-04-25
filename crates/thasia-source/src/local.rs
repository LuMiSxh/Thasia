use crate::Source;
use async_trait::async_trait;
use std::path::PathBuf;
use thasia_core::{Result, ThasiaError, models::DiscoveredImage};
use tokio::sync::mpsc;
use walkdir::WalkDir;

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "avif"];

#[derive(Debug, Default)]
pub struct LocalSource {
    root: PathBuf,
    _temp_dir_handle: Option<tempfile::TempDir>,
}

impl LocalSource {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            _temp_dir_handle: None,
        }
    }

    /// Use this when source is a ZIP/CBZ that was extracted to a temp dir.
    pub fn from_temp_dir(temp: tempfile::TempDir) -> Self {
        let root = temp.path().to_path_buf();
        Self {
            root,
            _temp_dir_handle: Some(temp),
        }
    }

    /// Extracts a ZIP or CBZ archive to a temp dir and returns a LocalSource over it.
    pub async fn from_archive(path: PathBuf) -> Result<Self> {
        let temp = tempfile::TempDir::new().map_err(ThasiaError::Io)?;
        let temp_path = temp.path().to_path_buf();

        tokio::task::spawn_blocking(move || {
            let file = std::fs::File::open(&path).map_err(ThasiaError::Io)?;
            let mut archive = zip::ZipArchive::new(file)
                .map_err(|e| ThasiaError::Fatal(format!("Failed to open archive: {e}")))?;
            archive
                .extract(&temp_path)
                .map_err(|e| ThasiaError::Fatal(format!("Failed to extract archive: {e}")))?;
            Ok::<(), ThasiaError>(())
        })
        .await
        .map_err(|e| ThasiaError::Fatal(e.to_string()))??;

        Ok(Self::from_temp_dir(temp))
    }

    /// Returns true if the given path looks like a ZIP or CBZ archive.
    pub fn is_archive(path: &std::path::Path) -> bool {
        matches!(
            path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                .as_deref(),
            Some("zip") | Some("cbz")
        )
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

            entries.sort_by(|a, b| {
                natord::compare(&a.path().to_string_lossy(), &b.path().to_string_lossy())
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
        tokio::fs::read(&img.absolute_path)
            .await
            .map_err(ThasiaError::Io)
    }
}
