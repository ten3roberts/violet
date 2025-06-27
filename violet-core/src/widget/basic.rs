use cosmic_text::{Style, Weight};
use glam::Vec2;
use palette::Srgba;

use crate::{
    components::{self, color, draw_shape, text, text_wrap},
    shape,
    style::{
        element_primary, element_secondary, spacing_small, text_large, text_medium, text_small,
        ResolvableStyle, SizeExt, StyleExt, ValueOrRef, WidgetSizeProps,
    },
    text::{TextSegment, Wrap},
    unit::Unit,
    Scope, Widget,
};

use super::Stack;

/// A rectangular widget
#[derive(Debug, Clone)]
pub struct Rectangle {
    color: ValueOrRef<Srgba>,
    size: WidgetSizeProps,
    aspect_ratio: Option<f32>,
}

impl Rectangle {
    pub fn new(color: impl Into<ValueOrRef<Srgba>>) -> Self {
        Self {
            color: color.into(),
            size: Default::default(),
            aspect_ratio: None,
        }
    }

    pub fn with_aspect_ratio(mut self, aspect_ratio: f32) -> Self {
        self.aspect_ratio = Some(aspect_ratio);
        self
    }
}

impl Widget for Rectangle {
    fn mount(self, scope: &mut Scope) {
        self.size.mount(scope);

        let c = self.color.resolve(scope.stylesheet());

        scope
            .set(draw_shape(shape::shape_rectangle()), ())
            .set_opt(components::aspect_ratio(), self.aspect_ratio)
            .set(color(), c);
    }
}

impl SizeExt for Rectangle {
    fn size_mut(&mut self) -> &mut WidgetSizeProps {
        &mut self.size
    }
}

/// Style and decorate text
#[derive(Clone, Debug)]
pub struct TextStyle {
    pub font_size: ValueOrRef<f32>,
    pub wrap: Wrap,
    pub color: ValueOrRef<Srgba>,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_size: text_small().into(),
            wrap: Wrap::None,
            color: element_primary().into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Text {
    text: Vec<TextSegment>,
    style: TextStyle,
    size: WidgetSizeProps,
}

impl Text {
    pub fn new(text: impl Into<String>) -> Self {
        Self::formatted([TextSegment::new(text.into())])
    }

    pub fn extra_bold(text: impl Into<String>) -> Self {
        Self::formatted([TextSegment::new(text.into()).with_weight(Weight::EXTRA_BOLD)])
    }

    pub fn bold(text: impl Into<String>) -> Self {
        Self::formatted([TextSegment::new(text.into()).with_weight(Weight::BOLD)])
    }

    pub fn medium(text: impl Into<String>) -> Self {
        Self::formatted([TextSegment::new(text.into()).with_weight(Weight::MEDIUM)])
    }

    pub fn light(text: impl Into<String>) -> Self {
        Self::formatted([TextSegment::new(text.into()).with_weight(Weight::LIGHT)])
    }

    pub fn extra_light(text: impl Into<String>) -> Self {
        Self::formatted([TextSegment::new(text.into()).with_weight(Weight::EXTRA_LIGHT)])
    }

    pub fn italic(text: impl Into<String>) -> Self {
        Self::formatted([TextSegment::new(text.into()).with_style(Style::Italic)])
    }

    pub fn formatted(text: impl IntoIterator<Item = TextSegment>) -> Self {
        Self {
            text: text.into_iter().collect(),
            style: TextStyle::default(),
            size: WidgetSizeProps {
                margin: Some(spacing_small().into()),
                ..Default::default()
            },
        }
    }

    /// Set the font_size
    pub fn with_font_size(mut self, font_size: impl Into<ValueOrRef<f32>>) -> Self {
        self.style.font_size = font_size.into();
        self
    }

    /// Set the text color
    pub fn with_color(mut self, color: impl Into<ValueOrRef<Srgba>>) -> Self {
        self.style.color = color.into();
        self
    }

    pub fn with_wrap(mut self, wrap: Wrap) -> Self {
        self.style.wrap = wrap;
        self
    }
}

impl StyleExt for Text {
    type Style = TextStyle;

    fn with_style(mut self, style: Self::Style) -> Self {
        self.style = style;
        self
    }
}

impl SizeExt for Text {
    fn size_mut(&mut self) -> &mut WidgetSizeProps {
        &mut self.size
    }
}

impl Widget for Text {
    fn mount(self, scope: &mut Scope) {
        self.size.mount(scope);

        let stylesheet = scope.stylesheet();
        let font_size = self.style.font_size.resolve(stylesheet);

        let font_color = self.style.color.resolve(stylesheet);
        scope
            .set(draw_shape(shape::shape_text()), ())
            .set(components::font_size(), font_size)
            .set(text_wrap(), self.style.wrap)
            .set(text(), self.text)
            .set(color(), font_color);
    }
}

/// A text with a margin
pub fn label(text: impl Into<String>) -> Text {
    Text::new(text).with_margin(spacing_small())
}

pub fn header(text: impl Into<String>) -> Stack<Text> {
    Stack::new(Text::new(text).with_margin(spacing_small())).with_padding(spacing_small())
}

/// A text with a margin
pub fn title(text: impl Into<String>) -> Text {
    Text::bold(text)
        .with_font_size(text_large())
        .with_margin(spacing_small())
}

pub fn subtitle(text: impl Into<String>) -> Text {
    Text::medium(text)
        .with_color(element_secondary())
        .with_font_size(text_medium())
        .with_margin(spacing_small())
}

pub fn bold(text: impl Into<String>) -> Text {
    Text::bold(text)
        .with_color(element_secondary())
        .with_margin(spacing_small())
}

/// Allows a widget to be manually positioned and offset
pub struct Positioned<W> {
    offset: Unit<Vec2>,
    anchor: Unit<Vec2>,
    widget: W,
}

impl<W> Positioned<W> {
    pub fn new(widget: W) -> Self {
        Self {
            offset: Unit::ZERO,
            anchor: Unit::ZERO,
            widget,
        }
    }

    /// Sets the anchor point of the widget
    pub fn with_anchor(mut self, anchor: Unit<Vec2>) -> Self {
        self.anchor = anchor;
        self
    }

    /// Offsets the widget relative to its original position
    pub fn with_offset(mut self, offset: Unit<Vec2>) -> Self {
        self.offset = offset;
        self
    }
}

impl<W> Widget for Positioned<W>
where
    W: Widget,
{
    fn mount(self, scope: &mut Scope<'_>) {
        self.widget.mount(scope);

        scope.set(components::anchor(), self.anchor);
        scope.set(components::offset(), self.offset);
    }
}
