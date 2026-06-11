use crate::{
    actions::{NavigateConvert, NavigateDiscover, NavigateHome, NavigateSettings, ToggleSidebar},
    models::{
        ConversionEvent, ConversionSummary, ConvertOptions, PageEditEntry, PageEditSource,
        PipelinePlan, VolumeEdit, VolumeMeta,
    },
    services::{conversion, discovery, pipeline_plan, scan},
    state::{SharedConvState, SharedDiscoveryState},
    updater,
    util::{
        paths::AppPaths,
        runtime,
        settings::{self, AppSettings, ThemePreference},
    },
    views::{
        discover::{DiscoverEvent, DiscoverView},
        home::{HomeEvent, HomeView},
        settings::{SettingsEvent, SettingsView},
    },
};
use nasrin::{
    Alert, Badge, Button, ButtonSize, ButtonVariant, FeedbackVariant, FlavourContextExt,
    GradientDirection, GradientStyleExt, Icon, IconName, Input, PathDisplay, ProgressBar,
    SemanticStyleExt,
    gpui::{
        AnyElement, Context, Div, Entity, ObjectFit, Render, SharedString, StyledImage, Window,
        div, img, prelude::*, rems,
    },
};
use std::{collections::HashSet, path::PathBuf, sync::Arc};
use thasia_core::prelude::{
    BundleMode, ColorEnhanceMode, Direction, ImageFormat, OutputFormat, SharpenMode,
};
use thasia_source::suwayomi::{
    ChapterMeta, ExtensionInfo, InstalledInfo, RuntimeState, SearchResult, SourceInfo,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Page {
    #[default]
    Home,
    Convert,
    Discover,
    Settings,
}

impl Page {
    const fn label(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Convert => "Convert",
            Self::Discover => "Discover",
            Self::Settings => "Settings",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum ConvertStep {
    #[default]
    Source,
    Output,
    Volumes,
    Pages,
    Review,
    Converting,
    Complete,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum DiscoverTab {
    #[default]
    Catalog,
    Suwayomi,
}

#[derive(Default)]
struct ConversionProgress {
    volume_name: String,
    volume_num: u32,
    total_volumes: u32,
    current: u32,
    total: u32,
    pages_per_sec: f64,
    elapsed_secs: f64,
    estimated_remaining_secs: Option<f64>,
    input_bytes: u64,
    output_bytes: u64,
    passthrough_pages: u32,
    encoded_pages: u32,
    fetch_ms: f64,
    decode_ms: f64,
    transform_ms: f64,
    encode_ms: f64,
    completed_outputs: Vec<String>,
}

pub struct ThasiaApp {
    page: Page,
    sidebar_open: bool,
    conv_state: SharedConvState,
    discovery_state: SharedDiscoveryState,
    paths: AppPaths,
    app_settings: AppSettings,
    home: Entity<HomeView>,
    discover_view: Entity<DiscoverView>,
    settings_view: Entity<SettingsView>,
    convert_step: ConvertStep,
    source_path: Option<PathBuf>,
    volumes: Vec<VolumeMeta>,
    volume_edits: Vec<VolumeEdit>,
    active_volume: usize,
    options: ConvertOptions,
    pipeline_plan: Option<PipelinePlan>,
    progress: ConversionProgress,
    summary: Option<ConversionSummary>,
    busy: bool,
    error: Option<String>,
    runtime_state: RuntimeState,
    search_input: Entity<Input>,
    sources: Vec<SourceInfo>,
    selected_source: Option<String>,
    search_results: Vec<SearchResult>,
    selected_series: Option<SearchResult>,
    chapters: Vec<ChapterMeta>,
    selected_chapters: HashSet<i64>,
    extensions: Vec<ExtensionInfo>,
    installed_suwayomi: Option<InstalledInfo>,
    download_current: usize,
    download_total: usize,
    download_label: String,
    discovery_busy: bool,
    discovery_message: Option<String>,
    discover_tab: DiscoverTab,
    update_available: Option<String>,
    update_message: Option<String>,
    update_busy: bool,
}

impl ThasiaApp {
    pub fn new(
        conv_state: SharedConvState,
        discovery_state: SharedDiscoveryState,
        paths: AppPaths,
        app_settings: AppSettings,
        cx: &mut Context<Self>,
    ) -> Self {
        let search_input = cx.new(|cx| Input::new("", "Search manga", cx));
        let home = cx.new(|_| HomeView::new(paths.clone()));
        let discover_view =
            cx.new(|cx| DiscoverView::new(discovery_state.clone(), cx));
        let settings_view =
            cx.new(|_| SettingsView::new(paths.clone(), app_settings.clone()));
        cx.subscribe(&home, |app, _, event, cx| match event {
            HomeEvent::Navigate(page) => app.navigate(*page, cx),
        })
        .detach();
        cx.subscribe(&discover_view, |app, _, event, cx| match event {
            DiscoverEvent::RuntimeChanged(runtime) => {
                app.runtime_state = runtime.clone();
                app.home
                    .update(cx, |home, cx| home.set_runtime(runtime.clone(), cx));
                cx.notify();
            }
        })
        .detach();
        cx.subscribe(&settings_view, |app, _, event, cx| match event {
            SettingsEvent::Changed(settings) => {
                app.app_settings = settings.clone();
                cx.notify();
            }
        })
        .detach();
        discover_view.update(cx, |view, cx| view.refresh(cx));
        let mut options = ConvertOptions::default();
        if let Some(output) = &app_settings.default_output_dir {
            options.output_dir = output.clone();
        }
        Self {
            page: Page::Home,
            sidebar_open: true,
            conv_state,
            discovery_state,
            paths,
            app_settings,
            home,
            discover_view,
            settings_view,
            convert_step: ConvertStep::Source,
            source_path: None,
            volumes: Vec::new(),
            volume_edits: Vec::new(),
            active_volume: 0,
            options,
            pipeline_plan: None,
            progress: ConversionProgress::default(),
            summary: None,
            busy: false,
            error: None,
            runtime_state: RuntimeState::NotRunning,
            search_input,
            sources: Vec::new(),
            selected_source: None,
            search_results: Vec::new(),
            selected_series: None,
            chapters: Vec::new(),
            selected_chapters: HashSet::new(),
            extensions: Vec::new(),
            installed_suwayomi: None,
            download_current: 0,
            download_total: 0,
            download_label: String::new(),
            discovery_busy: false,
            discovery_message: None,
            discover_tab: DiscoverTab::Catalog,
            update_available: None,
            update_message: None,
            update_busy: false,
        }
    }

    fn navigate(&mut self, page: Page, cx: &mut Context<Self>) {
        if self.convert_step != ConvertStep::Converting {
            self.page = page;
            cx.notify();
        }
    }

    fn set_theme_preference(
        &mut self,
        preference: ThemePreference,
        cx: &mut Context<Self>,
    ) {
        self.app_settings.theme = preference;
        let mode = match preference {
            ThemePreference::Light => nasrin::ThemeMode::Light,
            ThemePreference::Dark => nasrin::ThemeMode::Dark,
            ThemePreference::System => nasrin::ThemeMode::System,
        };
        cx.set_theme_mode(mode);
        let _ = settings::save_app(&self.paths, &self.app_settings);
        cx.notify();
    }

    pub fn check_for_updates(&mut self, cx: &mut Context<Self>) {
        if self.update_busy {
            return;
        }
        self.update_busy = true;
        self.update_message = Some("Checking for updates...".into());
        cx.spawn(async move |this, cx| {
            let result = runtime::app(async {
                tokio::task::spawn_blocking(updater::latest_version)
                    .await
                    .map_err(|error| crate::error::AppError::Message(error.to_string()))?
                    .map_err(|error| crate::error::AppError::Message(error.to_string()))
            })
            .await;
            let _ = this.update(cx, |app, cx| {
                app.update_busy = false;
                match result {
                    Ok(Some(version)) => {
                        app.update_available = Some(version.clone());
                        app.update_message = Some(format!("Thasia {version} is available."));
                    }
                    Ok(None) => {
                        app.update_available = None;
                        app.update_message = Some("Thasia is up to date.".into());
                    }
                    Err(error) => app.update_message = Some(error.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn install_update(&mut self, cx: &mut Context<Self>) {
        if self.update_busy {
            return;
        }
        self.update_busy = true;
        self.update_message = Some("Downloading and installing update...".into());
        cx.spawn(async move |this, cx| {
            let result = runtime::app(async {
                tokio::task::spawn_blocking(updater::install_latest)
                    .await
                    .map_err(|error| crate::error::AppError::Message(error.to_string()))?
                    .map_err(|error| crate::error::AppError::Message(error.to_string()))
            })
            .await;
            let _ = this.update(cx, |app, cx| {
                app.update_busy = false;
                app.update_message = Some(match result {
                    Ok(version) => {
                        format!("Updated to {version}. Restart Thasia to use the new version.")
                    }
                    Err(error) => error.to_string(),
                });
                cx.notify();
            });
        })
        .detach();
    }

    fn pick_source(&mut self, cx: &mut Context<Self>) {
        self.error = None;
        cx.spawn(async move |this, cx| {
            let selection = rfd::AsyncFileDialog::new()
                .add_filter("Manga archives", &["zip", "cbz"])
                .pick_file()
                .await
                .map(|file| file.path().to_path_buf());
            let selection = match selection {
                Some(path) => Some(path),
                None => rfd::AsyncFileDialog::new()
                    .pick_folder()
                    .await
                    .map(|folder| folder.path().to_path_buf()),
            };
            if let Some(path) = selection {
                let _ = this.update(cx, |app, cx| {
                    app.options.output_name = path
                        .file_stem()
                        .and_then(|name| name.to_str())
                        .filter(|name| !name.is_empty())
                        .unwrap_or("output")
                        .to_string();
                    app.source_path = Some(path);
                    cx.notify();
                });
            }
        })
        .detach();
    }

    fn pick_output(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let folder = rfd::AsyncFileDialog::new().pick_folder().await;
            if let Some(folder) = folder {
                let _ = this.update(cx, |app, cx| {
                    app.options.output_dir = folder.path().to_path_buf();
                    app.app_settings.default_output_dir = Some(app.options.output_dir.clone());
                    let _ = settings::save_app(&app.paths, &app.app_settings);
                    cx.notify();
                });
            }
        })
        .detach();
    }

    fn scan_source(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.source_path.clone() else {
            return;
        };
        self.busy = true;
        self.error = None;
        let state = self.conv_state.clone();
        cx.spawn(async move |this, cx| {
            let result = runtime::app(scan::scan_source(path, state)).await;
            let _ = this.update(cx, |app, cx| {
                app.busy = false;
                match result {
                    Ok(volumes) => {
                        app.volumes = volumes;
                        app.volume_edits = app.default_volume_edits();
                        app.active_volume = 0;
                        app.convert_step = ConvertStep::Output;
                    }
                    Err(error) => app.error = Some(error.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn default_volume_edits(&self) -> Vec<VolumeEdit> {
        self.volumes
            .iter()
            .map(|volume| VolumeEdit {
                volume_num: volume.volume_num,
                pages: volume
                    .pages
                    .iter()
                    .map(|page| PageEditEntry {
                        source: PageEditSource::Original {
                            page_index: page.page_index,
                            source_volume_num: Some(page.volume_num),
                        },
                        excluded: false,
                    })
                    .collect(),
            })
            .collect()
    }

    fn volume_edits(&self) -> Vec<VolumeEdit> {
        self.volume_edits.clone()
    }

    fn prepare_review(&mut self, cx: &mut Context<Self>) {
        if self.options.output_dir.as_os_str().is_empty() {
            self.error = Some("Select an output directory first.".into());
            cx.notify();
            return;
        }
        self.error = None;
        self.pipeline_plan = Some(pipeline_plan::build(
            self.options.clone(),
            self.volume_edits(),
        ));
        self.convert_step = ConvertStep::Review;
        cx.notify();
    }

    fn start_conversion(&mut self, cx: &mut Context<Self>) {
        let options = self.options.clone();
        let edits = self.volume_edits();
        let state = self.conv_state.clone();
        let (events_tx, mut events_rx) = tokio::sync::mpsc::unbounded_channel();
        let on_event = Arc::new(move |event| {
            let _ = events_tx.send(event);
        });

        self.convert_step = ConvertStep::Converting;
        self.busy = true;
        self.error = None;
        self.summary = None;
        self.progress = ConversionProgress::default();
        cx.notify();

        cx.spawn(async move |this, cx| {
            while let Some(event) = events_rx.recv().await {
                let _ = this.update(cx, |app, cx| {
                    app.apply_conversion_event(event);
                    cx.notify();
                });
            }
        })
        .detach();

        cx.spawn(async move |this, cx| {
            let result =
                runtime::app(conversion::run_conversion(options, edits, state, on_event)).await;
            let _ = this.update(cx, |app, cx| {
                app.busy = false;
                match result {
                    Ok(summary) => {
                        app.summary = Some(summary);
                        app.convert_step = ConvertStep::Complete;
                    }
                    Err(error) => {
                        app.error = Some(error.to_string());
                        app.convert_step = ConvertStep::Review;
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn apply_conversion_event(&mut self, event: ConversionEvent) {
        match event {
            ConversionEvent::VolumeStarted {
                volume_num,
                volume_name,
                total_volumes,
            } => {
                self.progress.volume_num = volume_num;
                self.progress.volume_name = volume_name;
                self.progress.total_volumes = total_volumes;
                self.progress.current = 0;
                self.progress.total = 0;
            }
            ConversionEvent::ImageProgress {
                volume_num,
                current,
                total,
                elapsed_secs,
                pages_per_sec,
                estimated_remaining_secs,
                input_bytes,
                output_bytes,
                passthrough_pages,
                encoded_pages,
                fetch_ms,
                decode_ms,
                transform_ms,
                encode_ms,
            } => {
                self.progress.volume_num = volume_num;
                self.progress.current = current;
                self.progress.total = total;
                self.progress.pages_per_sec = pages_per_sec;
                self.progress.elapsed_secs = elapsed_secs;
                self.progress.estimated_remaining_secs = estimated_remaining_secs;
                self.progress.input_bytes = input_bytes;
                self.progress.output_bytes = output_bytes;
                self.progress.passthrough_pages = passthrough_pages;
                self.progress.encoded_pages = encoded_pages;
                self.progress.fetch_ms = fetch_ms;
                self.progress.decode_ms = decode_ms;
                self.progress.transform_ms = transform_ms;
                self.progress.encode_ms = encode_ms;
            }
            ConversionEvent::VolumeCompleted {
                volume_num,
                success: false,
                error,
                output_path,
            } => {
                self.error = error.or_else(|| Some(format!("Volume {volume_num} failed")));
                if let Some(path) = output_path {
                    self.progress.completed_outputs.push(path);
                }
            }
            ConversionEvent::VolumeCompleted {
                output_path: Some(path),
                ..
            } => self.progress.completed_outputs.push(path),
            ConversionEvent::VolumeCompleted { .. } => {}
        }
    }

    fn cancel_conversion(&mut self, cx: &mut Context<Self>) {
        conversion::runner::cancel(&self.conv_state);
        self.error = Some("Cancellation requested.".into());
        cx.notify();
    }

    fn reset_conversion(&mut self, cx: &mut Context<Self>) {
        self.convert_step = ConvertStep::Source;
        self.source_path = None;
        self.volumes.clear();
        self.volume_edits.clear();
        self.active_volume = 0;
        self.options = ConvertOptions::default();
        if let Some(output) = &self.app_settings.default_output_dir {
            self.options.output_dir = output.clone();
        }
        self.pipeline_plan = None;
        self.progress = ConversionProgress::default();
        self.summary = None;
        self.error = None;
        cx.notify();
    }

    fn refresh_discovery(&mut self, cx: &mut Context<Self>) {
        self.discovery_busy = true;
        self.discovery_message = None;
        let state = self.discovery_state.clone();
        cx.spawn(async move |this, cx| {
            let backend = runtime::value(async move {
                let runtime_state = discovery::status(&state).await;
                let (sources, extensions) =
                    if matches!(runtime_state, RuntimeState::Ready { .. }) {
                        (
                            discovery::list_sources(&state).await,
                            discovery::list_extensions(&state).await,
                        )
                    } else {
                        (Ok(Vec::new()), Ok(Vec::new()))
                    };
                let installed = discovery::installed_info(&state).await;
                (runtime_state, sources, extensions, installed)
            })
            .await;
            let _ = this.update(cx, |app, cx| {
                app.discovery_busy = false;
                let Ok((runtime_state, sources, extensions, installed)) = backend else {
                    app.discovery_message = Some(backend.unwrap_err().to_string());
                    cx.notify();
                    return;
                };
                app.runtime_state = runtime_state.clone();
                app.home.update(cx, |home, cx| {
                    home.set_runtime(runtime_state.clone(), cx)
                });
                match sources {
                    Ok(sources) => {
                        app.sources = sources;
                        if app.selected_source.is_none() {
                            app.selected_source =
                                app.sources.first().map(|source| source.id.clone());
                        }
                    }
                    Err(error) => app.discovery_message = Some(error.to_string()),
                }
                match extensions {
                    Ok(extensions) => app.extensions = extensions,
                    Err(error) => app.discovery_message = Some(error.to_string()),
                }
                match installed {
                    Ok(info) => app.installed_suwayomi = info,
                    Err(error) => app.discovery_message = Some(error.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn install_suwayomi(&mut self, cx: &mut Context<Self>) {
        self.discovery_busy = true;
        self.discovery_message = Some("Downloading Suwayomi...".into());
        let state = self.discovery_state.clone();
        let (tx, mut rx) = tokio::sync::mpsc::channel(32);
        cx.spawn(async move |this, cx| {
            while let Some(progress) = rx.recv().await {
                let _ = this.update(cx, |app, cx| {
                    app.discovery_message = Some(format!("{progress:?}"));
                    cx.notify();
                });
            }
        })
        .detach();
        cx.spawn(async move |this, cx| {
            let result = runtime::app(discovery::install(state, tx)).await;
            let _ = this.update(cx, |app, cx| {
                app.discovery_busy = false;
                match result {
                    Ok(()) => {
                        app.runtime_state = RuntimeState::NotRunning;
                        app.discovery_message = Some("Suwayomi installed.".into());
                    }
                    Err(error) => app.discovery_message = Some(error.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn start_suwayomi(&mut self, cx: &mut Context<Self>) {
        self.discovery_busy = true;
        self.runtime_state = RuntimeState::Starting;
        let state = self.discovery_state.clone();
        cx.spawn(async move |this, cx| {
            let backend = runtime::value(async move {
                let result = discovery::start(state.clone()).await;
                let sources = match &result {
                    Ok(RuntimeState::Ready { .. }) => discovery::list_sources(&state).await,
                    _ => Ok(Vec::new()),
                };
                (result, sources)
            })
            .await;
            let _ = this.update(cx, |app, cx| {
                app.discovery_busy = false;
                let Ok((result, sources)) = backend else {
                    app.runtime_state = RuntimeState::Error {
                        message: backend.unwrap_err().to_string(),
                    };
                    cx.notify();
                    return;
                };
                match result {
                    Ok(runtime) => app.runtime_state = runtime,
                    Err(error) => {
                        app.runtime_state = RuntimeState::Error {
                            message: error.to_string(),
                        };
                    }
                }
                if let Ok(sources) = sources {
                    app.sources = sources;
                    app.selected_source = app.sources.first().map(|source| source.id.clone());
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn stop_suwayomi(&mut self, cx: &mut Context<Self>) {
        self.discovery_busy = true;
        let state = self.discovery_state.clone();
        cx.spawn(async move |this, cx| {
            let result = runtime::app(discovery::stop(state)).await;
            let _ = this.update(cx, |app, cx| {
                app.discovery_busy = false;
                match result {
                    Ok(runtime) => {
                        app.runtime_state = runtime;
                        app.sources.clear();
                        app.search_results.clear();
                    }
                    Err(error) => app.discovery_message = Some(error.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn restart_suwayomi(&mut self, cx: &mut Context<Self>) {
        self.discovery_busy = true;
        self.runtime_state = RuntimeState::Starting;
        let state = self.discovery_state.clone();
        cx.spawn(async move |this, cx| {
            let backend = runtime::value(async move {
                let result = discovery::restart(state.clone()).await;
                let extensions = discovery::list_extensions(&state).await;
                let sources = discovery::list_sources(&state).await;
                (result, extensions, sources)
            })
            .await;
            let _ = this.update(cx, |app, cx| {
                app.discovery_busy = false;
                let Ok((result, extensions, sources)) = backend else {
                    app.runtime_state = RuntimeState::Error {
                        message: backend.unwrap_err().to_string(),
                    };
                    cx.notify();
                    return;
                };
                match result {
                    Ok(runtime) => app.runtime_state = runtime,
                    Err(error) => {
                        app.runtime_state = RuntimeState::Error {
                            message: error.to_string(),
                        }
                    }
                }
                if let Ok(extensions) = extensions {
                    app.extensions = extensions;
                }
                if let Ok(sources) = sources {
                    app.sources = sources;
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn uninstall_suwayomi(&mut self, cx: &mut Context<Self>) {
        self.discovery_busy = true;
        let state = self.discovery_state.clone();
        cx.spawn(async move |this, cx| {
            let result = runtime::app(discovery::uninstall(state)).await;
            let _ = this.update(cx, |app, cx| {
                app.discovery_busy = false;
                match result {
                    Ok(()) => {
                        app.runtime_state = RuntimeState::NotInstalled;
                        app.installed_suwayomi = None;
                        app.sources.clear();
                        app.extensions.clear();
                        app.discovery_message = Some("Suwayomi was removed.".into());
                    }
                    Err(error) => app.discovery_message = Some(error.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn set_extension_installed(
        &mut self,
        package: String,
        installed: bool,
        cx: &mut Context<Self>,
    ) {
        self.discovery_busy = true;
        let state = self.discovery_state.clone();
        cx.spawn(async move |this, cx| {
            let backend = runtime::value(async move {
                let result = if installed {
                    discovery::uninstall_extension(&state, &package).await
                } else {
                    discovery::install_extension(&state, &package).await
                };
                let extensions = discovery::list_extensions(&state).await;
                let sources = discovery::list_sources(&state).await;
                (result, extensions, sources)
            })
            .await;
            let _ = this.update(cx, |app, cx| {
                app.discovery_busy = false;
                let Ok((result, extensions, sources)) = backend else {
                    app.discovery_message = Some(backend.unwrap_err().to_string());
                    cx.notify();
                    return;
                };
                if let Err(error) = result {
                    app.discovery_message = Some(error.to_string());
                }
                if let Ok(extensions) = extensions {
                    app.extensions = extensions;
                }
                if let Ok(sources) = sources {
                    app.sources = sources;
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn search_discovery(&mut self, cx: &mut Context<Self>) {
        let Some(source_id) = self.selected_source.clone() else {
            return;
        };
        let query = self.search_input.read(cx).value().trim().to_string();
        if query.is_empty() {
            return;
        }
        self.discovery_busy = true;
        self.discovery_message = None;
        let state = self.discovery_state.clone();
        cx.spawn(async move |this, cx| {
            let result =
                runtime::app(async move { discovery::search(&state, &source_id, &query, 1).await })
                    .await;
            let _ = this.update(cx, |app, cx| {
                app.discovery_busy = false;
                match result {
                    Ok(page) => app.search_results = page.results,
                    Err(error) => app.discovery_message = Some(error.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn select_series(&mut self, series: SearchResult, cx: &mut Context<Self>) {
        self.discovery_busy = true;
        self.selected_series = Some(series.clone());
        let state = self.discovery_state.clone();
        cx.spawn(async move |this, cx| {
            let result =
                runtime::app(async move { discovery::chapters(&state, series.id).await }).await;
            let _ = this.update(cx, |app, cx| {
                app.discovery_busy = false;
                match result {
                    Ok(chapters) => {
                        app.selected_chapters =
                            chapters.iter().map(|chapter| chapter.id).collect();
                        app.chapters = chapters;
                    }
                    Err(error) => app.discovery_message = Some(error.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn download_series(&mut self, cx: &mut Context<Self>) {
        let Some(series) = self.selected_series.clone() else {
            return;
        };
        let chapters = self
            .chapters
            .iter()
            .filter(|chapter| self.selected_chapters.contains(&chapter.id))
            .cloned()
            .collect::<Vec<_>>();
        if chapters.is_empty() {
            return;
        }
        self.discovery_busy = true;
        self.download_current = 0;
        self.download_total = chapters.len();
        self.download_label.clear();
        self.discovery_message = Some(format!("Downloading {} chapters...", chapters.len()));
        let state = self.discovery_state.clone();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let progress = Arc::new(move |current, total, label| {
            let _ = tx.send((current, total, label));
        });
        cx.spawn(async move |this, cx| {
            while let Some((current, total, label)) = rx.recv().await {
                let _ = this.update(cx, |app, cx| {
                    app.download_current = current;
                    app.download_total = total;
                    app.download_label = label;
                    cx.notify();
                });
            }
        })
        .detach();
        cx.spawn(async move |this, cx| {
            let result = runtime::app(async move {
                discovery::download_chapters(&state, series.id, &chapters, progress).await
            })
            .await;
            let _ = this.update(cx, |app, cx| {
                app.discovery_busy = false;
                app.discovery_message = Some(match result {
                    Ok(path) => format!("Downloaded to {}", path.display()),
                    Err(error) => error.to_string(),
                });
                cx.notify();
            });
        })
        .detach();
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> AnyElement {
        let this = cx.entity();
        let active = self.page;
        let open = self.sidebar_open;
        let wizard = self.page == Page::Convert;
        let items = [
            (Page::Home, IconName::LayoutDashboard),
            (Page::Convert, IconName::Layers),
            (Page::Discover, IconName::Search),
            (Page::Settings, IconName::Settings),
        ];
        let mut sidebar = div()
            .vertical()
            .flex_shrink_0()
            .w(if open { rems(11.0) } else { rems(3.0) })
            .min_w(if open { rems(11.0) } else { rems(3.0) })
            .h_full()
            .elevation_surface(cx)
            .border_r_1()
            .child(
                div()
                    .h(rems(2.75))
                    .px(if open { rems(1.0) } else { rems(0.5) })
                    .horizontal()
                    .align_center()
                    .gap_dense()
                    .border_b_1()
                    .child(
                        img(self.paths.assets.join("pfp.png"))
                            .size(if open { rems(2.0) } else { rems(1.5) })
                            .rounded_full()
                            .object_fit(ObjectFit::Cover),
                    )
                    .when(open, |header| {
                        header.child(
                            div()
                                .vertical()
                                .child(div().text_body().text_strong().child("THASIA"))
                                .child(
                                    div().text_size(rems(0.55)).text_muted(cx).child(if wizard {
                                        "STEPS"
                                    } else {
                                        "WORKSPACE"
                                    }),
                                ),
                        )
                    }),
            );

        if wizard {
            let steps = [
                (ConvertStep::Source, "Setup"),
                (ConvertStep::Output, "Output"),
                (ConvertStep::Volumes, "Volumes"),
                (ConvertStep::Pages, "Pages"),
                (ConvertStep::Review, "Review"),
                (ConvertStep::Converting, "Convert"),
            ];
            sidebar = sidebar.child(
                div()
                    .flex_1()
                    .vertical()
                    .gap_compact()
                    .p(rems(0.5))
                    .children(steps.into_iter().enumerate().map(|(index, (step, label))| {
                        let current = convert_step_index(self.convert_step);
                        let target = convert_step_index(step);
                        let locked = target > current && self.convert_step != ConvertStep::Complete;
                        let entity = this.clone();
                        Button::new(if open {
                            format!("{}  {}", index + 1, label)
                        } else {
                            format!("{}", index + 1)
                        })
                        .size(ButtonSize::Small)
                        .variant(if target == current {
                            ButtonVariant::Secondary
                        } else {
                            ButtonVariant::Ghost
                        })
                        .disabled(locked || self.convert_step == ConvertStep::Converting)
                        .on_click(move |_, _, cx| {
                            entity.update(cx, |app, cx| {
                                app.convert_step = step;
                                cx.notify();
                            });
                        })
                    })),
            );
        } else {
            sidebar = sidebar.child(
                div()
                    .flex_1()
                    .vertical()
                    .gap_compact()
                    .p(rems(0.5))
                    .children(items.into_iter().enumerate().map(|(index, (page, icon))| {
                        let this = this.clone();
                        if open {
                            Button::new(page.label())
                                .icon(icon)
                                .size(ButtonSize::Small)
                                .variant(if page == active {
                                    ButtonVariant::Secondary
                                } else {
                                    ButtonVariant::Ghost
                                })
                                .on_click(move |_, _, cx| {
                                    this.update(cx, |app, cx| app.navigate(page, cx));
                                })
                                .into_any_element()
                        } else {
                            div()
                                .id(("nav", index))
                                .size(rems(2.0))
                                .flex_shrink_0()
                                .centered()
                                .rounded_lg()
                                .text_size(rems(0.7))
                                .text_strong()
                                .when(page == active, |element| element.elevation_panel(cx))
                                .on_click(move |_, _, cx| {
                                    this.update(cx, |app, cx| app.navigate(page, cx));
                                })
                                .child(&page.label()[..1])
                                .into_any_element()
                        }
                    })),
            );
        }

        let theme = this.clone();
        let collapse = this.clone();
        sidebar
            .child(
                div()
                    .vertical()
                    .gap_compact()
                    .p(rems(0.5))
                    .border_t_1()
                    .child(if open {
                        Button::new("Toggle theme")
                            .icon(IconName::Moon)
                            .size(ButtonSize::Small)
                            .variant(ButtonVariant::Ghost)
                            .on_click(move |_, _, cx| {
                                theme.update(cx, |app, cx| {
                                    let preference = match cx.theme_mode() {
                                        nasrin::ThemeMode::Dark => ThemePreference::Light,
                                        _ => ThemePreference::Dark,
                                    };
                                    app.set_theme_preference(preference, cx);
                                });
                            })
                            .into_any_element()
                    } else {
                        compact_text_control("T", "theme", theme, |app, cx| {
                            let preference = match cx.theme_mode() {
                                nasrin::ThemeMode::Dark => ThemePreference::Light,
                                _ => ThemePreference::Dark,
                            };
                            app.set_theme_preference(preference, cx);
                        })
                    })
                    .child(if open {
                        Button::new("Collapse")
                            .icon(IconName::PanelLeft)
                            .size(ButtonSize::Small)
                            .variant(ButtonVariant::Ghost)
                            .on_click(move |_, _, cx| {
                                collapse.update(cx, |app, cx| {
                                    app.sidebar_open = !app.sidebar_open;
                                    cx.notify();
                                });
                            })
                            .into_any_element()
                    } else {
                        compact_text_control("<", "collapse", collapse, |app, _| {
                            app.sidebar_open = !app.sidebar_open;
                        })
                    }),
            )
            .into_any_element()
    }

    fn render_home(&self, cx: &mut Context<Self>) -> AnyElement {
        let convert = cx.entity();
        let discover = cx.entity();
        let settings = cx.entity();
        div()
            .fill()
            .relative()
            .overflow_hidden()
            .child(
                div()
                    .absolute()
                    .right_0()
                    .top_0()
                    .bottom_0()
                    .w(rems(34.0))
                    .min_w(rems(1.0))
                    .child(
                        img(self.paths.assets.join("pfp.png"))
                            .w(rems(34.0))
                            .h(rems(42.0))
                            .object_fit(ObjectFit::Cover)
                            .opacity(0.9),
                    ),
            )
            .child(
                div()
                    .relative()
                    .h_full()
                    .w(rems(35.0))
                    .vertical()
                    .justify_center()
                    .gap(rems(2.0))
                    .px(rems(3.5))
                    .child(
                        div()
                            .vertical()
                            .gap_dense()
                            .child(
                                div()
                                    .horizontal()
                                    .gap_dense()
                                    .child(Badge::new(format!("v{}", env!("CARGO_PKG_VERSION"))))
                                    .child(Badge::new(runtime_label(&self.runtime_state))),
                            )
                            .child(
                                div()
                                    .text_size(rems(4.5))
                                    .line_height(rems(4.5))
                                    .text_strong()
                                    .text_accent(cx)
                                    .child("Thasia"),
                            )
                            .child(
                                div()
                                    .text_size(rems(0.7))
                                    .text_muted(cx)
                                    .child("MANGA PROCESSING ENGINE"),
                            ),
                    )
                    .child(
                        div()
                            .vertical()
                            .gap_dense()
                            .child({
                                let colors = cx.flavour().colors.clone();
                                let accent = colors.accent_primary;
                                div()
                                    .id("home-convert")
                                    .h(rems(4.75))
                                    .w_full()
                                    .min_w(rems(18.0))
                                    .px(rems(1.25))
                                    .horizontal()
                                    .align_center()
                                    .gap_standard()
                                    .rounded_xl()
                                    .border_1()
                                    .border_color(accent.opacity(0.42))
                                    .linear_gradient(
                                        GradientDirection::ToRight,
                                        accent.opacity(0.28),
                                        accent.opacity(0.0),
                                    )
                                    .cursor_pointer()
                                    .hover(move |style| {
                                        style.border_color(accent.opacity(0.72))
                                    })
                                    .on_click(move |_, _, cx| {
                                        convert.update(cx, |app, cx| {
                                            app.navigate(Page::Convert, cx)
                                        });
                                    })
                                    .child(
                                        div()
                                            .size(rems(2.75))
                                            .flex_shrink_0()
                                            .centered()
                                            .rounded_lg()
                                            .bg(accent.opacity(0.13))
                                            .child(
                                                Icon::new(IconName::Layers)
                                                    .size(rems(1.25))
                                                    .color(accent),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .vertical()
                                            .child(
                                                div()
                                                    .text_body()
                                                    .text_strong()
                                                    .child("Convert local manga"),
                                            )
                                            .child(
                                                div()
                                                    .text_size(rems(0.72))
                                                    .text_muted(cx)
                                                    .child(
                                                        "Folder, ZIP, or CBZ into reader-ready output",
                                                    ),
                                            ),
                                    )
                                    .child(
                                        Icon::new(IconName::ChevronRight)
                                            .size(rems(1.0))
                                            .color(accent),
                                    )
                            })
                            .child(
                                div()
                                    .horizontal()
                                    .gap_dense()
                                    .child(
                                        Button::new("Discover")
                                            .icon(IconName::Search)
                                            .variant(ButtonVariant::Secondary)
                                            .on_click(move |_, _, cx| {
                                                discover.update(cx, |app, cx| {
                                                    app.navigate(Page::Discover, cx)
                                                });
                                            }),
                                    )
                                    .child(
                                        Button::new("Settings")
                                            .icon(IconName::Settings)
                                            .variant(ButtonVariant::Secondary)
                                            .on_click(move |_, _, cx| {
                                                settings.update(cx, |app, cx| {
                                                    app.navigate(Page::Settings, cx)
                                                });
                                            }),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .text_size(rems(0.75))
                            .text_muted(cx)
                            .child("Or press  Cmd/Ctrl + 2  to jump straight in"),
                    ),
            )
            .into_any_element()
    }

    fn render_convert(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut content = div()
            .fill()
            .vertical()
            .gap_standard()
            .padding_standard()
            .child(
                div()
                    .horizontal()
                    .align_center()
                    .gap_dense()
                    .child(
                        div()
                            .text_heading()
                            .text_strong()
                            .text_primary(cx)
                            .child("Convert"),
                    )
                    .child(Badge::new(match self.convert_step {
                        ConvertStep::Source => "Source",
                        ConvertStep::Output => "Output",
                        ConvertStep::Volumes => "Volumes",
                        ConvertStep::Pages => "Pages",
                        ConvertStep::Review => "Review",
                        ConvertStep::Converting => "Converting",
                        ConvertStep::Complete => "Complete",
                    })),
            );
        if let Some(error) = &self.error {
            content = content.child(
                Alert::new(FeedbackVariant::Danger)
                    .title("Conversion")
                    .child(error.clone()),
            );
        }
        content
            .child(match self.convert_step {
                ConvertStep::Source => self.render_source_step(cx),
                ConvertStep::Output => self.render_options_step(cx),
                ConvertStep::Volumes => self.render_volume_step(cx),
                ConvertStep::Pages => self.render_pages_step(cx),
                ConvertStep::Review => self.render_review_step(cx),
                ConvertStep::Converting => self.render_progress_step(cx),
                ConvertStep::Complete => self.render_complete_step(cx),
            })
            .into_any_element()
    }

    fn render_source_step(&self, cx: &mut Context<Self>) -> AnyElement {
        let pick = cx.entity();
        let scan = cx.entity();
        div()
            .vertical()
            .gap_standard()
            .child("Select a folder, ZIP, or CBZ archive.")
            .when_some(self.source_path.clone(), |element, path| {
                element.child(PathDisplay::new(Some(path.to_string_lossy().to_string())))
            })
            .child(
                div()
                    .horizontal()
                    .gap_dense()
                    .child(Button::new("Browse").on_click(move |_, _, cx| {
                        pick.update(cx, |app, cx| app.pick_source(cx));
                    }))
                    .child(
                        Button::new(if self.busy {
                            "Scanning..."
                        } else {
                            "Scan source"
                        })
                        .variant(ButtonVariant::Secondary)
                        .disabled(self.busy || self.source_path.is_none())
                        .on_click(move |_, _, cx| {
                            scan.update(cx, |app, cx| app.scan_source(cx));
                        }),
                    ),
            )
            .into_any_element()
    }

    fn render_options_step(&self, cx: &mut Context<Self>) -> AnyElement {
        let pick = cx.entity();
        let avif = cx.entity();
        let webp = cx.entity();
        let original = cx.entity();
        let cbz = cx.entity();
        let epub = cx.entity();
        let raw = cx.entity();
        let next = cx.entity();
        let force = cx.entity();
        let clean = cx.entity();
        let crop = cx.entity();
        let moire = cx.entity();
        let eink = cx.entity();
        let split = cx.entity();
        let color = cx.entity();
        let sharpen = cx.entity();
        let direction = cx.entity();
        let bundle = cx.entity();
        let hide_single = cx.entity();
        div()
            .vertical()
            .gap_standard()
            .child(format!(
                "{} volume groups, {} pages",
                self.volumes.len(),
                self.volumes.iter().map(|v| v.pages.len()).sum::<usize>()
            ))
            .child(
                section_card("DESTINATION", cx)
                    .child(format!("Output name: {}", self.options.output_name))
                    .child(Button::new("Output folder").on_click(move |_, _, cx| {
                        pick.update(cx, |app, cx| app.pick_output(cx));
                    }))
                    .when(!self.options.output_dir.as_os_str().is_empty(), |element| {
                        element.child(PathDisplay::new(Some(
                            self.options.output_dir.to_string_lossy().to_string(),
                        )))
                    }),
            )
            .child(
                section_card("ENCODING", cx)
                    .child("Image format")
                    .child(
                        div()
                            .horizontal()
                            .gap_dense()
                            .child(format_button(
                                "AVIF",
                                self.options.image_format == ImageFormat::Avif,
                                avif,
                                |app| app.options.image_format = ImageFormat::Avif,
                            ))
                            .child(format_button(
                                "WebP",
                                self.options.image_format == ImageFormat::Webp,
                                webp,
                                |app| app.options.image_format = ImageFormat::Webp,
                            ))
                            .child(format_button(
                                "Original",
                                self.options.image_format == ImageFormat::Original,
                                original,
                                |app| app.options.image_format = ImageFormat::Original,
                            )),
                    )
                    .child(
                        div()
                            .horizontal()
                            .flex_wrap()
                            .gap_dense()
                            .child(toggle_button(
                                "Force re-encode",
                                self.options.force_reencode,
                                force,
                                |options| options.force_reencode = !options.force_reencode,
                            ))
                            .child(toggle_button(
                                "Clean tones",
                                self.options.clean_tones,
                                clean,
                                |options| options.clean_tones = !options.clean_tones,
                            ))
                            .child(toggle_button(
                                "Auto crop",
                                self.options.auto_crop,
                                crop,
                                |options| options.auto_crop = !options.auto_crop,
                            ))
                            .child(toggle_button(
                                "Moiré reduction",
                                self.options.moire_reduction,
                                moire,
                                |options| options.moire_reduction = !options.moire_reduction,
                            ))
                            .child(toggle_button(
                                "E-ink dither",
                                self.options.eink_dither,
                                eink,
                                |options| options.eink_dither = !options.eink_dither,
                            ))
                            .child(toggle_button(
                                "Split double pages",
                                self.options.split_double_page,
                                split,
                                |options| options.split_double_page = !options.split_double_page,
                            )),
                    )
                    .child(
                        div()
                            .horizontal()
                            .gap_dense()
                            .child(format_button(
                                match self.options.color_enhance {
                                    ColorEnhanceMode::Off => "Color: off",
                                    ColorEnhanceMode::Mild => "Color: mild",
                                    ColorEnhanceMode::Balanced => "Color: balanced",
                                    ColorEnhanceMode::Strong => "Color: strong",
                                },
                                self.options.color_enhance != ColorEnhanceMode::Off,
                                color,
                                |app| {
                                    app.options.color_enhance = match app.options.color_enhance {
                                        ColorEnhanceMode::Off => ColorEnhanceMode::Mild,
                                        ColorEnhanceMode::Mild => ColorEnhanceMode::Balanced,
                                        ColorEnhanceMode::Balanced => ColorEnhanceMode::Strong,
                                        ColorEnhanceMode::Strong => ColorEnhanceMode::Off,
                                    }
                                },
                            ))
                            .child(format_button(
                                match self.options.sharpen {
                                    SharpenMode::Off => "Sharpen: off",
                                    SharpenMode::Mild => "Sharpen: mild",
                                },
                                self.options.sharpen != SharpenMode::Off,
                                sharpen,
                                |app| {
                                    app.options.sharpen = match app.options.sharpen {
                                        SharpenMode::Off => SharpenMode::Mild,
                                        SharpenMode::Mild => SharpenMode::Off,
                                    }
                                },
                            )),
                    ),
            )
            .child(
                section_card("PACKAGING", cx)
                    .child(
                        div()
                            .horizontal()
                            .gap_dense()
                            .child(format_button(
                                "CBZ",
                                self.options.output_format == OutputFormat::Cbz,
                                cbz,
                                |app| app.options.output_format = OutputFormat::Cbz,
                            ))
                            .child(format_button(
                                "EPUB",
                                self.options.output_format == OutputFormat::Epub,
                                epub,
                                |app| app.options.output_format = OutputFormat::Epub,
                            ))
                            .child(format_button(
                                "Raw",
                                self.options.output_format == OutputFormat::Raw,
                                raw,
                                |app| app.options.output_format = OutputFormat::Raw,
                            ))
                            .child(format_button(
                                if self.options.direction == Direction::Ltr {
                                    "Left to right"
                                } else {
                                    "Right to left"
                                },
                                true,
                                direction,
                                |app| {
                                    app.options.direction =
                                        if app.options.direction == Direction::Ltr {
                                            Direction::Rtl
                                        } else {
                                            Direction::Ltr
                                        }
                                },
                            )),
                    )
                    .child(
                        div()
                            .horizontal()
                            .gap_dense()
                            .child(format_button(
                                if self.options.bundle == BundleMode::Auto {
                                    "Bundle: automatic"
                                } else {
                                    "Bundle: flatten"
                                },
                                true,
                                bundle,
                                |app| {
                                    app.options.bundle = if app.options.bundle == BundleMode::Auto {
                                        BundleMode::Flatten
                                    } else {
                                        BundleMode::Auto
                                    }
                                },
                            ))
                            .child(toggle_button(
                                "Hide single volume",
                                self.options.hide_single_volume,
                                hide_single,
                                |options| options.hide_single_volume = !options.hide_single_volume,
                            )),
                    ),
            )
            .child(
                Button::new("Continue to volumes")
                    .disabled(self.options.output_dir.as_os_str().is_empty())
                    .on_click(move |_, _, cx| {
                        next.update(cx, |app, cx| {
                            app.convert_step = ConvertStep::Volumes;
                            cx.notify();
                        });
                    }),
            )
            .into_any_element()
    }

    fn render_volume_step(&self, cx: &mut Context<Self>) -> AnyElement {
        let back = cx.entity();
        let next = cx.entity();
        let total_pages = self
            .volume_edits
            .iter()
            .map(|volume| volume.pages.len())
            .sum::<usize>();
        div()
            .vertical()
            .gap_standard()
            .child(
                div()
                    .horizontal()
                    .gap_standard()
                    .child(stat_card("Detected", total_pages.to_string(), cx))
                    .child(stat_card(
                        "Assigned",
                        self.volume_edits
                            .iter()
                            .flat_map(|volume| &volume.pages)
                            .filter(|page| !page.excluded)
                            .count()
                            .to_string(),
                        cx,
                    ))
                    .child(stat_card(
                        "Volumes",
                        self.volume_edits.len().to_string(),
                        cx,
                    )),
            )
            .child(
                section_card("VOLUME ASSIGNMENT", cx).child(div().vertical().gap_dense().children(
                    self.volume_edits.iter().map(|volume| {
                        div()
                            .horizontal()
                            .justify_between()
                            .align_center()
                            .py(rems(0.55))
                            .border_b_1()
                            .child(
                                div()
                                    .text_body()
                                    .text_strong()
                                    .child(format!("Volume {}", volume.volume_num)),
                            )
                            .child(
                                div()
                                    .text_muted(cx)
                                    .child(format!("{} pages", volume.pages.len())),
                            )
                    }),
                )),
            )
            .child(
                div()
                    .horizontal()
                    .justify_between()
                    .child(Button::new("Back").on_click(move |_, _, cx| {
                        back.update(cx, |app, cx| {
                            app.convert_step = ConvertStep::Output;
                            cx.notify();
                        });
                    }))
                    .child(Button::new("Continue to pages").on_click(move |_, _, cx| {
                        next.update(cx, |app, cx| {
                            app.convert_step = ConvertStep::Pages;
                            cx.notify();
                        });
                    })),
            )
            .into_any_element()
    }

    fn render_pages_step(&self, cx: &mut Context<Self>) -> AnyElement {
        let add = cx.entity();
        let back = cx.entity();
        let review = cx.entity();
        let selected = self
            .volume_edits
            .get(self.active_volume)
            .map(|volume| volume.volume_num)
            .unwrap_or(1);
        let mut page = div().fill().horizontal().gap_standard();
        page = page.child(section_card("VOLUMES", cx).w(rems(11.0)).children(
            self.volume_edits.iter().enumerate().map(|(index, volume)| {
                let entity = cx.entity();
                Button::new(format!(
                    "Volume {}  ({})",
                    volume.volume_num,
                    volume.pages.iter().filter(|page| !page.excluded).count()
                ))
                .variant(if index == self.active_volume {
                    ButtonVariant::Secondary
                } else {
                    ButtonVariant::Ghost
                })
                .on_click(move |_, _, cx| {
                    entity.update(cx, |app, cx| {
                        app.active_volume = index;
                        cx.notify();
                    });
                })
            }),
        ));

        let mut editor = section_card(&format!("VOLUME {selected} PAGES"), cx)
            .flex_1()
            .child(
                div()
                    .horizontal()
                    .justify_between()
                    .align_center()
                    .child(
                        div()
                            .text_size(rems(0.75))
                            .text_muted(cx)
                            .child("Click a page to include or exclude it. The first included page is the cover."),
                    )
                    .child(Button::new("Add image").icon(IconName::Folder).on_click(
                        move |_, _, cx| {
                            add.update(cx, |app, cx| app.add_custom_page(cx));
                        },
                    )),
            );
        if let Some(volume) = self.volume_edits.get(self.active_volume) {
            editor = editor.child(div().horizontal().flex_wrap().gap_dense().children(
                volume.pages.iter().enumerate().map(|(index, entry)| {
                    let entity = cx.entity();
                    let path = self.page_path(entry);
                    let excluded = entry.excluded;
                    div()
                        .w(rems(7.2))
                        .vertical()
                        .gap_compact()
                        .child(
                            Button::new("")
                                .variant(if excluded {
                                    ButtonVariant::Ghost
                                } else {
                                    ButtonVariant::Secondary
                                })
                                .on_click(move |_, _, cx| {
                                    entity.update(cx, |app, cx| {
                                        if let Some(page) = app
                                            .volume_edits
                                            .get_mut(app.active_volume)
                                            .and_then(|volume| volume.pages.get_mut(index))
                                        {
                                            page.excluded = !page.excluded;
                                        }
                                        cx.notify();
                                    });
                                }),
                        )
                        .when_some(path, |card, path| {
                            card.child(
                                img(path)
                                    .w_full()
                                    .h(rems(9.2))
                                    .rounded_lg()
                                    .object_fit(ObjectFit::Cover)
                                    .opacity(if excluded { 0.35 } else { 1.0 }),
                            )
                        })
                        .child(
                                    div()
                                        .horizontal()
                                        .justify_between()
                                        .text_size(rems(0.7))
                                        .text_muted(cx)
                                        .child(
                                            self.page_file_name(entry)
                                                .unwrap_or_else(|| format!("{:03}", index + 1)),
                                        )
                                        .child(if excluded { "Excluded" } else { "Included" }),
                        )
                }),
            ));
        }
        page.child(
            editor.child(
                div()
                    .horizontal()
                    .justify_between()
                    .child(Button::new("Back").on_click(move |_, _, cx| {
                        back.update(cx, |app, cx| {
                            app.convert_step = ConvertStep::Volumes;
                            cx.notify();
                        });
                    }))
                    .child(Button::new("Review").on_click(move |_, _, cx| {
                        review.update(cx, |app, cx| app.prepare_review(cx));
                    })),
            ),
        )
        .into_any_element()
    }

    fn page_path(&self, entry: &PageEditEntry) -> Option<PathBuf> {
        match &entry.source {
            PageEditSource::Custom { path } => Some(path.clone()),
            PageEditSource::Original {
                page_index,
                source_volume_num,
            } => self
                .volumes
                .iter()
                .find(|volume| {
                    source_volume_num
                        .map(|number| volume.source_volume_num == number)
                        .unwrap_or(true)
                })
                .and_then(|volume| {
                    volume
                        .pages
                        .iter()
                        .find(|page| page.page_index == *page_index)
                })
                .map(|page| page.path.clone()),
        }
    }

    fn page_file_name(&self, entry: &PageEditEntry) -> Option<String> {
        match &entry.source {
            PageEditSource::Custom { path } => path
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned),
            PageEditSource::Original {
                page_index,
                source_volume_num,
            } => self
                .volumes
                .iter()
                .find(|volume| {
                    source_volume_num
                        .map(|number| volume.source_volume_num == number)
                        .unwrap_or(true)
                })
                .and_then(|volume| {
                    volume
                        .pages
                        .iter()
                        .find(|page| page.page_index == *page_index)
                })
                .map(|page| page.file_name.clone()),
        }
    }

    fn add_custom_page(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let image = rfd::AsyncFileDialog::new()
                .add_filter("Images", &["jpg", "jpeg", "png", "webp", "avif"])
                .pick_file()
                .await;
            if let Some(image) = image {
                let _ = this.update(cx, |app, cx| {
                    if let Some(volume) = app.volume_edits.get_mut(app.active_volume) {
                        volume.pages.push(PageEditEntry {
                            source: PageEditSource::Custom {
                                path: image.path().to_path_buf(),
                            },
                            excluded: false,
                        });
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }

    fn render_review_step(&self, cx: &mut Context<Self>) -> AnyElement {
        let back = cx.entity();
        let start = cx.entity();
        let plan = self.pipeline_plan.as_ref();
        let mut review = div()
            .vertical()
            .gap_standard()
            .child(section_card("CONVERSION SUMMARY", cx).child(
                div()
                    .horizontal()
                    .gap_standard()
                    .child(stat_card(
                        "Detected",
                        plan.map(|p| p.total_pages).unwrap_or_default().to_string(),
                        cx,
                    ))
                    .child(stat_card(
                        "Included",
                        plan.map(|p| p.included_pages)
                            .unwrap_or_default()
                            .to_string(),
                        cx,
                    ))
                    .child(stat_card(
                        "Excluded",
                        plan.map(|p| p.excluded_pages)
                            .unwrap_or_default()
                            .to_string(),
                        cx,
                    ))
                    .child(stat_card(
                        "Added",
                        plan.map(|p| p.added_pages).unwrap_or_default().to_string(),
                        cx,
                    )),
            ));
        if let Some(plan) = plan {
            review = review.child(
                section_card("PIPELINE", cx)
                    .child(format!(
                        "{:?} images · {:?} container · {} outputs",
                        plan.image_format, plan.output_format, plan.volumes
                    ))
                    .children(plan.stages.iter().map(|stage| {
                        section_card(
                            format!(
                                "{} · {}",
                                stage.label,
                                if stage.enabled { "enabled" } else { "disabled" }
                            ),
                            cx,
                        )
                        .child(
                            div()
                                .vertical()
                                .gap_compact()
                                .children(stage.steps.iter().map(|step| {
                                    let effects = [
                                        (step.effects.dimensions, "dimensions"),
                                        (step.effects.pixels, "pixels"),
                                        (step.effects.alpha, "alpha"),
                                        (step.effects.metadata, "metadata"),
                                        (step.effects.passthrough, "passthrough"),
                                    ]
                                    .into_iter()
                                    .filter_map(|(enabled, label)| enabled.then_some(label))
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                    div()
                                        .horizontal()
                                        .justify_between()
                                        .child(format!(
                                            "{} [{} / {}]",
                                            step.label, step.id, step.category
                                        ))
                                        .child(format!(
                                            "{} · {:?}{}{}{}{}{}",
                                            if step.enabled { "on" } else { "off" },
                                            step.cost,
                                            if step.default_enabled { " · default" } else { "" },
                                            if effects.is_empty() { "" } else { " · " },
                                            effects,
                                            step.exclusive_group
                                                .as_ref()
                                                .map(|group| format!(" · group {group}"))
                                                .unwrap_or_default(),
                                            if step.conflicts.is_empty() {
                                                String::new()
                                            } else {
                                                format!(
                                                    " · conflicts {}",
                                                    step.conflicts.join(", ")
                                                )
                                            }
                                        ))
                                })),
                        )
                        .when(!stage.id.is_empty(), |card| {
                            card.child(
                                div()
                                    .text_size(rems(0.65))
                                    .text_muted(cx)
                                    .child(format!("Stage id: {}", stage.id)),
                            )
                        })
                    })),
            );
        }
        review
            .child(
                div()
                    .horizontal()
                    .gap_dense()
                    .child(Button::new("Back").on_click(move |_, _, cx| {
                        back.update(cx, |app, cx| {
                            app.convert_step = ConvertStep::Pages;
                            cx.notify();
                        });
                    }))
                    .child(Button::new("Start conversion").on_click(move |_, _, cx| {
                        start.update(cx, |app, cx| app.start_conversion(cx));
                    })),
            )
            .into_any_element()
    }

    fn render_progress_step(&self, cx: &mut Context<Self>) -> AnyElement {
        let cancel = cx.entity();
        let value = self.progress.current as f32 / self.progress.total.max(1) as f32;
        div()
            .vertical()
            .gap_standard()
            .child(format!(
                "Volume {}/{}: {}",
                self.progress.volume_num, self.progress.total_volumes, self.progress.volume_name
            ))
            .child(ProgressBar::new(value).label(format!(
                "{}/{} pages, {:.1} pages/s",
                self.progress.current, self.progress.total, self.progress.pages_per_sec
            )))
            .child(
                div()
                    .horizontal()
                    .gap_standard()
                    .child(stat_card(
                        "Elapsed",
                        format_duration(self.progress.elapsed_secs),
                        cx,
                    ))
                    .child(stat_card(
                        "Remaining",
                        self.progress
                            .estimated_remaining_secs
                            .map(format_duration)
                            .unwrap_or_else(|| "Calculating".into()),
                        cx,
                    ))
                    .child(stat_card(
                        "Data",
                        format!(
                            "{} → {}",
                            format_bytes(self.progress.input_bytes),
                            format_bytes(self.progress.output_bytes)
                        ),
                        cx,
                    ))
                    .child(stat_card(
                        "Encoding",
                        format!(
                            "{} encoded / {} passthrough",
                            self.progress.encoded_pages, self.progress.passthrough_pages
                        ),
                        cx,
                    )),
            )
            .child(
                section_card("PIPELINE TIMING", cx).child(format!(
                    "Fetch {:.0} ms · Decode {:.0} ms · Transform {:.0} ms · Encode {:.0} ms",
                    self.progress.fetch_ms,
                    self.progress.decode_ms,
                    self.progress.transform_ms,
                    self.progress.encode_ms
                )),
            )
            .children(
                self.progress
                    .completed_outputs
                    .iter()
                    .cloned()
                    .map(|path| PathDisplay::new(Some(path))),
            )
            .child(
                Button::new("Cancel")
                    .variant(ButtonVariant::Secondary)
                    .on_click(move |_, _, cx| {
                        cancel.update(cx, |app, cx| app.cancel_conversion(cx));
                    }),
            )
            .into_any_element()
    }

    fn render_complete_step(&self, cx: &mut Context<Self>) -> AnyElement {
        let reset = cx.entity();
        let summary = self.summary.as_ref();
        let mut element = div().vertical().gap_standard().child(format!(
            "{} succeeded, {} failed in {:.1}s",
            summary.map(|s| s.successful).unwrap_or_default(),
            summary.map(|s| s.failed).unwrap_or_default(),
            summary.map(|s| s.duration_secs).unwrap_or_default()
        ));
        if let Some(summary) = summary {
            element = element
                .child(
                    div()
                        .horizontal()
                        .gap_standard()
                        .child(stat_card("Pages", summary.total_pages.to_string(), cx))
                        .child(stat_card(
                            "Data",
                            format!(
                                "{} → {}",
                                format_bytes(summary.input_bytes),
                                format_bytes(summary.output_bytes)
                            ),
                            cx,
                        ))
                        .child(stat_card(
                            "Encoded",
                            summary.encoded_pages.to_string(),
                            cx,
                        ))
                        .child(stat_card(
                            "Passthrough",
                            summary.passthrough_pages.to_string(),
                            cx,
                        )),
                )
                .child(section_card("PIPELINE TIMING", cx).child(format!(
                    "Fetch {:.0} ms · Decode {:.0} ms · Transform {:.0} ms · Encode {:.0} ms",
                    summary.fetch_ms, summary.decode_ms, summary.transform_ms, summary.encode_ms
                )));
            element = element.children(
                summary.outputs.iter().map(|output| {
                    section_card(
                        format!("VOLUME {} · {}", output.volume_num, output.volume_name),
                        cx,
                    )
                    .child(PathDisplay::new(Some(output.path.clone())))
                }),
            );
        }
        element
            .child(Button::new("Convert another").on_click(move |_, _, cx| {
                reset.update(cx, |app, cx| app.reset_conversion(cx));
            }))
            .into_any_element()
    }

    fn render_discover(&self, cx: &mut Context<Self>) -> AnyElement {
        let refresh = cx.entity();
        let install = cx.entity();
        let start = cx.entity();
        let stop = cx.entity();
        let restart = cx.entity();
        let uninstall = cx.entity();
        let search = cx.entity();
        let download = cx.entity();
        let catalog = cx.entity();
        let server = cx.entity();
        let mut content = div()
            .fill()
            .vertical()
            .child(
                div()
                    .px(rems(1.5))
                    .py(rems(1.0))
                    .horizontal()
                    .align_center()
                    .justify_between()
                    .border_b_1()
                    .child(
                        div()
                            .vertical()
                            .gap_compact()
                            .child(
                                div()
                                    .horizontal()
                                    .align_center()
                                    .gap_dense()
                                    .child(
                                        div()
                                            .text_heading()
                                            .text_strong()
                                            .text_primary(cx)
                                            .child("Discover"),
                                    )
                                    .child(Badge::new(runtime_label(&self.runtime_state)).variant(
                                        if matches!(self.runtime_state, RuntimeState::Ready { .. })
                                        {
                                            FeedbackVariant::Success
                                        } else {
                                            FeedbackVariant::Default
                                        },
                                    )),
                            )
                            .child(
                                div().text_size(rems(0.75)).text_muted(cx).child(
                                    "Search catalogs, manage Suwayomi, and download chapters.",
                                ),
                            ),
                    )
                    .child(
                        div()
                            .horizontal()
                            .gap_compact()
                            .child(
                                Button::new("Catalog")
                                    .variant(if self.discover_tab == DiscoverTab::Catalog {
                                        ButtonVariant::Secondary
                                    } else {
                                        ButtonVariant::Ghost
                                    })
                                    .on_click(move |_, _, cx| {
                                        catalog.update(cx, |app, cx| {
                                            app.discover_tab = DiscoverTab::Catalog;
                                            cx.notify();
                                        });
                                    }),
                            )
                            .child(
                                Button::new("Suwayomi")
                                    .variant(if self.discover_tab == DiscoverTab::Suwayomi {
                                        ButtonVariant::Secondary
                                    } else {
                                        ButtonVariant::Ghost
                                    })
                                    .on_click(move |_, _, cx| {
                                        server.update(cx, |app, cx| {
                                            app.discover_tab = DiscoverTab::Suwayomi;
                                            cx.notify();
                                        });
                                    }),
                            ),
                    ),
            )
            .child(
                div()
                    .p(rems(1.5))
                    .horizontal()
                    .gap_dense()
                    .child(Button::new("Refresh").on_click(move |_, _, cx| {
                        refresh.update(cx, |app, cx| app.refresh_discovery(cx));
                    }))
                    .when(
                        matches!(self.runtime_state, RuntimeState::NotInstalled),
                        |element| {
                            element.child(
                                Button::new("Install")
                                    .disabled(self.discovery_busy)
                                    .on_click(move |_, _, cx| {
                                        install.update(cx, |app, cx| app.install_suwayomi(cx));
                                    }),
                            )
                        },
                    )
                    .when(
                        matches!(self.runtime_state, RuntimeState::NotRunning),
                        |element| {
                            element.child(
                                Button::new("Start").disabled(self.discovery_busy).on_click(
                                    move |_, _, cx| {
                                        start.update(cx, |app, cx| app.start_suwayomi(cx));
                                    },
                                ),
                            )
                        },
                    )
                    .when(
                        matches!(self.runtime_state, RuntimeState::Ready { .. }),
                        |element| {
                            element
                                .child(
                                    Button::new("Stop")
                                        .variant(ButtonVariant::Secondary)
                                        .disabled(self.discovery_busy)
                                        .on_click(move |_, _, cx| {
                                            stop.update(cx, |app, cx| app.stop_suwayomi(cx));
                                        }),
                                )
                                .child(
                                    Button::new("Restart")
                                        .variant(ButtonVariant::Secondary)
                                        .disabled(self.discovery_busy)
                                        .on_click(move |_, _, cx| {
                                            restart.update(cx, |app, cx| {
                                                app.restart_suwayomi(cx)
                                            });
                                        }),
                                )
                        },
                    ),
            );
        if let Some(message) = &self.discovery_message {
            content = content.child(
                Alert::new(FeedbackVariant::Accent)
                    .title("Suwayomi")
                    .child(message.clone()),
            );
        }
        if self.discover_tab == DiscoverTab::Catalog
            && matches!(self.runtime_state, RuntimeState::Ready { .. })
        {
            content = content
                .child(
                    section_card("SOURCES", cx).child(
                        div()
                            .horizontal()
                            .flex_wrap()
                            .gap_compact()
                            .children(self.sources.iter().map(|source| {
                                let entity = cx.entity();
                                let id = source.id.clone();
                                Button::new(match &source.lang {
                                    Some(lang) => format!("{} · {}", source.name, lang),
                                    None => source.name.clone(),
                                })
                                .variant(
                                    if self.selected_source.as_ref() == Some(&source.id) {
                                        ButtonVariant::Secondary
                                    } else {
                                        ButtonVariant::Ghost
                                    },
                                )
                                .on_click(move |_, _, cx| {
                                    entity.update(cx, |app, cx| {
                                        app.selected_source = Some(id.clone());
                                        cx.notify();
                                    });
                                })
                            })),
                    ),
                )
                .child(
                    div()
                        .horizontal()
                        .gap_dense()
                        .child(self.search_input.clone())
                        .child(
                            Button::new(if self.discovery_busy {
                                "Working..."
                            } else {
                                "Search"
                            })
                            .disabled(self.discovery_busy || self.selected_source.is_none())
                            .on_click(move |_, _, cx| {
                                search.update(cx, |app, cx| app.search_discovery(cx));
                            }),
                        ),
                )
                .children(self.search_results.iter().cloned().map(|series| {
                    let entity = cx.entity();
                    let label = series.title.clone();
                    Button::new(label)
                        .variant(ButtonVariant::Ghost)
                        .on_click(move |_, _, cx| {
                            entity.update(cx, |app, cx| {
                                app.select_series(series.clone(), cx);
                            });
                        })
                }));
        }
        if self.discover_tab == DiscoverTab::Catalog {
            if let Some(series) = &self.selected_series {
                content = content
                    .child(format!(
                        "{}: {} chapters",
                        series.title,
                        self.chapters.len()
                    ))
                    .child(section_card("CHAPTERS", cx).children(
                        self.chapters.iter().map(|chapter| {
                            let entity = cx.entity();
                            let chapter_id = chapter.id;
                            let selected = self.selected_chapters.contains(&chapter.id);
                            Button::new(format!(
                                "{}{}",
                                if selected { "✓ " } else { "  " },
                                chapter.name
                            ))
                            .variant(if selected {
                                ButtonVariant::Secondary
                            } else {
                                ButtonVariant::Ghost
                            })
                            .on_click(move |_, _, cx| {
                                entity.update(cx, |app, cx| {
                                    if !app.selected_chapters.remove(&chapter_id) {
                                        app.selected_chapters.insert(chapter_id);
                                    }
                                    cx.notify();
                                });
                            })
                        }),
                    ))
                    .when(self.download_total > 0, |element| {
                        element.child(
                            ProgressBar::new(
                                self.download_current as f32 / self.download_total.max(1) as f32,
                            )
                            .label(format!(
                                "{}/{} · {}",
                                self.download_current, self.download_total, self.download_label
                            )),
                        )
                    })
                    .child(
                        Button::new(format!(
                            "Download {} selected chapters",
                            self.selected_chapters.len()
                        ))
                        .disabled(self.discovery_busy || self.selected_chapters.is_empty())
                        .on_click(move |_, _, cx| {
                            download.update(cx, |app, cx| app.download_series(cx));
                        }),
                    );
            }
        } else {
            let installed = self.installed_suwayomi.as_ref();
            content = content.child(
                div()
                    .px(rems(1.5))
                    .vertical()
                    .gap_standard()
                    .child(
                        section_card("INSTALLATION", cx)
                            .child(
                                div()
                                    .text_body()
                                    .text_strong()
                                    .child(installed.map_or_else(
                                        || "Not installed".to_string(),
                                        |info| format!("Installed {}", info.version),
                                    )),
                            )
                            .child(
                                div()
                                    .text_size(rems(0.75))
                                    .text_muted(cx)
                                    .child(installed.map_or_else(
                                        || "Install Suwayomi-Server to enable discovery.".into(),
                                        |info| {
                                            format!(
                                                "{} · Runtime {}",
                                                format_bytes(info.size),
                                                runtime_label(&self.runtime_state)
                                            )
                                        },
                                    )),
                            )
                            .when(installed.is_some(), |card| {
                                card.child(
                                    Button::new("Delete Suwayomi")
                                        .variant(ButtonVariant::Danger)
                                        .disabled(self.discovery_busy)
                                        .on_click(move |_, _, cx| {
                                            uninstall.update(cx, |app, cx| {
                                                app.uninstall_suwayomi(cx)
                                            });
                                        }),
                                )
                            }),
                    )
                    .child(
                        section_card("EXTENSIONS", cx).children(
                            self.extensions.iter().map(|extension| {
                                let entity = cx.entity();
                                let package = extension.pkg_name.clone();
                                let installed = extension.installed;
                                div()
                                    .horizontal()
                                    .justify_between()
                                    .align_center()
                                    .py(rems(0.45))
                                    .border_b_1()
                                    .child(format!(
                                        "{}{}{}",
                                        extension.name,
                                        extension
                                            .lang
                                            .as_ref()
                                            .map(|lang| format!(" · {lang}"))
                                            .unwrap_or_default(),
                                        extension
                                            .version_name
                                            .as_ref()
                                            .map(|version| format!(" · {version}"))
                                            .unwrap_or_default()
                                    ))
                                    .child(
                                        Button::new(if installed { "Uninstall" } else { "Install" })
                                            .variant(if installed {
                                                ButtonVariant::Danger
                                            } else {
                                                ButtonVariant::Secondary
                                            })
                                            .disabled(self.discovery_busy)
                                            .on_click(move |_, _, cx| {
                                                entity.update(cx, |app, cx| {
                                                    app.set_extension_installed(
                                                        package.clone(),
                                                        installed,
                                                        cx,
                                                    )
                                                });
                                            }),
                                    )
                            }),
                        ),
                    ),
            );
        }
        content.into_any_element()
    }

    fn render_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        let logs = self.paths.logs.clone();
        let data = self.paths.data_dir.clone();
        let config = self.paths.config_dir.clone();
        let cache = self.paths.cache_dir.clone();
        let light = cx.entity();
        let dark = cx.entity();
        let system = cx.entity();
        let hints = cx.entity();
        let check_update = cx.entity();
        let install_update = cx.entity();
        div()
            .fill()
            .vertical()
            .child(
                div()
                    .px(rems(1.5))
                    .py(rems(1.0))
                    .horizontal()
                    .justify_between()
                    .align_center()
                    .border_b_1()
                    .child(
                        div()
                            .vertical()
                            .gap_compact()
                            .child(
                                div()
                                    .text_heading()
                                    .text_strong()
                                    .text_primary(cx)
                                    .child("Settings"),
                            )
                            .child(
                                div()
                                    .text_size(rems(0.75))
                                    .text_muted(cx)
                                    .child("Configure defaults for conversion and the interface."),
                            ),
                    )
                    .child(Badge::new("Saved").variant(FeedbackVariant::Success)),
            )
            .child(
                div()
                    .flex_1()
                    .vertical()
                    .gap_standard()
                    .p(rems(1.5))
                    .child(
                        div()
                            .horizontal()
                            .gap_standard()
                            .child(
                                section_card("DESTINATION", cx)
                                    .flex_1()
                                    .child(
                                        div()
                                            .text_body()
                                            .child("Default conversion folder"),
                                    )
                                    .child(
                                        div()
                                            .text_size(rems(0.75))
                                            .text_muted(cx)
                                            .child("Choose a destination during each conversion."),
                                    )
                                    .child(Button::new("Open data folder").on_click(
                                        move |_, _, _| {
                                            let _ = open::that(&data);
                                        },
                                    )),
                            )
                            .child(
                                section_card("INTERFACE", cx)
                                    .flex_1()
                                    .child(
                                        div()
                                            .text_body()
                                            .text_strong()
                                            .child("Appearance"),
                                    )
                                    .child(
                                        div()
                                            .horizontal()
                                            .gap_dense()
                                            .child(Button::new("Light").on_click(
                                                move |_, _, cx| {
                                                    light.update(cx, |app, cx| {
                                                        app.set_theme_preference(
                                                            ThemePreference::Light,
                                                            cx,
                                                        );
                                                    });
                                                },
                                            ))
                                            .child(Button::new("Dark").on_click(
                                                move |_, _, cx| {
                                                    dark.update(cx, |app, cx| {
                                                        app.set_theme_preference(
                                                            ThemePreference::Dark,
                                                            cx,
                                                        );
                                                    });
                                                },
                                            ))
                                            .child(Button::new("System").on_click(
                                                move |_, _, cx| {
                                                    system.update(cx, |app, cx| {
                                                        app.set_theme_preference(
                                                            ThemePreference::System,
                                                            cx,
                                                        );
                                                    });
                                                },
                                            )),
                                    )
                                    .child(
                                        Button::new(if self.app_settings.show_key_hints {
                                            "Key hints: on"
                                        } else {
                                            "Key hints: off"
                                        })
                                        .variant(ButtonVariant::Secondary)
                                        .on_click(move |_, _, cx| {
                                            hints.update(cx, |app, cx| {
                                                app.app_settings.show_key_hints =
                                                    !app.app_settings.show_key_hints;
                                                let _ = settings::save_app(
                                                    &app.paths,
                                                    &app.app_settings,
                                                );
                                                cx.notify();
                                            });
                                        }),
                                    )
                                    .child(Button::new("Open logs folder").on_click(
                                        move |_, _, _| {
                                            let _ = open::that(&logs);
                                        },
                                    )),
                            ),
                    )
                    .child(
                        section_card("OUTPUT DEFAULTS", cx)
                            .child(
                                div()
                                    .text_body()
                                    .text_strong()
                                    .child("Image encoding and packaging"),
                            )
                            .child(
                                div()
                                    .text_size(rems(0.75))
                                    .text_muted(cx)
                                    .child("New conversions begin with AVIF images in a CBZ container. Every option remains editable in the wizard."),
                            ),
                    )
                    .child(
                        section_card("APPLICATION DATA", cx)
                            .child(Button::new("Open config folder").on_click(move |_, _, _| {
                                let _ = open::that(&config);
                            }))
                            .child(Button::new("Open cache folder").on_click(move |_, _, _| {
                                let _ = open::that(&cache);
                            })),
                    )
                    .child(
                        section_card("UPDATES", cx)
                            .child(
                                div()
                                    .text_size(rems(0.75))
                                    .text_muted(cx)
                                    .child(
                                        self.update_message
                                            .clone()
                                            .unwrap_or_else(|| "Check GitHub Releases for a newer version.".into()),
                                    ),
                            )
                            .child(
                                div()
                                    .horizontal()
                                    .gap_dense()
                                    .child(
                                        Button::new(if self.update_busy {
                                            "Working..."
                                        } else {
                                            "Check for updates"
                                        })
                                        .disabled(self.update_busy)
                                        .on_click(move |_, _, cx| {
                                            check_update.update(cx, |app, cx| {
                                                app.check_for_updates(cx)
                                            });
                                        }),
                                    )
                                    .when(self.update_available.is_some(), |row| {
                                        row.child(
                                            Button::new("Update now")
                                                .disabled(self.update_busy)
                                                .on_click(move |_, _, cx| {
                                                    install_update.update(cx, |app, cx| {
                                                        app.install_update(cx)
                                                    });
                                                }),
                                        )
                                    }),
                            ),
                    ),
            )
            .into_any_element()
    }
}

fn runtime_label(runtime: &RuntimeState) -> String {
    match runtime {
        RuntimeState::NotInstalled => "Setup needed".into(),
        RuntimeState::NotRunning => "Stopped".into(),
        RuntimeState::Starting => "Starting".into(),
        RuntimeState::Ready { port } => format!("Ready:{port}"),
        RuntimeState::Error { message } => format!("Error: {message}"),
    }
}

fn convert_step_index(step: ConvertStep) -> usize {
    match step {
        ConvertStep::Source => 0,
        ConvertStep::Output => 1,
        ConvertStep::Volumes => 2,
        ConvertStep::Pages => 3,
        ConvertStep::Review => 4,
        ConvertStep::Converting | ConvertStep::Complete => 5,
    }
}

fn section_card(label: impl Into<SharedString>, cx: &Context<ThasiaApp>) -> Div {
    div()
        .vertical()
        .gap_standard()
        .p(rems(1.0))
        .rounded_xl()
        .border_1()
        .elevation_surface(cx)
        .child(
            div()
                .text_size(rems(0.65))
                .text_strong()
                .text_muted(cx)
                .child(label.into()),
        )
}

fn stat_card(label: &'static str, value: String, cx: &Context<ThasiaApp>) -> Div {
    div()
        .flex_1()
        .vertical()
        .gap_compact()
        .p(rems(0.9))
        .rounded_lg()
        .border_1()
        .elevation_surface(cx)
        .child(div().text_size(rems(0.65)).text_muted(cx).child(label))
        .child(
            div()
                .text_heading()
                .text_strong()
                .text_primary(cx)
                .child(value),
        )
}

fn compact_text_control(
    label: &'static str,
    id: &'static str,
    entity: Entity<ThasiaApp>,
    update: impl Fn(&mut ThasiaApp, &mut Context<ThasiaApp>) + 'static,
) -> AnyElement {
    div()
        .id(id)
        .size(rems(2.0))
        .flex_shrink_0()
        .centered()
        .rounded_lg()
        .text_size(rems(0.7))
        .text_strong()
        .cursor_pointer()
        .on_click(move |_, _, cx| {
            entity.update(cx, |app, cx| {
                update(app, cx);
                cx.notify();
            });
        })
        .child(label)
        .into_any_element()
}

fn format_duration(seconds: f64) -> String {
    if seconds < 60.0 {
        format!("{seconds:.1}s")
    } else {
        format!("{}m {:02}s", (seconds / 60.0) as u64, seconds as u64 % 60)
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let bytes = bytes as f64;
    if bytes >= GB {
        format!("{:.1} GB", bytes / GB)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes / KB)
    } else {
        format!("{bytes:.0} B")
    }
}

fn format_button(
    label: &'static str,
    selected: bool,
    entity: Entity<ThasiaApp>,
    update: impl Fn(&mut ThasiaApp) + 'static,
) -> Button {
    Button::new(label)
        .variant(if selected {
            ButtonVariant::Secondary
        } else {
            ButtonVariant::Ghost
        })
        .on_click(move |_, _, cx| {
            entity.update(cx, |app, cx| {
                update(app);
                cx.notify();
            });
        })
}

fn toggle_button(
    label: &'static str,
    selected: bool,
    entity: Entity<ThasiaApp>,
    update: impl Fn(&mut ConvertOptions) + 'static,
) -> Button {
    Button::new(label)
        .variant(if selected {
            ButtonVariant::Secondary
        } else {
            ButtonVariant::Ghost
        })
        .on_click(move |_, _, cx| {
            entity.update(cx, |app, cx| {
                update(&mut app.options);
                cx.notify();
            });
        })
}

impl Render for ThasiaApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity();
        let home = this.clone();
        let convert = this.clone();
        let discover = this.clone();
        let settings = this.clone();
        let sidebar = this.clone();
        let content = match self.page {
            Page::Home => self.home.clone().into_any_element(),
            Page::Convert => self.render_convert(cx),
            Page::Discover => self.discover_view.clone().into_any_element(),
            Page::Settings => self.settings_view.clone().into_any_element(),
        };

        let locked = self.convert_step == ConvertStep::Converting;
        div()
            .fill()
            .vertical()
            .elevation_background(cx)
            .text_primary(cx)
            .on_action(move |_: &NavigateHome, _, cx| {
                home.update(cx, |app, cx| app.navigate(Page::Home, cx));
            })
            .on_action(move |_: &NavigateConvert, _, cx| {
                convert.update(cx, |app, cx| app.navigate(Page::Convert, cx));
            })
            .on_action(move |_: &NavigateDiscover, _, cx| {
                discover.update(cx, |app, cx| app.navigate(Page::Discover, cx));
            })
            .on_action(move |_: &NavigateSettings, _, cx| {
                settings.update(cx, |app, cx| app.navigate(Page::Settings, cx));
            })
            .on_action(move |_: &ToggleSidebar, _, cx| {
                sidebar.update(cx, |app, cx| {
                    app.sidebar_open = !app.sidebar_open;
                    cx.notify();
                });
            })
            .child(div().h(rems(2.0)).w_full().flex_shrink_0().border_b_1())
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .horizontal()
                    .child(self.render_sidebar(cx))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .vertical()
                            .child(div().flex_1().min_h_0().overflow_hidden().child(content))
                            .when(self.app_settings.show_key_hints, |bar| bar.child(
                                div()
                                    .h(rems(1.75))
                                    .px(rems(0.9))
                                    .horizontal()
                                    .align_center()
                                    .justify_between()
                                    .border_t_1()
                                    .text_size(rems(0.65))
                                    .text_muted(cx)
                                    .child(if locked {
                                        "Navigation locked during conversion"
                                    } else {
                                        "Cmd/Ctrl 1-4 navigate"
                                    })
                                    .child("Cmd/Ctrl B sidebar"),
                            )),
                    ),
            )
    }
}
