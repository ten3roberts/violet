use glam::Mat4;
use wgpu::{BindGroup, BindGroupLayout, BufferUsages, ShaderStages};

use super::{
    graphics::{BindGroupBuilder, BindGroupLayoutBuilder, TypedBuffer},
    mesh_buffer::MeshBuffer,
    Gpu,
};

pub struct RendererContext {
    pub globals: Globals,
    pub globals_buffer: TypedBuffer<Globals>,
    pub mesh_buffer: MeshBuffer,
    pub globals_bind_group: BindGroup,
    pub globals_layout: BindGroupLayout,
}

impl RendererContext {
    pub fn new(gpu: &Gpu) -> Self {
        let globals_layout = BindGroupLayoutBuilder::new("WindowRenderer::globals_layout")
            .bind_uniform_buffer(ShaderStages::VERTEX)
            .build(gpu);

        let globals = Globals {
            projview: Mat4::IDENTITY,
        };

        let globals_buffer = TypedBuffer::new(
            gpu,
            "WindowRenderer::globals_buffer",
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            &[globals],
        );

        let globals_bind_group = BindGroupBuilder::new("WindowRenderer::globals")
            .bind_buffer(&globals_buffer.buffer())
            .build(gpu, &globals_layout);

        let mesh_buffer = MeshBuffer::new(gpu, "MeshBuffer", 4);

        Self {
            globals_layout,
            globals,
            globals_bind_group,
            globals_buffer,
            mesh_buffer,
        }
    }
}

#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Globals {
    pub projview: Mat4,
}
