use glam::{vec2, BVec2, Vec2};

use crate::layout::{Direction, QueryArgs, SizeResolver, SizingHints};

pub struct FixedAreaConstraint {
    pub area: f32,
    pub unit_size: f32,
}

impl SizeResolver for FixedAreaConstraint {
    // fn resolve(
    //     &mut self,
    //     entity: &flax::EntityRef,
    //     content_area: Rect,
    //     limits: Option<crate::layout::LayoutLimits>,
    //     squeeze: Vec2,
    // ) -> (Vec2, Vec2) {
    //     if let Some(limits) = limits {
    //         let width = round(limits.max_size.x, 20.0);
    //         let height = round(self.area / width, 20.0);
    //         (vec2(width, height), vec2(width, height))
    //     } else {
    //         (vec2(1.0, self.area), vec2(self.area, 1.0))
    //     }
    // }

    fn query(&mut self, _: &flax::EntityRef, args: QueryArgs) -> (Vec2, Vec2, SizingHints) {
        let size = (args.limits.max_size / self.unit_size)
            .floor()
            .max(Vec2::ONE);

        let min = match args.direction {
            Direction::Horizontal => vec2((self.area / size.y).ceil(), size.y),
            Direction::Vertical => vec2(size.x, (self.area / size.x).ceil()),
        };

        (
            min * self.unit_size,
            vec2(size.x, (self.area / size.x).ceil()) * self.unit_size,
            SizingHints {
                can_grow: BVec2::TRUE,
                relative_size: BVec2::TRUE,
            },
        )
    }

    fn apply(
        &mut self,
        _: &flax::EntityRef,
        _: Vec2,
        limits: crate::layout::LayoutLimits,
    ) -> (Vec2, BVec2) {
        let width = (limits.max_size.x / self.unit_size).floor().max(1.0);

        let height = (self.area / width).ceil();

        (vec2(width, height) * self.unit_size, BVec2::TRUE)
    }
}
