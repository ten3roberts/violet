use glam::{vec2, BVec2, Vec2};

use crate::layout::{Direction, LayoutArgs, QueryArgs, SizeResolver, SizingHints};

pub struct FixedAreaConstraint {
    pub area: f32,
    pub unit_size: f32,
}

impl SizeResolver for FixedAreaConstraint {
    fn query_size(&mut self, _: &flax::EntityRef, args: QueryArgs) -> (Vec2, Vec2, SizingHints) {
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
                coupled_size: true,
            },
        )
    }

    fn apply_layout(&mut self, _: &flax::EntityRef, args: LayoutArgs) -> (Vec2, BVec2) {
        let width = (args.limits.max_size.x / self.unit_size).floor().max(1.0);

        let height = (self.area / width).ceil();

        (vec2(width, height) * self.unit_size, BVec2::TRUE)
    }
}
