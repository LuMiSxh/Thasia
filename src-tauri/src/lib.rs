mod app_error;
mod commands;
mod conversion;
mod events;
mod pipeline_plan;
mod protocol;
mod state;
mod tracing_setup;

use std::sync::RwLock;

#[cfg(debug_assertions)]
use specta_typescript::Typescript;
use tauri::{Builder, Manager, RunEvent};
use tauri_specta::{Builder as SpectaBuilder, Event, collect_commands, collect_events};
use thasia_source::suwayomi::{RuntimeState, SuwayomiClient};

fn make_specta_builder() -> SpectaBuilder<tauri::Wry> {
    SpectaBuilder::<tauri::Wry>::new()
        .commands(collect_commands![
            commands::scan::scan_source,
            commands::scan::scan_current_source,
            commands::convert::build_pipeline_plan,
            commands::convert::convert,
            commands::cancel::cancel_conversion,
            commands::discovery::get_discovery_settings,
            commands::discovery::set_discovery_settings,
            commands::discovery::suwayomi_status,
            commands::discovery::suwayomi_installed_info,
            commands::discovery::suwayomi_install,
            commands::discovery::suwayomi_uninstall,
            commands::discovery::suwayomi_check_update,
            commands::discovery::suwayomi_start,
            commands::discovery::suwayomi_stop,
            commands::discovery::suwayomi_restart,
            commands::discovery::suwayomi_reset_data,
            commands::discovery::suwayomi_open_data_folder,
            commands::discovery::list_installed_sources,
            commands::discovery::list_available_extensions,
            commands::discovery::install_extension,
            commands::discovery::uninstall_extension,
            commands::discovery::search_source,
            commands::discovery::list_chapters,
            commands::discovery::download_series,
        ])
        .events(collect_events![
            events::ScanProgressEvent,
            events::VolumeStartEvent,
            events::ImageProgressEvent,
            events::VolumeCompleteEvent,
            events::ConversionCompleteEvent,
            events::SuwayomiStateChangedEvent,
            events::SuwayomiInstallProgressEvent,
            events::DownloadStartEvent,
            events::ChapterDownloadEvent,
            events::DownloadCompleteEvent,
        ])
}

pub fn run() {
    // Initialize tracing before anything else so early errors are captured.
    // Dev builds log to stderr; release builds log to <app_data>/com.thasia/logs/thasia.log.
    // The guard binding keeps the non-blocking writer's worker thread alive
    // until run() returns (i.e., until the app exits).
    let _log_guard = tracing_setup::init();

    let specta_builder = make_specta_builder();

    #[cfg(debug_assertions)]
    specta_builder
        .export(Typescript::default(), "../src/types/bindings.ts")
        .expect("Failed to export TypeScript bindings");

    let invoke_handler = specta_builder.invoke_handler();
    let builder_for_setup = specta_builder;

    Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(RwLock::new(state::ConvState::default()))
        .register_uri_scheme_protocol("thasia", |_app, request| protocol::handle(request))
        .invoke_handler(invoke_handler)
        .setup(move |app| {
            let app_data_dir = app.path().app_data_dir()?;
            let discovery_state = state::DiscoveryState::new(app_data_dir)
                .map_err(Box::<dyn std::error::Error>::from)?;
            app.manage(discovery_state);
            builder_for_setup.mount_events(app);
            let handle = app.handle().clone();
            let discovery = app.state::<state::DiscoveryState>();
            let settings = tauri::async_runtime::block_on(async {
                discovery.refresh_installed_version().await?;
                Ok::<_, state::StateError>(discovery.settings.read().await.clone())
            })
            .map_err(Box::<dyn std::error::Error>::from)?;
            if settings.enabled && settings.auto_start && settings.installed_version.is_some() {
                let manager = discovery.manager.clone();
                let state_handle = handle.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = events::SuwayomiStateChangedEvent {
                        state: RuntimeState::Starting,
                    }
                    .emit(&state_handle);
                    if let Some(discovery) = state_handle.try_state::<state::DiscoveryState>()
                        && let Err(err) = discovery.prepare_suwayomi_config().await
                    {
                        let _ = events::SuwayomiStateChangedEvent {
                            state: RuntimeState::Error {
                                message: err.to_string(),
                            },
                        }
                        .emit(&state_handle);
                        return;
                    }
                    match manager.start().await {
                        Ok(port) => {
                            if let Some(discovery) =
                                state_handle.try_state::<state::DiscoveryState>()
                            {
                                *discovery.client.write().await =
                                    Some(std::sync::Arc::new(SuwayomiClient::new(port)));
                                commands::discovery::start_monitor(
                                    &discovery,
                                    state_handle.clone(),
                                )
                                .await;
                            }
                            let _ = events::SuwayomiStateChangedEvent {
                                state: RuntimeState::Ready { port },
                            }
                            .emit(&state_handle);
                        }
                        Err(err) => {
                            let _ = events::SuwayomiStateChangedEvent {
                                state: RuntimeState::Error {
                                    message: err.to_string(),
                                },
                            }
                            .emit(&state_handle);
                        }
                    }
                });
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let RunEvent::ExitRequested { .. } = event
                && let Some(discovery) = app.try_state::<state::DiscoveryState>()
            {
                let manager = discovery.manager.clone();
                tauri::async_runtime::block_on(async {
                    commands::discovery::stop_monitor(&discovery).await;
                    let _ = tokio::time::timeout(std::time::Duration::from_secs(3), manager.stop())
                        .await;
                });
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generates src/types/bindings.ts. Run with `cargo test export_bindings`.
    #[test]
    fn export_bindings() {
        let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/types/bindings.ts");
        make_specta_builder()
            .export(Typescript::default(), &out)
            .expect("Failed to export TypeScript bindings");
        assert!(out.exists(), "bindings.ts was not created");
    }
}
