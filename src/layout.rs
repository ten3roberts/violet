use flax::{EntityRef, World};
use glam::{vec2, Vec2};

use crate::{
    components::{self, children, layout, local_position, padding, Rect},
    constraints::widget_outer_bounds,
};

#[derive(Debug)]
pub struct Layout {}

impl Layout {
    /// Lays out the children of the given entity using all the provided available space
    fn apply(&self, world: &World, entity: &EntityRef, available: Rect) -> Rect {
        let children = entity.get(children()).ok();
        let children = children.as_ref().map(|v| v.as_slice()).unwrap_or_default();

        let available_size = available.size();

        let mut cursor = Vec2::ZERO;

        let mut line_height = 0.0f32;

        for &child in children {
            let child = world.entity(child).expect("Invalid child");
            // let local_rect = widget_outer_bounds(world, &child, size);
            let rect = update_subtree(
                world,
                &child,
                LayoutConstraints {
                    min: Vec2::ZERO,
                    max: available_size,
                },
            )
            .unwrap_or_default();

            if let Ok(mut pos) = child.get_mut(local_position()) {
                pos.x = cursor.x;
            }

            cursor.x += rect.size().x + 10.0;
            line_height = line_height.max(rect.size().y);
        }

        cursor.y += line_height;

        Rect {
            min: available.min,
            max: cursor,
        }
    }

    pub(crate) fn total_size(&self, world: &World, entity: &EntityRef, size: Vec2) -> Vec2 {
        let children = entity.get(children()).ok();
        let children = children.as_ref().map(|v| v.as_slice()).unwrap_or_default();

        let mut cursor = 0.0;
        let mut line_height = 0.0f32;

        for &child in children {
            let child = world.entity(child).expect("Invalid child");
            let rect = widget_outer_bounds(world, &child, size);

            cursor += rect.size().x + 10.0;
            line_height = line_height.max(rect.size().y);
        }

        vec2(cursor, line_height)
    }
}

/// Constraints for a child widget passed down from the parent
#[derive(Debug, Clone, Copy)]
pub(crate) struct LayoutConstraints {
    pub min: Vec2,
    pub max: Vec2,
}

/// Updates the layout of the given subtree given the passes constraints.
///
/// Returns the outer bounds of the subtree.
pub(crate) fn update_subtree(
    world: &World,
    entity: &EntityRef,
    constraints: LayoutConstraints,
) -> Option<Rect> {
    // let _span = tracing::info_span!( "Updating subtree", %entity, ?constraints).entered();
    let padding = entity
        .get(padding())
        .ok()
        .as_deref()
        .copied()
        .unwrap_or_default();

    // Flow
    if let Ok(layout) = entity.get(layout()) {
        // For a given layout use the largest size that fits within the constraints and then
        // potentially shrink it down.
        let available = Rect {
            min: Vec2::ZERO,
            max: constraints.max,
        }
        .pad(&padding);

        Some(layout.apply(world, entity, available))
    }
    // Stack
    else if let Ok(children) = entity.get(children()) {
        tracing::info!(?children, "Stacked");
        let total_bounds = Rect {
            min: Vec2::ZERO,
            max: Vec2::ONE,
        };
        for &child in &*children {
            let entity = world.entity(child).unwrap();

            // let local_rect = widget_outer_bounds(world, &entity, inner_rect.size());

            update_subtree(world, &entity, constraints);
        }
        Some(total_bounds)
    }
    // Leaf
    else if let Ok(v) = entity.get(components::constraints()) {
        let rect = v.apply(constraints.max);
        // tracing::info!("Constrained {rect:?}");

        if let Ok(mut v) = entity.get_mut(components::rect()) {
            *v = rect;
        }

        Some(rect)
    } else {
        tracing::warn!(id=%entity, "Widget is not positioned");
        None
    }
}
