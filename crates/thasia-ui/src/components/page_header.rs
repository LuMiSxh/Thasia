#![allow(unused_braces)]

use gpui::{
    AnyElement, App, IntoElement, ParentElement, RenderOnce, SharedString, Window, prelude::*,
};
use nasrin::{Badge, FeedbackVariant, Panel, SemanticStyleExt, view};

#[derive(IntoElement)]
pub struct PageHeader {
    title: SharedString,
    description: SharedString,
    badge: Option<(SharedString, FeedbackVariant)>,
    actions: Option<AnyElement>,
}

impl PageHeader {
    pub fn new(title: impl Into<SharedString>, description: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            description: description.into(),
            badge: None,
            actions: None,
        }
    }

    pub fn badge(mut self, label: impl Into<SharedString>, variant: FeedbackVariant) -> Self {
        self.badge = Some((label.into(), variant));
        self
    }

    pub fn actions(mut self, actions: impl IntoElement) -> Self {
        self.actions = Some(actions.into_any_element());
        self
    }
}

impl RenderOnce for PageHeader {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let badge = self
            .badge
            .map(|(label, variant)| Badge::new(label).variant(variant).into_any_element())
            .unwrap_or_else(|| view! { div {} }.into_any_element());
        Panel::new()
            .title(self.title)
            .actions(self.actions.unwrap_or_else(|| view! { div {} }.into_any_element()))
            .child(view! {
                div(horizontal, align_center, spread) {
                    div(text_muted) { { self.description } }
                    { badge }
                }
            })
    }
}
