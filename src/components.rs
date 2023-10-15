use flax::{component, Debuggable, Entity};
use glam::{vec2, Vec2};
use palette::Srgba;

use crate::{layout::Flow, shapes::FilledRect, unit::Unit};

component! {
    pub is_widget: () => [ Debuggable ],
    /// Ordered list of children for an entity
    pub children: Vec<Entity> => [ Debuggable ],
    // pub child_of(parent): Entity => [ Debuggable ],

    /// Defines the outer bounds of a widget relative to its position
    pub rect: Rect => [ Debuggable ],

    /// Position relative to parent
    pub local_position: Vec2 => [ Debuggable ],

    /// Specifies in screen space where the widget rect upper left corner is
    pub screen_position: Vec2 => [ Debuggable ],

    /// Offset the widget from its original position
    pub offset: Unit<Vec2> => [ Debuggable ],

    /// The preferred size of the widget.
    ///
    /// The final bounds of a widget may be smaller to fit within a layout
    pub size: Unit<Vec2> => [ Debuggable ],

    /// The minimum allowed size of a widget. A widgets bound will not be made any smaller even if
    /// that implies clipping.
    pub min_size: Unit<Vec2> => [ Debuggable ],

    /// Heuristic text size of a text widget
    pub intrinsic_size: Vec2 => [ Debuggable ],

    /// Sets the anchor point withing the bounds of the widget where position is applied
    pub anchor: Unit<Vec2> => [ Debuggable ],

    /// Manages the layout of the children in a flowing list
    pub flow: Flow => [ Debuggable ],

    /// Spacing between a outer and inner bounds
    pub padding: Edges => [ Debuggable ],
    /// Spacing between the item outer bounds and another items outer bounds
    ///
    /// Margins will be merged
    ///
    /// A margin is in essence a minimum allowed distance to another items bounds
    pub margin: Edges => [ Debuggable ],

    pub text: String => [ ],
    pub font_size: f32 => [ Debuggable ],

    /// The color of the widget
    pub color: Srgba => [ Debuggable ],

    /// The widget will be rendered as a filled rectange coverings its bounds
    pub filled_rect: FilledRect => [ Debuggable ],
}

/// Spacing between a outer and inner bounds
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Edges {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl std::ops::Sub for Edges {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            left: self.left - rhs.left,
            right: self.right - rhs.right,
            top: self.top - rhs.top,
            bottom: self.bottom - rhs.bottom,
        }
    }
}

impl std::ops::SubAssign for Edges {
    fn sub_assign(&mut self, rhs: Self) {
        self.left -= rhs.left;
        self.right -= rhs.right;
        self.top -= rhs.top;
        self.bottom -= rhs.bottom;
    }
}

impl std::ops::Add for Edges {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            left: self.left + rhs.left,
            right: self.right + rhs.right,
            top: self.top + rhs.top,
            bottom: self.bottom + rhs.bottom,
        }
    }
}

impl std::ops::Mul<f32> for Edges {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            left: self.left * rhs,
            right: self.right * rhs,
            top: self.top * rhs,
            bottom: self.bottom * rhs,
        }
    }
}

impl Edges {
    pub const ZERO: Self = Self {
        left: 0.0,
        right: 0.0,
        top: 0.0,
        bottom: 0.0,
    };

    pub const fn new(left: f32, right: f32, top: f32, bottom: f32) -> Self {
        Self {
            left,
            right,
            top,
            bottom,
        }
    }

    pub const fn even(distance: f32) -> Self {
        Self {
            left: distance,
            right: distance,
            top: distance,
            bottom: distance,
        }
    }

    pub(crate) fn size(&self) -> Vec2 {
        vec2(self.left + self.right, self.top + self.bottom)
    }

    pub(crate) fn in_axis(&self, axis: Vec2) -> (f32, f32) {
        let pos = vec2(self.right, self.top).dot(axis);
        let neg = vec2(-self.left, -self.bottom).dot(axis);

        let front_margin = pos.max(neg);
        let back_margin = -pos.min(neg);

        (front_margin, back_margin)
    }

    pub(crate) fn max(&self, other: Self) -> Self {
        Self {
            left: self.left.max(other.left),
            right: self.right.max(other.right),
            top: self.top.max(other.top),
            bottom: self.bottom.max(other.bottom),
        }
    }

    pub(crate) fn min(&self, other: Self) -> Self {
        Self {
            left: self.left.min(other.left),
            right: self.right.min(other.right),
            top: self.top.min(other.top),
            bottom: self.bottom.min(other.bottom),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
/// Defines the bounds of a widget
pub struct Rect {
    pub min: Vec2,
    pub max: Vec2,
}

impl Rect {
    pub const ZERO: Self = Self {
        min: Vec2::ZERO,
        max: Vec2::ZERO,
    };

    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    #[must_use]
    pub fn from_two_points(a: Vec2, b: Vec2) -> Self {
        Self {
            min: a.min(b),
            max: a.max(b),
        }
    }

    #[must_use]
    pub fn from_size_pos(size: Vec2, pos: Vec2) -> Self {
        Self {
            min: pos,
            max: pos + size,
        }
    }

    #[must_use]
    pub fn align_to_grid(&self) -> Self {
        Self {
            min: self.min.floor(),
            max: self.max.floor(),
        }
    }

    #[inline]
    pub fn size(&self) -> Vec2 {
        self.max - self.min
    }

    #[inline]
    pub fn pos(&self) -> Vec2 {
        self.min
    }

    /// Makes the rect smaller by the given padding
    #[must_use]
    pub fn inset(&self, padding: &Edges) -> Self {
        Self {
            min: self.min + vec2(padding.left, padding.top),
            max: self.max - vec2(padding.right, padding.bottom),
        }
    }

    /// Makes the rect larger by the given padding
    #[must_use]
    pub fn pad(&self, padding: &Edges) -> Self {
        Self {
            min: self.min - vec2(padding.left, padding.top),
            max: self.max + vec2(padding.right, padding.bottom),
        }
    }

    pub(crate) fn support(&self, axis: Vec2) -> f32 {
        let x = (self.min.x * -axis.x).max(0.0) + (self.max.x * axis.x).max(0.0);
        let y = (self.min.y * -axis.y).max(0.0) + (self.max.y * axis.y).max(0.0);

        vec2(x, y).dot(axis)
    }

    #[must_use]
    pub(crate) fn clamp_size(&self, min: Vec2, max: Vec2) -> Self {
        let size = self.size().clamp(min, max);
        Self {
            min: self.min,
            max: self.min + size,
        }
    }

    #[must_use]
    pub(crate) fn min_size(&self, size: Vec2) -> Self {
        let size = self.size().min(size);
        Self {
            min: self.min,
            max: self.min + size,
        }
    }

    #[must_use]
    pub(crate) fn max_size(&self, size: Vec2) -> Self {
        let size = self.size().max(size);
        Self {
            min: self.min,
            max: self.min + size,
        }
    }

    pub(crate) fn contains_point(&self, local_pos: Vec2) -> bool {
        local_pos.x >= self.min.x
            && local_pos.x <= self.max.x
            && local_pos.y >= self.min.y
            && local_pos.y <= self.max.y
    }

    #[must_use]
    pub(crate) fn translate(&self, pos: Vec2) -> Self {
        Self {
            min: self.min + pos,
            max: self.max + pos,
        }
    }
}
