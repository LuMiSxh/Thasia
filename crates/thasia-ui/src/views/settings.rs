#![allow(unused_braces)]

use crate::{
    components::{PageHeader, StatCard},
    updater,
    util::{
        paths::AppPaths,
        runtime,
        settings::{self, AppSettings, ThemePreference},
    },
};
use gpui::{Context, EventEmitter, IntoElement, ParentElement, Render, Window, prelude::*};
use nasrin::{
    Badge, Button, ButtonVariant, Card, FeedbackVariant, FieldRow, FlavourContextExt, Panel,
    SemanticStyleExt, ThemeMode, Toggle, view,
};

#[derive(Clone, Debug)]
pub enum SettingsEvent {
    Changed(AppSettings),
}

pub struct SettingsView {
    paths: AppPaths,
    settings: AppSettings,
    update_available: Option<String>,
    update_message: Option<String>,
    update_busy: bool,
}

impl SettingsView {
    pub fn new(paths: AppPaths, settings: AppSettings) -> Self {
        Self {
            paths,
            settings,
            update_available: None,
            update_message: None,
            update_busy: false,
        }
    }

    fn persist(&mut self, cx: &mut Context<Self>) {
        let _ = settings::save_app(&self.paths, &self.settings);
        cx.emit(SettingsEvent::Changed(self.settings.clone()));
        cx.notify();
    }

    fn set_theme(&mut self, preference: ThemePreference, cx: &mut Context<Self>) {
        self.settings.theme = preference;
        cx.set_theme_mode(match preference {
            ThemePreference::Light => ThemeMode::Light,
            ThemePreference::Dark => ThemeMode::Dark,
            ThemePreference::System => ThemeMode::System,
        });
        self.persist(cx);
    }

