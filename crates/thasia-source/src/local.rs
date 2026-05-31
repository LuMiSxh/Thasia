use crate::{FetchedImage, Source};
use async_trait::async_trait;
use memmap2::MmapOptions;
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
const MMAP_MIN_BYTES: u64 = 1024 * 1024;

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
                    .unwrap_or_else(|_| entry.path())
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

    async fn fetch(&self, img: &DiscoveredImage) -> Result<FetchedImage> {
        let path = img.absolute_path.clone();
        tokio::task::spawn_blocking(move || fetch_local_file(&path))
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

fn fetch_local_file(path: &Path) -> std::io::Result<FetchedImage> {
    let mut file = platform::open_file_for_sequential_read(path)?;
    platform::advise_sequential_read(&file);
    let len = file.metadata().map(|metadata| metadata.len()).unwrap_or(0);
    if len >= MMAP_MIN_BYTES {
        match map_file(&file) {
            Ok(map) => return Ok(FetchedImage::Mmap(map)),
            Err(err) => {
                tracing::debug!(
                    "Falling back to buffered read for {}: {err}",
                    path.display()
                );
            }
        }
    }

    read_open_file(&mut file, len).map(FetchedImage::Vec)
}

fn read_open_file(file: &mut File, len: u64) -> std::io::Result<Vec<u8>> {
    let capacity = usize::try_from(len).unwrap_or(0);
    let mut bytes = Vec::with_capacity(capacity);
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn map_file(file: &File) -> std::io::Result<memmap2::Mmap> {
    // SAFETY: The map is read-only and tied to this file's current contents.
    // If the user mutates/truncates source files while conversion is running,
    // the OS may invalidate the mapping; normal conversions treat sources as
    // immutable for the duration of the job.
    unsafe { MmapOptions::new().map(file) }
}

mod platform {
    use std::fs::File;
    use std::path::Path;

    #[cfg(target_os = "windows")]
    pub fn open_file_for_sequential_read(path: &Path) -> std::io::Result<File> {
        use std::os::windows::fs::OpenOptionsExt;

        const FILE_FLAG_SEQUENTIAL_SCAN: u32 = 0x0800_0000;

        std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(FILE_FLAG_SEQUENTIAL_SCAN)
            .open(path)
    }

    #[cfg(not(target_os = "windows"))]
    pub fn open_file_for_sequential_read(path: &Path) -> std::io::Result<File> {
        File::open(path)
    }

    #[cfg(target_os = "linux")]
    pub fn advise_sequential_read(file: &File) {
        use std::os::fd::AsRawFd;

        unsafe {
            libc::posix_fadvise(file.as_raw_fd(), 0, 0, libc::POSIX_FADV_SEQUENTIAL);
        }
    }

    #[cfg(not(target_os = "linux"))]
    pub fn advise_sequential_read(_file: &File) {}
}
