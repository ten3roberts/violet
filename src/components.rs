use flax::{component, Debuggable, Entity};
use glam::Vec2;

use crate::shapes::Shape;

component! {
    /// Ordered list of children for an entity
    pub children: Vec<Entity> => [ Debuggable ],
    // pub child_of(parent): Entity => [ Debuggable ],

    /// The shape of a widget when drawn
    pub shape: Shape => [ Debuggable ],

    /// The position of a widget
    pub position: Vec2 => [ Debuggable ],
}
