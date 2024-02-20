use std::{
    borrow::BorrowMut,
    ops::{Deref, DerefMut},
};

use atomic_refcell::{AtomicRef, AtomicRefCell};
use flax::{component::ComponentValue, system::SystemData, Component};
use glam::Vec2;
use image::codecs::png;
use once_cell::sync::Lazy;
use palette::{
    chromatic_adaptation::TransformMatrix,
    named::{
        BLACK, DARKSLATEGRAY, GRAY, GREEN, LIMEGREEN, ORANGE, RED, SLATEGRAY, TEAL, WHITE,
        WHITESMOKE,
    },
    Srgb, Srgba, WithAlpha,
};

use crate::{
    components::{color, draw_shape},
    declare_atom,
    shape::shape_rectangle,
    Frame, Scope,
};

/// Allows a widget to be styled
pub trait StyleExt {
    /// Stylesheet used to style the widget
    type Style;

    /// Set the style
    fn with_style(self, style: Self::Style) -> Self;
}

pub trait WidgetStyle {
    type Resolved;

    fn resolve_style(&self, stylesheet: &StyleSheet) -> Self::Resolved;
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

/// Universal style sheet for widgets to derive from
#[derive(Debug, Clone, Copy, Default)]
pub struct StyleSheet {
    /// Default unit size used for spacing, margins, padding, etc.
    pub spacing: Spacing,
    pub colors: SemanticColors,
}

#[derive(Debug, Clone, Copy)]
pub struct Spacing {
    /// The size of the default spacing unit
    pub base_scale: f32,
}

impl Default for Spacing {
    fn default() -> Self {
        Self { base_scale: 4.0 }
    }
}

impl Spacing {
    pub fn size(&self, size: usize) -> f32 {
        self.base_scale * size as f32
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SemanticColors {
    pub primary_element: Srgba,
    pub secondary_element: Srgba,
    pub accent_element: Srgba,
    pub success_element: Srgba,
    pub warning_element: Srgba,
    pub error_element: Srgba,

    pub primary_surface: Srgba,
    pub secondary_surface: Srgba,
    pub accent_surface: Srgba,
    pub success_surface: Srgba,
    pub warning_surface: Srgba,
    pub error_surface: Srgba,
}

impl Default for SemanticColors {
    fn default() -> Self {
        SemanticColors {
            primary_element: WHITE.with_alpha(1.0).into_format(),
            secondary_element: GRAY.with_alpha(1.0).into_format(),
            accent_element: LIMEGREEN.with_alpha(1.0).into_format(),

            success_element: GREEN.with_alpha(1.0).into_format(),
            warning_element: ORANGE.with_alpha(1.0).into_format(),
            error_element: RED.with_alpha(1.0).into_format(),

            primary_surface: BLACK.with_alpha(1.0).into_format(),
            secondary_surface: SLATEGRAY.with_alpha(1.0).into_format(),
            accent_surface: SLATEGRAY.with_alpha(1.0).into_format(),
            success_surface: SLATEGRAY.with_alpha(1.0).into_format(),
            warning_surface: SLATEGRAY.with_alpha(1.0).into_format(),
            error_surface: SLATEGRAY.with_alpha(1.0).into_format(),
        }
    }
}

static DEFAULT_STYLE_REF: Lazy<AtomicRefCell<StyleSheet>> =
    Lazy::new(|| AtomicRefCell::new(StyleSheet::default()));

pub fn get_style(frame: &Frame) -> AtomicRef<StyleSheet> {
    frame
        .get_atom(stylesheet())
        .unwrap_or_else(|| DEFAULT_STYLE_REF.borrow())
}

declare_atom! {
    /// Describes the general style of the application
    stylesheet: StyleSheet,
}
