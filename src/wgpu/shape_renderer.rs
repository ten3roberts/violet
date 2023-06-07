use std::{borrow::Cow, ops::Deref};

use flax::{child_of, Entity, EntityRef, FetchExt, Query, World};
use glam::{vec4, Mat4, Vec2, Vec3, Vec4};
use palette::{FromColor, Srgba};
use wgpu::{BindGroupLayout, BufferUsages, RenderPass, ShaderStages, TextureFormat};

use crate::{
    components::{children, position, shape, size},
    shapes::{Rect, Shape},
    Frame,
};

use super::{
    graphics::{
        shader::ShaderDesc, BindGroupBuilder, BindGroupLayoutBuilder, Mesh, Shader, TypedBuffer,
        Vertex, VertexDesc,
    },
    Gpu,
};

/// Draws shapes from the frame
pub struct ShapeRenderer {
    quad: Mesh,
    objects: Vec<ObjectData>,
    object_buffer: TypedBuffer<ObjectData>,
    object_bind_group: wgpu::BindGroup,
    object_bind_group_layout: wgpu::BindGroupLayout,
    shader: Shader,
}

impl ShapeRenderer {
    pub fn new(gpu: &Gpu, global_layout: &BindGroupLayout, color_format: TextureFormat) -> Self {
        let object_bind_group_layout =
            BindGroupLayoutBuilder::new("ShapeRenderer::object_bind_group_layout")
                .bind_storage_buffer(ShaderStages::VERTEX)
                .build(gpu);

        let object_buffer = TypedBuffer::new_uninit(
            gpu,
            "ShapeRenderer::object_buffer",
            BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            128,
        );

        let object_bind_group = BindGroupBuilder::new("ShapeRenderer::object_buffer")
            .bind_buffer(&object_buffer)
            .build(gpu, &object_bind_group_layout);

        let shader = Shader::new(
            gpu,
            ShaderDesc {
                label: "ShapeRenderer::shader",
                source: include_str!("../../assets/shaders/solid.wgsl").into(),
                format: color_format,
                vertex_layouts: Cow::Borrowed(&[Vertex::layout()]),
                layouts: &[global_layout, &object_bind_group_layout],
            },
        );

        Self {
            quad: Mesh::quad(gpu),
            objects: Vec::new(),
            object_buffer,
            object_bind_group_layout,
            object_bind_group,
            shader,
        }
    }

    pub fn update(&mut self, frame: &mut Frame, root: Entity) {
        let mut query = Query::new((position(), size(), shape())).topo(child_of);

        self.objects.clear();

        for (pos, size, shape) in &mut query.borrow(frame.world()) {
            match shape {
                Shape::Rect(Rect { color }) => {
                    self.objects.push(ObjectData {
                        world_matrix: Mat4::from_scale_rotation_translation(
                            size.extend(1.0),
                            Default::default(),
                            pos.extend(0.1),
                        ),
                        color: srgba_to_vec4(*color),
                    });
                }
            }
        }
    }

    pub fn draw<'a>(
        &'a mut self,
        gpu: &Gpu,
        globals_bind_group: &'a wgpu::BindGroup,
        render_pass: &mut RenderPass<'a>,
    ) -> anyhow::Result<()> {
        self.object_buffer.write(&gpu.queue, &self.objects);

        render_pass.set_pipeline(self.shader.pipeline());
        render_pass.set_bind_group(0, globals_bind_group, &[]);
        render_pass.set_bind_group(1, &self.object_bind_group, &[]);

        self.quad.bind(render_pass);

        render_pass.draw_indexed(0..6, 0, 0..self.objects.len() as u32);

        Ok(())
    }
}

fn accumulate_shapes(world: &World, id: Entity, f: &mut impl FnMut(Vec2, &Shape)) {
    let entity = world.entity(id).unwrap();
    if let Ok(shape) = entity.get(shape()) {
        let position = entity.get(position()).ok();
        (f)((position.map(|v| *v)).unwrap_or_default(), &*shape)
    }

    if let Ok(children) = entity.get(children()) {
        for &child in children.iter() {
            accumulate_shapes(world, child, f);
        }
    }
}

#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct ObjectData {
    world_matrix: Mat4,
    color: Vec4,
}

fn srgba_to_vec4(color: Srgba<u8>) -> Vec4 {
    let (r, g, b, a) = Srgba::<f32>::from_format(color)
        .into_linear()
        .into_components();

    vec4(r, g, b, a)
}
