pub mod base_colors;
pub mod color;
pub mod decoration;

use flax::{
    component::ComponentValue, components::child_of, Component, Entity, EntityBuilder, EntityRef,
    Exclusive, FetchExt, RelationExt,
};
use glam::Vec2;
use palette::{IntoColor, Oklab, Srgba};

pub use self::{color::*, decoration::*};
use crate::{
    components::{
        color, draw_shape, item_align, margin, max_size, maximize, min_size, padding, size,
        widget_corner_radius, LayoutAlignment,
    },
    icon::{Icon, IconSet},
    input::interactive,
    shape::shape_rectangle,
    text::FontFamily,
    unit::Unit,
    Edges, Scope,
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

/// Base properties for widget size and spacing
#[derive(Debug, Clone, Copy, Default)]
pub struct WidgetSizeProps {
    pub size: Option<Unit<Vec2>>,
    pub min_size: Option<Unit<Vec2>>,
    pub max_size: Option<Unit<Vec2>>,
    pub margin: Option<ValueOrRef<Edges>>,
    pub padding: Option<ValueOrRef<Edges>>,
    pub corner_radius: Option<ValueOrRef<Unit<f32>>>,
    pub maximize: Option<Vec2>,
    pub item_align: Option<LayoutAlignment>,
}

impl WidgetSizeProps {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn mount(&self, scope: &mut Scope<'_>) {
        let stylesheet = scope.stylesheet();

        let m = self.margin.map(|v| v.resolve(stylesheet));
        let p = self.padding.map(|v| v.resolve(stylesheet));
        let corner = self.corner_radius.map(|v| v.resolve(stylesheet));

        scope
            .set_opt(margin(), m)
            .set_opt(padding(), p)
            .set_opt(size(), self.size)
            .set_opt(widget_corner_radius(), corner)
            .set_opt(min_size(), self.min_size)
            .set_opt(max_size(), self.max_size)
            .set_opt(maximize(), self.maximize)
            .set_opt(item_align(), self.item_align);
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

    /// Set the corner radius
    pub fn with_corner_radius(mut self, corner_radius: impl Into<ValueOrRef<Unit<f32>>>) -> Self {
        self.corner_radius = Some(corner_radius.into());
        self
    }

    /// Maximize the widget to the available size with the given weight.
    pub fn with_maximize(mut self, maximize: Vec2) -> Self {
        self.maximize = Some(maximize);
        self
    }

    pub fn with_item_align(mut self, item_align: LayoutAlignment) -> Self {
        self.item_align = Some(item_align);
        self
    }
}

/// A widget that allows you to set its sizing properties
pub trait SizeExt {
    /// Override all the size properties of the widget
    fn with_size_props(mut self, size: WidgetSizeProps) -> Self
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

    fn with_exact_size(mut self, size: Unit<Vec2>) -> Self
    where
        Self: Sized,
    {
        self.size_mut().min_size = Some(size);
        self.size_mut().max_size = Some(size);
        self.size_mut().size = Some(size);
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

    /// Set the corner_radius
    fn with_corner_radius(mut self, corner_radius: impl Into<ValueOrRef<Unit<f32>>>) -> Self
    where
        Self: Sized,
    {
        self.size_mut().corner_radius = Some(corner_radius.into());
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

    /// Set the alignment of a single item for supported containers.
    fn with_item_align(mut self, item_align: LayoutAlignment) -> Self
    where
        Self: Sized,
    {
        self.size_mut().item_align = Some(item_align);
        self
    }

    fn size_mut(&mut self) -> &mut WidgetSizeProps;
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

impl<T: Clone + ComponentValue> ResolvableStyle for ValueOrRef<T> {
    type Value = T;
    fn resolve(&self, stylesheet: EntityRef<'_>) -> T {
        match self {
            ValueOrRef::Value(value) => value.clone(),
            ValueOrRef::Ref(component) => stylesheet.get_clone(*component).unwrap(),
        }
    }
}

pub trait ResolvableStyle {
    type Value;

    fn resolve(&self, stylesheet: EntityRef<'_>) -> Self::Value;
}

#[derive(Debug, Clone, Copy)]
pub struct Background {
    pub color: ValueOrRef<Srgba>,
    pub decoration: Decoration,
}

impl Background {
    pub fn new(color: impl Into<ValueOrRef<Srgba>>) -> Self {
        Self {
            color: color.into(),
            decoration: Decoration::default(),
        }
    }

    /// Draw a border of `width` pixels along the inside of the background's edge
    pub fn with_border(mut self, width: f32, color: impl Into<ValueOrRef<Srgba>>) -> Self {
        self.decoration = self.decoration.with_border(width, color);
        self
    }

    /// Cast a shadow around the background
    pub fn with_shadow(mut self, shadow: impl Into<ValueOrRef<BoxShadow>>) -> Self {
        self.decoration = self.decoration.with_shadow(shadow);
        self
    }

    /// Cast a shadow inside the background, sinking it into the surface
    pub fn with_inset_shadow(mut self, shadow: impl Into<ValueOrRef<BoxShadow>>) -> Self {
        self.decoration = self.decoration.with_inset_shadow(shadow);
        self
    }

    pub fn mount(self, scope: &mut Scope) {
        let c = self.color.resolve(scope.stylesheet());
        scope
            .set(draw_shape(shape_rectangle()), ())
            .set(color(), c)
            .set_default(interactive());
        self.decoration.mount(scope);
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

pub fn get_stylesheet_from_entity<'a>(entity: &EntityRef<'a>) -> EntityRef<'a> {
    let query = stylesheet.first_relation().traverse(child_of);

    let (id, _) = entity.query(&query).get().expect("No stylesheet found");

    entity.world().entity(id).unwrap()
}

/// Easily setup the default stylesheet
pub struct StylesheetOptions {
    pub icons: IconSet,
    pub colors: ColorSchemeConfig,
    pub base_spacing: f32,
    /// Size of body text; the other text sizes keep their ratio to it
    pub base_text_size: f32,
}

impl StylesheetOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_icons(mut self, icons: IconSet) -> Self {
        self.icons = icons;
        self
    }

    pub fn with_colors(mut self, colors: ColorSchemeConfig) -> Self {
        self.colors = colors;
        self
    }

    pub fn with_base_spacing(mut self, base_spacing: f32) -> Self {
        self.base_spacing = base_spacing;
        self
    }

    pub fn with_base_text_size(mut self, base_text_size: f32) -> Self {
        self.base_text_size = base_text_size;
        self
    }

    /// Build the stylesheet entity
    pub fn build(self) -> EntityBuilder {
        let mut builder = Entity::builder();

        ColorPalette::new(self.colors).install(&mut builder);

        builder
            // spacing
            .set(spacing_small(), Edges::even(self.base_spacing))
            .set(spacing_medium(), Edges::even(self.base_spacing * 2.0))
            .set(spacing_large(), Edges::even(self.base_spacing * 4.0))
            .set(default_separation(), Edges::even(self.base_spacing))
            .set(scrollbar_size(), self.base_spacing * 2.0)
            .set(
                slider_track_size(),
                Unit::px2(self.base_spacing * 64.0, self.base_spacing),
            )
            .set(
                slider_thumb_size(),
                Unit::px2(self.base_spacing * 3.0, self.base_spacing * 3.0),
            )
            .set(
                dropdown_size(),
                Unit::px2(self.base_spacing * 32.0, self.base_spacing * 8.0),
            )
            .set(default_corner_radius(), Unit::px(6.0))
            // text size, in the ratios of 12, 14, 18 and 24 pixels to the body's 14
            .set(text_xsmall(), self.base_text_size * (12.0 / 14.0))
            .set(text_small(), self.base_text_size)
            .set(text_medium(), self.base_text_size * (18.0 / 14.0))
            .set(text_large(), self.base_text_size * (24.0 / 14.0))
            .set(font_display(), FontFamily::named("Manrope"))
            // Raised controls and lifted planes cast one shadow each, standing in for the
            // sharp edge and soft cast shadow Verdalia stacks; sunken ones take an inset shadow.
            .set(
                shadow_raised(),
                BoxShadow::new(Vec2::new(0.0, 2.0), 4.0, shadow_color(0.5)),
            )
            .set(
                shadow_card(),
                BoxShadow::new(Vec2::new(0.0, 6.0), 8.0, shadow_color(0.5)),
            )
            .set(
                shadow_knob(),
                BoxShadow::new(Vec2::new(0.0, 1.0), 3.0, shadow_color(0.4)),
            )
            .set(
                inset_shadow_pressed(),
                BoxShadow::new(Vec2::new(0.0, 1.0), 2.0, shadow_color(0.3)),
            )
            .set(
                inset_shadow_well(),
                BoxShadow::new(Vec2::new(0.0, 1.0), 2.0, shadow_color(0.25)),
            );

        self.icons.install(&mut builder);

        builder
    }
}

impl Default for StylesheetOptions {
    fn default() -> Self {
        Self {
            icons: Default::default(),
            colors: ColorSchemeConfig::default(),
            base_spacing: 4.0,
            base_text_size: 14.0,
        }
    }
}

/// Colour of the shadows in the default stylesheet, at the given opacity
fn shadow_color(alpha: f32) -> Srgba {
    Srgba::new(14.0 / 255.0, 15.0 / 255.0, 18.0 / 255.0, alpha)
}

// Declares components attached to the currently active stylesheet entity.
//
// These declare dynamic (but type checked) properties that can be used to style widgets, similar
// to Figma variables.
flax::component! {
    pub stylesheet(id): () => [ Exclusive ],

    pub scrollbar_size: f32,

    pub spacing_small: Edges,
    pub spacing_medium: Edges,
    pub spacing_large: Edges,

    pub default_corner_radius: Unit<f32>,
    pub default_separation: Edges,

    pub slider_track_size: Unit<Vec2>,
    pub slider_thumb_size: Unit<Vec2>,
    pub dropdown_size: Unit<Vec2>,

    pub icon_chevron: Icon,
    pub icon_spinner: Icon,
    pub icon_warning: Icon,
    pub icon_error: Icon,
    pub icon_info: Icon,
    pub icon_check: Icon,
    pub icon_ellipsis: Icon,
    pub icon_search: Icon,

    pub text_xsmall: f32,
    pub text_small: f32,
    pub text_medium: f32,
    pub text_large: f32,

    /// Font family of titles and headings
    pub font_display: FontFamily,

    /// Shadow of a raised control, such as a button
    pub shadow_raised: BoxShadow,
    /// Shadow of a plane lifted above the page, such as a card or a popup
    pub shadow_card: BoxShadow,
    /// Shadow of a small handle, such as a slider's thumb
    pub shadow_knob: BoxShadow,
    /// Inset shadow of a control while pressed
    pub inset_shadow_pressed: BoxShadow,
    /// Inset shadow of a well carved into the surface, such as a text field or a slider's track
    pub inset_shadow_well: BoxShadow,
}
