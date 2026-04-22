mod commands;
mod events;
mod protocol;
mod state;

use std::sync::RwLock;

#[cfg(debug_assertions)]
use specta_typescript::Typescript;
use tauri::Builder;
use tauri_specta::{collect_commands, collect_events, Builder as SpectaBuilder};

fn make_specta_builder() -> SpectaBuilder<tauri::Wry> {
    SpectaBuilder::<tauri::Wry>::new()
        .commands(collect_commands![
            commands::scan::scan_source,
            commands::convert::convert,
        ])
        .events(collect_events![
            events::ScanProgressEvent,
            events::VolumeStartEvent,
            events::ImageProgressEvent,
            events::VolumeCompleteEvent,
            events::ConversionCompleteEvent,
        ])
}

pub fn run() {
    let specta_builder = make_specta_builder();

    #[cfg(debug_assertions)]
    specta_builder
        .export(Typescript::default(), "../src/types/bindings.ts")
        .expect("Failed to export TypeScript bindings");

    let invoke_handler = specta_builder.invoke_handler();
    let builder_for_setup = specta_builder;

    Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(RwLock::new(state::ConvState::default()))
        .register_uri_scheme_protocol("thasia", |_app, request| {
            protocol::handle(request)
        })
        .invoke_handler(invoke_handler)
        .setup(move |app| {
            builder_for_setup.mount_events(app);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generates src/types/bindings.ts. Run with `cargo test export_bindings`.
    #[test]
    fn export_bindings() {
        let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../src/types/bindings.ts");
        make_specta_builder()
            .export(Typescript::default(), &out)
            .expect("Failed to export TypeScript bindings");
        assert!(out.exists(), "bindings.ts was not created");
    }
}
