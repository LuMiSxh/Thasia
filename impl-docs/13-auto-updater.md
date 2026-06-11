# Auto-Updater

## Crate: `self_update`

[`self_update`](https://crates.io/crates/self_update) is the standard Rust crate for
in-place binary self-updates. It supports GitHub Releases as a backend out of the box —
which fits Thasia's existing release workflow perfectly.

```toml
# thasia-ui/Cargo.toml
self_update = { version = "0.41", features = ["archive-tar", "compression-flate2"] }
```

---

## How it works

1. App calls `self_update` at startup (or on a menu action).
2. Crate queries the GitHub Releases API for the latest release tag.
3. Compares against the running version (`env!("CARGO_PKG_VERSION")`).
4. If newer: downloads the matching asset for the current platform/arch.
5. Extracts the new binary and replaces the running one in-place.
6. App restarts (optional — can be a soft prompt instead).

---

## Implementation

```rust
// src/updater.rs
use self_update::cargo_crate_version;

pub fn check_and_update() -> Result<self_update::Status, self_update::errors::Error> {
    let status = self_update::backends::github::Update::configure()
        .repo_owner("LuMiSxh")
        .repo_name("Thasia")
        .bin_name("thasia")
        .show_download_progress(false)
        .current_version(cargo_crate_version!())
        .build()?
        .update()?;
    Ok(status)
}
```

**Call from GPUI (non-blocking):**

```rust
// In ThasiaApp, triggered by "Check for updates" button or on startup:
cx.spawn(|cx| async move {
    let result = cx.background_executor()
        .spawn(|| updater::check_and_update())
        .await;
    match result {
        Ok(self_update::Status::Updated(v)) => {
            // Show toast: "Updated to v{v} — restart to apply"
        }
        Ok(self_update::Status::UpToDate) => { /* silent */ }
        Err(e) => { /* show error banner */ }
    }
}).detach();
```

---

## Release asset naming

`self_update` matches assets by binary name + platform. Name release assets in CI as:

```
thasia-v0.4.1-x86_64-apple-darwin.tar.gz
thasia-v0.4.1-aarch64-apple-darwin.tar.gz
thasia-v0.4.1-x86_64-pc-windows-msvc.zip
thasia-v0.4.1-x86_64-unknown-linux-gnu.tar.gz
```

The crate infers the current platform from the compiled target triple.

---

## Update manifest (for version checks without full download)

For a lightweight "is there an update?" check (no download yet), use GitHub's Releases
API directly or generate a `manifest.json` in the release workflow:

```json
{
    "version": "0.4.1",
    "pub_date": "2025-11-01T00:00:00Z",
    "notes": "What's new in 0.4.1",
    "platforms": {
        "darwin-x86_64": {
            "url": "https://github.com/.../thasia-v0.4.1-x86_64-apple-darwin.tar.gz",
            "signature": "..."
        },
        "darwin-aarch64": {
            "url": "https://github.com/.../thasia-v0.4.1-aarch64-apple-darwin.tar.gz",
            "signature": "..."
        },
        "windows-x86_64": {
            "url": "https://github.com/.../thasia-v0.4.1-x86_64-pc-windows-msvc.zip",
            "signature": "..."
        },
        "linux-x86_64": {
            "url": "https://github.com/.../thasia-v0.4.1-x86_64-unknown-linux-gnu.tar.gz",
            "signature": "..."
        }
    }
}
```

This manifest is uploaded as a release asset and also hosted at a stable URL
(e.g. via GitHub Pages or a redirect). The app fetches it to show "version X is
available" in the UI before downloading.

### Are there GitHub Actions for manifest generation?

No - there are no ready-made GitHub Actions for generating Rust update manifests.
The only Action named "generate-update-manifest" (`2zqa/generate-update-manifest`)
is specifically designed for Firefox Extensions.

For **binary uploads**, however, there is an excellent pre-built action:
[`taiki-e/upload-rust-binary-action`](https://github.com/taiki-e/upload-rust-binary-action)
— handles cross-platform builds and uploads with the correct naming convention
(`thasia-x86_64-apple-darwin.tar.gz`, etc.). The manifest is generated via `thasia-dist`.

### CI-Workflow with `taiki-e/upload-rust-binary-action`

```yaml
# .github/workflows/release.yml
on:
    push:
        tags: ['v*']

jobs:
    build-upload:
        strategy:
            matrix:
                include:
                    - target: x86_64-apple-darwin
                      os: macos-13
                    - target: aarch64-apple-darwin
                      os: macos-14
                    - target: x86_64-pc-windows-msvc
                      os: windows-latest
                    - target: x86_64-unknown-linux-gnu
                      os: ubuntu-22.04
        runs-on: ${{ matrix.os }}
        steps:
            - uses: actions/checkout@v4
            - uses: taiki-e/upload-rust-binary-action@v1
              with:
                  bin: thasia
                  target: ${{ matrix.target }}
                  archive: thasia-$tag-$target # → thasia-v0.4.1-x86_64-apple-darwin.tar.gz
                  token: ${{ secrets.GITHUB_TOKEN }}

    generate-manifest:
        needs: build-upload
        runs-on: ubuntu-22.04
        steps:
            - uses: actions/checkout@v4
            - name: Build manifest
              run: cargo run -p thasia-dist -- manifest --version ${{ github.ref_name }} --notes "${{ github.event.release.body }}"
            - name: Upload manifest
              uses: softprops/action-gh-release@v2
              with:
                  files: update-manifest.json
```

Fetch it in the app to display "version X available" without triggering a full update:

```rust
pub async fn fetch_latest_version() -> Result<Option<String>, reqwest::Error> {
    let manifest: serde_json::Value = reqwest::get(MANIFEST_URL).await?.json().await?;
    Ok(manifest["version"].as_str().map(String::from))
}
```

---

## Update UX

Don't auto-apply silently. Recommended flow:

1. On startup: check manifest in background (no download).
2. If newer version found: show a dismissible `Alert` or `Toast`:
    > "Thasia 0.5.0 is available — [Update now] [Later]"
3. "Update now" → spawn `check_and_update()` → show progress → prompt restart.
