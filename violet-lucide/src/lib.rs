//! [Lucide](https://lucide.dev) icons for Violet.
//!
//! Install the pack, then draw icons with [`icon`](violet_core::widget::icon):
//! ```rust,ignore
//! AppBuilder::new()
//!     .with_icon_pack(violet_lucide::pack())
//!     .run(app())
//!
//! fn app() -> impl Widget {
//!     col(icon(LucideIcon::Code).with_color(OCEAN_400))
//! }
//! ```
//!
//! Regenerate `src/icons.rs` with `scripts/generate_icons.py lucide`.

use std::sync::Arc;

use violet_core::{
    icon::{IconPack, IconSet},
    text::Source,
};

mod icons;

pub use icons::{LucideIcon, FONT_FAMILY};

/// The icon font
pub fn font_source() -> Source {
    Source::Binary(Arc::new(include_bytes!("../bin/lucide/lucide.ttf")))
}

/// Lucide icons for the built-in widgets
pub fn icon_set() -> IconSet {
    IconSet {
        chevron: LucideIcon::ChevronRight.into(),
        spinner: LucideIcon::LoaderCircle.into(),
        warning: LucideIcon::TriangleAlert.into(),
        error: LucideIcon::CircleX.into(),
        info: LucideIcon::Info.into(),
        check: LucideIcon::Check.into(),
        ellipsis: LucideIcon::ChevronDown.into(),
        search: LucideIcon::Search.into(),
    }
}

/// The font and icon set, for [`AppBuilder::with_icon_pack`](https://docs.rs/violet-wgpu)
pub fn pack() -> IconPack {
    IconPack {
        font: font_source(),
        icons: icon_set(),
    }
}
