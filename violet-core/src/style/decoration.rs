use flax::EntityRef;
use glam::Vec2;
use palette::Srgba;

use super::{ResolvableStyle, ValueOrRef};
use crate::{
    components::{widget_border, widget_inset_shadow, widget_shadow},
    Scope,
};

/// A line of uniform width drawn along the inside of a shape's edge
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Border {
    pub width: f32,
    pub color: Srgba,
}

impl Border {
    /// A border that draws nothing
    pub const NONE: Self = Self {
        width: 0.0,
        color: Srgba::new(0.0, 0.0, 0.0, 0.0),
    };

    pub fn new(width: f32, color: Srgba) -> Self {
        Self { width, color }
    }
}

/// A blurred copy of a shape's outline, as CSS's `box-shadow`.
///
/// As an outer shadow ([`widget_shadow`]) it is drawn around the shape and hidden under it; as an
/// inset shadow ([`widget_inset_shadow`]) it is drawn inside the shape, darkening its edges as
/// if the shape were sunk into the surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoxShadow {
    /// Offset of the shadow from the shape, in pixels
    pub offset: Vec2,
    /// Blur radius in pixels; the shadow fades over this distance, as in CSS
    pub blur: f32,
    /// Grows (or with a negative value shrinks) the shadow's outline before blurring
    pub spread: f32,
    pub color: Srgba,
}

impl BoxShadow {
    /// A shadow that draws nothing
    pub const NONE: Self = Self {
        offset: Vec2::ZERO,
        blur: 0.0,
        spread: 0.0,
        color: Srgba::new(0.0, 0.0, 0.0, 0.0),
    };

    pub fn new(offset: Vec2, blur: f32, color: Srgba) -> Self {
        Self {
            offset,
            blur,
            spread: 0.0,
            color,
        }
    }

    pub fn with_spread(mut self, spread: f32) -> Self {
        self.spread = spread;
        self
    }
}

/// Border and shadows of a shape, resolved against the stylesheet when mounted
#[derive(Debug, Clone, Copy, Default)]
pub struct Decoration {
    pub border: Option<(f32, ValueOrRef<Srgba>)>,
    pub shadow: Option<ValueOrRef<BoxShadow>>,
    pub inset_shadow: Option<ValueOrRef<BoxShadow>>,
}

impl Decoration {
    pub fn with_border(mut self, width: f32, color: impl Into<ValueOrRef<Srgba>>) -> Self {
        self.border = Some((width, color.into()));
        self
    }

    pub fn with_shadow(mut self, shadow: impl Into<ValueOrRef<BoxShadow>>) -> Self {
        self.shadow = Some(shadow.into());
        self
    }

    pub fn with_inset_shadow(mut self, shadow: impl Into<ValueOrRef<BoxShadow>>) -> Self {
        self.inset_shadow = Some(shadow.into());
        self
    }

    pub fn mount(&self, scope: &mut Scope<'_>) {
        let stylesheet = scope.stylesheet();
        let border = self
            .border
            .map(|(width, color)| Border::new(width, color.resolve(stylesheet)));
        let shadow = self.shadow.map(|v| v.resolve(stylesheet));
        let inset_shadow = self.inset_shadow.map(|v| v.resolve(stylesheet));

        scope
            .set_opt(widget_border(), border)
            .set_opt(widget_shadow(), shadow)
            .set_opt(widget_inset_shadow(), inset_shadow);
    }
}

impl ResolvableStyle for Decoration {
    type Value = ResolvedDecoration;

    fn resolve(&self, stylesheet: EntityRef<'_>) -> ResolvedDecoration {
        ResolvedDecoration {
            border: self
                .border
                .map(|(width, color)| Border::new(width, color.resolve(stylesheet)))
                .unwrap_or(Border::NONE),
            shadow: self
                .shadow
                .map(|v| v.resolve(stylesheet))
                .unwrap_or(BoxShadow::NONE),
            inset_shadow: self
                .inset_shadow
                .map(|v| v.resolve(stylesheet))
                .unwrap_or(BoxShadow::NONE),
        }
    }
}

/// A [`Decoration`] resolved against the stylesheet, its absent parts drawing nothing.
///
/// For widgets that change their decoration with their state, such as a button when pressed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedDecoration {
    pub border: Border,
    pub shadow: BoxShadow,
    pub inset_shadow: BoxShadow,
}

impl ResolvedDecoration {
    /// Set the decoration of the widget being mounted
    pub fn mount(&self, scope: &mut Scope<'_>) {
        scope
            .set(widget_border(), self.border)
            .set(widget_shadow(), self.shadow)
            .set(widget_inset_shadow(), self.inset_shadow);
    }

    /// Replace the decoration of a widget mounted with [`Self::mount`]
    pub fn update(&self, entity: &EntityRef<'_>) {
        entity.update_dedup(widget_border(), self.border);
        entity.update_dedup(widget_shadow(), self.shadow);
        entity.update_dedup(widget_inset_shadow(), self.inset_shadow);
    }
}
