#![allow(unused_braces)]

use gpui::{App, IntoElement, ParentElement, RenderOnce, SharedString, Window, prelude::*};
use nasrin::{Card, SectionLabel, SemanticStyleExt, view};

#[derive(IntoElement)]
pub struct StatCard {
    label: SharedString,
    value: SharedString,
}

impl StatCard {
    pub fn new(label: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }
}

impl RenderOnce for StatCard {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        Card::new().child(view! {
            div(vertical, gap_compact) {
                { SectionLabel::new(self.label) }
                div(text_heading, text_strong, text_primary) { { self.value } }
            }
        })
    }
}
