use crate::Source;
use async_trait::async_trait;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use thasia_core::{Result, ThasiaError, models::DiscoveredImage};
use tokio::sync::mpsc;
use walkdir::WalkDir;

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "avif"];
const MAX_ARCHIVE_IMAGES: usize = 20_000;
const MAX_ARCHIVE_ENTRY_BYTES: u64 = 512 * 1024 * 1024;
const MAX_ARCHIVE_TOTAL_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_ARCHIVE_COMPRESSION_RATIO: u64 = 200;

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
            extract_archive_images_preserve_paths(&path, &temp_path)
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
            let mut total_uncompressed = 0u64;
            for i in 0..archive.len() {
                let entry = archive
                    .by_index(i)
                    .map_err(|e| ThasiaError::Fatal(format!("Archive read error: {e}")))?;
                if entry.is_dir() {
                    continue;
                }
                let Some(enclosed) = entry.enclosed_name() else {
                    continue;
                };
                if !is_supported_image_path(&enclosed) {
                    continue;
                }
                validate_archive_entry_size(entry.size(), entry.compressed_size())?;
                total_uncompressed = total_uncompressed.saturating_add(entry.size());
                if total_uncompressed > MAX_ARCHIVE_TOTAL_BYTES {
                    return Err(ThasiaError::Fatal(format!(
                        "Archive is too large after extraction: {} bytes",
                        total_uncompressed
                    )));
                }
                image_names.push(entry.name().to_string());
                if image_names.len() > MAX_ARCHIVE_IMAGES {
                    return Err(ThasiaError::Fatal(format!(
                        "Archive contains too many images: more than {MAX_ARCHIVE_IMAGES}"
                    )));
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
                validate_archive_entry_size(entry.size(), entry.compressed_size())?;
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

    pub fn is_supported_image_path(path: &std::path::Path) -> bool {
        is_supported_image_path(path)
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
        let path = img.absolute_path.clone();
        tokio::task::spawn_blocking(move || read_file_sequential(&path))
            .await
            .map_err(|e| ThasiaError::Fatal(e.to_string()))?
            .map_err(ThasiaError::Io)
    }
}

fn is_supported_image_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn validate_archive_entry_size(uncompressed: u64, compressed: u64) -> Result<()> {
    if uncompressed > MAX_ARCHIVE_ENTRY_BYTES {
        return Err(ThasiaError::Fatal(format!(
            "Archive entry is too large: {} bytes",
            uncompressed
        )));
    }
    if compressed > 0 && uncompressed / compressed > MAX_ARCHIVE_COMPRESSION_RATIO {
        return Err(ThasiaError::Fatal(format!(
            "Archive entry compression ratio is suspicious: {}:{}",
            uncompressed, compressed
        )));
    }
    Ok(())
}

fn extract_archive_images_preserve_paths(path: &Path, dest_root: &Path) -> Result<()> {
    let file = std::fs::File::open(path).map_err(ThasiaError::Io)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| ThasiaError::Fatal(format!("Failed to open archive: {e}")))?;

    let mut image_count = 0usize;
    let mut total_uncompressed = 0u64;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| ThasiaError::Fatal(format!("Archive read error: {e}")))?;
        if entry.is_dir() {
            continue;
        }
        let Some(enclosed) = entry.enclosed_name() else {
            continue;
        };
        if !is_supported_image_path(&enclosed) {
            continue;
        }

        validate_archive_entry_size(entry.size(), entry.compressed_size())?;
        total_uncompressed = total_uncompressed.saturating_add(entry.size());
        if total_uncompressed > MAX_ARCHIVE_TOTAL_BYTES {
            return Err(ThasiaError::Fatal(format!(
                "Archive is too large after extraction: {} bytes",
                total_uncompressed
            )));
        }
        image_count += 1;
        if image_count > MAX_ARCHIVE_IMAGES {
            return Err(ThasiaError::Fatal(format!(
                "Archive contains too many images: more than {MAX_ARCHIVE_IMAGES}"
            )));
        }

        let dest = dest_root.join(enclosed);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(ThasiaError::Io)?;
        }
        let entry_size = entry.size();
        let mut out = std::fs::File::create(&dest).map_err(ThasiaError::Io)?;
        copy_bounded(&mut entry, &mut out, entry_size)?;
    }

    Ok(())
}

fn copy_bounded<R: Read, W: Write>(reader: &mut R, writer: &mut W, max_bytes: u64) -> Result<u64> {
    let mut limited = reader.take(max_bytes.saturating_add(1));
    let written = std::io::copy(&mut limited, writer).map_err(ThasiaError::Io)?;
    if written > max_bytes {
        return Err(ThasiaError::Fatal(
            "Archive entry exceeded declared size".into(),
        ));
    }
    Ok(written)
}

fn read_file_sequential(path: &Path) -> std::io::Result<Vec<u8>> {
    let mut file = open_file_for_sequential_read(path)?;
    advise_sequential_read(&file);

    let capacity = file
        .metadata()
        .ok()
        .and_then(|metadata| usize::try_from(metadata.len()).ok())
        .unwrap_or(0);
    let mut bytes = Vec::with_capacity(capacity);
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

#[cfg(target_os = "windows")]
fn open_file_for_sequential_read(path: &Path) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;

    const FILE_FLAG_SEQUENTIAL_SCAN: u32 = 0x0800_0000;

    std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_SEQUENTIAL_SCAN)
        .open(path)
}

#[cfg(not(target_os = "windows"))]
fn open_file_for_sequential_read(path: &Path) -> std::io::Result<File> {
    File::open(path)
}

#[cfg(target_os = "linux")]
fn advise_sequential_read(file: &File) {
    use std::os::fd::AsRawFd;

    unsafe {
        libc::posix_fadvise(file.as_raw_fd(), 0, 0, libc::POSIX_FADV_SEQUENTIAL);
    }
}

#[cfg(not(target_os = "linux"))]
fn advise_sequential_read(_file: &File) {}
