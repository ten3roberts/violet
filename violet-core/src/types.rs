use std::fmt::Display;

use glam::{vec2, Vec2};

/// Spacing between a outer and inner bounds
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Edges {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl From<f32> for Edges {
    fn from(value: f32) -> Self {
        Self::even(value)
    }
}

impl std::ops::Sub for Edges {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            left: self.left - rhs.left,
            right: self.right - rhs.right,
            top: self.top - rhs.top,
            bottom: self.bottom - rhs.bottom,
        }
    }
}

impl std::ops::SubAssign for Edges {
    fn sub_assign(&mut self, rhs: Self) {
        self.left -= rhs.left;
        self.right -= rhs.right;
        self.top -= rhs.top;
        self.bottom -= rhs.bottom;
    }
}

impl std::ops::Add for Edges {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            left: self.left + rhs.left,
            right: self.right + rhs.right,
            top: self.top + rhs.top,
            bottom: self.bottom + rhs.bottom,
        }
    }
}

impl std::ops::Mul<f32> for Edges {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            left: self.left * rhs,
            right: self.right * rhs,
            top: self.top * rhs,
            bottom: self.bottom * rhs,
        }
    }
}

impl Edges {
    pub const ZERO: Self = Self {
        left: 0.0,
        right: 0.0,
        top: 0.0,
        bottom: 0.0,
    };

    pub const fn new(left: f32, right: f32, top: f32, bottom: f32) -> Self {
        Self {
            left,
            right,
            top,
            bottom,
        }
    }

    pub const fn even(distance: f32) -> Self {
        Self {
            left: distance,
            right: distance,
            top: distance,
            bottom: distance,
        }
    }

    pub(crate) fn size(&self) -> Vec2 {
        vec2(self.left + self.right, self.top + self.bottom)
    }

    pub(crate) fn in_axis(&self, axis: Vec2) -> (f32, f32) {
        let pos = vec2(self.right, self.bottom).dot(axis);
        let neg = vec2(-self.left, -self.top).dot(axis);

        let front_margin = pos.max(neg);
        let back_margin = -(pos.min(neg));

        (back_margin, front_margin)
    }

    pub fn max(&self, other: Self) -> Self {
        Self {
            left: self.left.max(other.left),
            right: self.right.max(other.right),
            top: self.top.max(other.top),
            bottom: self.bottom.max(other.bottom),
        }
    }

    pub fn min(&self, other: Self) -> Self {
        Self {
            left: self.left.min(other.left),
            right: self.right.min(other.right),
            top: self.top.min(other.top),
            bottom: self.bottom.min(other.bottom),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
/// Defines the bounds of a widget
pub struct Rect {
    pub min: Vec2,
    pub max: Vec2,
}

impl Rect {
    pub const ZERO: Self = Self {
        min: Vec2::ZERO,
        max: Vec2::ZERO,
    };

    pub fn from_size(size: Vec2) -> Self {
        Self {
            min: Vec2::ZERO,
            max: size,
        }
    }

    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    #[must_use]
    pub fn from_two_points(a: Vec2, b: Vec2) -> Self {
        Self {
            min: a.min(b),
            max: a.max(b),
        }
    }

    #[must_use]
    pub fn from_size_pos(size: Vec2, pos: Vec2) -> Self {
        Self {
            min: pos,
            max: pos + size,
        }
    }

    #[must_use]
    pub fn align_to_grid(&self) -> Self {
        Self {
            min: self.min.round(),
            max: self.max.round(),
        }
    }

    #[inline]
    pub fn size(&self) -> Vec2 {
        self.max - self.min
    }

    #[inline]
    pub fn pos(&self) -> Vec2 {
        self.min
    }

    /// Makes the rect smaller by the given padding
    #[must_use]
    pub fn inset(&self, padding: &Edges) -> Self {
        Self {
            min: self.min + vec2(padding.left, padding.top),
            max: self.max - vec2(padding.right, padding.bottom),
        }
    }

    /// Makes the rect larger by the given padding
    #[must_use]
    pub fn pad(&self, padding: &Edges) -> Self {
        Self {
            min: self.min - vec2(padding.left, padding.top),
            max: self.max + vec2(padding.right, padding.bottom),
        }
    }

    pub(crate) fn support(&self, axis: Vec2) -> f32 {
        let x = (self.min.x * -axis.x).max(0.0) + (self.max.x * axis.x).max(0.0);
        let y = (self.min.y * -axis.y).max(0.0) + (self.max.y * axis.y).max(0.0);

        vec2(x, y).dot(axis)
    }

    #[must_use]
    pub(crate) fn clamp_size(&self, min: Vec2, max: Vec2) -> Self {
        let size = self.size().clamp(min, max);
        Self {
            min: self.min,
            max: self.min + size,
        }
    }

    #[must_use]
    pub fn min_size(&self, size: Vec2) -> Self {
        let size = self.size().min(size);
        Self {
            min: self.min,
            max: self.min + size,
        }
    }

    #[must_use]
    pub fn max_size(&self, size: Vec2) -> Self {
        let size = self.size().max(size);
        Self {
            min: self.min,
            max: self.min + size,
        }
    }

    pub fn contains_point(&self, local_pos: Vec2) -> bool {
        local_pos.x >= self.min.x
            && local_pos.x <= self.max.x
            && local_pos.y >= self.min.y
            && local_pos.y <= self.max.y
    }

    #[must_use]
    pub fn translate(&self, pos: Vec2) -> Self {
        Self {
            min: self.min + pos,
            max: self.max + pos,
        }
    }

    pub fn clip(&self, mask: Rect) -> Rect {
        let min = self.min.max(mask.min);
        let max = self.max.min(mask.max);

        Rect { min, max }
    }

    pub(crate) fn with_size(&self, size: Vec2) -> Self {
        let min = self.min;
        let max = min + size;
        Rect { min, max }
    }
}

impl Display for Edges {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "({},{},{},{})",
            self.left, self.right, self.top, self.bottom
        ))
    }
}
