use nasrin::{
    Flavour, FlavourColors, FlavourMetrics, FlavourRegistry, FlavourTypography,
    gpui::{App, BorrowAppContext, rgb, rgba},
};

pub const LUMI_FLAVOUR: &str = "Lumi";

pub fn register(cx: &mut App) {
    cx.update_global::<FlavourRegistry, _>(|registry, _| {
        registry.register(lumi_light());
        registry.register(lumi_dark());
    });
}

fn lumi_light() -> Flavour {
    Flavour {
        name: LUMI_FLAVOUR,
        is_dark: false,
        colors: FlavourColors {
            background: rgb(0xf4f2f0).into(),
            surface: rgb(0xffffff).into(),
            panel: rgb(0xfaf9f8).into(),
            border: rgb(0xd9d3cf).into(),
            text_primary: rgb(0x161413).into(),
            text_muted: rgb(0x6e6763).into(),
            accent_primary: rgb(0xb85c00).into(),
            accent_strong: rgb(0x8f4700).into(),
            accent_text: rgb(0xffffff).into(),
            destructive_base: rgb(0xc23b32).into(),
            destructive_text: rgb(0x9d251e).into(),
            success_base: rgb(0x2e8b57).into(),
            success_text: rgb(0x206d42).into(),
            warning_base: rgb(0xd78500).into(),
            warning_text: rgb(0x995f00).into(),
            overlay: rgba(0x1614137a).into(),
        },
        metrics: FlavourMetrics::default(),
        typography: FlavourTypography::default(),
    }
}

fn lumi_dark() -> Flavour {
    Flavour {
        name: LUMI_FLAVOUR,
        is_dark: true,
        colors: FlavourColors {
            background: rgb(0x0e0c0b).into(),
            surface: rgb(0x1e1a18).into(),
            panel: rgb(0x161311).into(),
            border: rgb(0x3b3531).into(),
            text_primary: rgb(0xf5f2f0).into(),
            text_muted: rgb(0xa69f9b).into(),
            accent_primary: rgb(0xf29d24).into(),
            accent_strong: rgb(0xf6c06a).into(),
            accent_text: rgb(0x1a1005).into(),
            destructive_base: rgb(0xe4655d).into(),
            destructive_text: rgb(0xf08a83).into(),
            success_base: rgb(0x54b77c).into(),
            success_text: rgb(0x7bd29e).into(),
            warning_base: rgb(0xf1a52f).into(),
            warning_text: rgb(0xf7c56c).into(),
            overlay: rgba(0x000000ad).into(),
        },
        metrics: FlavourMetrics::default(),
        typography: FlavourTypography::default(),
    }
}
