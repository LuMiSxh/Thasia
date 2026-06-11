# File Dialogs & URL Opener

## Current: tauri-plugin-dialog

The frontend calls `open()` from `@tauri-apps/plugin-dialog`:

```typescript
import { open } from '@tauri-apps/plugin-dialog';

const path = await open({
    directory: false,
    multiple: false,
    filters: [{ name: 'Archives', extensions: ['zip', 'cbz'] }],
});
```

## Current: tauri-plugin-opener

```typescript
import { openUrl } from '@tauri-apps/plugin-opener';
await openUrl('https://github.com/...');
```

---

## GPUI replacement: `rfd` + `open`

### File / folder dialogs → `rfd`

`rfd` (Rust File Dialog) shows native OS dialogs synchronously on a blocking thread.

Add to `Cargo.toml`:

```toml
rfd = "0.15"
```

```rust
// src/util/dialog.rs
use rfd::FileDialog;
use std::path::PathBuf;

pub fn pick_source() -> Option<PathBuf> {
    FileDialog::new()
        .add_filter("Archives & folders", &["zip", "cbz"])
        .set_title("Select source archive or folder")
        .pick_file()
        .or_else(|| FileDialog::new().pick_folder())
}

pub fn pick_output_dir() -> Option<PathBuf> {
    FileDialog::new()
        .set_title("Select output folder")
        .pick_folder()
}
```

**Call from GPUI button handler:**

```rust
Button::new("Browse…")
    .on_click(|_, window, cx| {
        // rfd must run on a blocking thread (it spins a native modal event loop)
        let handle = cx.weak_entity::<ConvertView>();
        cx.background_executor().spawn_blocking(|| dialog::pick_source())
            .then(|result, cx| {
                if let Some(path) = result {
                    handle.update(cx, |view, cx| {
                        view.source_path = Some(path);
                        cx.notify();
                    });
                }
            })
            .detach();
    })
```

> **Note:** `rfd` on macOS uses `NSOpenPanel` which must run on the main thread.
> Use `rfd::AsyncFileDialog` instead — it returns a `Future` that can be awaited
> in `cx.spawn()`:

```rust
cx.spawn(|cx| async move {
    let file = rfd::AsyncFileDialog::new()
        .add_filter("Archives", &["zip", "cbz"])
        .pick_file()
        .await;
    if let Some(file) = file {
        handle.update(&mut cx, |view, cx| {
            view.source_path = Some(file.path().to_path_buf());
            cx.notify();
        });
    }
}).detach();
```

### Open URL in browser → `open` crate

Add to `Cargo.toml`:

```toml
open = "5"
```

```rust
// src/util/opener.rs
pub fn open_url(url: &str) {
    let _ = open::that(url);
}
```

Call inline from a button handler — `open::that` is synchronous and fast:

```rust
Button::new("Open in browser")
    .on_click(|_, _, _| opener::open_url("https://github.com/suwayomi"))
```

### Open folder in Finder/Explorer

Used by `suwayomiOpenDataFolder`:

```rust
pub fn reveal_in_finder(path: &std::path::Path) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(path).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("explorer").arg(path).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}
```

---

## Capability changes

The Tauri `capabilities/dialog.json` and `capabilities/opener.json` files are deleted.
There are no capability declarations needed — `rfd` and `open` are ordinary crate calls.
