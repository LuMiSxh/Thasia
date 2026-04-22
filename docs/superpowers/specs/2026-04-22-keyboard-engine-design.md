# Keyboard Engine Design

## Goal

Add a keyboard-first navigation layer to Thasia: a `KeyboardManager` for event routing, a scoped `KeyHintState` for contextual hints, a toggleable hint bar at the bottom of the UI, and per-step shortcut bindings throughout the wizard. Mouse still works for everything — keyboard is additive.

## Architecture

Four new files, two modified files.

### New files

**`src/lib/keyboard.ts`** — Singleton `KeyboardManager` class (ported from Palaxy). Maintains a `Map<combo, handler[]>` keyed by normalized combo strings like `"alt+arrowright"`. Dispatches newest-to-oldest on keydown. Skips letter-key combos and `Alt+Arrow` combos when the focused element is a text input, textarea, or contenteditable. Exposes:
- `register(combo, callback, id?)` → id
- `unregister(id)`
- `smartRegister(handlers[]) → cleanup fn` — registers all, returns one cleanup
- `mount() → unmount fn` — attaches/detaches the window listener

**`src/lib/keyhint.svelte.ts`** — Singleton `KeyHintState` class (ported from Palaxy) plus two Svelte actions. Manages named scopes of `[combo, label]` pairs. Exposes:
- `keyHint.register(pairs[], exclusive?) → cleanup fn` — adds a scope, returns cleanup
- `keyHint.get() → [combo, label][]` — flattens all active scopes (exclusive scopes hide others)
- `use:mountedHint={pairs}` — Svelte action: registers hints for the lifetime of the element (on mount → on destroy). Used on step root divs for always-on step shortcuts.
- `use:handleKeyHint={keys}` — Svelte action (ported from Palaxy): registers hints only while the element is focused. Used on individual interactive elements.

No manual `keyHint.register()` calls needed in component `onMount` — all hint display is action-based. Only `keyboard.smartRegister` (the actual shortcut handlers) lives in `onMount`.

**`src/components/ui/KeyComboDisplay.svelte`** — Converts a combo string to OS-aware symbols. Detects OS via `@tauri-apps/plugin-os`. Renders using the existing `Kbd` component. Examples: `"alt+arrowright"` → `⌥→` on macOS, `Alt →` on Windows.

**`src/components/ui/KeyHintBar.svelte`** — Thin strip (h-8) rendered inside `<main>` in `+layout.svelte`, flush at the bottom. `border-t border-thasia-border bg-thasia-surface`. Reads `keyHint.get()` reactively. Each hint: `[KeyComboDisplay] label`. Hints animate in/out with a width-collapse + translate + blur transition (glass motion, ported from Palaxy). Hidden entirely when `showKeyHints` setting is false — shortcuts still fire.

### Modified files

**`src/routes/+layout.svelte`** — On mount: call `keyboard.mount()` and save unmount cleanup. Register global shortcut handlers via `smartRegister`. Render `<KeyHintBar>` inside `<main>` as a flex child at the bottom (main becomes `flex flex-col`, content area is `flex-1 overflow-auto`, hint bar is `flex-shrink-0`). Use `use:mountedHint` on the layout root for global hints (`⌘1`–`⌘4`, `⌘B`).

**`src/routes/convert/+page.svelte`** — On mount: `smartRegister` `Alt+ArrowRight` (→ `goNext`) and `Alt+ArrowLeft` (→ `goBack`). Use `use:mountedHint` on the page root div for those two hints. Clean up on destroy.

**`src/routes/settings/+page.svelte`** — Add `showKeyHints: boolean` (default `true`) to the `Defaults` type and localStorage persistence. Add a new "Interface" panel (same `rounded-xl border bg-thasia-surface` style) with a single `Toggle` row: "Keyboard hint bar" / "Show shortcut hints at the bottom of the window".

## Keyboard Bindings

