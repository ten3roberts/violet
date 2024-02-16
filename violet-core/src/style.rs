use std::ops::{Deref, DerefMut};

use flax::{component::ComponentValue, Component};
use glam::Vec2;
use image::codecs::png;
use palette::{Srgb, Srgba};

use crate::{
    components::{self, color, draw_shape, Edges},
    shape::shape_rectangle,
    unit::Unit,
    Scope, Widget,
};

/// Allows a widget to be styled
pub trait StyleExt {
    /// Stylesheet used to style the widget
    type Style;

    /// Set the style
    fn with_style(self, style: Self::Style) -> Self;
}

#[derive(Debug, Clone)]
pub struct Background {
    pub color: Srgba,
}

impl Background {
    pub fn new(color: Srgba) -> Self {
        Self { color }
    }

    pub fn mount(self, scope: &mut Scope) {
        scope
            .set(draw_shape(shape_rectangle()), ())
            .set(color(), self.color);
    }
}
