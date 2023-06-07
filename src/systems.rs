use flax::{child_of, BoxedSystem, EntityRef, Query, QueryBorrow, System, World};
use glam::{vec2, Vec2};

use crate::components::{self, children, constraints, padding, rect, Rect};

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
    fn apply(&self, parent_rect: &Rect) -> Rect {
        let parent_size = parent_rect.size();

        let pos = self.abs_offset + self.rel_offset * parent_size;
        let size = self.abs_size + self.rel_size * parent_size;

        let pos = parent_rect.pos() + pos - self.anchor * size;

        Rect::from_size_pos(size, pos)
    }
}

fn entity_constraints(entity: &EntityRef, parent_rect: &Rect) -> Rect {
    entity
        .get(constraints())
        .map(|c| c.apply(parent_rect))
        .unwrap_or_default()
}

/// Updates the layout for entities using the given constraints
pub fn layout_system() -> BoxedSystem {
    System::builder()
        .read()
        .with(Query::new((rect(), children())).without_relation(child_of))
        .build(move |world: &World, mut roots: QueryBorrow<_, _>| {
            (&mut roots)
                .into_iter()
                .for_each(|(rect, children): (_, &Vec<_>)| {
                    for &child in children {
                        let entity = world.entity(child).unwrap();
                        let rect = entity_constraints(&entity, rect);

                        update_subtree(world, entity, rect);
                    }
                });
        })
        .boxed()
}

fn update_subtree(world: &World, entity: EntityRef, rect: Rect) -> Option<()> {
    *entity.get_mut(components::rect()).ok()? = rect;

    let padding = entity
        .get(padding())
        .as_deref()
        .copied()
        .unwrap_or_default();

    let inner_rect = Rect {
        min: rect.min + vec2(padding.left, padding.top),
        max: rect.max - vec2(padding.right, padding.bottom),
    };

    if let Ok(children) = entity.get(children()) {
        for &child in &*children {
            let entity = world.entity(child).unwrap();

            let rect = entity_constraints(&entity, &inner_rect);

            update_subtree(world, entity, rect);
        }
    }

    Some(())
}
