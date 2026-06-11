# Discover View: Suwayomi Integration

## Overview

The Discover page manages the Suwayomi server lifecycle and provides a manga browser.
It replaces `src/routes/discover/` and all Suwayomi-related Tauri commands.

---

## DiscoverView entity

```rust
pub struct DiscoverView {
    // Server lifecycle
    pub runtime_state: RuntimeState,
    pub install_progress: Option<InstallProgress>,

    // Extension management
    pub installed_sources: Vec<SourceInfo>,
    pub available_extensions: Vec<ExtensionInfo>,
    pub extensions_tab: ExtensionsTab,

    // Browse
    pub active_source: Option<String>,      // sourceId
    pub search_query: String,
    pub search_results: Option<SearchPage>,
    pub current_page: u32,

    // Series / chapters
    pub selected_series: Option<SeriesInfo>,
    pub chapters: Vec<ChapterMeta>,
    pub selected_chapters: HashSet<u64>,

    // Download
    pub download_progress: Option<DownloadProgress>,

    // Error
    pub error: Option<AppError>,

    // Event receiver handle
    pub event_task: Option<Task<()>>,
}

pub enum ExtensionsTab { Installed, Available }
```

---

## Suwayomi lifecycle management

Replace Tauri commands with direct calls to `SuwayomiManager` from GPUI:

```rust
impl DiscoverView {
    fn start_suwayomi(&mut self, cx: &mut Context<Self>) {
        let manager = cx.global::<DiscoveryState>().manager.clone().unwrap();
        let weak = cx.weak_entity();
        let (tx, mut rx) = tokio::sync::mpsc::channel::<SuwayomiEvent>(32);

        // Spawn listener task
        let listener = cx.spawn({
            let weak = weak.clone();
            |cx| async move {
                while let Some(event) = rx.recv().await {
                    weak.upgrade().map(|h| h.update(&mut cx.clone(), |view, cx| {
                        view.apply_suwayomi_event(event, cx);
                    }));
                }
            }
        });
        self.event_task = Some(listener);

        cx.spawn(|cx| async move {
            match manager.start(tx).await {
                Ok(_port) => { /* state update via event */ }
                Err(e) => weak.update(&mut cx, |view, cx| {
                    view.error = Some(e);
                    cx.notify();
                }),
            }
        }).detach();
    }

    fn apply_suwayomi_event(&mut self, event: SuwayomiEvent, cx: &mut Context<Self>) {
        match event {
            SuwayomiEvent::StateChanged(state) => self.runtime_state = state,
            SuwayomiEvent::InstallProgress(p)  => self.install_progress = Some(p),
            SuwayomiEvent::InstallComplete      => self.install_progress = None,
        }
        cx.notify();
    }
}
```

---

## Polling runtime state

Suwayomi's startup is async and goes through states:
`not_installed → not_running → starting → ready`

Check state after start and on a timer if needed:

```rust
// After start is called, poll until ready (or error):
cx.spawn(|cx| async move {
    loop {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let state = suwayomi_status(&manager).await;
        weak.update(&mut cx, |view, cx| {
            view.runtime_state = state.clone();
            cx.notify();
        });
        if matches!(state, RuntimeState::Ready | RuntimeState::Error(_)) {
            break;
        }
    }
}).detach();
```

---

## Browse UI layout

```
DiscoverView
├── SuwayomiStatusBar  (top — shows state + Start/Stop/Restart buttons)
├── TabBar [Browse | Extensions | Settings]
│   ├── Browse tab:
│   │   ├── SourceList (sidebar — installed sources)
│   │   └── BrowsePanel
│   │       ├── SearchInput + SearchButton
│   │       ├── SearchResults (grid of series)
│   │       └── SeriesDetail (chapters list + download button)
│   ├── Extensions tab:
│   │   ├── TabBar [Installed | Available]
│   │   ├── InstalledList (with Uninstall buttons)
│   │   └── AvailableList (with Install buttons)
│   └── Settings tab → inline DiscoverySettings form
└── DownloadProgressOverlay (shown when downloading)
```

---

## Download flow

```rust
fn download_series(&mut self, cx: &mut Context<Self>) {
    let series_id = self.selected_series.as_ref().unwrap().id.clone();
    let chapters = self.selected_chapters.iter().copied().collect::<Vec<_>>();
    let client = cx.global::<DiscoveryState>().client.clone();
    let weak = cx.weak_entity();

    cx.spawn(|cx| async move {
        // Start download
        client.download_chapters(&series_id, &chapters).await?;

        // Poll download queue progress
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let progress = client.download_progress().await?;
            weak.update(&mut cx, |view, cx| {
                view.download_progress = Some(progress.clone());
                cx.notify();
            });
            if progress.is_complete() { break; }
        }

        weak.update(&mut cx, |view, cx| {
            view.download_progress = None;
            cx.notify();
        });
        Ok::<_, AppError>(())
    }).detach();
}
```

---

## Status bar component

```rust
fn render_status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let (label, color) = match &self.runtime_state {
        RuntimeState::NotInstalled  => ("Not installed", BadgeVariant::Default),
        RuntimeState::NotRunning    => ("Stopped", BadgeVariant::Warning),
        RuntimeState::Starting      => ("Starting…", BadgeVariant::Info),
        RuntimeState::Ready         => ("Running", BadgeVariant::Success),
        RuntimeState::Error(_)      => ("Error", BadgeVariant::Error),
    };

    div()
        .flex()
        .gap_standard()
        .padding_dense()
        .elevation_surface(cx)
        .child(Badge::new(label).variant(color))
        .child(match &self.runtime_state {
            RuntimeState::NotInstalled => Button::new("Install")
                .on_click(cx.listener(Self::install_suwayomi))
                .into_any_element(),
            RuntimeState::NotRunning | RuntimeState::Error(_) => Button::new("Start")
                .on_click(cx.listener(Self::start_suwayomi))
                .into_any_element(),
            RuntimeState::Ready => div()
                .flex().gap_standard()
                .child(Button::new("Stop").on_click(cx.listener(Self::stop_suwayomi)))
                .child(Button::new("Restart").on_click(cx.listener(Self::restart_suwayomi)))
                .into_any_element(),
            _ => div().into_any_element(),
        })
}
```
