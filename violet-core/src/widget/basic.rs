use glam::Vec2;
use image::DynamicImage;
use palette::Srgba;

use crate::{
    assets::AssetKey,
    components::{self, color, draw_shape, font_size, text, text_wrap},
    shape,
    style::{colors::REDWOOD_DEFAULT, spacing_small, SizeExt, StyleExt, ValueOrRef, WidgetSize},
    text::{TextSegment, Wrap},
    unit::Unit,
    Scope, Widget,
};

/// A rectangular widget
#[derive(Debug, Clone)]
pub struct Rectangle {
    color: ValueOrRef<Srgba>,
    size: WidgetSize,
}

impl Rectangle {
    pub fn new(color: impl Into<ValueOrRef<Srgba>>) -> Self {
        Self {
            color: color.into(),
            size: Default::default(),
        }
    }
}

impl Widget for Rectangle {
    fn mount(self, scope: &mut Scope) {
        self.size.mount(scope);

        let c = self.color.resolve(scope.stylesheet());

        scope
            .set(draw_shape(shape::shape_rectangle()), ())
            .set(color(), c);
    }
}

impl SizeExt for Rectangle {
    fn size_mut(&mut self) -> &mut WidgetSize {
        &mut self.size
    }
}

pub struct Image<K> {
    image: K,
    size: WidgetSize,
    aspect_ratio: Option<f32>,
}

impl<K> Image<K> {
    pub fn new(image: K) -> Self {
        Self {
            image,
            size: Default::default(),
            aspect_ratio: None,
        }
    }

    pub fn with_aspect_ratio(mut self, aspect_ratio: f32) -> Self {
        self.aspect_ratio = Some(aspect_ratio);
        self
    }
}

impl<K> Widget for Image<K>
where
    K: AssetKey<DynamicImage>,
{
    fn mount(self, scope: &mut Scope) {
        let image = scope.assets_mut().try_load(&self.image).ok();
        if let Some(image) = image {
            self.size.mount(scope);
            scope
                .set(color(), Srgba::new(1.0, 1.0, 1.0, 1.0))
                .set(draw_shape(shape::shape_rectangle()), ())
                .set(components::image(), image)
                .set_opt(components::aspect_ratio(), self.aspect_ratio);
        } else {
            label("Image not found")
                .with_color(REDWOOD_DEFAULT)
                .mount(scope);
        }
    }
}

impl<K> SizeExt for Image<K> {
    fn size_mut(&mut self) -> &mut WidgetSize {
        &mut self.size
    }
}

/// Style and decorate text
pub struct TextStyle {
    font_size: f32,
    wrap: Wrap,
    color: Option<Srgba>,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_size: 16.0,
            wrap: Wrap::Word,
            color: None,
        }
    }
}

pub struct Text {
    text: Vec<TextSegment>,
    style: TextStyle,
    size: WidgetSize,
}

impl Text {
    pub fn new(text: impl Into<String>) -> Self {
        Self::rich([TextSegment::new(text.into())])
    }

    pub fn rich(text: impl IntoIterator<Item = TextSegment>) -> Self {
        Self {
            text: text.into_iter().collect(),
            style: TextStyle::default(),
            size: Default::default(),
        }
    }

    /// Set the font_size
    pub fn with_font_size(mut self, font_size: f32) -> Self {
        self.style.font_size = font_size;
        self
    }

    /// Set the text color
    pub fn with_color(mut self, color: Srgba) -> Self {
        self.style.color = Some(color);
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
    fn size_mut(&mut self) -> &mut WidgetSize {
        &mut self.size
    }
}

impl Widget for Text {
    fn mount(self, scope: &mut Scope) {
        self.size.mount(scope);

        scope
            .set(draw_shape(shape::shape_text()), ())
            .set(font_size(), self.style.font_size)
            .set(text_wrap(), self.style.wrap)
            .set(text(), self.text)
            .set_opt(color(), self.style.color);
    }
}

/// A text with a margin
pub fn label(text: impl Into<String>) -> Text {
    Text::new(text).with_margin(spacing_small())
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
