use std::sync::Arc;

use bytemuck::Zeroable;
use cosmic_text::FontSystem;
use flax::{
    component,
    components::child_of,
    entity_ids,
    fetch::{entity_refs, nth_relation, EntityRefs, NthRelation},
    CommandBuffer, Component, EntityRef, Fetch, FetchExt, Opt, OptOr, Query, QueryBorrow, System,
};
use glam::{vec4, Mat4, Vec4};
use image::DynamicImage;
use itertools::Itertools;
use palette::Srgba;
use slab::Slab;
use slotmap::{new_key_type, SlotMap};
use wgpu::{BindGroup, BufferUsages, RenderPass, ShaderStages, TextureFormat};

use crate::{
    assets::{map::HandleMap, Handle},
    components::{color, draw_shape},
    shapes::Shape,
    Frame,
};

use super::{
    components::{draw_cmd, mesh_handle, model_matrix, object_data},
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
    pub(crate) shader: usize,
    pub(crate) mesh: Arc<MeshHandle>,
    /// TODO: generate inside renderer
    pub(crate) bind_group: usize,

    pub(crate) index_count: u32,
    pub(crate) vertex_offset: i32,
}

/// Compatible draw commands are given an instance in the object buffer and merged together
struct InstancedDrawCommand {
    cmd: DrawCommand,
    first_instance: u32,
    instance_count: u32,
}

pub(crate) struct InstancedDrawCommandRef<'a> {
    cmd: &'a DrawCommand,

    pub(crate) first_instance: u32,
    pub(crate) instance_count: u32,
}

component! {
    draw_cmd_id: usize,
}

#[derive(Debug, Default)]
pub struct RendererStore {
    pub bind_groups: Slab<BindGroup>,
    pub shaders: Slab<Shader>,
}

impl RendererStore {
    pub fn push_bind_group(&mut self, bind_group: BindGroup) -> usize {
        self.bind_groups.insert(bind_group)
    }

    pub fn push_shader(&mut self, shader: Shader) -> usize {
        self.shaders.insert(shader)
    }
}

#[derive(Fetch)]
pub(crate) struct DrawQuery {
    pub(crate) entity: EntityRefs,
    pub(crate) object_data: Component<ObjectData>,
    pub(crate) shape: NthRelation<()>,
    pub(crate) draw_cmd: Component<DrawCommand>,
}

impl DrawQuery {
    pub fn new() -> Self {
        Self {
            entity: entity_refs(),
            object_data: object_data(),
            shape: nth_relation(draw_shape, 0),
            draw_cmd: draw_cmd(),
        }
    }
}

/// Draws shapes from the frame
pub struct WidgetRenderer {
    store: RendererStore,
    quad: Mesh,
    objects: Vec<ObjectData>,
    object_buffer: TypedBuffer<ObjectData>,
    bind_group: wgpu::BindGroup,

    register_objects: flax::system::BoxedSystem,

    commands: Vec<InstancedDrawCommand>,

    rect_renderer: RectRenderer,
    text_renderer: TextRenderer,
}

impl WidgetRenderer {
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

        let register_objects = flax::system::System::builder()
            .with_cmd_mut()
            .with_query(Query::new(entity_ids()).with_relation(draw_shape))
            .build(|cmd: &mut CommandBuffer, mut query: QueryBorrow<_, _>| {
                (&mut query).into_iter().for_each(|id| {
                    cmd.set(id, object_data(), ObjectData::zeroed());
                });
            })
            .boxed();

        // let solid_layout = BindGroupLayoutBuilder::new("RectRenderer::layout")
        //     .bind_sampler(ShaderStages::FRAGMENT)
        //     .bind_texture(ShaderStages::FRAGMENT)
        //     .build(&ctx.gpu);

        let mut store = RendererStore::default();

        Self {
            quad: Mesh::quad(&ctx.gpu),
            objects: Vec::new(),
            object_buffer,
            bind_group,
            commands: Vec::new(),
            rect_renderer: RectRenderer::new(
                ctx,
                frame,
                color_format,
                &object_bind_group_layout,
                &mut store,
            ),
            text_renderer: TextRenderer::new(
                ctx,
                frame,
                font_system,
                color_format,
                &object_bind_group_layout,
            ),
            store,
            register_objects,
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

        self.register_objects.run(&mut frame.world)?;

        self.rect_renderer.update(&ctx.gpu, frame);
        self.rect_renderer
            .build_commands(&ctx.gpu, frame, &mut self.store);

        self.text_renderer.update_meshes(ctx, frame);
        self.text_renderer.update(&ctx.gpu, frame);

        let mut query = Query::new(DrawQuery::new()).topo(child_of);

        let mut query = query.borrow(&frame.world);

        self.objects.clear();

        let commands = query
            .iter()
            .map(|item| {
                let instance_index = self.objects.len() as u32;

                self.objects.push(*item.object_data);

                let draw_cmd = item.draw_cmd;
                // tracing::info!(?mesh, "drawing");
                InstancedDrawCommandRef {
                    cmd: draw_cmd,
                    first_instance: instance_index,
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

        ctx.mesh_buffer.bind(render_pass);

        self.commands.iter().for_each(|instanced_cmd| {
            let cmd = &instanced_cmd.cmd;

            let shader = &self.store.shaders[cmd.shader];
            let bind_group = &self.store.bind_groups[cmd.bind_group];

            render_pass.set_pipeline(shader.pipeline());

            render_pass.set_bind_group(0, &ctx.globals_bind_group, &[]);
            render_pass.set_bind_group(1, &self.bind_group, &[]);
            render_pass.set_bind_group(2, bind_group, &[]);

            let mesh = &instanced_cmd.cmd.mesh;
            let first_index = mesh.ib().offset() as u32;

            render_pass.draw_indexed(
                first_index..(first_index + cmd.index_count),
                cmd.vertex_offset + mesh.vb().offset() as i32,
                instanced_cmd.first_instance
                    ..(instanced_cmd.first_instance + instanced_cmd.instance_count),
            )
        });

        Ok(())
    }
}

#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
/// Per object uniform data
pub(crate) struct ObjectData {
    pub(crate) model_matrix: Mat4,
    pub(crate) color: Vec4,
}

pub fn srgba_to_vec4(color: Srgba) -> Vec4 {
    let (r, g, b, a) = color.into_linear().into_components();

    vec4(r, g, b, a)
}
