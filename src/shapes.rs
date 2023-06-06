use glam::Vec2;
use palette::{Srgb, Srgba};

/// Rectangle with a min and max corner
#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub size: Vec2,
    pub color: Srgba<u8>,
}

#[derive(Clone, Copy, Debug)]
pub enum Shape {
    Rect(Rect),
}
