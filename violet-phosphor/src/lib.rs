//! [Phosphor](https://phosphoricons.com) icons (regular weight) for Violet.
//!
//! Install the pack, then draw icons with [`icon`](violet_core::widget::icon):
//! ```rust,ignore
//! AppBuilder::new()
//!     .with_icon_pack(violet_phosphor::pack())
//!     .run(app())
//!
//! fn app() -> impl Widget {
//!     col(icon(PhosphorIcon::Code).with_color(OCEAN_400))
//! }
//! ```
//!
//! The font is `@phosphor-icons/web` 2.1.2 (MIT, see `bin/LICENSE`). Regenerate `src/icons.rs`
//! with `scripts/generate_icons.py phosphor`.

use std::sync::Arc;

use violet_core::{
    icon::{IconPack, IconSet},
    text::Source,
};

mod icons;

pub use icons::{PhosphorIcon, FONT_FAMILY};

/// The icon font
pub fn font_source() -> Source {
    Source::Binary(Arc::new(include_bytes!("../bin/Phosphor.ttf")))
}

/// Phosphor icons for the built-in widgets
pub fn icon_set() -> IconSet {
    IconSet {
        chevron: PhosphorIcon::CaretRight.into(),
        spinner: PhosphorIcon::Spinner.into(),
        warning: PhosphorIcon::Warning.into(),
        error: PhosphorIcon::XCircle.into(),
        info: PhosphorIcon::Info.into(),
        check: PhosphorIcon::Check.into(),
        ellipsis: PhosphorIcon::CaretDown.into(),
        search: PhosphorIcon::MagnifyingGlass.into(),
    }
}

/// The font and icon set, for `AppBuilder::with_icon_pack`
pub fn pack() -> IconPack {
    IconPack {
        font: font_source(),
        icons: icon_set(),
    }
}
