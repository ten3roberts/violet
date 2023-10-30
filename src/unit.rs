use std::ops;

use glam::Vec2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Unit<T> {
    px: T,
    rel: T,
}

impl<T: Zero> Default for Unit<T> {
    fn default() -> Self {
        Self::ZERO
    }
}

impl<T: Zero> Unit<T> {
    pub const ZERO: Self = Self {
        px: T::ZERO,
        rel: T::ZERO,
    };

    pub fn px(px: T) -> Self {
        Self { px, rel: T::ZERO }
    }

    pub fn rel(rel: T) -> Self {
        Self { px: T::ZERO, rel }
    }
}

impl<T> Unit<T>
where
    T: ops::Add<Output = T> + ops::Mul<Output = T> + Copy,
{
    pub fn resolve(&self, parent: T) -> T {
        self.px + self.rel * parent
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

impl<T> std::ops::Add for Unit<T>
where
    T: ops::Add<Output = T> + ops::Mul<Output = T> + Copy,
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
    T: ops::Sub<Output = T> + ops::Mul<Output = T> + Copy,
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            px: self.px - rhs.px,
            rel: self.rel - rhs.rel,
        }
    }
}
