use flax::{component, Debuggable, Entity};
use glam::Vec2;

use crate::{shapes::Shape, systems::Constraints};

component! {
    /// Ordered list of children for an entity
    pub children: Vec<Entity> => [ Debuggable ],
    // pub child_of(parent): Entity => [ Debuggable ],

    /// The shape of a widget when drawn
    pub shape: Shape => [ Debuggable ],

    /// The top-left position of a widget
    pub position: Vec2 => [ Debuggable ],
    /// The overall size or extent of a widget
    pub size: Vec2 => [ Debuggable ],


    /// Linear constraints for widget positioning and size
    pub constraints: Constraints => [ Debuggable ],
}
