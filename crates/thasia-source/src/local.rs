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

    /// Extracts a CBZ/ZIP archive into `dest_dir`, renaming images to
    /// `001.ext`, `002.ext`, … (sorted by natural order of the original names).
    ///
    /// Only image files are extracted; directory entries and non-image files
    /// are skipped.
    pub async fn extract_chapter_cbz(cbz_path: PathBuf, dest_dir: PathBuf) -> Result<()> {
        tokio::task::spawn_blocking(move || {
            std::fs::create_dir_all(&dest_dir).map_err(ThasiaError::Io)?;
            let file = std::fs::File::open(&cbz_path).map_err(ThasiaError::Io)?;
            let mut archive = zip::ZipArchive::new(file)
                .map_err(|e| ThasiaError::Fatal(format!("Failed to open archive: {e}")))?;

            // Phase 1: collect image entry names sorted by natural order.
            let mut image_names: Vec<String> = Vec::new();
            for i in 0..archive.len() {
                let entry = archive
                    .by_index(i)
                    .map_err(|e| ThasiaError::Fatal(format!("Archive read error: {e}")))?;
                if entry.is_dir() {
                    continue;
                }
                let name = entry.name().to_string();
                let is_image = std::path::Path::new(&name)
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| IMAGE_EXTENSIONS.contains(&e.to_lowercase().as_str()))
                    .unwrap_or(false);
                if is_image {
                    image_names.push(name);
                }
            }
            image_names.sort_by(|a, b| natord::compare(a, b));

            // Phase 2: extract and rename sequentially.
            for (index, name) in image_names.iter().enumerate() {
                let ext = std::path::Path::new(name.as_str())
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("jpg")
                    .to_lowercase();
                let dest_file = dest_dir.join(format!("{:03}.{ext}", index + 1));
                let mut entry = archive
                    .by_name(name.as_str())
                    .map_err(|e| ThasiaError::Fatal(format!("Archive entry not found: {e}")))?;
                let mut out = std::fs::File::create(&dest_file).map_err(ThasiaError::Io)?;
                std::io::copy(&mut entry, &mut out).map_err(ThasiaError::Io)?;
            }

            Ok::<(), ThasiaError>(())
        })
        .await
        .map_err(|e| ThasiaError::Fatal(e.to_string()))?
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
