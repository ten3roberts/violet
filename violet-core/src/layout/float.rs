use flax::{Entity, EntityRef, World};
use glam::{BVec2, Vec2};

use super::{apply_layout, ApplyLayoutArgs, LayoutBlock, LayoutLimits, QueryArgs, Sizing};
use crate::{
    components,
    layout::{
        query_layout_size, ContainerLayoutArgs, ContainerQueryArgs, Direction, LayoutArgs,
        SizingHints,
    },
    Edges, Rect,
};

/// A floating layout positions its children similar to the stack layout, but it does grow to accommodate the children.
///
/// This means that the children are *detached* from the normal flow of the layout, and they can overlap with other neighboring widgets.
///
/// This is the preferred layout for things like tooltips, popups, and other floating UI elements.
#[derive(Default, Debug, Clone)]
pub struct FloatLayout {}

impl FloatLayout {
    pub(crate) fn apply(
        &self,
        world: &World,
        _: &EntityRef,
        args: ContainerLayoutArgs,
    ) -> LayoutBlock {
        puffin::profile_function!();
        let _span = tracing::debug_span!("FloatLayout::apply").entered();

        let mut maximize = Vec2::ZERO;
        args.children.iter().for_each(|&child| {
            let entity = world.entity(child).expect("invalid child");

            let limits = LayoutLimits {
                layout_min_size: Vec2::ZERO,
                layout_max_size: Vec2::MAX,
            };

            let block = apply_layout(
                world,
                &entity,
                LayoutArgs {
                    content_area: args.content_area,
                    limits,
                },
            );

            maximize = (maximize + block.maximize).min(Vec2::ONE);
            entity.update_dedup(components::rect(), block.rect);
            entity.update_dedup(components::local_position(), Vec2::ZERO);
        });

        LayoutBlock::new(Rect::ZERO, Edges::ZERO, BVec2::FALSE, maximize)
    }

    pub(crate) fn query_size(
        &self,
        world: &World,
        children: &[Entity],
        args: ContainerQueryArgs,
        _: Vec2,
    ) -> Sizing {
        puffin::profile_function!();

        let mut hints = SizingHints::default();

        for &child in children.iter() {
            let entity = world.entity(child).expect("invalid child");

            let sizing = query_layout_size(
                world,
                &entity,
                QueryArgs {
                    limits: LayoutLimits {
                        layout_min_size: Vec2::ZERO,
                        layout_max_size: Vec2::MAX,
                    },
                    content_area: args.content_area,
                    direction: Direction::Horizontal,
                },
            );

            hints = hints.combine(sizing.hints);
        }

        Sizing {
            min: Rect::ZERO,
            desired: Rect::ZERO,
            margin: Edges::ZERO,
            hints,
            maximize: Vec2::ZERO,
        }
    }
}
