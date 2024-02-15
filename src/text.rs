pub use cosmic_text::{Style, Weight, Wrap};
use glam::{vec2, Vec2};
use palette::Srgba;
use std::{
    borrow::{Borrow, Cow},
    fmt::Display,
    ops::Index,
    sync::Arc,
};

use crate::components::Rect;

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
#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, Copy)]
pub struct LayoutGlyph {
    /// Index within a row.
    ///
    /// As a row may be broken into multiple lines, the vector index may no be the same as the
    /// glyphs index withing the row.
    pub index: usize,
    pub start: usize,
    pub end: usize,
    pub bounds: Rect,
}

#[derive(Debug, Clone)]
pub struct LayoutLineGlyphs {
    pub row: usize,
    pub bounds: Rect,
    pub start: usize,
    pub end: usize,
    pub glyphs: Vec<LayoutGlyph>,
}

#[derive(Debug, Clone)]
pub struct LayoutGlyphs {
    lines: Arc<[LayoutLineGlyphs]>,
    line_height: f32,
}

impl Default for LayoutGlyphs {
    fn default() -> Self {
        Self {
            lines: Vec::new().into(),
            line_height: 0.0,
        }
    }
}

impl LayoutGlyphs {
    pub fn new(lines: Vec<LayoutLineGlyphs>, line_height: f32) -> Self {
        Self {
            lines: lines.into(),
            line_height,
        }
    }

    pub fn hit(&self, pos: Vec2) -> Option<CursorLocation> {
        self.lines
            .iter()
            .enumerate()
            .find(|&(ln, _)| {
                let h = ln as f32 * self.line_height;
                pos.y >= h && pos.y <= h + self.line_height
            })
            .map(|(_, line)| {
                if let Some(glyph) = line
                    .glyphs
                    .iter()
                    .find(|v| pos.x >= v.bounds.min.x && pos.x <= v.bounds.max.x)
                {
                    if pos.x > glyph.bounds.min.x + glyph.bounds.size().x / 2.0 {
                        CursorLocation::new(line.row, glyph.end)
                    } else {
                        CursorLocation::new(line.row, glyph.start)
                    }
                } else if pos.x > line.bounds.max.x {
                    // place eol
                    CursorLocation::new(line.row, line.end)
                } else {
                    CursorLocation::new(line.row, line.start)
                }
            })
    }

    /// Returns the line and glyph index for the given cursor location
    pub fn to_glyph_boundary(&self, cursor: CursorLocation) -> Option<Vec2> {
        for (ln, line) in self.find_lines_indices(cursor.row) {
            if line.row == cursor.row {
                for glyph in &line.glyphs {
                    if glyph.start == cursor.col {
                        return Some(vec2(glyph.bounds.min.x, ln as f32 * self.line_height));
                    }
                }

                // Account for end-of-run whitespace which are not present as glyphs in the final
                // layout.
                if let (Some(last_glyph), Some(next_line)) =
                    (line.glyphs.last(), self.lines.get(ln + 1))
                {
                    if next_line
                        .glyphs
                        .first()
                        .is_some_and(|v| v.start == cursor.col + 1)
                    {
                        return Some(vec2(last_glyph.bounds.max.x, ln as f32 * self.line_height));
                    }
                }
            }
        }

        None
    }

    pub fn find_lines(&self, row: usize) -> impl Iterator<Item = &LayoutLineGlyphs> {
        self.lines
            .iter()
            .skip_while(move |v| v.row < row)
            .take_while(move |v| v.row == row)
    }

    pub fn find_lines_indices(
        &self,
        row: usize,
    ) -> impl Iterator<Item = (usize, &LayoutLineGlyphs)> {
        self.lines
            .iter()
            .enumerate()
            .skip_while(move |(_, v)| v.row < row)
            .take_while(move |(_, v)| v.row == row)
    }

    pub fn line_height(&self) -> f32 {
        self.line_height
    }

    pub fn lines(&self) -> &[LayoutLineGlyphs] {
        self.lines.as_ref()
    }
}

impl Index<usize> for LayoutLineGlyphs {
    type Output = LayoutGlyph;

    fn index(&self, index: usize) -> &Self::Output {
        &self.glyphs[index]
    }
}

impl Index<usize> for LayoutGlyphs {
    type Output = LayoutLineGlyphs;

    fn index(&self, index: usize) -> &Self::Output {
        &self.lines[index]
    }
}

impl Index<LayoutCursorLocation> for LayoutGlyphs {
    type Output = LayoutGlyph;

    fn index(&self, index: LayoutCursorLocation) -> &Self::Output {
        &self.lines[index.line_index][index.index]
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorLocation {
    /// The row index of the non-wrapped original text
    pub row: usize,
    /// Byte offset to the column in the row
    pub col: usize,
}

impl CursorLocation {
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutCursorLocation {
    /// Wrapped line index
    pub line_index: usize,
    /// Glyph index within the line
    pub index: usize,
}

impl LayoutCursorLocation {
    pub fn new(line_index: usize, index: usize) -> Self {
        Self { line_index, index }
    }
}

impl<'a> From<&'a FontFamily> for cosmic_text::Family<'a> {
    fn from(value: &'a FontFamily) -> Self {
        match value {
            FontFamily::Named(name) => cosmic_text::Family::Name(name.borrow()),
            FontFamily::Serif => cosmic_text::Family::Serif,
            FontFamily::SansSerif => cosmic_text::Family::SansSerif,
            FontFamily::Cursive => cosmic_text::Family::Cursive,
            FontFamily::Fantasy => cosmic_text::Family::Fantasy,
            FontFamily::Monospace => cosmic_text::Family::Monospace,
        }
    }
}
