use glam::Vec2;
use palette::{Srgb, Srgba};

/// A rectangle sized to the widget
#[derive(Clone, Copy, Debug)]
pub struct FilledRect {
    pub color: Srgba<u8>,
}

#[derive(Clone, Copy, Debug)]
pub enum Shape {
    FilledRect(FilledRect),
}
