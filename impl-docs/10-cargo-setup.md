# Cargo Workspace Setup

## Current workspace

```toml
# Thasia/Cargo.toml (today)
[workspace]
members = [
    "src-tauri",
    "crates/thasia-core",
    "crates/thasia-processor",
    "crates/thasia-parser",
    "crates/thasia-source",
    "crates/thasia-packager",
]
```

## Target workspace

`src-tauri` is replaced by `thasia-ui` (the GPUI app) and `thasia-dist` (packaging/release tooling).

```toml
# Thasia/Cargo.toml (after migration)
[workspace]
members = [
    "thasia-ui",             # ← GPUI app (replaces src-tauri + all of src/)
    "thasia-dist",           # ← xtask-style distribution/bundling helper
    "crates/thasia-core",
    "crates/thasia-processor",
    "crates/thasia-parser",
    "crates/thasia-source",
    "crates/thasia-packager",
]

[workspace.dependencies]
tokio        = { version = "1", features = ["full"] }
serde        = { version = "1", features = ["derive"] }
serde_json   = "1"
tracing      = "0.1"
tracing-subscriber = "0.3"
```

---

## `thasia-ui/Cargo.toml` — the app

```toml
[package]
name = "thasia"
version = "0.4.1"
edition = "2021"

[dependencies]
# UI framework
nasrin = { git = "https://github.com/LuMiSxh/nasrin", branch = "main" }
# or local path during development:
# nasrin = { path = "../../nasrin/crates/nasrin" }

# Internal crates
thasia-core      = { path = "../crates/thasia-core" }
thasia-processor = { path = "../crates/thasia-processor" }
thasia-parser    = { path = "../crates/thasia-parser" }
thasia-source    = { path = "../crates/thasia-source" }
thasia-packager  = { path = "../crates/thasia-packager" }

# Platform utilities
rfd          = "0.15"     # native file dialogs (AsyncFileDialog)
open         = "5"        # open URLs in default browser
directories  = "5"        # platform config/data/cache dirs (ProjectDirs)
confy        = "0.6"      # zero-boilerplate settings persistence

# Auto-update
self_update  = { version = "0.41", features = ["archive-tar", "compression-flate2"] }

# Async + serialization
tokio       = { workspace = true }
serde       = { workspace = true }
serde_json  = { workspace = true }

# Logging
tracing            = { workspace = true }
tracing-subscriber = { workspace = true }

# Embed assets (icons, etc.)
rust-embed = { version = "8", features = ["include-exclude"] }
```

---

## `thasia-dist/Cargo.toml` — distribution tooling

```toml
[package]
name = "thasia-dist"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "thasia-dist"
path = "src/main.rs"

[dependencies]
tauri-bundler = "2"        # .app/.dmg/.msi/.deb/.rpm/.AppImage
apple-bundle  = "0.2"      # typed Info.plist generation
plist         = "1"        # plist serialization
embed_plist   = "1"        # embed plist in binary (used via build.rs)
serde         = { workspace = true }
serde_json    = { workspace = true }
chrono        = "0.4"      # pub_date in update manifest
clap          = "4"        # CLI argument parsing
```

```
thasia-dist/
└── src/
    ├── main.rs       # CLI dispatcher (bundle / manifest / check-icons)
    ├── bundle.rs     # tauri-bundler invocation
    ├── plist.rs      # Info.plist generation
    ├── manifest.rs   # update-manifest.json generation
    └── icons.rs      # icon asset validation
```

---

## Directory layout after migration

```
Thasia/
├── thasia-ui/           ← was: src-tauri/ + src/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── app.rs
│       ├── actions.rs
│       ├── updater.rs
│       ├── state/
│       ├── views/
│       ├── components/
│       └── util/
│           ├── paths.rs       # AppPaths (directories crate)
│           ├── settings.rs    # confy-based settings
│           ├── dialog.rs      # rfd wrappers
│           ├── opener.rs      # open crate wrapper
│           └── persistence.rs # window state save/load
├── thasia-dist/         ← new: packaging + release tooling
│   ├── Cargo.toml
│   └── src/
├── crates/              ← unchanged
│   ├── thasia-core/
│   ├── thasia-processor/
│   ├── thasia-parser/
│   ├── thasia-source/
│   └── thasia-packager/
├── assets/              ← was: src-tauri/icons/ + src/assets/
│   └── icons/
│       ├── icon.icns
│       ├── icon.ico
│       ├── 32x32.png
│       ├── 128x128.png
│       └── 128x128@2x.png
├── Cargo.toml           (workspace)
├── Cargo.lock
└── impl-docs/
```

---

## Removing Tauri dependencies from crates

`thasia-source` currently imports Tauri for event emission. Replace with a channel:

```toml
# crates/thasia-source/Cargo.toml — BEFORE
tauri = { version = "2", optional = true }

# AFTER: remove entirely. SuwayomiManager accepts a Sender<SuwayomiEvent> parameter.
```

This change is isolated to `thasia-source` and does not affect `thasia-core` or
any other crate.

---

## Build profiles

Preserve `.cargo/config.toml` as-is — the O3 overrides for `rav1e`, `ravif`, `webp`
are independent of Tauri.

---

## Running in development

```bash
# Before (Tauri):
cargo tauri dev

# After (GPUI):
cargo run -p thasia
cargo run -p thasia --profile dev-opt   # with image crate optimizations
```

No Node, no Vite, no pnpm.

---

## Building for release

```bash
# Compile release binary:
cargo build -p thasia --release

# Bundle for current platform (via thasia-dist):
cargo run -p thasia-dist -- bundle

# → macOS:   target/release/bundle/macos/Thasia.app
#             target/release/bundle/macos/Thasia.dmg
# → Windows: target/release/bundle/windows/Thasia_0.4.1_x64-setup.exe
# → Linux:   target/release/bundle/linux/thasia_0.4.1_amd64.deb

# Generate update manifest for a GitHub release:
cargo run -p thasia-dist -- manifest --version 0.4.1 --notes "Release notes here"
# → update-manifest.json (upload as a release asset)
```
