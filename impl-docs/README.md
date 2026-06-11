# impl-docs: Tauri → GPUI Migration

Implementation reference for replacing Tauri + SvelteKit with GPUI-CE + Nasrin.

## Documents

| #   | File                                                     | Topic                                                       |
| --- | -------------------------------------------------------- | ----------------------------------------------------------- |
| 00  | [00-overview.md](00-overview.md)                         | What changes, what stays, migration phases                  |
| 01  | [01-architecture.md](01-architecture.md)                 | App structure, entity hierarchy, module layout              |
| 02  | [02-state-management.md](02-state-management.md)         | GPUI globals, per-view state, settings persistence          |
| 03  | [03-ipc-replacement.md](03-ipc-replacement.md)           | Deleting IPC: commands → direct calls, events → cx.notify() |
| 04  | [04-image-rendering.md](04-image-rendering.md)           | Replacing `thasia://` scheme with GPUI `img()`              |
| 05  | [05-async-and-events.md](05-async-and-events.md)         | cx.spawn(), WeakEntity callbacks, progress events           |
| 06  | [06-window-bundling-dock.md](06-window-bundling-dock.md) | WindowOptions, cargo-bundle, dock icon, single instance     |
| 07  | [07-navigation-keyboard.md](07-navigation-keyboard.md)   | Actions, cx.bind_keys(), view state machine                 |
| 08  | [08-dialogs-opener.md](08-dialogs-opener.md)             | rfd (file dialogs), open crate, reveal in Finder            |
| 09  | [09-ui-component-mapping.md](09-ui-component-mapping.md) | Anasthasia → Nasrin, Svelte patterns → GPUI                 |
| 10  | [10-cargo-setup.md](10-cargo-setup.md)                   | Workspace: `thasia-ui` + `thasia-dist`, build commands      |
| 11  | [11-convert-wizard.md](11-convert-wizard.md)             | 9-step wizard: WizardStep enum, step rendering              |
| 12  | [12-discover-view.md](12-discover-view.md)               | Suwayomi lifecycle, browse UI, download flow                |
| 13  | [13-auto-updater.md](13-auto-updater.md)                 | `self_update` crate, GitHub releases, update manifest format |
| 14  | [14-persistence-paths.md](14-persistence-paths.md)       | `directories` crate, `confy`, platform paths, Suwayomi resource dir |
| 15  | [15-distribution.md](15-distribution.md)                 | `tauri-bundler` standalone, `thasia-dist` xtask crate, plist automation, CI |

## Key decisions

- **No IPC layer** — UI is Rust, commands become plain `async fn`
- **No Node/pnpm/Vite/SvelteKit** — deleted entirely
- **crates/ unchanged** — all processing logic stays as-is
- **`thasia-ui`** — app crate name (replaces src-tauri + src)
- **`thasia-dist`** — xtask crate for bundling, plist, update manifests
- **Nasrin** replaces Anasthasia + Tailwind
- **rfd** replaces tauri-plugin-dialog
- **`tauri-bundler` (standalone)** replaces `tauri bundle` — produces .app/.dmg/.msi/.deb/.AppImage
- **`directories`** replaces Tauri path API — resolves platform data/config/cache dirs
- **`confy`** handles settings persistence (zero-boilerplate, TOML)
- **`self_update`** handles auto-updates from GitHub Releases
- **`apple-bundle` + `plist`** automates Info.plist generation
- **GPUI globals** replace Tauri managed state
- **cx.notify()** replaces Tauri events
