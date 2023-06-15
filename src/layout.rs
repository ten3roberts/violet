use flax::{EntityRef, World};
use glam::{vec2, Vec2};

use crate::{
    components::{self, children, layout, padding, Rect},
    constraints::widget_outer_bounds,
};

#[derive(Debug)]
pub struct Layout {}

impl Layout {
    /// Position and size the children of the given entity using all the provided available space
    fn apply(&self, world: &World, entity: &EntityRef, content_area: Rect) -> Rect {
        let children = entity.get(children()).ok();
        let children = children.as_ref().map(|v| v.as_slice()).unwrap_or_default();

        // Start at the corner of the rect. May not be 0,0 due to padding

        let mut window = content_area;

        let mut line_height = 0.0f32;

        for &child in children {
            let entity = world.entity(child).expect("Invalid child");
            // let local_rect = widget_outer_bounds(world, &child, size);
            let rect = update_subtree(
                world,
                &entity,
                window,
                LayoutConstraints {
                    min: Vec2::ZERO,
                    max: content_area.size(),
                },
            )
            .unwrap_or_default();

            entity.update(components::rect(), |v| *v = rect);

            // Slide
            window = window.translate((rect.size().x + 10.0) * Vec2::X);

            line_height = line_height.max(rect.size().y);
        }

        window = window.translate(line_height * Vec2::Y);

        Rect {
            min: content_area.min,
            max: window.min,
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

/// Constraints for a child widget passed down from the parent.
///
/// Allows for the parent to control the size of the children, such as stretching
#[derive(Debug, Clone, Copy)]
pub(crate) struct LayoutConstraints {
    pub min: Vec2,
    pub max: Vec2,
}

/// Updates the layout of the given subtree given the passes constraints.
///
/// Returns the outer bounds of the subtree.
#[must_use = "This function does not mutate the entity"]
pub(crate) fn update_subtree(
    world: &World,
    entity: &EntityRef,
    // The area in which children can be placed without clipping
    content_area: Rect,
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

        tracing::info!("Layout with padding: {padding:?}");
        let inner_rect = layout
            .apply(world, entity, content_area.inset(&padding))
            .pad(&padding);

        Some(inner_rect)
    }
    // Stack
    else if let Ok(children) = entity.get(children()) {
        let total_bounds = Rect {
            min: Vec2::ZERO,
            max: Vec2::ONE,
        };

        let content_area = content_area.inset(&padding);

        for &child in &*children {
            let entity = world.entity(child).unwrap();

            // let local_rect = widget_outer_bounds(world, &entity, inner_rect.size());

            let rect = update_subtree(
                world,
                &entity,
                content_area,
                LayoutConstraints {
                    min: Vec2::ZERO,
                    max: content_area.size(),
                },
            );

            entity.update(components::rect(), |v| *v = rect.unwrap_or_default());
        }
        Some(total_bounds)
    }
    // Leaf
    else if let Ok(v) = entity.get(components::constraints()) {
        let rect = v.apply(content_area);
        // tracing::info!("Constrained {rect:?}");

        Some(rect)
    } else {
        tracing::warn!(%entity, "Widget is not positioned");
        None
    }
}