### Global — registered in `+layout.svelte`

| Combo | Action |
|---|---|
| `⌘1` | Navigate to `/` (Home) |
| `⌘2` | Navigate to `/convert` (Convert) |
| `⌘3` | Navigate to `/settings` (Settings) |
| `⌘4` | Navigate to `/about` (About) |
| `⌘B` | Toggle sidebar |

### Wizard navigation — registered in `convert/+page.svelte`

| Combo | Action |
|---|---|
| `Alt+→` | Next step (only fires if current step is valid) |
| `Alt+←` | Back step |

### Step-specific — `use:mountedHint` on step root div for hints, `onMount` for handlers

| Step | Combo | Action |
|---|---|---|
| Step 3 Image Format | `A` | Select AVIF |
| Step 3 Image Format | `W` | Select WebP |
| Step 3 Image Format | `O` | Select Original |
| Step 4 Container | `C` | Select CBZ |
| Step 4 Container | `E` | Select EPUB |
| Step 4 Container | `R` | Select Raw |
| Step 5 Direction | `L` | Select LTR |
| Step 5 Direction | `R` | Select RTL |
| Step 6 Bundling | `A` | Select Auto |
| Step 6 Bundling | `F` | Select Flatten |
| Step 7 Page Editor | `←` / `→` | Previous / next page |
| Step 7 Page Editor | `X` | Toggle exclude on current page |
| Step 9 Convert | `Enter` | Start over (when status is `done`) |

## Hint Bar Visual

```
┌─────────────────────────────────────────────────────────────────┐
│  [⌥→] Next step   [⌥←] Back   [A] AVIF   [W] WebP   [O] Orig  │  ← h-8, border-t
└─────────────────────────────────────────────────────────────────┘
```

- Horizontal flex, `items-center`, `gap-6`, `px-4`
- Each hint: `KeyComboDisplay` + `text-xs text-thasia-muted` label
- Dividers between hints: `h-3 w-px bg-thasia-border`
- Hints animate individually: width-collapse + translateX(-20px) + blur on enter/exit
- Hints from exclusive scopes hide all others (e.g., no global hints while page editor is active)

## Settings Integration

New "Interface" panel added to `settings/+page.svelte`, placed below the existing two-column grid:

```
┌─ Interface ──────────────────────────────────────────────────────┐
│  [monitor icon]  Keyboard hint bar     [Toggle: on]              │
│                  Show shortcut hints at the bottom of the window │
└──────────────────────────────────────────────────────────────────┘
```

- Same `rounded-xl border border-thasia-border bg-thasia-surface overflow-hidden` panel style
- Panel header: `bg-thasia-panel px-4 py-2.5` + `text-[10px] font-bold tracking-widest uppercase`
- Row: `flex items-center justify-between px-4 py-4` with icon + label/hint on left, Toggle on right
- `showKeyHints` persisted in `thasia:settings` localStorage key alongside other defaults

## Combo String Format

All combos use lowercase normalized format matching Palaxy's `KeyboardManager`:
- Modifiers: `ctrl`, `alt`, `shift`, `meta` (joined with `+`)
- Keys: browser `event.code` values lowercased: `arrowright`, `arrowleft`, `keya`, `keyb`, etc.
- Example: `Alt+ArrowRight` → stored/matched as `"alt+arrowright"`
- Single letters registered as `"keya"`, `"keyw"`, `"keyo"`, etc.
- Number keys: `"digit1"`, `"digit2"`, etc. — so `⌘1` is stored as `"meta+digit1"`

## OS Detection

`KeyComboDisplay` calls `type()` from `@tauri-apps/plugin-os` once on mount to determine `"macos"` vs other. On macOS: `alt` → `⌥`, `meta` → `⌘`, arrow keys → `←→↑↓`. On Windows/Linux: `alt` → `Alt`, `meta` → `Win`.

The hint bar uses the same display for consistency.
