use std::ops::Deref;

use flax::Component;
use glam::Vec2;
use image::DynamicImage;
use palette::{
    named::{BLACK, GREEN},
    Srgba, WithAlpha,
};
use winit::event::{ElementState, MouseButton};

use crate::{
    assets::AssetKey,
    components::{
        self, anchor, aspect_ratio, color, draw_shape, font_size, margin, min_size, offset, size,
        text, text_wrap, Edges,
    },
    input::{focusable, on_focus, on_mouse_input},
    shape,
    style::{
        accent_surface, get_stylesheet, secondary_surface, spacing, Background, StyleExt, Theme,
    },
    text::{TextSegment, Wrap},
    unit::Unit,
    Frame, Scope, Widget,
};

use super::{container::ContainerStyle, Stack};

/// A rectangular widget
#[derive(Debug, Clone)]
pub struct Rectangle {
    color: Srgba,
}

impl Rectangle {
    pub fn new(color: Srgba) -> Self {
        Self { color }
    }
}

impl Widget for Rectangle {
    fn mount(self, scope: &mut Scope) {
        scope
            .set(draw_shape(shape::shape_rectangle()), ())
            .set(color(), self.color);
    }
}

pub struct Image<K> {
    image: K,
}

impl<K> Image<K> {
    pub fn new(image: K) -> Self {
        Self { image }
    }
}

impl<K> Widget for Image<K>
where
    K: AssetKey<DynamicImage>,
{
    fn mount(self, scope: &mut Scope) {
        let image = scope.assets_mut().try_load(&self.image).ok();
        if let Some(image) = image {
            scope
                .set(color(), Srgba::new(1.0, 1.0, 1.0, 1.0))
                .set(draw_shape(shape::shape_rectangle()), ())
                .set(components::image(), image);
        } else {
            Text::new("Image not found")
                .with_color(Srgba::new(1.0, 0.0, 0.0, 1.0))
                .mount(scope);
        }
    }
}

pub struct Text {
    color: Option<Srgba>,
    text: Vec<TextSegment>,
    font_size: f32,
    wrap: Wrap,
}

impl Text {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: vec![TextSegment::new(text.into())],
            color: None,
            font_size: 16.0,
            wrap: Wrap::Word,
        }
    }

    pub fn rich(text: impl IntoIterator<Item = TextSegment>) -> Self {
        Self {
            text: text.into_iter().collect(),
            color: None,
            font_size: 16.0,
            wrap: Wrap::Word,
        }
    }

    /// Set the font_size
    pub fn with_font_size(mut self, font_size: f32) -> Self {
        self.font_size = font_size;
        self
    }

    /// Set the text color
    pub fn with_color(mut self, color: Srgba) -> Self {
        self.color = Some(color);
        self
    }

    pub fn with_wrap(mut self, wrap: Wrap) -> Self {
        self.wrap = wrap;
        self
    }
}

impl Widget for Text {
    fn mount(self, scope: &mut Scope) {
        scope
            .set(draw_shape(shape::shape_text()), ())
            .set(font_size(), self.font_size)
            .set(text_wrap(), self.wrap)
            .set(text(), self.text)
            .set_opt(color(), self.color);
    }
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

pub struct BoxSized<W> {
    size: Unit<Vec2>,
    min_size: Unit<Vec2>,
    aspect_ratio: f32,
    widget: W,
}

impl<W> BoxSized<W> {
    pub fn new(widget: W) -> Self {
        Self {
            size: Unit::ZERO,
            min_size: Unit::ZERO,
            widget,
            aspect_ratio: 0.0,
        }
    }

    pub fn with_size(mut self, size: Unit<Vec2>) -> Self {
        self.size = size;
        self
    }

    pub fn with_min_size(mut self, min_size: Unit<Vec2>) -> Self {
        self.min_size = min_size;
        self
    }

    /// Set the aspect ratio
    pub fn with_aspect_ratio(mut self, aspect_ratio: f32) -> Self {
        self.aspect_ratio = aspect_ratio;
        self
    }
}

impl<W: Widget> Widget for BoxSized<W> {
    fn mount(self, scope: &mut Scope<'_>) {
        self.widget.mount(scope);

        scope
            .set(size(), self.size)
            .set(min_size(), self.min_size)
            .set(aspect_ratio(), self.aspect_ratio);
    }
}
