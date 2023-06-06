use glam::{Mat4, Quat, Vec2, Vec3};

use crate::shapes::{Rect, Shape};

/// Allows painting the tree with a command list
pub struct Painter {
    list: Vec<Shape>,
}

impl Painter {
    pub fn new() -> Self {
        Self { list: Vec::new() }
    }

    pub fn draw_rect(&mut self, rect: Rect) {
        self.list.push(Shape::Rect(rect));
    }
}

#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct ObjectData {
    world_matrix: Mat4,
}
