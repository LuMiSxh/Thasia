use crate::suwayomi::installer::SuwayomiInstaller;
use crate::suwayomi::types::RuntimeState;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use thasia_core::{Result, ThasiaError};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, RwLock};
use tokio::time::{Duration, Instant, sleep, timeout};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

const LOG_MAX_FILES: usize = 7;

// JVM `-D` properties Suwayomi reads for its server config.
//
// KCEF is intentionally enabled. Suwayomi initializes KCEF either way (the
// `kcefEnabled=false` setting is honored in the config dump but the JCEF
// native lib still gets bootstrapped on first use), and the half-disabled
// state was causing crashes downstream. Better to let it run as designed.
//
// WebUI is disabled because Thasia ships its own UI; we never need Suwayomi
// to serve its own web frontend.
const SERVER_PROPERTIES: &[(&str, &str)] = &[
    ("suwayomi.tachidesk.config.server.ip", "127.0.0.1"),
    ("suwayomi.tachidesk.config.server.webUIEnabled", "false"),
    (
        "suwayomi.tachidesk.config.server.initialOpenInBrowserEnabled",
        "false",
    ),
    (
        "suwayomi.tachidesk.config.server.systemTrayEnabled",
        "false",
    ),
    ("suwayomi.tachidesk.config.server.downloadAsCbz", "true"),
    (
        "suwayomi.tachidesk.config.server.autoDownloadNewChapters",
        "false",
    ),
    ("suwayomi.tachidesk.config.server.updateMangas", "false"),
    ("suwayomi.tachidesk.config.server.globalUpdateInterval", "0"),
];

pub struct SuwayomiManager {
    installer: Arc<SuwayomiInstaller>,
    state: Arc<RwLock<RuntimeState>>,
    child: Mutex<Option<Child>>,
    http: reqwest::Client,
}

impl SuwayomiManager {
    pub fn new(installer: Arc<SuwayomiInstaller>) -> Self {
        Self {
            installer,
            state: Arc::new(RwLock::new(RuntimeState::NotRunning)),
            child: Mutex::new(None),
            http: reqwest::Client::new(),
        }
    }

    pub async fn start(&self) -> Result<u16> {
        if self.installer.installed_version().await.is_none() {
            self.set_state(RuntimeState::NotInstalled).await;
            return Err(ThasiaError::Discovery(
                "Suwayomi-Server is not installed".into(),
            ));
        }
        if let RuntimeState::Ready { port } = self.snapshot().await {
            return Ok(port);
        }

        self.set_state(RuntimeState::Starting).await;
        tokio::fs::create_dir_all(self.installer.data_dir())
            .await
            .map_err(ThasiaError::Io)?;

        // Captured-log directory sits next to thasia.log under the app data
        // root (NOT inside suwayomi-data, so Suwayomi's own rootDir stays
        // contained). Suwayomi's internal application.log is unaffected; it
        // continues to live under <rootDir>/logs/ and rotates on its own.
        let log_dir = self.installer.root().join("logs");
        tokio::fs::create_dir_all(&log_dir)
            .await
            .map_err(ThasiaError::Io)?;

        // Sweep stale JVM crash dumps from previous sessions. We use a single
        // `hs_err.log` going forward; this drops the historical N-per-crash files.
        sweep_old_crash_dumps(&log_dir).await;

        // Repair the KCEF directory layout on macOS. KCEF extracts the CEF
        // framework inside `cef_server.app/Contents/Frameworks/`, but JCEF
        // (running outside an .app bundle, as we do via `java -jar`) looks
        // for it at `kcef/Frameworks/` directly. Without these symlinks,
        // `dlopen()` returns NULL and JCEF crashes the JVM on `FindClass()`.
        fixup_kcef_layout(&self.installer.data_dir()).await;

        let port = free_port()?;
        let java = self.installer.java_path();
        let jar = self.installer.jar_path();
        if !java.exists() || !jar.exists() {
            self.set_state(RuntimeState::NotInstalled).await;
            return Err(ThasiaError::Discovery(
                "Suwayomi Java runtime or server jar is missing".into(),
            ));
        }

        let mut cmd = Command::new(java);
        // `apple.awt.UIElement=true` hides the dock icon without disabling AWT.
        // We do NOT set `java.awt.headless=true` because JCEF needs AWT alive
        // to bootstrap its native side.
        cmd.arg("-Dapple.awt.UIElement=true").arg(format!(
            "-XX:ErrorFile={}",
            log_dir.join("hs_err.log").display()
        ));
        for (key, value) in SERVER_PROPERTIES {
            cmd.arg(format!("-D{key}={value}"));
        }
        cmd.arg(format!("-Dsuwayomi.tachidesk.config.server.port={port}"))
            .arg(format!(
                "-Dsuwayomi.tachidesk.config.server.rootDir={}",
                self.installer.data_dir().display()
            ))
            .arg("-jar")
            .arg(jar)
            .current_dir(self.installer.server_dir())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let mut child = cmd.spawn().map_err(ThasiaError::Io)?;

        // Pipe child stdout/stderr through Rust so we can rotate the captured
        // logs daily (max 7 files each, ≈ 1 week retention). Without this the
        // subprocess holds the FD open and we can't rotate while it runs.
        if let Some(stdout) = child.stdout.take() {
            spawn_log_pipe(stdout, log_dir.clone(), "suwayomi.stdout");
        }
        if let Some(stderr) = child.stderr.take() {
            spawn_log_pipe(stderr, log_dir.clone(), "suwayomi.stderr");
        }

        *self.child.lock().await = Some(child);

        match self.wait_ready(port).await {
            Ok(()) => {
                self.set_state(RuntimeState::Ready { port }).await;
                Ok(port)
            }
            Err(e) => {
                let _ = self.kill_child().await;
                self.set_state(RuntimeState::Error {
                    message: e.to_string(),
                })
                .await;
                Err(e)
            }
        }
    }

