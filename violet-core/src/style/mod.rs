pub mod colors;

use flax::{
    components::child_of, Entity, EntityBuilder, EntityRef, EntityRefMut, Exclusive, FetchExt,
    RelationExt,
};
use glam::{vec2, IVec2, Vec2};
use palette::{
    named::{BLACK, GRAY, GREEN, LIMEGREEN, ORANGE, RED, SLATEGRAY, WHITE},
    IntoColor, Oklab, Srgba, WithAlpha,
};

use crate::{
    components::{color, draw_shape, max_size, min_size, size},
    shape::shape_rectangle,
    unit::Unit,
    Edges, Scope,
};

use self::colors::{
    EERIE_BLACK_600, EERIE_BLACK_700, EERIE_BLACK_800, EERIE_BLACK_DEFAULT, JADE_400, JADE_600,
    JADE_DEFAULT, LION_DEFAULT, PLATINUM_DEFAULT, REDWOOD_DEFAULT,
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

/// Base properties for widget size
#[derive(Debug, Clone, Default)]
pub struct WidgetSize {
    pub size: Option<Unit<Vec2>>,
    pub min_size: Option<Unit<Vec2>>,
    pub max_size: Option<Unit<Vec2>>,
}

impl WidgetSize {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn mount(&self, scope: &mut Scope<'_>) {
        scope
            .set_opt(size(), self.size)
            .set_opt(min_size(), self.min_size)
            .set_opt(max_size(), self.max_size);
    }
}

/// A widget that allows you to set its sizing properties
pub trait SizeExt {
    /// Set the preferred size
    fn with_size(mut self, size: Unit<Vec2>) -> Self
    where
        Self: Sized,
    {
        self.size_mut().size = Some(size);
        self
    }

    /// Set the min size
    fn with_min_size(mut self, size: Unit<Vec2>) -> Self
    where
        Self: Sized,
    {
        self.size_mut().min_size = Some(size);
        self
    }

    /// Set the max size
    fn with_max_size(mut self, size: Unit<Vec2>) -> Self
    where
        Self: Sized,
    {
        self.size_mut().max_size = Some(size);
        self
    }

    fn size_mut(&mut self) -> &mut WidgetSize;
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

#[derive(Debug, Clone, Copy)]
pub struct SpacingConfig {
    /// The size of the default spacing unit
    pub base_scale: f32,
}

impl Default for SpacingConfig {
    fn default() -> Self {
        Self { base_scale: 4.0 }
    }
}

impl SpacingConfig {
    pub fn small<T: FromSize<usize>>(&self) -> T {
        T::from_spacing(self.base_scale, 1)
    }

    pub fn medium<T: FromSize<usize>>(&self) -> T {
        T::from_spacing(self.base_scale, 2)
    }

    pub fn large<T: FromSize<usize>>(&self) -> T {
        T::from_spacing(self.base_scale, 4)
    }

    pub fn size<T: FromSize<S>, S>(&self, size: S) -> T {
        T::from_spacing(self.base_scale, size)
    }
}

/// Converts a size to a pixel value
pub trait FromSize<S> {
    fn from_spacing(base_scale: f32, size: S) -> Self;
}

impl<T, S> FromSize<Unit<S>> for Unit<T>
where
    T: FromSize<S>,
{
    fn from_spacing(base_scale: f32, size: Unit<S>) -> Self {
        Unit::new(
            T::from_spacing(base_scale, size.px),
            T::from_spacing(base_scale, size.rel),
        )
    }
}

impl FromSize<IVec2> for Vec2 {
    fn from_spacing(base_scale: f32, size: IVec2) -> Self {
        vec2(base_scale * size.x as f32, base_scale * size.y as f32)
    }
}

impl FromSize<usize> for f32 {
    fn from_spacing(base_scale: f32, size: usize) -> Self {
        base_scale * size as f32
    }
}

impl FromSize<usize> for Edges {
    fn from_spacing(base_scale: f32, size: usize) -> Self {
        Edges::even(base_scale * size as f32)
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
        .set(spacing(), SpacingConfig { base_scale: 4.0 });

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

    pub spacing: SpacingConfig,

    /// Used for the main parts of interactive elements
    pub interactive_active: Srgba,
    pub interactive_inactive: Srgba,
    pub interactive_hover: Srgba,
    pub interactive_pressed: Srgba,
}
