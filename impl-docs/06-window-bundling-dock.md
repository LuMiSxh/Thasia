# Window Management, Bundling & Dock Icon

## Window creation

Tauri's `tauri.conf.json` window options map directly to GPUI `WindowOptions`:

| tauri.conf.json                 | GPUI WindowOptions field                                               |
| ------------------------------- | ---------------------------------------------------------------------- |
| `width: 1120, height: 760`      | `window_bounds: Some(Bounds { size: size(px(1120.), px(760.)) })`      |
| `minWidth: 900, minHeight: 620` | Not in WindowOptions — enforce in resize handler                       |
| `titleBarStyle: "Overlay"`      | `titlebar: Some(TitlebarOptions { appears_transparent: true, .. })`    |
| `resizable: true`               | default (GPUI windows are resizable by default)                        |
| `title: "Thasia"`               | `titlebar: Some(TitlebarOptions { title: Some("Thasia".into()), .. })` |

```rust
WindowOptions {
    titlebar: Some(TitlebarOptions {
        title: Some("Thasia".into()),
        appears_transparent: true,
        traffic_light_position: Some(point(px(12.0), px(12.0))),
    }),
    window_bounds: Some(WindowBounds::Windowed(Bounds {
        origin: point(px(100.0), px(100.0)),
        size: size(px(1120.0), px(760.0)),
    })),
    focus: true,
    show: true,
    kind: WindowKind::Normal,
    ..Default::default()
}
```

---

## Window state persistence

`tauri-plugin-window-state` saves/restores window position and size automatically.
In GPUI, implement this manually:

```rust
// src/util/persistence.rs

#[derive(Serialize, Deserialize)]
pub struct WindowState {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub fn load_window_state() -> Option<WindowState> {
    let path = window_state_path();
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub fn save_window_state(bounds: Bounds<Pixels>) {
    let state = WindowState {
        x: bounds.origin.x.0,
        y: bounds.origin.y.0,
        width: bounds.size.width.0,
        height: bounds.size.height.0,
    };
    if let Ok(json) = serde_json::to_vec(&state) {
        let _ = std::fs::write(window_state_path(), json);
    }
}
```

Subscribe to window resize/move in `main.rs`:

```rust
cx.open_window(options, |window, cx| {
    // Observe window bounds changes
    cx.on_window_should_close(|_, cx| { true });
    // Save on close:
    cx.on_action(|_: &WindowClosed, cx| {
        if let Some(bounds) = cx.window_bounds() {
            save_window_state(bounds);
        }
    });
    cx.new(|cx| NasrinRoot::new(app, cx))
});
```

---

## Single instance

`tauri-plugin-single-instance` prevents a second process from opening.
Implement with a Unix domain socket lock:

```rust
// src/util/single_instance.rs
use std::net::TcpListener;

pub fn acquire_single_instance() -> bool {
    // Try to bind a fixed port — if it fails, another instance is running.
    TcpListener::bind("127.0.0.1:47832").is_ok()
    // For a proper implementation, use a named pipe (Windows) or
    // a lock file with fcntl (macOS/Linux).
}
```

Call this in `main()` before `nasrin::application().run(...)`:

```rust
fn main() {
    if !single_instance::acquire() {
        eprintln!("Thasia is already running.");
        std::process::exit(0);
    }
    nasrin::application().run(...)
}
```

---

## Dock icon (macOS)

The dock icon comes from the app bundle's `Info.plist` and the `.icns` file —
not from runtime code. GPUI apps use standard macOS app bundles.

### Development (no bundle)

When running `cargo run`, macOS shows a generic icon. To show the Thasia icon in
development, set the dock icon programmatically:

```rust
// GPUI-CE exposes this on macOS via AppContext:
cx.set_dock_icon(image_from_bytes(include_bytes!("../assets/icon.png")));
// (Exact API may be: cx.activate(true) + NSApp setApplicationIconImage via objc)
```

Check GPUI-CE's `AppContext` for a `set_application_icon` or dock image API.
If not available, use `objc2` crate to call `NSApplication.setApplicationIconImage_`.

### Release bundle (cargo-bundle)

See the Bundling section below.

---

## Bundling

Tauri's `bundle` config is replaced by **`cargo-bundle`** or a hand-crafted `.app`.

### `cargo-bundle` approach

Install: `cargo install cargo-bundle`

Add to the app `Cargo.toml`:

```toml
[package.metadata.bundle]
name = "Thasia"
identifier = "com.lumisxh.thasia"
icon = ["assets/icons/icon.icns"]
version = "0.4.1"
category = "public.app-category.utilities"
short_description = "Comic/manga conversion utility"
copyright = "Copyright © 2024 LuMiSxh"
osx_minimum_system_version = "13.0"
resources = ["assets/"]
```

Build:

```bash
cargo bundle --release
# → target/release/bundle/osx/Thasia.app
```

**Produces:** `Thasia.app` with proper `Info.plist`, dock icon, and bundled binary.

### Icon files needed

Same set as Tauri's `bundle.icon`:

```
assets/icons/
├── icon.icns          ← macOS (all sizes inside)
├── icon.ico           ← Windows
├── 32x32.png
├── 128x128.png
└── 128x128@2x.png
```

The existing Tauri icon assets in `src-tauri/icons/` can be moved to `assets/icons/`.

### App resources (embedded assets)

Static files that used to be in `src-tauri/` resources or the SvelteKit `build/`
output are now embedded via `rust-embed`:

```rust
#[derive(rust_embed::Embed)]
#[folder = "assets/"]
struct Assets;

// Access at runtime:
let icon_bytes = Assets::get("icons/icon.png").unwrap().data;
```

Nasrin's `nasrin::application()` already handles this pattern internally.
Register your own embed source if needed:

```rust
nasrin::application()
    .with_assets(Assets)   // if API exists, else use cx.asset_source()
    .run(...)
```

---

## macOS permissions / entitlements

Tauri's capability files (`capabilities/core.json`, `dialog.json`, `opener.json`)
become standard macOS entitlements in the `.entitlements` file (for App Store /
Hardened Runtime). For a direct-distribution `.app` these are usually not needed.

If hardened runtime is required:

```xml
<!-- Thasia.entitlements -->
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" ...>
<plist version="1.0">
<dict>
    <key>com.apple.security.app-sandbox</key><false/>
    <key>com.apple.security.files.user-selected.read-write</key><true/>
    <key>com.apple.security.network.client</key><true/>
</dict>
</plist>
```

Needed for: file dialogs (`rfd` opens a panel), Suwayomi HTTP traffic, and disk writes.
