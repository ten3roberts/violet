use flax::{component, Debuggable, Entity};
use glam::Vec2;

use crate::{shapes::Shape, systems::Constraints};

component! {
    /// Ordered list of children for an entity
    pub children: Vec<Entity> => [ Debuggable ],
    // pub child_of(parent): Entity => [ Debuggable ],

    /// The shape of a widget when drawn
    pub shape: Shape => [ Debuggable ],

    /// Defines the outer bounds of a widget
    pub rect: Rect => [ Debuggable ],

    /// Linear constraints for widget positioning and size
    pub constraints: Constraints => [ Debuggable ],

    /// Spacing between a outer and inner bounds
    pub padding: Padding => [ Debuggable ],
}

/// Spacing between a outer and inner bounds
#[derive(Clone, Copy, Debug, Default)]
pub struct Padding {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Padding {
    pub fn even(distance: f32) -> Self {
        Self {
            left: distance,
            right: distance,
            top: distance,
            bottom: distance,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
/// Defines the penultimate bounds of a widget
pub struct Rect {
    pub min: Vec2,
    pub max: Vec2,
}

impl Rect {
    pub fn from_size_pos(size: Vec2, pos: Vec2) -> Self {
        Self {
            min: pos,
            max: pos + size,
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
}
