use std::borrow::Cow;

use flax::{child_of, Entity, EntityRef, FetchExt, Query, World};
use glam::Mat4;
use wgpu::{BindGroupLayout, BufferUsages, RenderPass, ShaderStages, TextureFormat};

use crate::{
    components::{children, position, shape},
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
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
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
        let mut query =
            Query::new((position(), shape(), children().opt_or_default())).topo(child_of);

        self.objects.clear();

        for (position, shape, children) in &mut query.borrow(frame.world()) {
            match shape {
                Shape::Rect(Rect { size }) => {
                    self.objects.push(ObjectData {
                        world_matrix: Mat4::from_scale_rotation_translation(
                            size.extend(1.0),
                            Default::default(),
                            position.extend(0.1),
                        ),
                    });
                }
            }
        }

        tracing::debug!("Objects: {}", self.objects.len());
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

fn accumulate_shapes(world: &World, entity: EntityRef, res: &mut Vec<Shape>) {
    if let Ok(shape) = entity.get(shape()) {
        res.push(*shape);
    }

    if let Ok(children) = entity.get(children()) {
        for &child in children.iter() {
            let child = world.entity(child).expect("Invalid entity");
            accumulate_shapes(world, child, res);
        }
    }
}

#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct ObjectData {
    world_matrix: Mat4,
}
