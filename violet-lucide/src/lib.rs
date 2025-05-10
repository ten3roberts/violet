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

use violet_core::text::Source;

/// All icon glyphs.
pub mod icons;

/// Icons font source
pub fn font_source() -> Source {
    Source::Binary(Arc::new(include_bytes!("../bin/lucide/lucide.ttf")))
}
