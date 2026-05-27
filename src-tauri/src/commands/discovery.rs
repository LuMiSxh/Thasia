use crate::events::{
    ChapterDownloadEvent, ChapterDownloadPhase, DownloadCompleteEvent, DownloadStartEvent,
    SuwayomiInstallProgressEvent, SuwayomiStateChangedEvent,
};
use crate::state::{ConvState, DiscoverySettings, DiscoveryState};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;
use tauri_specta::Event;
use thasia_source::LocalSource;
use thasia_source::suwayomi::{
    ChapterMeta, ExtensionInfo, InstalledInfo, RuntimeState, SearchPage, SourceInfo,
    SuwayomiClient, UpdateInfo,
};
use tokio::sync::{Semaphore, mpsc};
use tokio::task::JoinSet;
use tokio::time::{Duration, sleep};

#[tauri::command]
#[specta::specta]
pub async fn get_discovery_settings(
    state: State<'_, DiscoveryState>,
) -> Result<DiscoverySettings, String> {
    Ok(state.settings.read().await.clone())
}

#[tauri::command]
#[specta::specta]
pub async fn set_discovery_settings(
    settings: DiscoverySettings,
    state: State<'_, DiscoveryState>,
) -> Result<(), String> {
    {
        let mut current = state.settings.write().await;
        *current = settings.clone();
    }
    state.persist_settings(&settings).await
}

#[tauri::command]
#[specta::specta]
pub async fn suwayomi_status(state: State<'_, DiscoveryState>) -> Result<RuntimeState, String> {
    let snapshot = state.manager.snapshot().await;
    if matches!(snapshot, RuntimeState::NotRunning)
        && state.installer.installed_version().await.is_none()
    {
        Ok(RuntimeState::NotInstalled)
    } else {
        Ok(snapshot)
    }
}

#[tauri::command]
#[specta::specta]
pub async fn suwayomi_installed_info(
    state: State<'_, DiscoveryState>,
) -> Result<Option<InstalledInfo>, String> {
    state
        .installer
        .installed_info()
        .await
        .map_err(command_error)
}

