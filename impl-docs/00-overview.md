# Migration Overview: Tauri + SvelteKit → GPUI (Nasrin)

## What is this migration?

Thasia currently uses **Tauri 2** as its native shell and **SvelteKit** as its UI layer.
This migration replaces both with **GPUI-CE** (via the **Nasrin** component library) — a
GPU-accelerated, fully native Rust UI framework.

The core processing logic (`crates/`) is **untouched**. Only the presentation layer and
the IPC plumbing between frontend and backend disappears.

---

## Layer comparison

| Concern            | Current (Tauri)                 | Target (GPUI/Nasrin)                           |
| ------------------ | ------------------------------- | ---------------------------------------------- |
| UI language        | TypeScript + Svelte             | Rust                                           |
| UI framework       | SvelteKit + Anasthasia          | GPUI-CE + Nasrin                               |
| IPC                | Tauri commands + events         | Direct Rust function calls                     |
| Async progress     | Tauri events over WebSocket     | GPUI entity updates (`cx.notify()`)            |
| Image serving      | `thasia://` custom URI scheme   | In-memory bytes or disk paths via GPUI `img()` |
| File dialogs       | `tauri-plugin-dialog`           | `rfd` (Rust File Dialog)                       |
| Open in browser    | `tauri-plugin-opener`           | `open` crate                                   |
| Window state       | `tauri-plugin-window-state`     | Manual `WindowOptions` + persisted JSON        |
| Single instance    | `tauri-plugin-single-instance`  | Platform socket lock (custom) or GPUI handle   |
| Bundling           | `tauri bundle`                  | `cargo-bundle` or manual `.app` packaging      |
| Dock icon          | Tauri bundle icons              | GPUI `ApplicationMenu` / bundle `Info.plist`   |
| Keyboard shortcuts | Frontend `keyboard.ts` + Svelte | GPUI `Action` + `cx.bind_keys()`               |
| Routing            | SvelteKit router                | GPUI view state machine                        |
| Theme              | Anasthasia + CSS variables      | Nasrin `ThemeMode` / `Flavour`                 |

---

## What stays

- `crates/thasia-core` — image models, serialization
- `crates/thasia-processor` — full image pipeline
- `crates/thasia-parser` — filename parsing
- `crates/thasia-source` — LocalSource, SuwayomiClient/Manager/Installer
- `crates/thasia-packager` — CBZ/EPUB/Raw output
- `src-tauri/src/commands/` logic (moved into plain async functions)
- `src-tauri/src/state.rs` (adapted to GPUI globals)

---

## What is deleted

- `src/` — entire SvelteKit frontend
- `src-tauri/` — Tauri bootstrap, capabilities, build script
- `package.json`, `pnpm-lock.yaml`, `pnpm-workspace.yaml`
- `svelte.config.js`, `vite.config.ts`, `tsconfig.json`, `eslint.config.js`
- All Node/pnpm tooling

---

## New top-level structure

```
Thasia/
├── crates/
│   ├── thasia-core/
│   ├── thasia-processor/
│   ├── thasia-parser/
│   ├── thasia-source/
│   └── thasia-packager/
├── src/                      ← new GPUI app (replaces src-tauri/src + all of src/)
│   ├── main.rs
│   ├── app.rs                ← root entity
│   ├── state/                ← global app state (replaces state.rs)
│   ├── actions.rs            ← keyboard action definitions
│   ├── views/
│   │   ├── home.rs
│   │   ├── convert/
│   │   ├── discover/
│   │   └── settings.rs
│   └── components/           ← shared Nasrin-based widgets
├── assets/                   ← embedded icons & images (rust-embed)
├── Cargo.toml
└── impl-docs/
```

---

## Migration phases

| Phase | Scope                                                                 |
| ----- | --------------------------------------------------------------------- |
| 1     | Scaffold new GPUI binary, window boots, Nasrin `NasrinRoot` renders   |
| 2     | Port state management (ConvState, DiscoveryState as GPUI globals)     |
| 3     | Port command logic to plain `async fn`, wire into GPUI entity actions |
| 4     | Implement Home + Settings views                                       |
| 5     | Implement Convert wizard (9-step state machine)                       |
| 6     | Implement Discover views + Suwayomi lifecycle                         |
| 7     | Image preview rendering (replace `thasia://` scheme)                  |
| 8     | File dialogs, opener, single-instance lock                            |
| 9     | Window state persistence                                              |
| 10    | Bundling, dock icon, app icons                                        |
