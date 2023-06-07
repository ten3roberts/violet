use flax::{child_of, BoxedSystem, Entity, FetchExt, Query, QueryBorrow, System, World};
use glam::Vec2;

use crate::components::{children, constraints, position, size};

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
    /// Anchor point within self
    /// 0,0, refers to the top-left corner, and 1,1 the bottom right of the widgets bounds
    pub anchor: Vec2,
}

struct ConstraintResult {
    size: Vec2,
    pos: Vec2,
}

fn apply_contraints(c: &Constraints, parent_size: Vec2) -> ConstraintResult {
    let pos = c.abs_offset + c.rel_offset * parent_size;
    let size = c.abs_size + c.rel_size * parent_size;

    let pos = pos - c.anchor * size;

    ConstraintResult { size, pos }
}

/// Updates the layout for entities using the given constraints
pub fn layout_system() -> BoxedSystem {
    System::builder()
        .read()
        .with(Query::new((size(), position(), children())).without_relation(child_of))
        .build(move |world: &World, mut roots: QueryBorrow<_, _>| {
            (&mut roots)
                .into_iter()
                .for_each(|(size, pos, children): (&Vec2, &Vec2, &Vec<_>)| {
                    for &child in children {
                        update_subtree(world, child, *size, *pos);
                    }
                });
        })
        .boxed()
}

fn update_subtree(world: &World, id: Entity, parent_size: Vec2, parent_position: Vec2) {
    let mut query = Query::new((
        size().as_mut(),
        position().as_mut(),
        children().opt_or_default(),
        constraints().opt_or_default(),
    ))
    .entity(id);

    let mut query = query.borrow(world);

    if let Ok((size, pos, children, constraints)) = query.get() {
        let res = apply_contraints(constraints, parent_size);

        *size = res.size;
        *pos = res.pos + parent_position;

        for &child in children {
            update_subtree(world, child, *size, *pos);
        }
    }
}
