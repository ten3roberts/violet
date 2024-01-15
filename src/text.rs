pub use cosmic_text::{Style, Weight};
use palette::Srgba;
use std::{borrow::Cow, fmt::Display};

#[derive(Debug, Clone)]
// Inspired by: https://github.com/pop-os/cosmic-text
pub enum FontFamily {
    Named(Cow<'static, str>),

    /// Serif fonts represent the formal text style for a script.
    Serif,

    /// Glyphs in sans-serif fonts, as the term is used in CSS, are generally low contrast
    /// and have stroke endings that are plain — without any flaring, cross stroke,
    /// or other ornamentation.
    SansSerif,

    /// Glyphs in cursive fonts generally use a more informal script style,
    /// and the result looks more like handwritten pen or brush writing than printed letterwork.
    Cursive,

    /// Fantasy fonts are primarily decorative or expressive fonts that
    /// contain decorative or expressive representations of characters.
    Fantasy,

    /// The sole criterion of a monospace font is that all glyphs have the same fixed width.
    Monospace,
}

impl Display for FontFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FontFamily::Named(name) => write!(f, "{name}"),
            FontFamily::Serif => write!(f, "serif"),
            FontFamily::SansSerif => write!(f, "sans-serif"),
            FontFamily::Cursive => write!(f, "cursive"),
            FontFamily::Fantasy => write!(f, "fantasy"),
            FontFamily::Monospace => write!(f, "monospace"),
        }
    }
}

impl FontFamily {
    pub fn named(name: impl Into<Cow<'static, str>>) -> Self {
        Self::Named(name.into())
    }
}

impl<T> From<T> for FontFamily
where
    T: Into<Cow<'static, str>>,
{
    fn from(value: T) -> Self {
        Self::named(value)
    }
}

/// A segment of rich text
pub struct TextSegment {
    pub text: String,
    pub family: FontFamily,
    pub style: Style,
    pub weight: Weight,
    pub color: Srgba,
}

impl TextSegment {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            family: FontFamily::Serif,
            style: Style::Normal,
            weight: Weight::NORMAL,
            color: Srgba::new(1.0, 1.0, 1.0, 1.0),
        }
    }

    pub fn with_family(mut self, family: impl Into<FontFamily>) -> Self {
        self.family = family.into();
        self
    }

    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn with_weight(mut self, weight: Weight) -> Self {
        self.weight = weight;
        self
    }

    pub fn with_color(mut self, color: Srgba) -> Self {
        self.color = color;
        self
    }
}
