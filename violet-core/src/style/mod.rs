pub mod colors;

use flax::{
    component::ComponentValue, components::child_of, Component, Entity, EntityBuilder, EntityRef,
    Exclusive, FetchExt, RelationExt,
};
use glam::Vec2;
use palette::{IntoColor, Oklab, Srgba};

use crate::{
    components::{color, draw_shape, margin, max_size, maximize, min_size, padding, size},
    shape::shape_rectangle,
    unit::Unit,
    Edges, Scope,
};

use self::colors::*;

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

/// Base properties for widget size and spacing
#[derive(Debug, Clone, Default)]
pub struct WidgetSize {
    pub size: Option<Unit<Vec2>>,
    pub min_size: Option<Unit<Vec2>>,
    pub max_size: Option<Unit<Vec2>>,
    pub margin: Option<ValueOrRef<Edges>>,
    pub padding: Option<ValueOrRef<Edges>>,
    pub maximize: Option<Vec2>,
}

impl WidgetSize {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn mount(&self, scope: &mut Scope<'_>) {
        let stylesheet = scope.stylesheet();

        let m = self.margin.map(|v| v.resolve(stylesheet));
        let p = self.padding.map(|v| v.resolve(stylesheet));

        scope
            .set_opt(margin(), m)
            .set_opt(padding(), p)
            .set_opt(size(), self.size)
            .set_opt(min_size(), self.min_size)
            .set_opt(max_size(), self.max_size)
            .set_opt(maximize(), self.maximize);
    }

    /// Set the size
    pub fn with_size(mut self, size: Unit<Vec2>) -> Self {
        self.size = Some(size);
        self
    }

    /// Set the min size
    pub fn with_min_size(mut self, size: Unit<Vec2>) -> Self {
        self.min_size = Some(size);
        self
    }

    /// Set the max size
    pub fn with_max_size(mut self, size: Unit<Vec2>) -> Self {
        self.max_size = Some(size);
        self
    }

    /// Set the margin
    pub fn with_margin(mut self, margin: impl Into<ValueOrRef<Edges>>) -> Self {
        self.margin = Some(margin.into());
        self
    }

    /// Set the padding around inner content.
    pub fn with_padding(mut self, padding: impl Into<ValueOrRef<Edges>>) -> Self {
        self.padding = Some(padding.into());
        self
    }

    /// Maximize the widget to the available size with the given weight.
    pub fn with_maximize(mut self, maximize: Vec2) -> Self {
        self.maximize = Some(maximize);
        self
    }
}

/// A widget that allows you to set its sizing properties
pub trait SizeExt {
    /// Override all the size properties of the widget
    fn with_size_props(mut self, size: WidgetSize) -> Self
    where
        Self: Sized,
    {
        *self.size_mut() = size;
        self
    }

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

    /// Set the margin
    fn with_margin(mut self, margin: impl Into<ValueOrRef<Edges>>) -> Self
    where
        Self: Sized,
    {
        self.size_mut().margin = Some(margin.into());
        self
    }

    /// Set the padding around inner content.
    ///
    /// **NOTE**: Padding has no effect on widgets without children. Padding strictly affect
    /// the distance between the widget and the contained children. Notable examples include lists
    /// and stacks. This is merely added for consistency and not adding **too** many different
    /// traits to implement :P
    fn with_padding(mut self, padding: impl Into<ValueOrRef<Edges>>) -> Self
    where
        Self: Sized,
    {
        self.size_mut().padding = Some(padding.into());
        self
    }

    fn with_maximize(mut self, maximize: Vec2) -> Self
    where
        Self: Sized,
    {
        self.size_mut().maximize = Some(maximize);
        self
    }

    fn size_mut(&mut self) -> &mut WidgetSize;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueOrRef<T> {
    Value(T),
    Ref(Component<T>),
}

impl<T> ValueOrRef<T> {
    pub fn value(value: T) -> Self {
        Self::Value(value)
    }

    pub fn ref_(component: Component<T>) -> Self {
        Self::Ref(component)
    }
}

impl<T: Default> Default for ValueOrRef<T> {
    fn default() -> Self {
        Self::Value(Default::default())
    }
}

impl<T> From<Component<T>> for ValueOrRef<T> {
    fn from(v: Component<T>) -> Self {
        Self::Ref(v)
    }
}

impl<T> From<T> for ValueOrRef<T> {
    fn from(v: T) -> Self {
        Self::Value(v)
    }
}

impl<T: Copy + ComponentValue> ValueOrRef<T> {
    pub(crate) fn resolve(self, stylesheet: EntityRef<'_>) -> T {
        match self {
            ValueOrRef::Value(value) => value,
            ValueOrRef::Ref(component) => stylesheet.get_copy(component).unwrap(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Background {
    pub color: ValueOrRef<Srgba>,
}

impl Background {
    pub fn new(color: impl Into<ValueOrRef<Srgba>>) -> Self {
        Self {
            color: color.into(),
        }
    }

    pub fn mount(self, scope: &mut Scope) {
        let c = self.color.resolve(scope.stylesheet());
        scope.set(draw_shape(shape_rectangle()), ()).set(color(), c);
    }
}

impl From<Component<Srgba>> for Background {
    fn from(v: Component<Srgba>) -> Self {
        Self::new(v)
    }
}

impl From<Srgba> for Background {
    fn from(v: Srgba) -> Self {
        Self::new(v)
    }
}

pub enum Spacing {
    Small,
    Medium,
    Large,
}

pub fn get_stylesheet_from_entity<'a>(entity: &EntityRef<'a>) -> EntityRef<'a> {
    let query = stylesheet.first_relation().traverse(child_of);

    let (id, _) = entity.query(&query).get().expect("No stylesheet found");

    entity.world().entity(id).unwrap()
}

pub fn setup_stylesheet() -> EntityBuilder {
    let mut builder = Entity::builder();

    builder
        // colors
        .set(primary_surface(), STONE_950)
        .set(primary_element(), PLATINUM_100)
        .set(secondary_surface(), STONE_900)
        .set(accent_surface(), PLATINUM_800)
        .set(accent_element(), EMERALD_500)
        .set(success_surface(), EMERALD_800)
        .set(success_element(), EMERALD_500)
        .set(warning_surface(), AMBER_800)
        .set(warning_element(), AMBER_500)
        .set(danger_surface(), REDWOOD_800)
        .set(danger_element(), REDWOOD_500)
        .set(interactive_active(), EMERALD_500)
        .set(interactive_passive(), ZINC_800)
        .set(interactive_hover(), EMERALD_400)
        .set(interactive_pressed(), EMERALD_500)
        .set(interactive_inactive(), ZINC_700)
        // spacing
        .set(spacing_small(), 4.0.into())
        .set(spacing_medium(), 8.0.into())
        .set(spacing_large(), 16.0.into());

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
    pub secondary_item: Srgba,

    pub accent_surface: Srgba,
    pub accent_element: Srgba,

    pub success_surface: Srgba,
    pub success_element: Srgba,

    pub warning_surface: Srgba,
    pub warning_element: Srgba,

    pub danger_surface: Srgba,
    pub danger_element: Srgba,


    /// Used for the main parts of interactive elements
    pub interactive_active: Srgba,
    pub interactive_passive: Srgba,
    pub interactive_inactive: Srgba,
    pub interactive_hover: Srgba,
    pub interactive_pressed: Srgba,

    pub spacing_small: Edges,
    pub spacing_medium: Edges,
    pub spacing_large: Edges,
}
