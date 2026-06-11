# UI Component Mapping: Anasthasia → Nasrin

## Overview

The frontend currently uses **Anasthasia** (custom SvelteKit component library) with
**Tailwind CSS 4** for styling. The target uses **Nasrin** components with GPUI-CE
semantic tokens. No Tailwind, no CSS, no HTML — everything is Rust.

---

## Component mapping table

| Anasthasia / Svelte pattern              | Nasrin / GPUI equivalent                                                                                |
| ---------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| `<Button variant="primary">`             | `Button::new("label").variant(ButtonVariant::Primary)`                                                  |
| `<Button variant="secondary">`           | `Button::new("label")` (default is secondary)                                                           |
| `<Button variant="ghost">`               | `Button::new("label").variant(ButtonVariant::Ghost)`                                                    |
| `<Input bind:value />`                   | `Input::new(entity_handle)`                                                                             |
| `<Toggle bind:checked />`                | `Toggle::new(entity_handle)`                                                                            |
| `<Checkbox bind:checked />`              | `Checkbox::new(entity_handle)`                                                                          |
| `<Select bind:value options={...}>`      | `Select::new(entity_handle, options)`                                                                   |
| `<Badge color="green">`                  | `Badge::new("text").variant(BadgeVariant::Success)`                                                     |
| `<Alert type="error">`                   | `Alert::new("msg").variant(AlertVariant::Error)`                                                        |
| `<Spinner />`                            | `Spinner::new()`                                                                                        |
| `<ProgressBar value percent />`          | `ProgressBar::new(0.0..=1.0, value)`                                                                    |
| `<Card>`                                 | `Card::new().child(...)`                                                                                |
| `<Divider />`                            | `Divider::new()`                                                                                        |
| `<Tab label active>`                     | `Tab::new("label").active(bool)`                                                                        |
| `<TabBar>`                               | `TabBar::new().children(tabs)`                                                                          |
| `toast.push("msg", { type: "success" })` | `cx.global::<ToastStore>().update(cx, \|mgr, cx\| mgr.show(ToastVariant::Success, "Title", "msg", cx))` |
| `<Dialog bind:open>`                     | `Dialog::new(focus_handle).child(...).show(condition)`                                                  |
| `<Tooltip content="...">`                | `Tooltip::new("text").child(trigger)`                                                                   |
| `<ContextMenu>`                          | `ContextMenu::new(items)`                                                                               |
| `<ScrollArea>`                           | `ScrollArea::new().child(...)`                                                                          |
| `<Table columns rows>`                   | `Table::new(columns, rows)`                                                                             |
| `{#each items}` (virtualized)            | `uniform_list` or `VirtualList`                                                                         |
| `{#each items}` (small list)             | `.children(items.iter().map(render_item))`                                                              |
| `class="flex gap-2"`                     | `.flex().gap_standard()`                                                                                |
| `class="p-4"`                            | `.padding_standard()`                                                                                   |
| `class="w-full h-full"`                  | `.fill_width().fill_height()` or `.fill()`                                                              |
| `class="text-muted"`                     | `.text_muted(cx)`                                                                                       |
| `class="text-primary"`                   | `.text_primary(cx)`                                                                                     |
| `class="bg-surface"`                     | `.elevation_surface(cx)`                                                                                |
| `class="bg-background"`                  | `.elevation_background(cx)`                                                                             |
| CSS dark/light theme toggle              | `cx.set_theme_mode(ThemeMode::System)`                                                                  |

---

## Layout patterns

### Two-column app shell (sidebar + content)

```rust
// Current Svelte (+layout.svelte):
// <Sidebar /> + <slot />

// GPUI:
div()
    .fill()
    .flex()
    .child(Sidebar::new(self.page.clone(), self.sidebar_open))
    .child(
        div()
            .flex_1()
            .fill_height()
            .elevation_background(cx)
            .child(/* active page view */)
    )
```

### Wizard step container

```rust
// Multi-step wizard wrapper
div()
    .fill()
    .flex_col()
    .padding_standard()
    .gap_standard()
    .child(WizardStepHeader::new(step, total_steps))
    .child(/* step content */)
    .child(WizardNavButtons::new(can_back, can_next))
```

### Settings form

```rust
Card::new()
    .child(
        div()
            .flex_col()
            .gap_standard()
            .child(SectionLabel::new("Suwayomi"))
            .child(FieldRow::new(
                "Server URL",
                Input::new(url_entity.clone()),
            ).hint("Default: http://127.0.0.1:4567"))
            .child(FieldRow::new(
                "Auto-start",
                Toggle::new(autostart_entity.clone()),
            ).hint("Start Suwayomi when Thasia opens"))
    )
```

---

## Icon usage

Tauri frontend uses `@tabler/icons-svelte`. In GPUI, icons are either:

1. **SVG paths rendered with GPUI canvas** — for custom/tabler icons
2. **Embedded PNG/WebP** via `img(ImageSource::Embedded(...))` — for simple cases
3. **Nasrin icon API** (if it wraps GPUI's icon rendering)

For the Thasia icon set, the simplest path is to render SVG icons as GPUI canvas
elements. A thin wrapper:

```rust
pub fn icon(path_data: &'static str, size: Pixels) -> impl IntoElement {
    canvas(move |bounds, _, cx| {
        // draw SVG path data into bounds using GPUI's paint_svg or path rendering
    })
    .w(size)
    .h(size)
}
```

Alternatively, pre-rasterize the Tabler icon set to PNGs at 1x/2x and embed with
`rust-embed`. This is simpler but less crisp at non-standard DPIs.

---

## Theme

Tauri frontend toggles dark/light via Anasthasia's `theme` store and CSS variables.
In GPUI:

```rust
// On app init:
nasrin::init(cx);  // registers default themes
cx.set_theme_mode(ThemeMode::System);  // follow OS setting

// To manually switch:
cx.set_theme_mode(ThemeMode::Dark);
cx.set_theme_mode(ThemeMode::Light);
```

Nasrin's `NasrinRoot` automatically observes system appearance changes and re-applies
the theme — equivalent to Anasthasia's `prefers-color-scheme` listener.

---

## Reactive state (replacing Svelte 5 runes)

| Svelte pattern             | GPUI equivalent                                                      |
| -------------------------- | -------------------------------------------------------------------- |
| `let count = $state(0)`    | field on `Entity<MyView>`                                            |
| `$effect(() => ...)`       | `cx.observe(&other_entity, \|this, cx\| { ... })`                    |
| `$derived(expr)`           | computed in `render()` or cached in a field updated by `cx.observe`  |
| `bind:value={x}` (two-way) | `on_change` callback mutates the field + `cx.notify()`               |
| `{#if condition}`          | `condition.then_some(element)` or `a_if` in `view!` macro            |
| `{#each list}`             | `.children(list.iter().map(...))` or `uniform_list`                  |
| store subscription         | `cx.observe(&entity, callback)` or `cx.subscribe(&entity, callback)` |
