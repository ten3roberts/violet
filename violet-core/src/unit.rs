use std::{
    fmt::{Display, Formatter},
    ops::{Add, AddAssign, Mul, MulAssign, Sub, SubAssign},
};

use glam::{BVec2, IVec2, Vec2};
use tween::TweenValue;

/// Represents a unit of measurement
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Unit<T> {
    pub px: T,
    pub rel: T,
}

impl<T: Zero> Default for Unit<T> {
    fn default() -> Self {
        Self::ZERO
    }
}

impl<T> Unit<T> {
    pub fn new(px: T, rel: T) -> Self {
        Self { px, rel }
    }
}

impl<T: Zero> Unit<T> {
    pub const ZERO: Self = Self {
        px: T::ZERO,
        rel: T::ZERO,
    };

    pub const fn px(px: T) -> Self {
        Self { px, rel: T::ZERO }
    }

    pub const fn rel(rel: T) -> Self {
        Self { px: T::ZERO, rel }
    }
}

impl Unit<Vec2> {
    pub const fn px2(x: f32, y: f32) -> Self {
        Self {
            px: Vec2::new(x, y),
            rel: Vec2::ZERO,
        }
    }

    pub const fn rel2(x: f32, y: f32) -> Self {
        Self {
            px: Vec2::ZERO,
            rel: Vec2::new(x, y),
        }
    }

    pub(crate) fn is_relative(&self) -> BVec2 {
        self.rel.cmpgt(Vec2::ZERO)
    }
}

impl Unit<IVec2> {
    pub fn px2i(x: i32, y: i32) -> Self {
        Self {
            px: IVec2::new(x, y),
            rel: IVec2::ZERO,
        }
    }

    pub fn rel2i(x: i32, y: i32) -> Self {
        Self {
            px: IVec2::ZERO,
            rel: IVec2::new(x, y),
        }
    }
}

impl<T> Unit<T>
where
    T: Add<Output = T> + Mul<Output = T> + Copy,
{
    /// Resolve the unit to an absolute value based on the reference value
    pub fn resolve(&self, reference: T) -> T {
        self.px + self.rel * reference
    }
}

pub trait Zero {
    const ZERO: Self;
}

impl Zero for f32 {
    const ZERO: Self = 0.0;
}

impl Zero for Vec2 {
    const ZERO: Self = Vec2::ZERO;
}

impl Zero for IVec2 {
    const ZERO: Self = IVec2::ZERO;
}

impl<T> std::ops::Add for Unit<T>
where
    T: Add<Output = T> + Mul<Output = T> + Copy,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            px: self.px + rhs.px,
            rel: self.rel + rhs.rel,
        }
    }
}

impl<T> std::ops::Sub for Unit<T>
where
    T: Sub<Output = T> + Mul<Output = T> + Copy,
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            px: self.px - rhs.px,
            rel: self.rel - rhs.rel,
        }
    }
}

impl<T> AddAssign for Unit<T>
where
    T: AddAssign + Mul<Output = T> + Copy,
{
    fn add_assign(&mut self, rhs: Self) {
        self.px += rhs.px;
        self.rel += rhs.rel;
    }
}

impl<T> SubAssign for Unit<T>
where
    T: SubAssign + Mul<Output = T> + Copy,
{
    fn sub_assign(&mut self, rhs: Self) {
        self.px -= rhs.px;
        self.rel -= rhs.rel;
    }
}

impl Mul<f32> for Unit<Vec2> {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            px: self.px * rhs,
            rel: self.rel * rhs,
        }
    }
}

impl Mul<Self> for Unit<Vec2> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            px: self.px * rhs.px,
            rel: self.rel * rhs.rel,
        }
    }
}

impl Add<Vec2> for Unit<Vec2> {
    type Output = Self;

    fn add(self, rhs: Vec2) -> Self::Output {
        Self {
            px: self.px + rhs,
            rel: self.rel + rhs,
        }
    }
}

impl MulAssign<f32> for Unit<Vec2> {
    fn mul_assign(&mut self, rhs: f32) {
        self.px *= rhs;
        self.rel *= rhs;
    }
}

impl<T: Display> Display for Unit<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "(px: {}, rel: {})", self.px, self.rel)
    }
}

impl<T: TweenValue> TweenValue for Unit<T>
where
    T: Mul<Output = T> + Add<Output = T>,
{
    fn scale(self, scale: f32) -> Self {
        Self {
            px: self.px.scale(scale),
            rel: self.rel.scale(scale),
        }
    }
}
