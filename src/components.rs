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
    /// The overall size or scale of a widget
    pub size: Vec2 => [ Debuggable ],

    pub absolute_offset: Vec2 => [ Debuggable ],
    pub absolute_size: Vec2 => [ Debuggable ],

    /// Offset relative to the parent size
    pub relative_offset:Vec2 => [ Debuggable ],
    /// Size relative to the parent widget
    pub relative_size: Vec2 => [ Debuggable ],
}
