use crate::suwayomi::types::{InstallProgress, InstalledInfo, ReleaseInfo};
use crate::{Result, SourceError};
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

const REPO_API: &str = "https://api.github.com/repos/Suwayomi/Suwayomi-Server/releases";

#[derive(Clone)]
pub struct SuwayomiInstaller {
    root: PathBuf,
    http: reqwest::Client,
}

impl SuwayomiInstaller {
    pub fn new(root: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&root)?;
        let http = reqwest::Client::builder().user_agent("Thasia").build()?;
        Ok(Self { root, http })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn bin_dir(&self) -> PathBuf {
        self.root.join("suwayomi-bin")
    }

    pub fn data_dir(&self) -> PathBuf {
        self.root.join("suwayomi-data")
    }

    pub fn launcher_path(&self) -> PathBuf {
        self.jar_path()
    }

    pub fn java_path(&self) -> PathBuf {
        let name = if cfg!(windows) { "java.exe" } else { "java" };
        self.bin_dir()
            .join("Suwayomi-Server")
            .join("jre")
            .join("bin")
            .join(name)
    }

    pub fn jar_path(&self) -> PathBuf {
        self.bin_dir()
            .join("Suwayomi-Server")
            .join("bin")
            .join("Suwayomi-Server.jar")
    }

    pub fn server_dir(&self) -> PathBuf {
        self.bin_dir().join("Suwayomi-Server")
    }

    pub async fn latest_release(&self) -> Result<ReleaseInfo> {
        self.release("latest").await
    }

    pub async fn release_for_version(&self, version: &str) -> Result<ReleaseInfo> {
        self.release(&format!("tags/{version}")).await
    }

    pub async fn installed_info(&self) -> Result<Option<InstalledInfo>> {
        let version = match self.installed_version().await {
            Some(v) => v,
            None => return Ok(None),
        };
        let size = dir_size(self.bin_dir()).await.unwrap_or(0);
        Ok(Some(InstalledInfo { version, size }))
    }

    pub async fn installed_version(&self) -> Option<String> {
        let version_path = self.bin_dir().join("VERSION");
        let version = tokio::fs::read_to_string(&version_path)
            .await
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())?;
        // Use async try_exists to avoid blocking the runtime thread.
        let launcher_present = tokio::fs::try_exists(self.launcher_path())
            .await
            .unwrap_or(false);
        launcher_present.then_some(version)
    }

    pub async fn install(
        &self,
        version: &str,
        progress: mpsc::Sender<InstallProgress>,
    ) -> Result<()> {
        let release = if version == "latest" {
            self.latest_release().await?
        } else {
            self.release_for_version(version).await?
        };

        tokio::fs::create_dir_all(&self.root).await?;
        let temp = tempfile::tempdir_in(&self.root)?;
        let archive_path = temp.path().join(&release.asset_name);
        self.download_asset(&release, &archive_path, progress.clone())
            .await?;

        let _ = progress.send(InstallProgress::Verifying).await;
        self.verify_checksum(&release, &archive_path).await?;

        let _ = progress.send(InstallProgress::Extracting).await;
        let extracted = temp.path().join("extract");
        let normalized = temp.path().join("normalized");
        tokio::fs::create_dir_all(&extracted).await?;
        extract_archive(&archive_path, &extracted, &release.asset_name).await?;
        let server_root = find_server_root(&extracted).await?;
        tokio::fs::create_dir_all(&normalized).await?;
        tokio::fs::rename(server_root, normalized.join("Suwayomi-Server")).await?;

        let final_dir = self.bin_dir();
        if final_dir.exists() {
            tokio::fs::remove_dir_all(&final_dir).await?;
        }
        tokio::fs::rename(&normalized, &final_dir).await?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if self.java_path().exists() {
                let mut perms = tokio::fs::metadata(self.java_path()).await?.permissions();
                perms.set_mode(perms.mode() | 0o755);
                tokio::fs::set_permissions(self.java_path(), perms).await?;
            }
        }

        tokio::fs::write(final_dir.join("VERSION"), &release.version).await?;
        let _ = progress
            .send(InstallProgress::Complete {
                version: release.version,
            })
            .await;
        Ok(())
    }

    pub async fn uninstall(&self) -> Result<()> {
        let dir = self.bin_dir();
        if dir.exists() {
            tokio::fs::remove_dir_all(dir).await?;
        }
        Ok(())
    }

    pub async fn reset_data(&self) -> Result<()> {
        let dir = self.data_dir();
        if dir.exists() {
            tokio::fs::remove_dir_all(&dir).await?;
        }
        tokio::fs::create_dir_all(dir).await?;
        Ok(())
    }

    async fn release(&self, suffix: &str) -> Result<ReleaseInfo> {
        let url = format!("{REPO_API}/{suffix}");
        let release: GithubRelease = self
            .http
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let suffix = asset_suffix();
        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name.ends_with(suffix))
            .ok_or_else(|| {
                SourceError::suwayomi(format!("No Suwayomi release asset for {suffix}"))
            })?;
        let checksum_url = release
            .assets
            .iter()
            .find(|asset| asset.name.eq_ignore_ascii_case("Checksums.sha256"))
            .map(|asset| asset.browser_download_url.clone());

        Ok(ReleaseInfo {
            version: release.tag_name,
            asset_name: asset.name.clone(),
            download_url: asset.browser_download_url.clone(),
            checksum_url,
            size: asset.size,
        })
    }

    async fn download_asset(
        &self,
        release: &ReleaseInfo,
        archive_path: &Path,
        progress: mpsc::Sender<InstallProgress>,
    ) -> Result<()> {
        let response = self
            .http
            .get(&release.download_url)
            .send()
            .await?
            .error_for_status()?;
        let total = response.content_length().or(Some(release.size));
        let mut stream = response.bytes_stream();
        let mut file = tokio::fs::File::create(archive_path).await?;
        let mut downloaded = 0u64;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
            let _ = progress
                .send(InstallProgress::Downloading {
                    bytes: downloaded,
                    total,
                })
                .await;
        }
        file.flush().await?;
        Ok(())
    }

    async fn verify_checksum(&self, release: &ReleaseInfo, archive_path: &Path) -> Result<()> {
        let Some(url) = &release.checksum_url else {
            return Ok(());
        };
        let checksums = self
            .http
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        let expected = checksums
            .lines()
            .find(|line| line.contains(&release.asset_name))
            .and_then(|line| line.split_whitespace().next())
            .ok_or_else(|| SourceError::suwayomi("Checksum for Suwayomi asset not found"))?
            .to_string();

        // Stream the archive through the hasher in 64 KB chunks instead of
        // loading the entire ~330 MB file into heap memory at once.
        let archive_path = archive_path.to_path_buf();
        let actual = tokio::task::spawn_blocking(move || -> Result<String> {
            use std::io::Read;
            let file = std::fs::File::open(&archive_path)?;
            let mut reader = std::io::BufReader::new(file);
            let mut hasher = Sha256::new();
            let mut buf = [0u8; 65536];
            loop {
                let n = reader.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                hasher.update(&buf[..n]);
            }
            let digest = hasher.finalize();
            let mut hex = String::with_capacity(64);
            for byte in digest.iter() {
                use std::fmt::Write;
                let _ = write!(hex, "{byte:02x}");
            }
            Ok(hex)
        })
        .await??;

        if actual != expected {
            return Err(SourceError::suwayomi("Suwayomi checksum mismatch"));
        }
        Ok(())
    }
}

