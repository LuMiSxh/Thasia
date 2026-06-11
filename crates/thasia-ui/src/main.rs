mod actions;
mod app;
mod components;
mod error;
mod models;
mod services;
mod state;
mod theme;
mod updater;
mod util;
mod views;

use app::ThasiaApp;
use nasrin::{
    FlavourContextExt, NasrinRoot, ThemeMode,
    gpui::{
        App, AppContext, Bounds, TitlebarOptions, WindowBounds, WindowOptions, point, px, size,
    },
};

fn main() {
    util::runtime::init().expect("failed to initialize Tokio runtime");
    let paths = util::paths::AppPaths::resolve().expect("failed to resolve application paths");
    let _instance_guard = util::single_instance::InstanceGuard::acquire(&paths.data_dir)
        .expect("failed to acquire application lock")
        .unwrap_or_else(|| {
            eprintln!("Thasia is already running.");
            std::process::exit(0);
        });
    let _log_guard = util::logging::init(&paths.logs);
    let app_settings = util::settings::load_app(&paths);
    let conv_state = std::sync::Arc::new(std::sync::RwLock::new(state::ConvState::default()));
    let discovery_settings = util::settings::load_discovery(&paths);
    let installer = std::sync::Arc::new(
        thasia_source::suwayomi::SuwayomiInstaller::new(paths.suwayomi.clone())
            .expect("failed to initialize Suwayomi installer"),
    );
    let discovery_state = std::sync::Arc::new(state::DiscoveryState {
        paths: paths.clone(),
        settings: std::sync::Arc::new(tokio::sync::RwLock::new(discovery_settings)),
        manager: std::sync::Arc::new(thasia_source::suwayomi::SuwayomiManager::new(
            installer.clone(),
        )),
        installer,
        client: std::sync::Arc::new(tokio::sync::RwLock::new(None)),
    });

    nasrin::application().run(move |cx: &mut App| {
        nasrin::init(cx);
        theme::register(cx);
        actions::register(cx);
        cx.set_theme(
            theme::LUMI_FLAVOUR,
            match app_settings.theme {
                util::settings::ThemePreference::Light => ThemeMode::Light,
                util::settings::ThemePreference::Dark => ThemeMode::Dark,
                util::settings::ThemePreference::System => ThemeMode::System,
            },
        );

        let window_bounds = util::window_state::load(&paths).unwrap_or_else(|| {
            WindowBounds::Windowed(Bounds {
                origin: point(px(100.0), px(100.0)),
                size: size(px(1120.0), px(760.0)),
            })
        });
        cx.open_window(
            WindowOptions {
                titlebar: Some(TitlebarOptions {
                    title: Some("Thasia".into()),
                    appears_transparent: true,
                    ..Default::default()
                }),
                window_bounds: Some(window_bounds),
                ..Default::default()
            },
            |window, cx| {
                let app = cx.new(|cx| {
                    ThasiaApp::new(
                        conv_state.clone(),
                        discovery_state.clone(),
                        paths.clone(),
                        app_settings.clone(),
                        cx,
                    )
                });
                let state_paths = paths.clone();
                app.update(cx, |_, cx| {
                    cx.observe_window_bounds(window, move |_, window, _| {
                        util::window_state::save(&state_paths, window.window_bounds());
                    })
                    .detach();
                });
                app.update(cx, |app, cx| app.check_for_updates(cx));
                cx.new(|cx| NasrinRoot::new(app, cx))
            },
        )
        .expect("failed to open Thasia");
    });
}
