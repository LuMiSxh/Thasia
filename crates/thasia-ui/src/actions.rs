use gpui::{App, KeyBinding, actions};

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

pub fn register(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("cmd-1", NavigateHome, None),
        KeyBinding::new("cmd-2", NavigateConvert, None),
        KeyBinding::new("cmd-3", NavigateDiscover, None),
        KeyBinding::new("cmd-4", NavigateSettings, None),
        KeyBinding::new("cmd-b", ToggleSidebar, None),
        KeyBinding::new("ctrl-1", NavigateHome, None),
        KeyBinding::new("ctrl-2", NavigateConvert, None),
        KeyBinding::new("ctrl-3", NavigateDiscover, None),
        KeyBinding::new("ctrl-4", NavigateSettings, None),
        KeyBinding::new("ctrl-b", ToggleSidebar, None),
    ]);
}
