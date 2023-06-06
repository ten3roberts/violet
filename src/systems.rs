use flax::{
    child_of, BoxedSystem, Component, Dfs, Entity, EntityRef, Fetch, FetchExt, GraphQuery, OptOr,
    Query, QueryBorrow, System, World,
};
use glam::Vec2;
use tracing::info_span;

use crate::components::{
    absolute_offset, absolute_size, children, position, relative_offset, relative_size, size,
};

struct ConstraintResult {
    size: Vec2,
    pos: Vec2,
}

#[derive(Debug, Fetch)]
struct ConstraintQuery {
    rel_size: OptOr<Component<Vec2>, Vec2>,
    abs_size: OptOr<Component<Vec2>, Vec2>,
    rel_offset: OptOr<Component<Vec2>, Vec2>,
    abs_offset: OptOr<Component<Vec2>, Vec2>,
}

impl ConstraintQuery {
    fn new() -> Self {
        Self {
            rel_size: relative_size().opt_or_default(),
            abs_size: absolute_size().opt_or_default(),
            rel_offset: relative_offset().opt_or_default(),
            abs_offset: absolute_offset().opt_or_default(),
        }
    }
}

fn apply_contraints(constraints: ConstraintQueryItem, parent_size: Vec2) -> ConstraintResult {
    let pos = *constraints.abs_offset + *constraints.rel_offset * parent_size;
    let size = *constraints.abs_size + *constraints.rel_size * parent_size;

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
    tracing::info!(?id, %parent_size, %parent_position, "Updating subtree");
    let mut query = Query::new((
        size().as_mut(),
        position().as_mut(),
        children().opt_or_default(),
        ConstraintQuery::new(),
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
    } else {
        tracing::warn!("Subtree query failed for {id}");
    }
}
