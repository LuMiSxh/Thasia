# New App Architecture

## Entry point

```rust
// src/main.rs
fn main() {
    nasrin::application().run(|cx: &mut App| {
        nasrin::init(cx);
        register_actions(cx);        // see 08-navigation.md
        load_persisted_state(cx);    // see 02-state-management.md

        cx.open_window(
            WindowOptions {
                titlebar: Some(TitlebarOptions {
                    title: Some("Thasia".into()),
                    appears_transparent: true,
                    traffic_light_position: Some(point(px(12.0), px(12.0))),
                }),
                window_bounds: Some(/* restored or default 1120×760 */),
                ..Default::default()
            },
            |_window, cx| {
                let app = cx.new(ThasiaApp::new);
                cx.new(|cx| NasrinRoot::new(app, cx))
            },
        )
        .expect("window");
    });
}
```

---

## Root entity: ThasiaApp

`ThasiaApp` is a GPUI `Entity<ThasiaApp>`. It holds the **current page** and all
cross-view mutable state. Every view is a child of this entity.

```rust
// src/app.rs
pub enum Page {
    Home,
    Convert,
    Discover,
    Settings,
}

pub struct ThasiaApp {
    pub page: Page,
    pub sidebar_open: bool,
    // Global entity handles for sub-views (created once, reused):
    pub home: Entity<HomeView>,
    pub convert: Entity<ConvertView>,
    pub discover: Entity<DiscoverView>,
    pub settings: Entity<SettingsView>,
}

impl Render for ThasiaApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .fill()
            .flex()
            .child(Sidebar::render(self, cx))
            .child(match self.page {
                Page::Home     => self.home.clone().into_any_element(),
                Page::Convert  => self.convert.clone().into_any_element(),
                Page::Discover => self.discover.clone().into_any_element(),
                Page::Settings => self.settings.clone().into_any_element(),
            })
    }
}
```

---

## Entity hierarchy

```
App (GPUI runtime)
└── ThasiaApp  (root entity, owns page state)
    ├── Sidebar (RenderOnce stateless widget, reads ThasiaApp)
    ├── HomeView (Entity)
    ├── ConvertView (Entity)
    │   ├── WizardState (embedded in ConvertView)
    │   └── ImagePreviewPane (Entity, optional)
    ├── DiscoverView (Entity)
    │   ├── SuwayomiPanel (Entity)
    │   └── SeriesBrowser (Entity)
    └── SettingsView (Entity)
```

---

## Communication patterns

### User action → state change

```
User clicks button
  → on_click closure
  → cx.update_entity(&app_handle, |app, cx| { app.page = Page::Convert; cx.notify(); })
  → GPUI re-renders ThasiaApp
```

### Long-running async task → UI progress update

```
Convert button pressed
  → cx.spawn(async move { ... })          // async task on background executor
  → per-progress: entity.update(cx, |view, cx| { view.progress = p; cx.notify(); })
  → GPUI re-renders ConvertView with new progress bar value
```

No IPC, no WebSocket, no serialization — it's all in the same process.

---

## Module layout

```
src/
├── main.rs              entry point, window creation
├── app.rs               ThasiaApp entity + Page enum
├── actions.rs           Action types + key bindings
├── state/
│   ├── mod.rs
│   ├── conv.rs          ConvState (replaces src-tauri/src/state.rs ConvState)
│   └── discovery.rs     DiscoveryState (replaces DiscoveryState)
├── views/
│   ├── home.rs
│   ├── convert/
│   │   ├── mod.rs       ConvertView entity + WizardState
│   │   ├── steps/       one file per wizard step (9 steps)
│   │   └── preview.rs   image preview pane
│   ├── discover/
│   │   ├── mod.rs       DiscoverView entity
│   │   ├── suwayomi.rs  Suwayomi panel + lifecycle controls
│   │   └── browser.rs   Source/series/chapter browser
│   └── settings.rs      SettingsView entity
├── components/
│   ├── sidebar.rs       Sidebar widget (RenderOnce)
│   ├── progress_row.rs  reusable progress display
│   └── error_banner.rs  reusable error display
└── util/
    ├── image.rs         load image bytes for GPUI img()
    ├── dialog.rs        rfd wrappers (open file/folder)
    └── persistence.rs   window bounds + settings to ~/.config/thasia/
```