#[derive(Debug, serde::Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubAsset>,
}

#[derive(Debug, serde::Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

fn asset_suffix() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => "-windows-x64.zip",
        ("macos", "x86_64") => "-macOS-x64.tar.gz",
        ("macos", "aarch64") => "-macOS-arm64.tar.gz",
        ("linux", "x86_64") => "-linux-x64.tar.gz",
        _ => "-linux-x64.tar.gz",
    }
}

async fn extract_archive(archive: &Path, destination: &Path, name: &str) -> Result<()> {
    let archive = archive.to_path_buf();
    let destination = destination.to_path_buf();
    let name = name.to_string();
    tokio::task::spawn_blocking(move || -> Result<()> {
        let file = std::fs::File::open(&archive)?;
        if name.ends_with(".zip") {
            let mut zip = zip::ZipArchive::new(file)
                .map_err(|e| SourceError::suwayomi(format!("Failed to open Suwayomi zip: {e}")))?;
            zip.extract(&destination).map_err(|e| {
                SourceError::suwayomi(format!("Failed to extract Suwayomi zip: {e}"))
            })?;
        } else {
            let gz = flate2::read::GzDecoder::new(file);
            let mut tar = tar::Archive::new(gz);
            tar.unpack(&destination).map_err(|e| {
                SourceError::suwayomi(format!("Failed to extract Suwayomi archive: {e}"))
            })?;
        }
        Ok(())
    })
    .await?
}

async fn find_server_root(extracted: &Path) -> Result<PathBuf> {
    let extracted = extracted.to_path_buf();
    tokio::task::spawn_blocking(move || {
        for entry in walkdir::WalkDir::new(&extracted)
            .into_iter()
            .filter_map(|entry| entry.ok())
        {
            if !entry.file_type().is_file()
                || entry.file_name().to_str() != Some("Suwayomi-Server.jar")
            {
                continue;
            }
            let path = entry.path();
            if let Some(root) = jar_root(path) {
                return Ok(root);
            }
        }
        Err(SourceError::suwayomi(
            "Extracted Suwayomi archive did not contain bin/Suwayomi-Server.jar",
        ))
    })
    .await?
}

fn jar_root(path: &Path) -> Option<PathBuf> {
    let bin_dir = path.parent()?;
    if bin_dir.file_name().and_then(|name| name.to_str()) != Some("bin") {
        return None;
    }
    bin_dir.parent().map(Path::to_path_buf)
}

async fn dir_size(path: PathBuf) -> std::io::Result<u64> {
    tokio::task::spawn_blocking(move || {
        let mut total = 0;
        // WalkDir silently yields no entries when the path is absent, so
        // the pre-existence check (TOCTOU) is unnecessary.
        for entry in walkdir::WalkDir::new(&path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                total += entry.metadata()?.len();
            }
        }
        Ok(total)
    })
    .await
    .map_err(std::io::Error::other)?
}
