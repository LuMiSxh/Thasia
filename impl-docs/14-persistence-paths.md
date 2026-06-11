# Persistence & Platform Paths

## The problem Tauri solved for us

Tauri's path API provided:

```typescript
appDataDir(); // ~/Library/Application Support/com.lumisxh.thasia/
appConfigDir(); // same on macOS
appLogDir(); // ~/Library/Logs/thasia/
```

In GPUI there's no equivalent — we resolve paths ourselves. Two crates handle this.

---

## Crate: `directories`

[`directories`](https://crates.io/crates/directories) provides `ProjectDirs` — the
direct equivalent of Tauri's path resolver.

```toml
directories = "5"
```

```rust
use directories::ProjectDirs;

fn project_dirs() -> ProjectDirs {
    ProjectDirs::from("com", "lumisxh", "Thasia")
        .expect("could not resolve platform directories")
}
```

| Method          | macOS                                               | Windows                            | Linux                      |
| --------------- | --------------------------------------------------- | ---------------------------------- | -------------------------- |
| `config_dir()`  | `~/Library/Application Support/com.lumisxh.Thasia/` | `%APPDATA%\lumisxh\Thasia\config\` | `~/.config/thasia/`        |
| `data_dir()`    | `~/Library/Application Support/com.lumisxh.Thasia/` | `%APPDATA%\lumisxh\Thasia\data\`   | `~/.local/share/thasia/`   |
| `cache_dir()`   | `~/Library/Caches/com.lumisxh.Thasia/`              | `%LOCALAPPDATA%\lumisxh\Thasia\`   | `~/.cache/thasia/`         |
| `runtime_dir()` | `None`                                              | `None`                             | `$XDG_RUNTIME_DIR/thasia/` |

**Concrete paths we need:**

```rust
// src/util/paths.rs
use directories::ProjectDirs;
use std::path::PathBuf;

pub struct AppPaths {
    pub config:        PathBuf,   // settings.json lives here
    pub data:          PathBuf,   // Suwayomi data, installed JARs
    pub cache:         PathBuf,   // temp/converted preview cache
    pub window_state:  PathBuf,   // window-state.json
    pub logs:          PathBuf,   // app.log
}

impl AppPaths {
    pub fn resolve() -> Self {
        let dirs = ProjectDirs::from("com", "lumisxh", "Thasia")
            .expect("platform dirs unavailable");
        let data = dirs.data_dir().to_path_buf();
        let config = dirs.config_dir().to_path_buf();
        let cache = dirs.cache_dir().to_path_buf();
        // Ensure dirs exist:
        for d in [&data, &config, &cache] {
            std::fs::create_dir_all(d).ok();
        }
        AppPaths {
            window_state: config.join("window-state.json"),
            logs:         data.join("thasia.log"),
            config:       config.join("settings.json"),
            data:         data.clone(),
            cache,
        }
    }
}
```

Set this as a GPUI global once on startup:

```rust
// main.rs
cx.set_global(AppPaths::resolve());
```

---

## Suwayomi resource path

Suwayomi's JAR, config, and downloaded manga data all live under `data_dir()`:

```
~/Library/Application Support/com.lumisxh.Thasia/
├── settings.json
├── window-state.json
├── suwayomi/
│   ├── suwayomi-server-*.jar      ← installed JAR
│   ├── server.conf                ← Suwayomi config
│   └── downloads/                 ← downloaded manga
└── thasia.log
```

```rust
impl AppPaths {
    pub fn suwayomi_dir(&self) -> PathBuf  { self.data.join("suwayomi") }
    pub fn suwayomi_jar(&self, version: &str) -> PathBuf {
        self.suwayomi_dir().join(format!("suwayomi-server-{version}.jar"))
    }
    pub fn suwayomi_downloads(&self) -> PathBuf { self.suwayomi_dir().join("downloads") }
}
```

The `SuwayomiManager` and `SuwayomiInstaller` in `thasia-source` already accept a
base path — pass `app_paths.suwayomi_dir()` instead of hardcoding.

---

## Crate: `confy` (settings persistence)

[`confy`](https://crates.io/crates/confy) is zero-boilerplate config persistence.
It handles serialization, deserialization, defaults, and platform paths automatically.

```toml
confy = "0.6"
```

```rust
// src/util/settings.rs
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AppSettings {
    pub discovery: DiscoverySettings,
    pub theme: ThemePreference,
    pub last_output_dir: Option<std::path::PathBuf>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            discovery: DiscoverySettings::default(),
            theme: ThemePreference::System,
            last_output_dir: None,
        }
    }
}