#[tauri::command]
#[specta::specta]
pub async fn suwayomi_install(
    version: Option<String>,
    state: State<'_, DiscoveryState>,
    app: AppHandle,
) -> Result<(), String> {
    let _guard = state.install_lock.lock().await;
    let (tx, mut rx) = mpsc::channel(64);
    let app_for_events = app.clone();
    tokio::spawn(async move {
        while let Some(progress) = rx.recv().await {
            let _ = SuwayomiInstallProgressEvent { progress }.emit(&app_for_events);
        }
    });

    state
        .installer
        .install(version.as_deref().unwrap_or("latest"), tx)
        .await
        .map_err(command_error)?;
    let settings = state.refresh_installed_version().await?;
    let _ = SuwayomiStateChangedEvent {
        state: RuntimeState::NotRunning,
    }
    .emit(&app);
    if settings.enabled && settings.auto_start {
        state.prepare_suwayomi_config().await?;
        let port = state.manager.start().await.map_err(command_error)?;
        *state.client.write().await = Some(Arc::new(SuwayomiClient::new(port)));
        start_monitor(&state, app.clone()).await;
        let _ = SuwayomiStateChangedEvent {
            state: RuntimeState::Ready { port },
        }
        .emit(&app);
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn suwayomi_uninstall(
    state: State<'_, DiscoveryState>,
    app: AppHandle,
) -> Result<(), String> {
    stop_monitor(&state).await;
    let _ = state.manager.stop().await;
    *state.client.write().await = None;
    state.installer.uninstall().await.map_err(command_error)?;
    state.refresh_installed_version().await?;
    let _ = SuwayomiStateChangedEvent {
        state: RuntimeState::NotInstalled,
    }
    .emit(&app);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn suwayomi_check_update(state: State<'_, DiscoveryState>) -> Result<UpdateInfo, String> {
    let latest = state
        .installer
        .latest_release()
        .await
        .map_err(command_error)?;
    let current = state.installer.installed_version().await;
    {
        let mut settings = state.settings.write().await;
        settings.last_update_check = Some(now_string());
        let out = settings.clone();
        drop(settings);
        state.persist_settings(&out).await?;
    }
    Ok(UpdateInfo {
        available: current.as_ref() != Some(&latest.version),
        current_version: current,
        latest_version: latest.version,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn suwayomi_start(
    state: State<'_, DiscoveryState>,
    app: AppHandle,
) -> Result<u16, String> {
    let _ = SuwayomiStateChangedEvent {
        state: RuntimeState::Starting,
    }
    .emit(&app);
    state.prepare_suwayomi_config().await?;
    let port = state.manager.start().await.map_err(command_error)?;
    *state.client.write().await = Some(Arc::new(SuwayomiClient::new(port)));
    start_monitor(&state, app.clone()).await;
    let _ = SuwayomiStateChangedEvent {
        state: RuntimeState::Ready { port },
    }
    .emit(&app);
    Ok(port)
}

#[tauri::command]
#[specta::specta]
pub async fn suwayomi_stop(state: State<'_, DiscoveryState>, app: AppHandle) -> Result<(), String> {
    stop_monitor(&state).await;
    state.manager.stop().await.map_err(command_error)?;
    *state.client.write().await = None;
    let runtime = state.manager.snapshot().await;
    let _ = SuwayomiStateChangedEvent { state: runtime }.emit(&app);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn suwayomi_restart(
    state: State<'_, DiscoveryState>,
    app: AppHandle,
) -> Result<u16, String> {
    let _ = SuwayomiStateChangedEvent {
        state: RuntimeState::Starting,
    }
    .emit(&app);
    stop_monitor(&state).await;
    state.prepare_suwayomi_config().await?;
    let port = state.manager.restart().await.map_err(command_error)?;
    *state.client.write().await = Some(Arc::new(SuwayomiClient::new(port)));
    start_monitor(&state, app.clone()).await;
    let _ = SuwayomiStateChangedEvent {
        state: RuntimeState::Ready { port },
    }
    .emit(&app);
    Ok(port)
}

#[tauri::command]
#[specta::specta]
pub async fn suwayomi_reset_data(state: State<'_, DiscoveryState>) -> Result<(), String> {
    state.installer.reset_data().await.map_err(command_error)
}

#[tauri::command]
#[specta::specta]
pub async fn suwayomi_open_data_folder(
    state: State<'_, DiscoveryState>,
    app: AppHandle,
) -> Result<(), String> {
    tokio::fs::create_dir_all(state.installer.data_dir())
        .await
        .map_err(command_error)?;
    app.opener()
        .open_path(state.installer.data_dir().to_string_lossy(), None::<&str>)
        .map_err(command_error)
}

#[tauri::command]
#[specta::specta]
pub async fn list_installed_sources(
    state: State<'_, DiscoveryState>,
) -> Result<Vec<SourceInfo>, String> {
    client(&state)
        .await?
        .list_sources()
        .await
        .map_err(command_error)
}

#[tauri::command]
#[specta::specta]
pub async fn list_available_extensions(
    state: State<'_, DiscoveryState>,
) -> Result<Vec<ExtensionInfo>, String> {
    client(&state)
        .await?
        .list_extensions()
        .await
        .map_err(command_error)
}

#[tauri::command]
#[specta::specta]
pub async fn install_extension(
    pkg: String,
    state: State<'_, DiscoveryState>,
) -> Result<(), String> {
    client(&state)
        .await?
        .install_extension(&pkg)
        .await
        .map_err(command_error)
}

#[tauri::command]
#[specta::specta]
pub async fn uninstall_extension(
    pkg: String,
    state: State<'_, DiscoveryState>,
) -> Result<(), String> {
    client(&state)
        .await?
        .uninstall_extension(&pkg)
        .await
        .map_err(command_error)
}

#[tauri::command]
#[specta::specta]
pub async fn search_source(
    source_id: String,
    query: String,
    page: u32,
    state: State<'_, DiscoveryState>,
) -> Result<SearchPage, String> {
    client(&state)
        .await?
        .search(&source_id, &query, page)
        .await
        .map_err(command_error)
}

#[tauri::command]
#[specta::specta]
pub async fn list_chapters(
    manga_id: i64,
    state: State<'_, DiscoveryState>,
) -> Result<Vec<ChapterMeta>, String> {
    client(&state)
        .await?
        .chapters(manga_id)
        .await
        .map_err(command_error)
}

#[tauri::command]
#[specta::specta]
pub async fn download_series(
    manga_id: i64,
    chapters: Vec<ChapterMeta>,
    convert_after: bool,
    state: State<'_, DiscoveryState>,
    conv_state: State<'_, RwLock<ConvState>>,
    app: AppHandle,
) -> Result<(), String> {
    let client = client(&state).await?;
    let manga = client.manga(manga_id).await.map_err(command_error)?;
    let total = chapters.len() as u32;
    let _ = DownloadStartEvent {
        series_title: manga.title.clone(),
        total_chapters: total,
    }
    .emit(&app);

    let cancel = {
        let s = conv_state.read().map_err(command_error)?;
        s.cancel.clone()
    };
    cancel.store(false, Ordering::SeqCst);

    let chapter_ids: Vec<i64> = chapters.iter().map(|ch| ch.id).collect();
    let archive_temp = tempfile::TempDir::new().map_err(command_error)?;
    let chapter_paths = download_chapter_archives(
        client.clone(),
        &chapter_ids,
        cancel,
        &app,
        archive_temp.path(),
    )
    .await?;

    // Resolve the manga output directory.
    // If the user hasn't configured a download dir, fall back to
    // <suwayomi-data>/manga-downloads so the files are stable and findable.
    let configured_download_dir = {
        let settings = state.settings.read().await;
        settings.download_dir.clone()
    };
    let manga_dir = if let Some(dir) = configured_download_dir {
        PathBuf::from(dir).join(sanitize_file_name(&manga.title))
    } else {
        state
            .installer
            .data_dir()
            .join("manga-downloads")
            .join(sanitize_file_name(&manga.title))
    };

    // Extract each downloaded CBZ into a Hakuneko-style directory:
    //   <manga_dir>/<vol:03>-<chapter>/<001.ext>, <002.ext>, …
    //
    // This layout is already understood by the parser's HAKUNEKO_RE rule, so
    // the existing scan→parse→encode pipeline works without modification.
    for (chapter_path, chapter_meta) in chapter_paths.iter().zip(chapters.iter()) {
        let vol = chapter_meta.volume_number.unwrap_or(1);
        let ch = chapter_meta.chapter_number;
        let chapter_dest = manga_dir.join(hakuneko_dir_name(vol, ch));
        LocalSource::extract_chapter_cbz(chapter_path.clone(), chapter_dest)
            .await
            .map_err(command_error)?;
    }

    // CBZ temp archives are no longer needed; drop them before emitting events.
    drop(archive_temp);

    let output_dir = if convert_after {
        // Hand the directory to the convert pipeline via conv_state.
        // applyDiscoverySource() on the frontend will call scanCurrentSource()
        // which reads this source and proceeds through the wizard.
        let local = Arc::new(LocalSource::new(manga_dir.clone()));
        let mut s = conv_state.write().map_err(command_error)?;
        s.source = Some(local);
        s.scan_result = None;
        None
    } else {
        Some(manga_dir.to_string_lossy().to_string())
    };

    let _ = DownloadCompleteEvent {
        success: true,
        error: None,
        output_dir,
    }
    .emit(&app);
    Ok(())
}

async fn client(state: &DiscoveryState) -> Result<Arc<SuwayomiClient>, String> {
    match state.manager.snapshot().await {
        RuntimeState::Ready { .. } => {}
        runtime => {
            *state.client.write().await = None;
            return Err(match runtime {
                RuntimeState::NotInstalled => "Suwayomi-Server is not installed".to_string(),
                RuntimeState::NotRunning => "Suwayomi-Server is not running".to_string(),
                RuntimeState::Starting => "Suwayomi-Server is still starting".to_string(),
                RuntimeState::Error { message } => {
                    format!("Suwayomi-Server is not running: {message}")
                }
                RuntimeState::Ready { .. } => unreachable!(),
            });
        }
    }

    state
        .client
        .read()
        .await
        .clone()
        .ok_or_else(|| "Suwayomi-Server is not ready".to_string())
}

async fn download_chapter_archives(
    client: Arc<SuwayomiClient>,
    chapter_ids: &[i64],
    cancel: Arc<std::sync::atomic::AtomicBool>,
    app: &AppHandle,
    destination: &std::path::Path,
) -> Result<Vec<PathBuf>, String> {
    let total = chapter_ids.len() as u32;
    let semaphore = Arc::new(Semaphore::new(4));
    let mut tasks = JoinSet::new();

    for (index, chapter_id) in chapter_ids.iter().copied().enumerate() {
        let client = client.clone();
        let semaphore = semaphore.clone();
        let destination = destination.join(format!("{:04}-{chapter_id}.cbz", index + 1));
        tasks.spawn(async move {
            let _permit = semaphore.acquire_owned().await.map_err(|e| e.to_string())?;
            client
                .download_chapter_cbz(chapter_id, &destination)
                .await
                .map_err(command_error)?;
            Ok::<_, String>((index, chapter_id, destination))
        });
    }

    let mut paths: Vec<Option<PathBuf>> = vec![None; chapter_ids.len()];
    let mut completed = 0_u32;
    let mut tick = 0_u32;
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    while completed < total {
        tokio::select! {
            _ = interval.tick() => {
                tick = tick.saturating_add(1);
                if cancel.load(Ordering::SeqCst) {
                    tasks.abort_all();
                    let _ = DownloadCompleteEvent {
                        success: false,
                        error: Some("Cancelled".into()),
                        output_dir: None,
                    }
                    .emit(app);
                    return Err("Cancelled".to_string());
                }
                let _ = ChapterDownloadEvent {
                    current_chapter: format!("Downloading chapters… {tick}s"),
                    current: completed,
                    total,
                    phase: ChapterDownloadPhase::Downloading,
                    tick,
                }
                .emit(app);
            }
            result = tasks.join_next() => {
                let Some(result) = result else {
                    break;
                };
                if cancel.load(Ordering::SeqCst) {
                    tasks.abort_all();
                    let _ = DownloadCompleteEvent {
                        success: false,
                        error: Some("Cancelled".into()),
                        output_dir: None,
                    }
                    .emit(app);
                    return Err("Cancelled".to_string());
                }

                let (index, chapter_id, path) = result.map_err(|e| e.to_string())??;
                paths[index] = Some(path);
                completed += 1;
                let _ = ChapterDownloadEvent {
                    current_chapter: chapter_id.to_string(),
                    current: completed,
                    total,
                    phase: ChapterDownloadPhase::Complete,
                    tick,
                }
                .emit(app);
            }
        }
    }

    if completed == total {
        Ok(paths.into_iter().flatten().collect())
    } else {
        Err(format!(
            "Download stopped before all chapters completed ({completed}/{total})."
        ))
    }
}

pub(crate) async fn start_monitor(state: &DiscoveryState, app: AppHandle) {
    stop_monitor(state).await;

    let manager = state.manager.clone();
    let client = state.client.clone();
    let handle = tokio::spawn(async move {
        let mut restarts = 0_u32;
        loop {
            sleep(Duration::from_secs(2)).await;
            match manager.snapshot().await {
                RuntimeState::Ready { .. } | RuntimeState::Starting => {
                    restarts = 0;
                }
                RuntimeState::Error { message } if is_recoverable_exit(&message) => {
                    if restarts >= 3 {
                        *client.write().await = None;
                        let _ = SuwayomiStateChangedEvent {
                            state: RuntimeState::Error {
                                message: format!(
                                    "Suwayomi-Server keeps crashing after automatic restarts: {message}"
                                ),
                            },
                        }
                        .emit(&app);
                        break;
                    }

                    restarts += 1;
                    *client.write().await = None;
                    let _ = SuwayomiStateChangedEvent {
                        state: RuntimeState::Starting,
                    }
                    .emit(&app);

                    sleep(Duration::from_secs(restarts.min(3) as u64)).await;
                    match manager.start().await {
                        Ok(port) => {
                            *client.write().await = Some(Arc::new(SuwayomiClient::new(port)));
                            let _ = SuwayomiStateChangedEvent {
                                state: RuntimeState::Ready { port },
                            }
                            .emit(&app);
                        }
                        Err(err) => {
                            *client.write().await = None;
                            let _ = SuwayomiStateChangedEvent {
                                state: RuntimeState::Error {
                                    message: err.to_string(),
                                },
                            }
                            .emit(&app);
                        }
                    }
                }
                RuntimeState::NotRunning | RuntimeState::NotInstalled => break,
                RuntimeState::Error { .. } => {}
            }
        }
    });
    *state.monitor.lock().await = Some(handle);
}

pub(crate) async fn stop_monitor(state: &DiscoveryState) {
    if let Some(handle) = state.monitor.lock().await.take() {
        handle.abort();
    }
}

fn is_recoverable_exit(message: &str) -> bool {
    message.contains("exited unexpectedly")
        || message.contains("SIGABRT")
        || message.contains("signal: 6")
        || message.contains("SIGSEGV")
        || message.contains("signal: 11")
}

fn now_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".into())
}

fn command_error(error: impl std::fmt::Display) -> String {
    let message = error.to_string();
    message
        .strip_prefix("Discovery Error: ")
        .unwrap_or(&message)
        .to_string()
}

/// Produces the Hakuneko-style directory name for a chapter, e.g. `"001-10"`
/// or `"001-10.5"`. This matches the classifier's `HAKUNEKO_RE` pattern
/// (`^(\d+)-(\d+(?:\.\d+)?)$`) so the existing parse pipeline handles it.
///
/// We format with one decimal place and strip trailing zeros to avoid f32
/// precision artifacts (e.g. `10.1f32` → `"10.100001"` via Display).
fn hakuneko_dir_name(vol: u32, ch: f32) -> String {
    let ch_str = format!("{:.1}", ch);
    let ch_str = ch_str.trim_end_matches('0').trim_end_matches('.');
    format!("{vol:03}-{ch_str}")
}

fn sanitize_file_name(value: &str) -> String {
    let cleaned = value
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            ch if ch.is_control() => '_',
            ch => ch,
        })
        .collect::<String>()
        .trim()
        .trim_matches('.')
        .to_string();

    if cleaned.is_empty() {
        "download".to_string()
    } else {
        cleaned
    }
}