    pub fn check_for_updates(&mut self, cx: &mut Context<Self>) {
        if self.update_busy {
            return;
        }
        self.update_busy = true;
        self.update_message = Some("Checking for updates...".into());
        cx.spawn(async move |this, cx| {
            let result = runtime::app(async {
                tokio::task::spawn_blocking(updater::latest_version)
                    .await
                    .map_err(|error| crate::error::AppError::Message(error.to_string()))?
                    .map_err(|error| crate::error::AppError::Message(error.to_string()))
            })
            .await;
            let _ = this.update(cx, |view, cx| {
                view.update_busy = false;
                match result {
                    Ok(Some(version)) => {
                        view.update_available = Some(version.clone());
                        view.update_message = Some(format!("Thasia {version} is available."));
                    }
                    Ok(None) => {
                        view.update_available = None;
                        view.update_message = Some("Thasia is up to date.".into());
                    }
                    Err(error) => view.update_message = Some(error.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn install_update(&mut self, cx: &mut Context<Self>) {
        if self.update_busy {
            return;
        }
        self.update_busy = true;
        self.update_message = Some("Downloading and installing update...".into());
        cx.spawn(async move |this, cx| {
            let result = runtime::app(async {
                tokio::task::spawn_blocking(updater::install_latest)
                    .await
                    .map_err(|error| crate::error::AppError::Message(error.to_string()))?
                    .map_err(|error| crate::error::AppError::Message(error.to_string()))
            })
            .await;
            let _ = this.update(cx, |view, cx| {
                view.update_busy = false;
                view.update_message = Some(match result {
                    Ok(version) => {
                        format!("Updated to {version}. Restart Thasia to apply the update.")
                    }
                    Err(error) => error.to_string(),
                });
                cx.notify();
            });
        })
        .detach();
    }
}

impl EventEmitter<SettingsEvent> for SettingsView {}

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let light = cx.entity();
        let dark = cx.entity();
        let system = cx.entity();
        let hints = cx.entity();
        let check = cx.entity();
        let install = cx.entity();
        let data = self.paths.data_dir.clone();
        let logs = self.paths.logs.clone();
        let config = self.paths.config_dir.clone();
        let cache = self.paths.cache_dir.clone();

        let theme_actions = view! {
            div(horizontal, gap_compact) {
                { Button::new("Light").variant(ButtonVariant::Ghost).on_click(move |_, _, cx| {
                    light.update(cx, |view, cx| view.set_theme(ThemePreference::Light, cx));
                }) }
                { Button::new("Dark").variant(ButtonVariant::Ghost).on_click(move |_, _, cx| {
                    dark.update(cx, |view, cx| view.set_theme(ThemePreference::Dark, cx));
                }) }
                { Button::new("System").variant(ButtonVariant::Ghost).on_click(move |_, _, cx| {
                    system.update(cx, |view, cx| view.set_theme(ThemePreference::System, cx));
                }) }
            }
        };

        let paths = Panel::new()
            .eyebrow("APPLICATION DATA")
            .title("Paths")
            .child(FieldRow::new(
                "Data",
                Button::new("Open").on_click(move |_, _, _| {
                    let _ = open::that(&data);
                }),
            ))
            .child(FieldRow::new(
                "Logs",
                Button::new("Open").on_click(move |_, _, _| {
                    let _ = open::that(&logs);
                }),
            ))
            .child(FieldRow::new(
                "Config",
                Button::new("Open").on_click(move |_, _, _| {
                    let _ = open::that(&config);
                }),
            ))
            .child(FieldRow::new(
                "Cache",
                Button::new("Open").on_click(move |_, _, _| {
                    let _ = open::that(&cache);
                }),
            ));

        let update_message = self
            .update_message
            .clone()
            .unwrap_or_else(|| "Check GitHub Releases for a newer version.".into());
        let update_button = if self.update_available.is_some() {
            Button::new("Update now")
                .disabled(self.update_busy)
                .on_click(move |_, _, cx| {
                    install.update(cx, |view, cx| view.install_update(cx))
                })
                .into_any_element()
        } else {
            view! { div {} }.into_any_element()
        };
        let updates = Panel::new()
            .eyebrow("UPDATES")
            .title("Application")
            .child(StatCard::new("Current version", env!("CARGO_PKG_VERSION")))
            .child(view! { div(text_muted) { { update_message } } })
            .child(view! {
                div(horizontal, gap_dense) {
                    { Button::new(if self.update_busy { "Working..." } else { "Check for updates" })
                        .disabled(self.update_busy)
                        .on_click(move |_, _, cx| check.update(cx, |view, cx| view.check_for_updates(cx))) }
                    { update_button }
                }
            });

        view! {
            div(fill, vertical, gap_standard, padding_standard) {
                { PageHeader::new("Settings", "Configure conversion defaults and the interface.")
                    .badge("Saved", FeedbackVariant::Success)
                    .actions(theme_actions) }
                div(horizontal, gap_standard) {
                    { Panel::new()
                        .eyebrow("INTERFACE")
                        .title("Appearance")
                        .child(FieldRow::new("Theme", Badge::new(format!("{:?}", self.settings.theme))))
                        .child(FieldRow::new(
                            "Keyboard hints",
                            Toggle::new(self.settings.show_key_hints).on_change(move |checked, _, cx| {
                                hints.update(cx, |view, cx| {
                                    view.settings.show_key_hints = checked;
                                    view.persist(cx);
                                });
                            })
                        ))
                        .child(Card::new().child(view! {
                            div(text_muted) { "The Lumi flavour follows the selected light, dark, or system appearance." }
                        })) }
                    { Panel::new()
                        .eyebrow("DESTINATION")
                        .title("Conversion output")
                        .child(StatCard::new(
                            "Default folder",
                            self.settings.default_output_dir
                                .as_ref()
                                .map(|path| path.display().to_string())
                                .unwrap_or_else(|| "Choose per conversion".into())
                        )) }
                }
                { paths }
                { updates }
            }
        }
    }
}
