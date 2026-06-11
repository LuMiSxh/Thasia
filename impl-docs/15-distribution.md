# Distribution: Bundling, Installers & the `thasia-dist` crate

## Bundler choice: `tauri-bundler` (standalone)

[`tauri-bundler`](https://crates.io/crates/tauri-bundler) is the bundler from Tauri,
extracted as a standalone library. It's a fork of the original `cargo-bundle`, is
actively maintained, and produces **real installers** — not just a raw binary.

> `cargo-bundle` (original) is effectively abandoned. `tauri-bundler` is the maintained successor.

### What it produces

| Platform | Formats                                            |
| -------- | -------------------------------------------------- |
| macOS    | `.app` bundle, `.dmg` disk image                   |
| Windows  | `.msi` (WiX Toolset), `.exe` setup (NSIS)          |
| Linux    | `.deb` (Debian), `.rpm` (Fedora/RHEL), `.AppImage` |

### How to use it

`tauri-bundler` is a **library**, not a CLI. You call it programmatically from a
`xtask` binary or a standalone build script.

```toml
# xtask/Cargo.toml (build/packaging helper binary)
[dependencies]
tauri-bundler = "2"
```

```rust
// xtask/src/main.rs — "cargo xtask bundle"
use tauri_bundler::{bundle_project, PackageType, Settings, SettingsBuilder};

fn main() {
    let settings = SettingsBuilder::new()
        .package_types(vec![PackageType::MacOsBundle, PackageType::Dmg])
        .bundle_name("Thasia")
        .bundle_identifier("com.lumisxh.thasia")
        .icon(vec!["assets/icons/icon.icns".into()])
        .version("0.4.1")
        .binary_path("target/release/thasia")
        .build()
        .expect("bundler settings");

    bundle_project(settings).expect("bundle failed");
    // → target/release/bundle/macos/Thasia.app
    //   target/release/bundle/macos/Thasia.dmg
}
```

---

## plist automation

### Crate: `apple-bundle`

[`apple-bundle`](https://lib.rs/crates/apple-bundle) provides a strongly-typed
`InfoPlist` struct for generating `Info.plist` files programmatically.

```toml
apple-bundle = "0.2"
plist = "1"
```

```rust
use apple_bundle::prelude::*;

let info = InfoPlist {
    bundle_display_name: Some("Thasia".into()),
    bundle_executable: Some("thasia".into()),
    bundle_identifier: Some("com.lumisxh.thasia".into()),
    bundle_name: Some("Thasia".into()),
    bundle_version: Some("0.4.1".into()),
    bundle_short_version_string: Some("0.4.1".into()),
    bundle_icon_file: Some("icon.icns".into()),
    bundle_package_type: Some("APPL".into()),
    minimum_system_version: Some("13.0".into()),
    ns_principal_class: Some("NSApplication".into()),
    ns_high_resolution_capable: Some(true),
    ..Default::default()
};

plist::to_file_xml("Thasia.app/Contents/Info.plist", &info)?;
```

`tauri-bundler` already generates `Info.plist` internally. Use `apple-bundle` directly
only if you need custom plist keys that tauri-bundler doesn't expose (e.g. entitlements,
document types, URL schemes).

### Crate: `embed_plist`

For hardened-runtime builds, embed the plist into the binary itself:

```rust
// build.rs or main.rs
embed_plist::embed_info_plist!("assets/Info.plist");
embed_plist::embed_launchd_plist!("assets/Launchd.plist"); // if needed
```

---

## The `thasia-dist` crate

### Rationale

There are enough distribution concerns that a dedicated workspace member is warranted:

- `Info.plist` generation
- Update manifest generation
- Bundle configuration (icon paths, identifiers, platform targets)
- `tauri-bundler` invocation
- GitHub release asset naming convention

This is better as a `xtask`-style binary than a library — cargo xtask pattern.

### Workspace addition

```toml
# Cargo.toml (workspace)
members = [
    "thasia-ui",
    "thasia-dist",   # ← new xtask-style crate
    "crates/thasia-core",
    "crates/thasia-processor",
    "crates/thasia-parser",
    "crates/thasia-source",
    "crates/thasia-packager",
]
```

### `thasia-dist` structure

```
thasia-dist/
├── Cargo.toml
└── src/
    ├── main.rs          # CLI: cargo xtask (or cargo dist)
    ├── bundle.rs        # tauri-bundler invocation for all platforms
    ├── plist.rs         # Info.plist generation via apple-bundle
    ├── manifest.rs      # update-manifest.json generation
    └── icons.rs         # icon asset validation/resizing
```

### Commands

```bash
# Bundle for current platform:
cargo run -p thasia-dist -- bundle

# Generate update manifest for a release:
cargo run -p thasia-dist -- manifest --version 0.4.1 --notes "Changelog text"

# Validate icon assets:
cargo run -p thasia-dist -- check-icons
```

### Update manifest generation

```rust
// thasia-dist/src/manifest.rs
use serde::Serialize;

#[derive(Serialize)]
struct UpdateManifest {
    version: String,
    pub_date: String,
    notes: String,
    platforms: HashMap<String, PlatformAsset>,
}

pub fn generate(version: &str, notes: &str) -> String {
    let base = format!(
        "https://github.com/LuMiSxh/Thasia/releases/download/v{version}"
    );
    let platforms = [
        ("darwin-aarch64", format!("{base}/thasia-v{version}-aarch64-apple-darwin.tar.gz")),
        ("darwin-x86_64",  format!("{base}/thasia-v{version}-x86_64-apple-darwin.tar.gz")),
        ("windows-x86_64", format!("{base}/thasia-v{version}-x86_64-pc-windows-msvc.zip")),
        ("linux-x86_64",   format!("{base}/thasia-v{version}-x86_64-unknown-linux-gnu.tar.gz")),
    ].into_iter().map(|(k, url)| (k.to_string(), PlatformAsset { url })).collect();

    let manifest = UpdateManifest {
        version: version.into(),
        pub_date: chrono::Utc::now().to_rfc3339(),
        notes: notes.into(),
        platforms,
    };
    serde_json::to_string_pretty(&manifest).unwrap()
}
```

---

## CI / Release workflow summary

```yaml
# .github/workflows/release.yml
jobs:
    build-macos:
        runs-on: macos-14 # Apple Silicon
        steps:
            - cargo build --release --target aarch64-apple-darwin
            - cargo build --release --target x86_64-apple-darwin
            - cargo run -p thasia-dist -- bundle --target macos
            # → Thasia.dmg for both arches (universal binary or separate)

    build-windows:
        runs-on: windows-latest
        steps:
            - cargo build --release --target x86_64-pc-windows-msvc
            - cargo run -p thasia-dist -- bundle --target windows
            # → Thasia_0.4.1_x64-setup.exe + Thasia_0.4.1_x64_en-US.msi

    build-linux:
        runs-on: ubuntu-22.04
        steps:
            - cargo build --release --target x86_64-unknown-linux-gnu
            - cargo run -p thasia-dist -- bundle --target linux
            # → thasia_0.4.1_amd64.deb + thasia-0.4.1-1.x86_64.rpm + thasia_0.4.1_amd64.AppImage

    publish:
        needs: [build-macos, build-windows, build-linux]
        steps:
            -  # Upload all artifacts to GitHub Release
            - cargo run -p thasia-dist -- manifest --version ${{ github.ref_name }}
            -  # Upload update-manifest.json to release
```

---

## Summary: what each crate handles

| Concern                             | Crate                                        |
| ----------------------------------- | -------------------------------------------- |
| Platform-specific dirs              | `directories`                                |
| Settings persistence                | `confy` (or `directories` + `serde_json`)    |
| Auto-update (binary replace)        | `self_update`                                |
| macOS `.app` + `.dmg`               | `tauri-bundler` (via `thasia-dist`)          |
| Windows `.msi` / `.exe`             | `tauri-bundler` (via `thasia-dist`)          |
| Linux `.deb` / `.rpm` / `.AppImage` | `tauri-bundler` (via `thasia-dist`)          |
| Info.plist generation               | `apple-bundle` + `plist` (via `thasia-dist`) |
| Embedded plist in binary            | `embed_plist`                                |
| Update manifest JSON                | custom in `thasia-dist`                      |
| Icon validation/resizing            | `image` crate (via `thasia-dist`)            |
