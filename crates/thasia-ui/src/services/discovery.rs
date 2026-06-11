use crate::{
    error::{AppError, AppResult},
    state::SharedDiscoveryState,
    util::settings,
};
use std::{path::PathBuf, sync::Arc};
use thasia_source::suwayomi::{
    ChapterMeta, ExtensionInfo, InstallProgress, InstalledInfo, RuntimeState, SearchPage,
    SourceInfo, SuwayomiClient,
};
use tokio::sync::mpsc;

pub async fn status(state: &SharedDiscoveryState) -> RuntimeState {
    let snapshot = state.manager.snapshot().await;
    if matches!(snapshot, RuntimeState::NotRunning)
        && state.installer.installed_version().await.is_none()
    {
        RuntimeState::NotInstalled
    } else {
        snapshot
    }
}

pub async fn install(
    state: SharedDiscoveryState,
    progress: mpsc::Sender<InstallProgress>,
) -> AppResult<()> {
    state.installer.install("latest", progress).await?;
    let mut current = state.settings.write().await;
    current.installed_version = state.installer.installed_version().await;
    current.enabled = current.installed_version.is_some();
    settings::save_discovery(&state.paths, &current)?;
    Ok(())
}

pub async fn start(state: SharedDiscoveryState) -> AppResult<RuntimeState> {
    prepare_config(&state).await?;
    let port = state.manager.start().await?;
    *state.client.write().await = Some(Arc::new(SuwayomiClient::new(port)));
    Ok(RuntimeState::Ready { port })
}

pub async fn stop(state: SharedDiscoveryState) -> AppResult<RuntimeState> {
    state.manager.stop().await?;
    *state.client.write().await = None;
    Ok(state.manager.snapshot().await)
}

pub async fn restart(state: SharedDiscoveryState) -> AppResult<RuntimeState> {
    prepare_config(&state).await?;
    let port = state.manager.restart().await?;
    *state.client.write().await = Some(Arc::new(SuwayomiClient::new(port)));
    Ok(RuntimeState::Ready { port })
}

pub async fn uninstall(state: SharedDiscoveryState) -> AppResult<()> {
    let _ = state.manager.stop().await;
    *state.client.write().await = None;
    state.installer.uninstall().await?;
    let mut current = state.settings.write().await;
    current.installed_version = None;
    current.enabled = false;
    settings::save_discovery(&state.paths, &current)?;
    Ok(())
}

pub async fn installed_info(state: &SharedDiscoveryState) -> AppResult<Option<InstalledInfo>> {
    Ok(state.installer.installed_info().await?)
}

pub async fn list_extensions(state: &SharedDiscoveryState) -> AppResult<Vec<ExtensionInfo>> {
    Ok(client(state).await?.list_extensions().await?)
}

pub async fn install_extension(state: &SharedDiscoveryState, package: &str) -> AppResult<()> {
    client(state).await?.install_extension(package).await?;
    Ok(())
}

pub async fn uninstall_extension(state: &SharedDiscoveryState, package: &str) -> AppResult<()> {
    client(state).await?.uninstall_extension(package).await?;
    Ok(())
}

pub async fn list_sources(state: &SharedDiscoveryState) -> AppResult<Vec<SourceInfo>> {
    Ok(client(state).await?.list_sources().await?)
}

pub async fn search(
    state: &SharedDiscoveryState,
    source_id: &str,
    query: &str,
    page: u32,
) -> AppResult<SearchPage> {
    Ok(client(state).await?.search(source_id, query, page).await?)
}

pub async fn chapters(state: &SharedDiscoveryState, manga_id: i64) -> AppResult<Vec<ChapterMeta>> {
    Ok(client(state).await?.chapters(manga_id).await?)
}

pub async fn download_chapters(
    state: &SharedDiscoveryState,
    manga_id: i64,
    chapters: &[ChapterMeta],
    progress: Arc<dyn Fn(usize, usize, String) + Send + Sync>,
) -> AppResult<PathBuf> {
    let client = client(state).await?;
    let manga = client.manga(manga_id).await?;
    let configured = state.settings.read().await.download_dir.clone();
    let root = configured.unwrap_or_else(|| state.installer.data_dir().join("manga-downloads"));
    let destination = root.join(sanitize_file_name(&manga.title));
    let total = chapters.len();
    for (index, chapter) in chapters.iter().enumerate() {
        progress(index, total, chapter.name.clone());
        let chapter_dir = destination.join(format!(
            "{:03}-{:07.2}",
            chapter.volume_number.unwrap_or(1),
            chapter.chapter_number
        ));
        client
            .download_chapter_pages(chapter.id, &chapter_dir)
            .await?;
    }
    progress(total, total, "Complete".into());
    Ok(destination)
}

async fn client(state: &SharedDiscoveryState) -> AppResult<Arc<SuwayomiClient>> {
    state
        .client
        .read()
        .await
        .clone()
        .ok_or_else(|| AppError::Message("Suwayomi is not ready.".into()))
}

async fn prepare_config(state: &SharedDiscoveryState) -> AppResult<()> {
    let repos = state.settings.read().await.extension_repos.clone();
    let path = state.installer.data_dir().join("server.conf");
    tokio::fs::create_dir_all(state.installer.data_dir()).await?;
    let repos = repos
        .iter()
        .map(|repo| format!("\"{}\"", repo.replace('"', "\\\"")))
        .collect::<Vec<_>>()
        .join(", ");
    tokio::fs::write(
        path,
        format!(
            "server {{\n  ip = \"127.0.0.1\"\n  webUIEnabled = false\n  systemTrayEnabled = false\n  extensionRepos = [{repos}]\n}}\n"
        ),
    )
    .await?;
    Ok(())
}

fn sanitize_file_name(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => ch,
        })
        .collect::<String>();
    let sanitized = sanitized.trim().trim_end_matches(['.', ' ']);
    if sanitized.is_empty() {
        "download".into()
    } else {
        sanitized.to_string()
    }
}
