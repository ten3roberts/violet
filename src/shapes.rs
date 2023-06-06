use glam::Vec2;

/// Rectangle with a min and max corner
#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub size: Vec2,
}

#[derive(Clone, Copy, Debug)]
pub enum Shape {
    Rect(Rect),
}
