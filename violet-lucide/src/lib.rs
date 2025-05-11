//! Provides lucide icons for Violet.
//!
//! To use, add the provided font to the app and use the icon in any formatted text. It even works
//! with color.
//! ```rust,ignore
//! AppBuilder::new()
//!     .with_font(Source::Binary(Arc::new(include_bytes!(
//!         "../violet-lucide/bin/lucide/lucide.ttf"
//!     ))))
//!     .with_renderer_config(MainRendererConfig { debug_mode: false })
//!     .run(app())
//!
//! fn app() -> impl Widget {
//!     col(label(LUCIDE_CODE).with_color(OCEAN_400))
//! }
//! ```

use std::sync::Arc;

use icons::LUCIDE_CHEVRON_RIGHT;
use violet_core::{style::IconSet, text::Source};

/// All icon glyphs.
pub mod icons;

/// Icons font source
pub fn font_source() -> Source {
    Source::Binary(Arc::new(include_bytes!("../bin/lucide/lucide.ttf")))
}

/// Returns an icon set for styling
pub fn icon_set() -> IconSet {
    IconSet {
        chevron: LUCIDE_CHEVRON_RIGHT.into(),
    }
}