    pub async fn stop(&self) -> Result<()> {
        let port = match self.snapshot().await {
            RuntimeState::Ready { port } => Some(port),
            _ => None,
        };
        if let Some(port) = port {
            let _ = self
                .http
                .post(format!("http://127.0.0.1:{port}/api/v1/global/shutdown"))
                .send()
                .await;
        }

        let mut child = self.child.lock().await;
        if let Some(mut child) = child.take() {
            let graceful = timeout(Duration::from_secs(3), child.wait()).await;
            if !matches!(graceful, Ok(Ok(_))) {
                let _ = child.kill().await;
                let _ = child.wait().await;
            }
        }
        self.set_state(if self.installer.installed_version().await.is_some() {
            RuntimeState::NotRunning
        } else {
            RuntimeState::NotInstalled
        })
        .await;
        Ok(())
    }

    pub async fn restart(&self) -> Result<u16> {
        self.stop().await?;
        self.start().await
    }

    pub async fn snapshot(&self) -> RuntimeState {
        {
            let mut child = self.child.lock().await;
            if let Some(child_process) = child.as_mut() {
                match child_process.try_wait() {
                    Ok(Some(status)) => {
                        *child = None;
                        let state = RuntimeState::Error {
                            message: format!("Suwayomi-Server exited unexpectedly ({status})"),
                        };
                        self.set_state(state.clone()).await;
                        return state;
                    }
                    Ok(None) => {}
                    Err(e) => {
                        *child = None;
                        let state = RuntimeState::Error {
                            message: format!("Could not inspect Suwayomi-Server process: {e}"),
                        };
                        self.set_state(state.clone()).await;
                        return state;
                    }
                }
            }
        }

        self.state.read().await.clone()
    }

    async fn set_state(&self, state: RuntimeState) {
        *self.state.write().await = state;
    }

    async fn wait_ready(&self, port: u16) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(15);
        let url = format!("http://127.0.0.1:{port}/api/v1/settings/about");
        while Instant::now() < deadline {
            if self
                .http
                .get(&url)
                .send()
                .await
                .map(|r| r.status().is_success())
                .unwrap_or(false)
            {
                return Ok(());
            }
            sleep(Duration::from_millis(500)).await;
        }
        Err(ThasiaError::Discovery(
            "Timed out waiting for Suwayomi-Server".into(),
        ))
    }

    async fn kill_child(&self) -> Result<()> {
        let mut child = self.child.lock().await;
        if let Some(mut child) = child.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        Ok(())
    }
}

