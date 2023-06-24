use std::{borrow::Cow, collections::HashMap};

use flax::{child_of, Entity, FetchExt, Query, World};
use glam::{vec4, Mat4, Vec4};
use image::{DynamicImage, ImageBuffer};
use itertools::Itertools;
use palette::Srgba;
use slotmap::{new_key_type, SlotMap};
use wgpu::{
    BindGroup, BindGroupLayout, BufferUsages, RenderPass, Sampler, SamplerDescriptor, ShaderStages,
    TextureFormat,
};

use crate::{
    assets::{map::HandleMap, Handle},
    components::{children, color, local_position, rect, screen_position, Rect},
    shapes::{FilledRect, Shape},
    Frame,
};

use super::{
    components::{draw_cmd, model_matrix},
    graphics::{
        shader::ShaderDesc, texture::Texture, BindGroupBuilder, BindGroupLayoutBuilder, Mesh,
        Shader, TypedBuffer, Vertex, VertexDesc,
    },
    rect_renderer::RectRenderer,
    Gpu,
};

new_key_type! {
    pub struct MeshKey;
    pub struct BindGroupKey;
}

struct DrawResources {}

#[derive(Debug, PartialEq)]
pub(crate) enum DrawShape {
    Rect {
        fill_image: Handle<DynamicImage>,
    },
    /// Draws an arbitrary mesh
    Mesh {
        mesh: Handle<Mesh>,
        first_index: u32,
        index_count: u32,
    },
}

/// Specifies what to use when drawing a single entity
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DrawCommand {
    pub(crate) mesh: Handle<Mesh>,
    pub(crate) bind_group: Handle<BindGroup>,
}

/// Compatible draw commands are given an instance in the object buffer and merged together
struct InstancedDrawCommand {
    cmd: DrawCommand,
    first_instance: u32,
    instance_count: u32,
}

struct InstancedDrawCommandRef<'a> {
    cmd: &'a DrawCommand,
    first_instance: u32,
    instance_count: u32,
}

/// Draws shapes from the frame
pub struct ShapeRenderer {
    quad: Mesh,
    objects: Vec<ObjectData>,
    object_buffer: TypedBuffer<ObjectData>,
    object_bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    shader: Shader,

    commands: Vec<InstancedDrawCommand>,

    rect_renderer: RectRenderer,
}

impl ShapeRenderer {
    pub fn new(
        gpu: &Gpu,
        frame: &mut Frame,
        global_layout: &BindGroupLayout,
        color_format: TextureFormat,
    ) -> Self {
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

        let bind_group = BindGroupBuilder::new("ShapeRenderer::object_bind_group")
            .bind_buffer(&object_buffer)
            .build(gpu, &object_bind_group_layout);

        let solid_layout = BindGroupLayoutBuilder::new("RectRenderer::layout")
            .bind_sampler(ShaderStages::FRAGMENT)
            .bind_texture(ShaderStages::FRAGMENT)
            .build(gpu);

        let shader = Shader::new(
            gpu,
            ShaderDesc {
                label: "ShapeRenderer::shader",
                source: include_str!("../../assets/shaders/solid.wgsl").into(),
                format: color_format,
                vertex_layouts: Cow::Borrowed(&[Vertex::layout()]),
                layouts: &[global_layout, &object_bind_group_layout, &solid_layout],
            },
        );

        Self {
            quad: Mesh::quad(gpu),
            objects: Vec::new(),
            object_buffer,
            object_bind_group_layout,
            bind_group,
            shader,
            commands: Vec::new(),
            rect_renderer: RectRenderer::new(gpu, frame),
        }
    }

    pub fn draw<'a>(
        &'a mut self,
        gpu: &Gpu,
        frame: &mut Frame,
        globals_bind_group: &'a wgpu::BindGroup,
        render_pass: &mut RenderPass<'a>,
    ) -> anyhow::Result<()> {
        self.object_buffer.write(&gpu.queue, &self.objects);

        render_pass.set_pipeline(self.shader.pipeline());
        render_pass.set_bind_group(0, globals_bind_group, &[]);

        self.quad.bind(render_pass);

        // tracing::info!("Draw commands: {}", self.commands.len());

        self.rect_renderer.update(gpu, frame);
        self.rect_renderer.build_commands(gpu, frame);

        let mut query = Query::new((
            color().opt_or(Srgba::new(1.0, 1.0, 1.0, 1.0)),
            model_matrix(),
            draw_cmd(),
        ))
        .topo(child_of);
        let mut query = query.borrow(&frame.world);

        self.objects.clear();

        let commands = query
            .iter()
            .map(|(&color, &model, cmd)| {
                let instance = self.objects.len() as u32;

                self.objects.push(ObjectData {
                    model_matrix: model,
                    color: srgba_to_vec4(color),
                });

                InstancedDrawCommandRef {
                    cmd,
                    first_instance: instance,
                    instance_count: 1,
                }
            })
            .coalesce(|prev, current| {
                if prev.cmd == current.cmd {
                    assert!(prev.first_instance + prev.instance_count == current.first_instance);
                    Ok(InstancedDrawCommandRef {
                        cmd: prev.cmd,
                        first_instance: prev.first_instance,
                        instance_count: prev.instance_count + 1,
                    })
                } else {
                    Err((prev, current))
                }
            })
            .map(|cmd| InstancedDrawCommand {
                cmd: cmd.cmd.clone(),
                first_instance: cmd.first_instance,
                instance_count: cmd.instance_count,
            });

        self.commands.clear();
        self.commands.extend(commands);

        self.commands.iter().for_each(|instanced_cmd| {
            let cmd = &instanced_cmd.cmd;
            cmd.mesh.bind(render_pass);

            render_pass.set_bind_group(1, &self.bind_group, &[]);
            render_pass.set_bind_group(2, &cmd.bind_group, &[]);

            render_pass.draw_indexed(
                0..6,
                0,
                instanced_cmd.first_instance
                    ..(instanced_cmd.first_instance + instanced_cmd.instance_count),
            )
        });

        // for cmd in query.iter() {
        //     let bind_group = self.bind_groups.get(&cmd.fill_image).unwrap();
        //     match &cmd.shape {
        //         DrawShape::Mesh {
        //             mesh,
        //             first_index,
        //             index_count,
        //         } => {
        //             mesh.bind(render_pass);
        //             render_pass.set_bind_group(1, bind_group, &[]);
        //
        //             render_pass.draw_indexed(
        //                 *first_index..(*first_index + *index_count),
        //                 0,
        //                 cmd.first_instance..(cmd.first_instance + cmd.count),
        //             )
        //         }
        //         DrawShape::Rect => {
        //             // tracing::debug!(
        //             //     "Drawing instances {}..{}",
        //             //     cmd.first_instance,
        //             //     cmd.first_instance + cmd.count
        //             // );
        //             render_pass.set_bind_group(1, bind_group, &[]);
        //
        //             render_pass.draw_indexed(
        //                 0..6,
        //                 0,
        //                 cmd.first_instance..(cmd.first_instance + cmd.count),
        //             )
        //         }
        //     }
        // }

        Ok(())
    }
}

#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct ObjectData {
    model_matrix: Mat4,
    color: Vec4,
}

fn srgba_to_vec4(color: Srgba) -> Vec4 {
    let (r, g, b, a) = color.into_linear().into_components();

    vec4(r, g, b, a)
}
