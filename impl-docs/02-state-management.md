# State Management

## Current Tauri pattern

Tauri uses `tauri::Manager::manage()` to inject shared state into the process.
Commands receive it via `State<'_, T>` extractor parameters.

```rust
// Tauri
app.manage(RwLock::new(ConvState::default()));
app.manage(DiscoveryState::new(settings));

#[tauri::command]
async fn convert(state: State<'_, RwLock<ConvState>>, ...) { ... }
```

---

## GPUI replacement: Globals

GPUI provides `cx.set_global(T)` / `cx.global::<T>()` for process-wide singletons.
These are the direct equivalent of Tauri's managed state.

```rust
// src/state/conv.rs
#[derive(Default)]
pub struct ConvState {
    pub source: Option<LocalSource>,
    pub scan_result: Option<Vec<VolumeMeta>>,
    pub cancel_flag: Arc<AtomicBool>,
    pub active_task: Option<Task<()>>,
}
impl Global for ConvState {}

// src/state/discovery.rs
pub struct DiscoveryState {
    pub settings: DiscoverySettings,
    pub manager: Option<SuwayomiManager>,
    pub client: SuwayomiClient,
    pub runtime_state: RuntimeState,
}
impl Global for DiscoveryState {}
```

**Initialization (in `main.rs`):**

```rust
cx.set_global(ConvState::default());
cx.set_global(DiscoveryState::new(load_discovery_settings()));
```

**Reading from any view:**

```rust
let cancel = cx.global::<ConvState>().cancel_flag.clone();
```

**Mutating from async task callback:**

```rust
cx.update_global::<ConvState, _>(|state, cx| {
    state.scan_result = Some(result);
    // cx.notify() is not needed for globals — views must observe explicitly
});
// Then notify the specific view entity:
view_handle.update(cx, |_, cx| cx.notify());
```

---

## Per-view state

Each view entity holds its **own** mutable UI state. This replaces Svelte stores.

### ConvertView (wizard)

```rust
pub struct ConvertView {
    pub step: WizardStep,         // current wizard step (replaces wizard/state.svelte)
    pub source_path: Option<PathBuf>,
    pub scan_result: Option<Vec<VolumeMeta>>,
    pub pipeline_plan: Option<PipelinePlan>,
    pub options: ConvertOptions,
    pub edits: Vec<VolumeEdit>,
    pub progress: Option<ConversionProgress>,
    pub error: Option<AppError>,
    pub is_running: bool,
}
```

### DiscoverView

```rust
pub struct DiscoverView {
    pub runtime_state: RuntimeState,
    pub sources: Vec<SourceInfo>,
    pub search_query: String,
    pub search_results: Option<SearchPage>,
    pub selected_series: Option<SeriesInfo>,
    pub chapters: Vec<ChapterMeta>,
    pub selected_chapters: HashSet<u64>,
    pub download_progress: Option<DownloadProgress>,
    pub error: Option<AppError>,
}
```

### SettingsView

```rust
pub struct SettingsView {
    pub discovery_settings: DiscoverySettings,
    pub dirty: bool,
}
```

---

## Settings persistence

App settings (discovery URL, auto-start, etc.) are saved to disk, not via Tauri's
store plugin. Use `serde_json` + a well-known path:

```rust
// src/util/persistence.rs
fn settings_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("thasia")
        .join("settings.json")
}

pub fn load_settings() -> AppSettings { /* read + deserialize or Default */ }
pub fn save_settings(s: &AppSettings) { /* serialize + write */ }
```

`AppSettings` combines what was previously split between Tauri's managed state and
the frontend's `settings.ts`.

---

## Cancel flag

The conversion cancel flag is an `Arc<AtomicBool>` stored in `ConvState`. The async
pipeline task checks it between volumes — same logic as today, just accessed directly
instead of via Tauri state extractor.

```rust
// Start task
let flag = Arc::new(AtomicBool::new(false));
cx.update_global::<ConvState, _>(|s, _| s.cancel_flag = flag.clone());
cx.spawn(|_cx| async move {
    run_conversion(options, edits, flag, /* progress callback */).await;
});

// Cancel button handler
cx.update_global::<ConvState, _>(|s, _| {
    s.cancel_flag.store(true, Ordering::Relaxed);
});
```