fn free_port() -> Result<u16> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(ThasiaError::Io)?;
    Ok(listener.local_addr().map_err(ThasiaError::Io)?.port())
}

/// Stream a child stdout/stderr pipe into a daily-rotating log file
/// (`<prefix>.YYYY-MM-DD.log`, keeping at most `LOG_MAX_FILES`).
///
/// We use line-buffered reads + a `tracing_appender::non_blocking` writer so
/// the read loop never blocks on file I/O. The non-blocking guard lives
/// inside the spawned task; dropping it at task end flushes pending writes.
fn spawn_log_pipe<R>(reader: R, dir: PathBuf, prefix: &'static str)
where
    R: tokio::io::AsyncRead + Send + Unpin + 'static,
{
    tokio::spawn(async move {
        let appender = match RollingFileAppender::builder()
            .rotation(Rotation::DAILY)
            .filename_prefix(prefix)
            .filename_suffix("log")
            .max_log_files(LOG_MAX_FILES)
            .build(&dir)
        {
            Ok(appender) => appender,
            Err(err) => {
                tracing::warn!("Could not create rolling log {prefix} in {dir:?}: {err}");
                return;
            }
        };
        let (mut writer, _guard) = tracing_appender::non_blocking(appender);
        use std::io::Write as _;
        let mut buf = BufReader::new(reader);
        let mut line = Vec::new();
        loop {
            line.clear();
            match buf.read_until(b'\n', &mut line).await {
                Ok(0) => break, // EOF: subprocess closed the pipe
                Ok(_) => {
                    let _ = writer.write_all(&line);
                }
                Err(_) => break,
            }
        }
        // `_guard` drops here, flushing the worker thread's queue.
    });
}

/// Repair the KCEF directory layout on macOS so JCEF's `dlopen()` succeeds.
///
/// KCEF downloads put the CEF framework inside
/// `bin/kcef/Frameworks/cef_server.app/Contents/Frameworks/`, but JCEF's
/// native loader (when not running inside an `.app` bundle) looks for it
/// at `bin/kcef/Frameworks/` directly. We bridge that with symlinks.
#[cfg(target_os = "macos")]
async fn fixup_kcef_layout(data_dir: &std::path::Path) {
    let frameworks = data_dir.join("bin").join("kcef").join("Frameworks");
    let source_dir = frameworks
        .join("cef_server.app")
        .join("Contents")
        .join("Frameworks");
    if !tokio::fs::try_exists(&source_dir).await.unwrap_or(false) {
        return;
    }

    // Everything JCEF resolves under `Frameworks/`: the framework itself plus
    // every helper .app bundle the renderer needs.
    const ITEMS: &[&str] = &[
        "Chromium Embedded Framework.framework",
        "jcef Helper.app",
        "jcef Helper (GPU).app",
        "jcef Helper (Plugin).app",
        "jcef Helper (Renderer).app",
    ];
    for item in ITEMS {
        let target = frameworks.join(item);
        if tokio::fs::symlink_metadata(&target).await.is_ok() {
            continue;
        }
        let source = source_dir.join(item);
        if !tokio::fs::try_exists(&source).await.unwrap_or(false) {
            continue;
        }
        if let Err(err) = tokio::fs::symlink(&source, &target).await {
            tracing::warn!("KCEF symlink {target:?} failed: {err}");
        } else {
            tracing::info!("KCEF symlink created: {item}");
        }
    }
}

#[cfg(not(target_os = "macos"))]
async fn fixup_kcef_layout(_data_dir: &std::path::Path) {}

/// Delete legacy per-PID JVM crash dumps (`hs_err_pid*.log`) left from older
/// builds. The current build writes a single overwriting `hs_err.log`, so the
/// historical dumps just clutter the logs directory.
async fn sweep_old_crash_dumps(log_dir: &std::path::Path) {
    let Ok(mut read_dir) = tokio::fs::read_dir(log_dir).await else {
        return;
    };
    while let Ok(Some(entry)) = read_dir.next_entry().await {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if name.starts_with("hs_err_pid") && name.ends_with(".log") {
            let _ = tokio::fs::remove_file(entry.path()).await;
        }
    }
}
