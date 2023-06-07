use flax::EntityRef;
use glam::Vec2;

use crate::components::{constraints, Rect};

#[derive(Clone, Debug, Default)]
pub struct Constraints {
    /// Absolute size offset
    pub abs_size: Vec2,
    /// Absolute offset
    pub abs_offset: Vec2,
    /// Size relative to parent
    pub rel_size: Vec2,
    /// Offset relative to parent size
    pub rel_offset: Vec2,
    /// Anchor point within self.
    ///
    /// 0,0, refers to the top-left corner, and 1,1 the bottom right of the widgets bounds
    pub anchor: Vec2,
}

impl Constraints {
    pub(crate) fn apply(&self, parent_rect: &Rect) -> Rect {
        let parent_size = parent_rect.size();

        let pos = self.abs_offset + self.rel_offset * parent_size;
        let size = self.abs_size + self.rel_size * parent_size;

        let pos = parent_rect.pos() + pos - self.anchor * size;

        Rect::from_size_pos(size, pos)
    }
}

pub(crate) fn entity_constraints(entity: &EntityRef, parent_rect: &Rect) -> Rect {
    entity
        .get(constraints())
        .map(|c| c.apply(parent_rect))
        .unwrap_or_default()
}
