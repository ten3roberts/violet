pub mod colors;

use core::prelude;
use std::{
    borrow::BorrowMut,
    ops::{Deref, DerefMut},
};

use atomic_refcell::{AtomicRef, AtomicRefCell};
use flax::{
    component::ComponentValue, components::child_of, fetch::entity_refs, filter::Filtered,
    query::QueryOne, system::SystemData, Component, Entity, EntityBuilder, EntityRef, Exclusive,
    FetchExt, RelationExt,
};
use glam::{IVec2, Vec2};
use image::codecs::png;
use once_cell::sync::Lazy;
use palette::{
    chromatic_adaptation::TransformMatrix,
    named::{
        BLACK, DARKSLATEGRAY, GRAY, GREEN, LIMEGREEN, ORANGE, RED, SLATEGRAY, TEAL, WHITE,
        WHITESMOKE,
    },
    num::Clamp,
    IntoColor, Oklab, Srgb, Srgba, WithAlpha,
};

use crate::{
    components::{color, draw_shape},
    declare_atom,
    shape::shape_rectangle,
    unit::Unit,
    Frame, Scope,
};

use self::colors::{
    DARK_CYAN_DEFAULT, EERIE_BLACK_200, EERIE_BLACK_400, EERIE_BLACK_600, EERIE_BLACK_700,
    EERIE_BLACK_DEFAULT, JADE_400, JADE_500, JADE_600, JADE_DEFAULT, LION_DEFAULT,
    PLATINUM_DEFAULT, REDWOOD_DEFAULT,
};

#[macro_export]
/// Create a color from a hex string
macro_rules! srgba {
    ($color:literal) => {{
        let [r, g, b] = color_hex::color_from_hex!($color);

        Srgba::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0)
    }};
}

pub fn tint(base_color: Srgba, tint: usize) -> Srgba {
    let f = (tint as f32 / 1000.0) * 2.0;
    let mut color: Oklab = base_color.into_color();

    let base_luminance = color.l;
    let target_luminance = base_luminance * f;
    color.l = target_luminance.clamp(0.0, 1.0);

    color.into_color()
}

/// Allows overriding a style for a widget
pub trait StyleExt {
    /// Stylesheet used to style the widget
    type Style;

    /// Set the style
    fn with_style(self, style: Self::Style) -> Self;
}

pub trait WidgetStyle {
    type Resolved;

    fn resolve_style(&self, stylesheet: &Theme) -> Self::Resolved;
}

#[derive(Debug, Clone, Copy)]
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

/// Universal set of properties to derive default widget styles from
#[derive(Debug, Clone, Copy, Default)]
pub struct Theme {
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

    pub fn resolve_unit(&self, unit: Unit<IVec2>) -> Unit<Vec2> {
        Unit {
            px: Vec2::new(unit.px.x as f32, unit.px.y as f32) * self.base_scale,
            rel: Vec2::new(unit.rel.x as f32, unit.rel.y as f32) * self.base_scale,
        }
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

pub fn get_stylesheet<'a>(scope: &'a Scope<'_>) -> EntityRef<'a> {
    let query = stylesheet.first_relation().traverse(child_of);

    let (id, _) = scope
        .entity()
        .query(&query)
        .get()
        .expect("No stylesheet found");

    scope.frame().world.entity(id).unwrap()
}

pub fn setup_stylesheet() -> EntityBuilder {
    let mut builder = Entity::builder();

    builder
        .set(primary_surface(), EERIE_BLACK_DEFAULT)
        .set(primary_element(), PLATINUM_DEFAULT)
        .set(secondary_surface(), EERIE_BLACK_600)
        .set(accent_surface(), EERIE_BLACK_DEFAULT)
        .set(accent_element(), JADE_DEFAULT)
        .set(success_surface(), EERIE_BLACK_DEFAULT)
        .set(success_element(), JADE_DEFAULT)
        .set(warning_surface(), EERIE_BLACK_DEFAULT)
        .set(warning_element(), LION_DEFAULT)
        .set(error_surface(), EERIE_BLACK_DEFAULT)
        .set(error_element(), REDWOOD_DEFAULT)
        .set(interactive_active(), JADE_DEFAULT)
        .set(interactive_hover(), JADE_600)
        .set(interactive_pressed(), JADE_400)
        .set(interactive_inactive(), EERIE_BLACK_700)
        .set(spacing(), Spacing { base_scale: 4.0 });

    builder
}

// Declares components attached to the currently active stylesheet entity.
//
// These declare dynamic (but type checked) properties that can be used to style widgets, similar
// to Figma variables.
flax::component! {
    pub stylesheet(id): () => [ Exclusive ],
    /// The primary surface color
    pub primary_surface: Srgba,
    pub primary_element: Srgba,

    /// Used for secondary surfaces, such as card backgrounds
    pub secondary_surface: Srgba,
    pub secondary_element: Srgba,

    pub accent_surface: Srgba,
    pub accent_element: Srgba,

    pub success_surface: Srgba,
    pub success_element: Srgba,

    pub warning_surface: Srgba,
    pub warning_element: Srgba,

    pub error_surface: Srgba,
    pub error_element: Srgba,

    pub spacing: Spacing,

    /// Used for the main parts of interactive elements
    pub interactive_active: Srgba,
    pub interactive_hover: Srgba,
    pub interactive_pressed: Srgba,
    pub interactive_inactive: Srgba,

}
