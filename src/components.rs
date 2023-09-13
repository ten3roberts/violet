use flax::{component, Debuggable, Entity};
use glam::{vec2, Vec2};
use palette::Srgba;

use crate::{layout::Layout, shapes::FilledRect, unit::Unit};

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

    /// Manages the layout of the children
    pub layout: Layout => [ Debuggable ],

    /// Spacing between a outer and inner bounds
    pub padding: Edges => [ Debuggable ],
    pub margin: Edges => [ Debuggable ],


    pub text: String => [ ],
    pub font_size: f32 => [ Debuggable ],

    /// The color of the widget
    pub color: Srgba => [ Debuggable ],

    /// The widget will be rendered as a filled rectange coverings its bounds
    pub filled_rect: FilledRect => [ Debuggable ],
}

/// Spacing between a outer and inner bounds
#[derive(Clone, Copy, Debug, Default)]
pub struct Edges {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Edges {
    pub fn new(left: f32, right: f32, top: f32, bottom: f32) -> Self {
        Self {
            left,
            right,
            top,
            bottom,
        }
    }

    pub fn even(distance: f32) -> Self {
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
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
/// Defines the bounds of a widget
pub struct Rect {
    pub min: Vec2,
    pub max: Vec2,
}

impl Rect {
    pub fn from_two_points(a: Vec2, b: Vec2) -> Self {
        Self {
            min: a.min(b),
            max: a.max(b),
        }
    }

    pub fn from_size_pos(size: Vec2, pos: Vec2) -> Self {
        Self {
            min: pos,
            max: pos + size,
        }
    }

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
    pub fn inset(&self, padding: &Edges) -> Rect {
        Self {
            min: self.min + vec2(padding.left, padding.top),
            max: self.max - vec2(padding.right, padding.bottom),
        }
    }

    /// Makes the rect larger by the given padding
    pub fn pad(&self, padding: &Edges) -> Rect {
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

    pub(crate) fn clamp(&self, min: Vec2, max: Vec2) -> Self {
        let size = self.size().clamp(min, max);
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

    pub(crate) fn translate(&self, pos: Vec2) -> Self {
        Self {
            min: self.min + pos,
            max: self.max + pos,
        }
    }
}
