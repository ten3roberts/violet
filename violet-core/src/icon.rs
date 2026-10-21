//! Icons rendered from icon fonts.
//!
//! An [`Icon`] is a glyph together with the font family that draws it. Icon packs (such as
//! `violet-lucide` or `violet-phosphor`) provide an enum of their icons which converts into an
//! [`Icon`], plus an [`IconPack`] which bundles the font with an [`IconSet`] for the built-in
//! widgets.
//!
//! Prefer [`icon`](crate::widget::icon) over putting a glyph into a [`label`](crate::widget::label):
//! icon fonts share private-use codepoints, so a glyph without its family may render from
//! whichever icon font happens to be loaded.

use std::borrow::Cow;

use cosmic_text::fontdb::Source;
use flax::EntityBuilder;

use crate::{
    style::{
        icon_check, icon_chevron, icon_ellipsis, icon_error, icon_info, icon_search, icon_spinner,
        icon_warning,
    },
    text::{FontFamily, TextSegment},
};

/// A single glyph drawn with a specific font family
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Icon {
    pub family: FontFamily,
    pub glyph: Cow<'static, str>,
}

impl Icon {
    /// An icon drawn from the named font family
    pub const fn new(family: &'static str, glyph: &'static str) -> Self {
        Self {
            family: FontFamily::Named(Cow::Borrowed(family)),
            glyph: Cow::Borrowed(glyph),
        }
    }

    /// An icon drawn as plain text using the default font
    pub fn text(glyph: impl Into<Cow<'static, str>>) -> Self {
        Self {
            family: FontFamily::SansSerif,
            glyph: glyph.into(),
        }
    }

    /// A text segment drawing this icon, for mixing icons and text in the same run
    pub fn segment(&self) -> TextSegment {
        TextSegment::new(self.glyph.clone()).with_family(self.family.clone())
    }
}

/// Error returned when parsing an icon name that the icon pack does not contain
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownIcon {
    pub name: String,
}

impl UnknownIcon {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl std::fmt::Display for UnknownIcon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown icon {:?}", self.name)
    }
}

impl std::error::Error for UnknownIcon {}

/// Glyphs used by the built-in widgets, such as chevrons, spinners and status icons
#[derive(Debug, Clone)]
pub struct IconSet {
    pub chevron: Icon,
    pub spinner: Icon,
    pub warning: Icon,
    pub error: Icon,
    pub info: Icon,
    pub check: Icon,
    pub ellipsis: Icon,
    pub search: Icon,
}

impl IconSet {
    /// Writes the icons into a stylesheet, overriding any previously set
    pub fn install(self, stylesheet: &mut EntityBuilder) {
        stylesheet
            .set(icon_chevron(), self.chevron)
            .set(icon_spinner(), self.spinner)
            .set(icon_warning(), self.warning)
            .set(icon_error(), self.error)
            .set(icon_info(), self.info)
            .set(icon_check(), self.check)
            .set(icon_ellipsis(), self.ellipsis)
            .set(icon_search(), self.search);
    }

    /// Iterates over every icon in the set
    pub fn iter(&self) -> impl Iterator<Item = &Icon> {
        let Self {
            chevron,
            spinner,
            warning,
            error,
            info,
            check,
            ellipsis,
            search,
        } = self;

        [
            chevron, spinner, warning, error, info, check, ellipsis, search,
        ]
        .into_iter()
    }
}

impl Default for IconSet {
    /// Plain-text glyphs, usable without any icon font
    fn default() -> Self {
        Self {
            chevron: Icon::text(">"),
            spinner: Icon::text("⟳"),
            warning: Icon::text("!"),
            error: Icon::text("x"),
            info: Icon::text("i"),
            check: Icon::text("✓"),
            ellipsis: Icon::text("⋯"),
            search: Icon::text(">"),
        }
    }
}

/// An icon font together with the icon set the built-in widgets should use from it
pub struct IconPack {
    pub font: Source,
    pub icons: IconSet,
}
