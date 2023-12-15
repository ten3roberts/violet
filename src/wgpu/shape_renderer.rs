use std::sync::Arc;

use cosmic_text::FontSystem;
use flax::{components::child_of, FetchExt, Query};
use glam::{vec4, Mat4, Vec4};
use itertools::Itertools;
use palette::Srgba;
use slotmap::new_key_type;
use wgpu::{BindGroup, BufferUsages, RenderPass, ShaderStages, TextureFormat};

use crate::{assets::Handle, components::color, Frame};

use super::{
    components::{draw_cmd, mesh_handle, model_matrix},
    graphics::{BindGroupBuilder, BindGroupLayoutBuilder, Mesh, Shader, TypedBuffer},
    mesh_buffer::MeshHandle,
    rect_renderer::RectRenderer,
    renderer::RendererContext,
    text_renderer::TextRenderer,
};

new_key_type! {
    pub struct MeshKey;
    pub struct BindGroupKey;
}

/// Specifies what to use when drawing a single entity
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DrawCommand {
    pub(crate) shader: Handle<Shader>,
    /// TODO: generate inside renderer
    pub(crate) bind_group: Handle<BindGroup>,
    pub(crate) index_count: u32,
    pub(crate) vertex_offset: i32,
}

/// Compatible draw commands are given an instance in the object buffer and merged together
struct InstancedDrawCommand {
    cmd: DrawCommand,
    mesh: Arc<MeshHandle>,
    first_instance: u32,
    instance_count: u32,
}

struct InstancedDrawCommandRef<'a> {
    cmd: &'a DrawCommand,
    mesh: &'a Arc<MeshHandle>,
    first_instance: u32,
    instance_count: u32,
}

/// Draws shapes from the frame
pub struct ShapeRenderer {
    quad: Mesh,
    objects: Vec<ObjectData>,
    object_buffer: TypedBuffer<ObjectData>,
    bind_group: wgpu::BindGroup,

    commands: Vec<InstancedDrawCommand>,

    rect_renderer: RectRenderer,
    text_renderer: TextRenderer,
}

impl ShapeRenderer {
    pub fn new(
        frame: &mut Frame,
        ctx: &mut RendererContext,
        font_system: Arc<parking_lot::Mutex<FontSystem>>,
        color_format: TextureFormat,
    ) -> Self {
        let object_bind_group_layout =
            BindGroupLayoutBuilder::new("ShapeRenderer::object_bind_group_layout")
                .bind_storage_buffer(ShaderStages::VERTEX)
                .build(&ctx.gpu);

        let object_buffer = TypedBuffer::new_uninit(
            &ctx.gpu,
            "ShapeRenderer::object_buffer",
            BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            128,
        );

        let bind_group = BindGroupBuilder::new("ShapeRenderer::object_bind_group")
            .bind_buffer(object_buffer.buffer())
            .build(&ctx.gpu, &object_bind_group_layout);

        // let solid_layout = BindGroupLayoutBuilder::new("RectRenderer::layout")
        //     .bind_sampler(ShaderStages::FRAGMENT)
        //     .bind_texture(ShaderStages::FRAGMENT)
        //     .build(&ctx.gpu);

        Self {
            quad: Mesh::quad(&ctx.gpu),
            objects: Vec::new(),
            object_buffer,
            bind_group,
            commands: Vec::new(),
            rect_renderer: RectRenderer::new(ctx, frame, color_format, &object_bind_group_layout),
            text_renderer: TextRenderer::new(
                ctx,
                frame,
                font_system,
                color_format,
                &object_bind_group_layout,
            ),
        }
    }

    pub fn draw<'a>(
        &'a mut self,
        ctx: &'a mut RendererContext,
        frame: &mut Frame,
        render_pass: &mut RenderPass<'a>,
    ) -> anyhow::Result<()> {
        self.object_buffer.write(&ctx.gpu.queue, 0, &self.objects);

        self.quad.bind(render_pass);

        self.rect_renderer.update(&ctx.gpu, frame);
        self.rect_renderer.build_commands(&ctx.gpu, frame);

        self.text_renderer.update_meshes(ctx, frame);
        self.text_renderer.update(&ctx.gpu, frame);

        let mut query = Query::new((
            color().opt_or(Srgba::new(1.0, 1.0, 1.0, 1.0)),
            mesh_handle(),
            model_matrix(),
            draw_cmd(),
        ))
        .topo(child_of);

        let mut query = query.borrow(&frame.world);

        self.objects.clear();

        let commands = query
            .iter()
            .map(|(&color, mesh, &model, cmd)| {
                let instance = self.objects.len() as u32;

                self.objects.push(ObjectData {
                    model_matrix: model,
                    color: srgba_to_vec4(color),
                });

                // tracing::info!(?mesh, "drawing");
                InstancedDrawCommandRef {
                    cmd,
                    mesh,
                    first_instance: instance,
                    instance_count: 1,
                }
            })
            .coalesce(|prev, current| {
                if prev.cmd == current.cmd && prev.mesh == current.mesh {
                    assert!(prev.first_instance + prev.instance_count == current.first_instance);
                    Ok(InstancedDrawCommandRef {
                        cmd: prev.cmd,
                        mesh: prev.mesh,
                        first_instance: prev.first_instance,
                        instance_count: prev.instance_count + 1,
                    })
                } else {
                    Err((prev, current))
                }
            })
            .map(|cmd| InstancedDrawCommand {
                cmd: cmd.cmd.clone(),
                mesh: cmd.mesh.clone(),
                first_instance: cmd.first_instance,
                instance_count: cmd.instance_count,
            });

        self.commands.clear();
        self.commands.extend(commands);

        ctx.mesh_buffer.bind(render_pass);

        self.commands.iter().for_each(|instanced_cmd| {
            let cmd = &instanced_cmd.cmd;

            render_pass.set_pipeline(cmd.shader.pipeline());

            render_pass.set_bind_group(0, &ctx.globals_bind_group, &[]);
            render_pass.set_bind_group(1, &self.bind_group, &[]);
            render_pass.set_bind_group(2, &cmd.bind_group, &[]);

            let first_index = instanced_cmd.mesh.ib().offset() as u32;

            render_pass.draw_indexed(
                first_index..(first_index + cmd.index_count),
                cmd.vertex_offset + instanced_cmd.mesh.vb().offset() as i32,
                instanced_cmd.first_instance
                    ..(instanced_cmd.first_instance + instanced_cmd.instance_count),
            )
        });

        Ok(())
    }
}

#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct ObjectData {
    model_matrix: Mat4,
    color: Vec4,
}

pub fn srgba_to_vec4(color: Srgba) -> Vec4 {
    let (r, g, b, a) = color.into_linear().into_components();

    vec4(r, g, b, a)
}
