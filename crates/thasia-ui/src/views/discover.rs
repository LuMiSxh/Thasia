#![allow(unused_braces)]

use crate::{
    components::PageHeader,
    error::AppResult,
    services::discovery,
    state::SharedDiscoveryState,
    util::runtime,
};
use gpui::{Context, Entity, EventEmitter, IntoElement, ParentElement, Render, Window, prelude::*};
use nasrin::{
    Alert, Badge, Button, ButtonVariant, FeedbackVariant, Input, Panel, ProgressBar,
    SemanticStyleExt, view,
};
use std::{collections::HashSet, future::Future, pin::Pin, sync::Arc};
use thasia_source::suwayomi::{
    ChapterMeta, ExtensionInfo, InstalledInfo, RuntimeState, SearchResult, SourceInfo,
};

#[derive(Clone, Debug)]
pub enum DiscoverEvent {
    RuntimeChanged(RuntimeState),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum DiscoverTab {
    #[default]
    Catalog,
    Suwayomi,
}

pub struct DiscoverView {
    state: SharedDiscoveryState,
    tab: DiscoverTab,
    runtime: RuntimeState,
    installed: Option<InstalledInfo>,
    sources: Vec<SourceInfo>,
    extensions: Vec<ExtensionInfo>,
    selected_source: Option<String>,
    search_input: Entity<Input>,
    results: Vec<SearchResult>,
    selected_series: Option<SearchResult>,
    chapters: Vec<ChapterMeta>,
    selected_chapters: HashSet<i64>,
    busy: bool,
    message: Option<String>,
    download_current: usize,
    download_total: usize,
    download_label: String,
}

impl DiscoverView {
    pub fn new(state: SharedDiscoveryState, cx: &mut Context<Self>) -> Self {
        Self {
            state,
            tab: DiscoverTab::Catalog,
            runtime: RuntimeState::NotRunning,
            installed: None,
            sources: Vec::new(),
            extensions: Vec::new(),
            selected_source: None,
            search_input: cx.new(|cx| Input::new("", "Search manga", cx)),
            results: Vec::new(),
            selected_series: None,
            chapters: Vec::new(),
            selected_chapters: HashSet::new(),
            busy: false,
            message: None,
            download_current: 0,
            download_total: 0,
            download_label: String::new(),
        }
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.busy = true;
        self.message = None;
        let state = self.state.clone();
        cx.spawn(async move |this, cx| {
            let result = runtime::value(async move {
                let runtime_state = discovery::status(&state).await;
                let installed = discovery::installed_info(&state).await;
                let (sources, extensions) =
                    if matches!(runtime_state, RuntimeState::Ready { .. }) {
                        (
                            discovery::list_sources(&state).await,
                            discovery::list_extensions(&state).await,
                        )
                    } else {
                        (Ok(Vec::new()), Ok(Vec::new()))
                    };
                (runtime_state, installed, sources, extensions)
            })
            .await;
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                match result {
                    Ok((runtime_state, installed, sources, extensions)) => {
                        view.runtime = runtime_state.clone();
                        view.installed = installed.unwrap_or_else(|error| {
                            view.message = Some(error.to_string());
                            None
                        });
                        view.sources = sources.unwrap_or_else(|error| {
                            view.message = Some(error.to_string());
                            Vec::new()
                        });
                        view.extensions = extensions.unwrap_or_else(|error| {
                            view.message = Some(error.to_string());
                            Vec::new()
                        });
                        if view.selected_source.is_none() {
                            view.selected_source =
                                view.sources.first().map(|source| source.id.clone());
                        }
                        cx.emit(DiscoverEvent::RuntimeChanged(runtime_state));
                    }
                    Err(error) => view.message = Some(error.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn install(&mut self, cx: &mut Context<Self>) {
        self.busy = true;
        self.message = Some("Preparing Suwayomi download...".into());
        let state = self.state.clone();
        let (tx, mut rx) = tokio::sync::mpsc::channel(32);
        cx.spawn(async move |this, cx| {
            while let Some(progress) = rx.recv().await {
                let _ = this.update(cx, |view, cx| {
                    view.message = Some(format!("{progress:?}"));
                    cx.notify();
                });
            }
        })
        .detach();
        cx.spawn(async move |this, cx| {
            let result = runtime::app(discovery::install(state, tx)).await;
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                view.message = Some(match result {
                    Ok(()) => "Suwayomi installed.".into(),
                    Err(error) => error.to_string(),
                });
                view.refresh(cx);
            });
        })
        .detach();
    }

    fn lifecycle(
        &mut self,
        operation: impl FnOnce(
            SharedDiscoveryState,
        ) -> Pin<Box<dyn Future<Output = AppResult<RuntimeState>> + Send>>
            + Send
            + 'static,
        cx: &mut Context<Self>,
    ) {
        self.busy = true;
        let state = self.state.clone();
        cx.spawn(async move |this, cx| {
            let result = runtime::app(operation(state)).await;
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                match result {
                    Ok(runtime_state) => {
                        view.runtime = runtime_state.clone();
                        cx.emit(DiscoverEvent::RuntimeChanged(runtime_state));
                    }
                    Err(error) => view.message = Some(error.to_string()),
                }
                view.refresh(cx);
            });
        })
        .detach();
    }

    fn uninstall(&mut self, cx: &mut Context<Self>) {
        self.busy = true;
        let state = self.state.clone();
        cx.spawn(async move |this, cx| {
            let result = runtime::app(discovery::uninstall(state)).await;
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                view.message = Some(match result {
                    Ok(()) => "Suwayomi removed.".into(),
                    Err(error) => error.to_string(),
                });
                view.refresh(cx);
            });
        })
        .detach();
    }

