# Navigation & Keyboard Shortcuts

## Current model

SvelteKit handles routing with URL-based navigation (`/`, `/convert`, `/discover`,
`/settings`). Keyboard shortcuts are registered in `src/lib/keyboard.ts` using
browser event listeners. The sidebar triggers `goto("/convert")` etc.

---

## GPUI model: Actions + page enum

GPUI uses a typed **action** system. Actions are zero-sized structs that can be
dispatched from keyboard shortcuts or button clicks.

### Define actions

```rust
// src/actions.rs
use gpui::actions;

actions!(
    thasia,
    [
        NavigateHome,
        NavigateConvert,
        NavigateDiscover,
        NavigateSettings,
        ToggleSidebar,
    ]
);
```

### Bind keyboard shortcuts

```rust
// src/main.rs — inside the run closure
cx.bind_keys([
    KeyBinding::new("cmd-1", NavigateHome, None),
    KeyBinding::new("cmd-2", NavigateConvert, None),
    KeyBinding::new("cmd-3", NavigateDiscover, None),
    KeyBinding::new("cmd-4", NavigateSettings, None),
    KeyBinding::new("cmd-b", ToggleSidebar, None),
]);
```

### Handle actions in ThasiaApp

```rust
impl ThasiaApp {
    fn register_handlers(&self, cx: &mut Context<Self>) {
        cx.on_action(|this: &mut ThasiaApp, _: &NavigateHome, cx| {
            this.page = Page::Home;
            cx.notify();
        });
        cx.on_action(|this, _: &NavigateConvert, cx| {
            this.page = Page::Convert;
            cx.notify();
        });
        cx.on_action(|this, _: &NavigateDiscover, cx| {
            this.page = Page::Discover;
            cx.notify();
        });
        cx.on_action(|this, _: &NavigateSettings, cx| {
            this.page = Page::Settings;
            cx.notify();
        });
        cx.on_action(|this, _: &ToggleSidebar, cx| {
            this.sidebar_open = !this.sidebar_open;
            cx.notify();
        });
    }
}
```

### Trigger from Sidebar buttons

```rust
// src/components/sidebar.rs
div()
    .on_click(|_, window, cx| cx.dispatch_action(Box::new(NavigateConvert)))
    .child("Convert")
```

Or use Nasrin `Button`:

```rust
Button::new("Convert")
    .on_click(|_, window, cx| cx.dispatch_action(Box::new(NavigateConvert)))
```

---

## Active page highlighting

The Sidebar reads `ThasiaApp.page` to apply active styles:

```rust
// Inside ThasiaApp::render():
Sidebar::new(self.page.clone(), self.sidebar_open)
```

```rust
// src/components/sidebar.rs
pub struct Sidebar {
    active: Page,
    open: bool,
}

impl RenderOnce for Sidebar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let items = [
            (Page::Home,     "Home",     NavigateHome.boxed_clone()),
            (Page::Convert,  "Convert",  NavigateConvert.boxed_clone()),
            (Page::Discover, "Discover", NavigateDiscover.boxed_clone()),
            (Page::Settings, "Settings", NavigateSettings.boxed_clone()),
        ];
        div()
            .w(if self.open { px(200.0) } else { px(56.0) })
            .flex_col()
            .elevation_surface(cx)
            .children(items.map(|(page, label, action)| {
                let active = self.active == page;
                Button::new(label)
                    .variant(if active { ButtonVariant::Primary } else { ButtonVariant::Secondary })
                    .on_click(move |_, _, cx| cx.dispatch_action(action.clone()))
            }))
    }
}
```

---

## Wizard step navigation

The Convert wizard has ~9 steps. These are **not** URL routes — they're state in
`ConvertView.step`. Navigation between steps is handled by methods on `ConvertView`:

```rust
pub enum WizardStep {
    SelectSource,
    Scanning,
    Configure,
    Preview,
    EditVolumes,
    OutputOptions,
    PipelinePlan,
    Converting,
    Complete,
}

impl ConvertView {
    fn next_step(&mut self, cx: &mut Context<Self>) {
        self.step = self.step.next();
        cx.notify();
    }
    fn prev_step(&mut self, cx: &mut Context<Self>) {
        self.step = self.step.prev();
        cx.notify();
    }
}
```

Each `WizardStep` variant renders a different child view inside `ConvertView::render()`.

---

## Deep links / initial state

Tauri could receive file arguments via OS file associations. If this is needed later,
GPUI-CE provides `cx.on_open_urls()` / argument parsing at launch. Not needed for v1.
