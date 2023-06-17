use flax::{EntityRef, World};
use glam::Vec2;

use crate::{
    components::{constraints, layout, padding, Rect},
    layout::LayoutConstraints,
};

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
    pub(crate) fn apply(&self, content_area: Rect, constraints: LayoutConstraints) -> Rect {
        let parent_size = content_area.size();

        // let _span = tracing::info_span!("Applying constraints", ?parent_size).entered();
        let pos = self.abs_offset + self.rel_offset * parent_size;
        let size = self.abs_size + self.rel_size * parent_size;
        // let size = size.clamp(constraints.min, constraints.max);

        let pos = content_area.pos() + pos - self.anchor * size;

        Rect::from_size_pos(size, pos)
    }
}

pub(crate) fn widget_outer_bounds(
    world: &World,
    entity: &EntityRef,
    mut parent_size: Vec2,
) -> Rect {
    todo!()
    // let mut rect = entity
    //     .get(constraints())
    //     .map(|c| {
    //         c.apply(Rect {
    //             min: Vec2::ZERO,
    //             max: parent_size,
    //         })
    //     })
    //     .unwrap_or_default();

    // if let Ok(padding) = entity.get(padding()) {
    //     parent_size -= padding.left + padding.right + padding.top + padding.bottom;
    // };

    // if let Ok(layout) = entity.get(layout()) {
    //     let total_size = layout.total_size(world, entity, parent_size);
    //     rect
    // } else {
    //     rect
    // }
}