    fn search(&mut self, cx: &mut Context<Self>) {
        let Some(source) = self.selected_source.clone() else {
            return;
        };
        let query = self.search_input.read(cx).value().trim().to_string();
        if query.is_empty() {
            return;
        }
        self.busy = true;
        let state = self.state.clone();
        cx.spawn(async move |this, cx| {
            let result =
                runtime::app(async move { discovery::search(&state, &source, &query, 1).await })
                    .await;
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                match result {
                    Ok(page) => view.results = page.results,
                    Err(error) => view.message = Some(error.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn select_series(&mut self, series: SearchResult, cx: &mut Context<Self>) {
        self.selected_series = Some(series.clone());
        self.busy = true;
        let state = self.state.clone();
        cx.spawn(async move |this, cx| {
            let result =
                runtime::app(async move { discovery::chapters(&state, series.id).await }).await;
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                match result {
                    Ok(chapters) => {
                        view.selected_chapters =
                            chapters.iter().map(|chapter| chapter.id).collect();
                        view.chapters = chapters;
                    }
                    Err(error) => view.message = Some(error.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn set_extension(&mut self, package: String, installed: bool, cx: &mut Context<Self>) {
        self.busy = true;
        let state = self.state.clone();
        cx.spawn(async move |this, cx| {
            let result = runtime::app(async move {
                if installed {
                    discovery::uninstall_extension(&state, &package).await
                } else {
                    discovery::install_extension(&state, &package).await
                }
            })
            .await;
            let _ = this.update(cx, |view, cx| {
                if let Err(error) = result {
                    view.message = Some(error.to_string());
                }
                view.refresh(cx);
            });
        })
        .detach();
    }

    fn download(&mut self, cx: &mut Context<Self>) {
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
        self.busy = true;
        self.download_total = chapters.len();
        self.download_current = 0;
        let state = self.state.clone();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let progress = Arc::new(move |current, total, label| {
            let _ = tx.send((current, total, label));
        });
        cx.spawn(async move |this, cx| {
            while let Some((current, total, label)) = rx.recv().await {
                let _ = this.update(cx, |view, cx| {
                    view.download_current = current;
                    view.download_total = total;
                    view.download_label = label;
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
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                view.message = Some(match result {
                    Ok(path) => format!("Downloaded to {}", path.display()),
                    Err(error) => error.to_string(),
                });
                cx.notify();
            });
        })
        .detach();
    }

    fn runtime_label(&self) -> String {
        match &self.runtime {
            RuntimeState::NotInstalled => "Setup needed".into(),
            RuntimeState::NotRunning => "Stopped".into(),
            RuntimeState::Starting => "Starting".into(),
            RuntimeState::Ready { port } => format!("Ready:{port}"),
            RuntimeState::Error { message } => format!("Error: {message}"),
        }
    }

    fn render_catalog(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let search = cx.entity();
        let download = cx.entity();
        let source_buttons = self.sources.iter().map(|source| {
            let entity = cx.entity();
            let source_id = source.id.clone();
            Button::new(source.name.clone())
                .variant(if self.selected_source.as_ref() == Some(&source.id) {
                    ButtonVariant::Secondary
                } else {
                    ButtonVariant::Ghost
                })
                .on_click(move |_, _, cx| {
                    entity.update(cx, |view, cx| {
                        view.selected_source = Some(source_id.clone());
                        cx.notify();
                    });
                })
        });
        let result_buttons = self.results.iter().cloned().map(|series| {
            let entity = cx.entity();
            Button::new(series.title.clone())
                .variant(ButtonVariant::Ghost)
                .on_click(move |_, _, cx| {
                    entity.update(cx, |view, cx| view.select_series(series.clone(), cx));
                })
        });
        let chapter_buttons = self.chapters.iter().map(|chapter| {
            let entity = cx.entity();
            let id = chapter.id;
            let selected = self.selected_chapters.contains(&id);
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
                entity.update(cx, |view, cx| {
                    if !view.selected_chapters.remove(&id) {
                        view.selected_chapters.insert(id);
                    }
                    cx.notify();
                });
            })
        });
        let source_list = gpui::div()
            .horizontal()
            .gap_dense()
            .children(source_buttons);
        let result_list = gpui::div()
            .horizontal()
            .flex_wrap()
            .gap_dense()
            .children(result_buttons);
        let chapter_list = gpui::div()
            .vertical()
            .gap_compact()
            .children(chapter_buttons);
        let progress = if self.download_total > 0 {
            ProgressBar::new(
                self.download_current as f32 / self.download_total.max(1) as f32,
            )
            .label(format!(
                "{}/{} · {}",
                self.download_current, self.download_total, self.download_label
            ))
            .into_any_element()
        } else {
            view! { div {} }.into_any_element()
        };

        Panel::new()
            .eyebrow("CATALOG")
            .title("Browse")
            .child(view! {
                div(vertical, gap_standard) {
                    { source_list }
                    div(horizontal, gap_dense) {
                        { self.search_input.clone() }
                        { Button::new("Search").disabled(self.busy || self.selected_source.is_none())
                            .on_click(move |_, _, cx| search.update(cx, |view, cx| view.search(cx))) }
                    }
                    { result_list }
                    { chapter_list }
                    { progress }
                    { Button::new(format!("Download {} selected", self.selected_chapters.len()))
                        .disabled(self.busy || self.selected_chapters.is_empty())
                        .on_click(move |_, _, cx| download.update(cx, |view, cx| view.download(cx))) }
                }
            })
    }

    fn render_server(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let uninstall = cx.entity();
        let extension_rows = self.extensions.iter().map(|extension| {
            let entity = cx.entity();
            let package = extension.pkg_name.clone();
            let installed = extension.installed;
            view! {
                div(horizontal, align_center, spread, border_bottom, padding_dense) {
                    div(vertical) {
                        div(text_strong) { { extension.name.clone() } }
                        div(text_caption, text_muted) {
                            { format!("{} · {}", extension.lang.clone().unwrap_or_default(), extension.version_name.clone().unwrap_or_default()) }
                        }
                    }
                    { Button::new(if installed { "Uninstall" } else { "Install" })
                        .variant(if installed { ButtonVariant::Danger } else { ButtonVariant::Secondary })
                        .disabled(self.busy)
                        .on_click(move |_, _, cx| entity.update(cx, |view, cx| view.set_extension(package.clone(), installed, cx))) }
                }
            }
        });
        let extension_list = gpui::div().vertical().children(extension_rows);
        Panel::new()
            .eyebrow("SUWAYOMI")
            .title("Server and extensions")
            .child(Badge::new(self.runtime_label()))
            .child(view! {
                div(text_muted) {
                    { self.installed.as_ref().map(|info| format!("Installed {} · {} bytes", info.version, info.size)).unwrap_or_else(|| "Not installed".into()) }
                }
            })
            .child(
                Button::new("Delete Suwayomi")
                    .variant(ButtonVariant::Danger)
                    .disabled(self.busy || self.installed.is_none())
                    .on_click(move |_, _, cx| {
                        uninstall.update(cx, |view, cx| view.uninstall(cx))
                    }),
            )
            .child(extension_list)
    }
}

impl EventEmitter<DiscoverEvent> for DiscoverView {}

impl Render for DiscoverView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let refresh = cx.entity();
        let install = cx.entity();
        let start = cx.entity();
        let stop = cx.entity();
        let restart = cx.entity();
        let catalog = cx.entity();
        let server = cx.entity();
        let ready = matches!(self.runtime, RuntimeState::Ready { .. });
        let status_variant = if ready {
            FeedbackVariant::Success
        } else {
            FeedbackVariant::Default
        };
        let actions = view! {
            div(horizontal, gap_dense) {
                { Button::new("Catalog").variant(if self.tab == DiscoverTab::Catalog { ButtonVariant::Secondary } else { ButtonVariant::Ghost })
                    .on_click(move |_, _, cx| catalog.update(cx, |view, cx| { view.tab = DiscoverTab::Catalog; cx.notify(); })) }
                { Button::new("Suwayomi").variant(if self.tab == DiscoverTab::Suwayomi { ButtonVariant::Secondary } else { ButtonVariant::Ghost })
                    .on_click(move |_, _, cx| server.update(cx, |view, cx| { view.tab = DiscoverTab::Suwayomi; cx.notify(); })) }
            }
        };
        let lifecycle = view! {
            div(horizontal, gap_dense) {
                { Button::new("Refresh").variant(ButtonVariant::Ghost).on_click(move |_, _, cx| refresh.update(cx, |view, cx| view.refresh(cx))) }
                { Button::new("Install").disabled(self.busy || self.installed.is_some()).on_click(move |_, _, cx| install.update(cx, |view, cx| view.install(cx))) }
                { Button::new("Start").disabled(self.busy || ready).on_click(move |_, _, cx| start.update(cx, |view, cx| {
                    view.lifecycle(|state| Box::pin(discovery::start(state)), cx)
                })) }
                { Button::new("Stop").variant(ButtonVariant::Secondary).disabled(self.busy || !ready).on_click(move |_, _, cx| stop.update(cx, |view, cx| {
                    view.lifecycle(|state| Box::pin(discovery::stop(state)), cx)
                })) }
                { Button::new("Restart").variant(ButtonVariant::Secondary).disabled(self.busy || !ready).on_click(move |_, _, cx| restart.update(cx, |view, cx| {
                    view.lifecycle(|state| Box::pin(discovery::restart(state)), cx)
                })) }
            }
        };
        let message = self
            .message
            .clone()
            .map(|message| {
                Alert::new(FeedbackVariant::Accent)
                    .title("Discovery")
                    .child(message)
                    .into_any_element()
            })
            .unwrap_or_else(|| view! { div {} }.into_any_element());

        view! {
            div(fill, vertical, gap_standard, padding_standard) {
                { PageHeader::new("Discover", "Search catalogs, manage Suwayomi, and download chapters.")
                    .badge(self.runtime_label(), status_variant)
                    .actions(actions) }
                { lifecycle }
                { message }
                { if self.tab == DiscoverTab::Catalog {
                    self.render_catalog(cx).into_any_element()
                } else {
                    self.render_server(cx).into_any_element()
                } }
            }
        }
    }
}