pub fn load() -> AppSettings {
    confy::load("thasia", "settings").unwrap_or_default()
}

pub fn save(settings: &AppSettings) {
    let _ = confy::store("thasia", "settings", settings);
}
```

Confy resolves the path using the same `etcetera`/XDG conventions as `directories`,
so the file lands in the correct platform location automatically.

**Note on macOS path convention**: Confy 2.0+ uses XDG on macOS by default
(`~/.config/thasia/settings.toml`). If you want the native macOS path
(`~/Library/Application Support/`), use `confy` with the `native` strategy or
just use `directories` + `serde_json` directly (which gives more control).
=> Settings should generally be loacted inside the Application Support for MacOS.

**Recommendation**: Use `confy` for simple settings, `directories` + `serde_json`
for fine-grained path control (Suwayomi JARs, window state). Both in parallel is fine.

---

## Logging (migration from `tracing_setup.rs`)

`tracing` + `tracing-appender` sind komplett framework-agnostisch — kein Tauri-Code drin.
`src-tauri/src/tracing_setup.rs` wandert **fast 1:1** nach `thasia-ui/src/util/logging.rs`.

**Zwei Änderungen:**

1. Pfad-Auflösung: `dirs::data_dir().join("com.thasia/logs")` → `AppPaths.logs` (konsistent mit dem Rest)
2. Crate-Name im EnvFilter: `thasia_tauri_lib=debug` → `thasia=debug`

```rust
// thasia-ui/src/util/logging.rs
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

const LOG_FILE_PREFIX: &str = "thasia";
const LOG_FILE_SUFFIX: &str = "log";
const LOG_MAX_FILES: usize = 7;
const DEFAULT_FILTER_DEV: &str =
    "info,thasia=debug,thasia_source=debug,thasia_processor=debug,thasia_parser=debug";
const DEFAULT_FILTER_RELEASE: &str = "info";

/// `log_dir` kommt von `AppPaths.logs`.
/// Der zurückgegebene Guard muss in `main()` am Leben gehalten werden.
pub fn init(log_dir: &std::path::Path) -> Option<WorkerGuard> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(default_filter()));

    if cfg!(debug_assertions) {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().with_writer(std::io::stderr).with_ansi(true))
            .init();
        return None;
    }

    if std::fs::create_dir_all(log_dir).is_err() {
        fallback_stderr(filter);
        return None;
    }

    let appender = match RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix(LOG_FILE_PREFIX)
        .filename_suffix(LOG_FILE_SUFFIX)
        .max_log_files(LOG_MAX_FILES)
        .build(log_dir)
    {
        Ok(a) => a,
        Err(_) => { fallback_stderr(filter); return None; }
    };

    let (writer, guard) = tracing_appender::non_blocking(appender);
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(writer).with_ansi(false))
        .init();
    Some(guard)
}

fn fallback_stderr(filter: EnvFilter) {
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(std::io::stderr).with_ansi(false))
        .init();
}

fn default_filter() -> &'static str {
    if cfg!(debug_assertions) { DEFAULT_FILTER_DEV } else { DEFAULT_FILTER_RELEASE }
}
```

**`main.rs`:**

```rust
let paths = AppPaths::resolve();
let _log_guard = logging::init(&paths.logs); // guard hält Worker-Thread am Leben
cx.set_global(paths);
nasrin::application().run(...);
```

**`thasia-ui/Cargo.toml`** — `tracing-appender` explizit hinzufügen (war bisher implizit in `src-tauri`):

```toml
tracing-appender = "0.2"
```

Der Suwayomi-Subprozess kann weiterhin `log_dir()` aus `AppPaths` nutzen, um
stdout/stderr in dieselbe Log-Directory zu schreiben.

---

## Window state persistence (manual, using `directories`)

```rust
// src/util/persistence.rs
use crate::util::paths::AppPaths;

pub fn load_window_state(paths: &AppPaths) -> Option<WindowStateData> {
    let bytes = std::fs::read(&paths.window_state).ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub fn save_window_state(paths: &AppPaths, bounds: Bounds<Pixels>) {
    let data = WindowStateData {
        x: bounds.origin.x.0,
        y: bounds.origin.y.0,
        width: bounds.size.width.0,
        height: bounds.size.height.0,
    };
    if let Ok(json) = serde_json::to_vec_pretty(&data) {
        let _ = std::fs::write(&paths.window_state, json);
    }
}
```
