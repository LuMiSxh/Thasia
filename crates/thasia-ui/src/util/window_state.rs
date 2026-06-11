use crate::util::paths::AppPaths;
use nasrin::gpui::{Bounds, Pixels, WindowBounds, point, px, size};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct PersistedWindowState {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    maximized: bool,
}

pub fn load(paths: &AppPaths) -> Option<WindowBounds> {
    let bytes = std::fs::read(&paths.window_state).ok()?;
    let state: PersistedWindowState = serde_json::from_slice(&bytes).ok()?;
    if state.width < 900.0 || state.height < 620.0 {
        return None;
    }
    let bounds = Bounds {
        origin: point(px(state.x), px(state.y)),
        size: size(px(state.width), px(state.height)),
    };
    Some(if state.maximized {
        WindowBounds::Maximized(bounds)
    } else {
        WindowBounds::Windowed(bounds)
    })
}

pub fn save(paths: &AppPaths, bounds: WindowBounds) {
    let maximized = matches!(bounds, WindowBounds::Maximized(_));
    let bounds = bounds.get_bounds();
    let state = PersistedWindowState {
        x: pixels(bounds.origin.x),
        y: pixels(bounds.origin.y),
        width: pixels(bounds.size.width).max(900.0),
        height: pixels(bounds.size.height).max(620.0),
        maximized,
    };
    if let Ok(bytes) = serde_json::to_vec_pretty(&state) {
        let _ = std::fs::write(&paths.window_state, bytes);
    }
}

fn pixels(value: Pixels) -> f32 {
    value.into()
}
