#![allow(unused_braces)]

use crate::{app::Page, util::paths::AppPaths};
use gpui::{
    Context, EventEmitter, IntoElement, ObjectFit, Render, StyledImage, Window, img, prelude::*,
    rems,
};
use nasrin::{
    Badge, Button, ButtonVariant, Card, FeedbackVariant, FlavourContextExt, GradientDirection,
    GradientStyleExt, Icon, IconName, SemanticStyleExt, view,
};
use thasia_source::suwayomi::RuntimeState;

#[derive(Clone, Copy, Debug)]
pub enum HomeEvent {
    Navigate(Page),
}

pub struct HomeView {
    paths: AppPaths,
    runtime: RuntimeState,
}

impl HomeView {
    pub fn new(paths: AppPaths) -> Self {
        Self {
            paths,
            runtime: RuntimeState::NotRunning,
        }
    }

    pub fn set_runtime(&mut self, runtime: RuntimeState, cx: &mut Context<Self>) {
        self.runtime = runtime;
        cx.notify();
    }
}

impl EventEmitter<HomeEvent> for HomeView {}

impl Render for HomeView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.flavour().colors.clone();
        let this = cx.entity();
        let convert = this.clone();
        let discover = this.clone();
        let settings = this.clone();
        let convert_card = Card::new()
            .interactive(true)
            .on_click(move |_, _, cx| {
                convert.update(cx, |_, cx| cx.emit(HomeEvent::Navigate(Page::Convert)))
            })
            .child(
                gpui::div()
                    .h(rems(3.25))
                    .horizontal()
                    .align_center()
                    .gap_standard()
                    .linear_gradient(
                        GradientDirection::ToRight,
                        colors.accent_primary.opacity(0.24),
                        colors.accent_primary.opacity(0.0),
                    )
                    .child(view! {
                        div(horizontal, align_center, gap_standard, w_full) {
                            { Icon::new(IconName::Layers).size(rems(1.3)).color(colors.accent_primary) }
                            div(flex_1, vertical) {
                                div(text_body, text_strong) { "Convert local manga" }
                                div(text_size = rems(0.72), text_muted) { "Folder, ZIP, or CBZ into reader-ready output" }
                            }
                            { Icon::new(IconName::ChevronRight).size(rems(1.0)).color(colors.accent_primary) }
                        }
                    }),
            );
        let runtime_label = match &self.runtime {
            RuntimeState::NotInstalled => "Setup needed".to_string(),
            RuntimeState::NotRunning => "Stopped".to_string(),
            RuntimeState::Starting => "Starting".to_string(),
            RuntimeState::Ready { port } => format!("Ready:{port}"),
            RuntimeState::Error { .. } => "Error".to_string(),
        };

        view! {
            div(fill, relative, overflow_hidden) {
                div(absolute, right_0, top_0, bottom_0, w = rems(34.0), min_w = rems(1.0)) {
                    { img(self.paths.assets.join("pfp.png"))
                        .w(rems(34.0))
                        .h(rems(42.0))
                        .object_fit(ObjectFit::Cover)
                        .opacity(0.9) }
                }
                div(relative, h_full, w = rems(35.0), vertical, justify_center, gap = rems(2.0), px = rems(3.5)) {
                    div(vertical, gap_dense) {
                        div(horizontal, gap_dense) {
                            { Badge::new(format!("v{}", env!("CARGO_PKG_VERSION"))) }
                            { Badge::new(runtime_label).variant(FeedbackVariant::Default) }
                        }
                        div(text_size = rems(4.5), line_height = rems(4.5), text_strong, text_accent) { "Thasia" }
                        div(text_size = rems(0.7), text_muted) { "MANGA PROCESSING ENGINE" }
                    }
                    div(vertical, gap_dense) {
                        { convert_card }
                        div(horizontal, gap_dense) {
                            { Button::new("Discover")
                                .icon(IconName::Search)
                                .variant(ButtonVariant::Secondary)
                                .on_click(move |_, _, cx| discover.update(cx, |_, cx| cx.emit(HomeEvent::Navigate(Page::Discover)))) }
                            { Button::new("Settings")
                                .icon(IconName::Settings)
                                .variant(ButtonVariant::Secondary)
                                .on_click(move |_, _, cx| settings.update(cx, |_, cx| cx.emit(HomeEvent::Navigate(Page::Settings)))) }
                        }
                    }
                    div(text_size = rems(0.75), text_muted) { "Or press Cmd/Ctrl + 2 to jump straight in" }
                }
            }
        }
    }
}
