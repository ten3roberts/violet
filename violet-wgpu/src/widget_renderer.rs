use std::{collections::VecDeque, sync::Arc};

use bytemuck::Zeroable;
use flax::{
    component,
    components::child_of,
    entity_ids,
    fetch::{entity_refs, nth_relation, EntityRefs, NthRelation},
    CommandBuffer, Component, Entity, EntityRef, Fetch, Query, QueryBorrow, World,
};
use glam::{vec4, Mat4, Vec4};
use itertools::Itertools;
use palette::Srgba;
use parking_lot::Mutex;
use wgpu::{BindGroup, BindGroupLayout, BufferUsages, RenderPass, ShaderStages, TextureFormat};

use violet_core::{
    components::{children, draw_shape},
    layout::cache::LayoutUpdate,
    stored::{self, Store},
    Frame,
};

use crate::debug_renderer::DebugRenderer;

use super::{
    components::{draw_cmd, object_data},
    graphics::{BindGroupBuilder, BindGroupLayoutBuilder, Mesh, Shader, TypedBuffer},
    mesh_buffer::MeshHandle,
    rect_renderer::RectRenderer,
    renderer::RendererContext,
    text_renderer::{TextRenderer, TextSystem},
};

const CHUNK_SIZE: usize = 32;

/// Specifies what to use when drawing a single entity
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DrawCommand {
    pub(crate) shader: stored::Handle<Shader>,
    pub(crate) bind_group: stored::Handle<BindGroup>,

    pub(crate) mesh: Arc<MeshHandle>,
    /// TODO: generate inside renderer
    pub(crate) index_count: u32,
}

/// Compatible draw commands are given an instance in the object buffer and merged together
#[derive(Debug)]
struct InstancedDrawCommand {
    draw_cmd: DrawCommand,
    first_instance: u32,
    instance_count: u32,
}

component! {
    draw_cmd_id: usize,
}

#[derive(Debug, Default)]
pub struct RendererStore {
    pub bind_groups: Store<BindGroup>,
    pub shaders: Store<Shader>,
}

#[derive(Fetch)]
#[fetch(item_derives = [ Debug ])]
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

fn create_object_bindings(
    ctx: &mut RendererContext,
    bind_group_layout: &BindGroupLayout,
    object_count: usize,
) -> (BindGroup, TypedBuffer<ObjectData>) {
    let object_buffer = TypedBuffer::new_uninit(
        &ctx.gpu,
        "ShapeRenderer::object_buffer",
        BufferUsages::UNIFORM | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        object_count,
    );

    let bind_group = BindGroupBuilder::new("ShapeRenderer::object_bind_group")
        .bind_buffer(object_buffer.buffer())
        .build(&ctx.gpu, bind_group_layout);

    (bind_group, object_buffer)
}

/// Draws shapes from the frame
pub struct WidgetRenderer {
    store: RendererStore,
    quad: Mesh,
    object_data: Vec<ObjectData>,
    object_buffers: Vec<(BindGroup, TypedBuffer<ObjectData>)>,

    register_objects: flax::system::BoxedSystem,

    commands: Vec<(usize, InstancedDrawCommand)>,

    rect_renderer: RectRenderer,
    text_renderer: TextRenderer,
    debug_renderer: DebugRenderer,

    object_bind_group_layout: BindGroupLayout,
}

impl WidgetRenderer {
    pub(crate) fn new(
        frame: &mut Frame,
        ctx: &mut RendererContext,
        text_system: Arc<Mutex<TextSystem>>,
        color_format: TextureFormat,
        layout_changes_rx: flume::Receiver<(Entity, LayoutUpdate)>,
    ) -> Self {
        let object_bind_group_layout =
            BindGroupLayoutBuilder::new("ShapeRenderer::object_bind_group_layout")
                .bind_uniform_buffer(ShaderStages::VERTEX)
                .build(&ctx.gpu);

        let register_objects = flax::system::System::builder()
            .with_cmd_mut()
            .with_query(Query::new(entity_ids()).with_relation(draw_shape))
            .build(|cmd: &mut CommandBuffer, mut query: QueryBorrow<_, _>| {
                (&mut query).into_iter().for_each(|id| {
                    cmd.set(id, object_data(), ObjectData::zeroed());
                });
            })
            .boxed();

        let mut store = RendererStore::default();

        Self {
            quad: Mesh::quad(&ctx.gpu),
            object_data: Vec::new(),
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
                text_system,
                color_format,
                &object_bind_group_layout,
                &mut store,
            ),
            debug_renderer: DebugRenderer::new(
                ctx,
                frame,
                color_format,
                &object_bind_group_layout,
                &mut store,
                layout_changes_rx,
            ),
            store,
            register_objects,
            object_bind_group_layout,
            object_buffers: Vec::new(),
        }
    }

    pub fn draw<'a>(
        &'a mut self,
        ctx: &'a mut RendererContext,
        frame: &mut Frame,
        render_pass: &mut RenderPass<'a>,
    ) -> anyhow::Result<()> {
        let _span = tracing::info_span!("draw").entered();
        self.quad.bind(render_pass);

        self.register_objects.run(&mut frame.world)?;

        self.rect_renderer.update(&ctx.gpu, frame);
        self.rect_renderer
            .build_commands(&ctx.gpu, frame, &mut self.store);

        self.text_renderer
            .update_meshes(ctx, frame, &mut self.store);
        self.text_renderer.update(&ctx.gpu, frame);
        self.debug_renderer.update(frame);

        let query = DrawQuery::new();

        let roots = Query::new(entity_ids())
            .without_relation(child_of)
            .borrow(&frame.world)
            .iter()
            .map(|id| frame.world.entity(id).unwrap())
            .collect();

        let commands = RendererIter {
            world: &frame.world,
            queue: roots,
        }
        .filter_map(|entity| {
            let mut query = entity.query(&query);
            let item = query.get()?;

            Some((item.draw_cmd.clone(), *item.object_data))
        })
        .chain(self.debug_renderer.draw_commands().iter().cloned());

        self.commands.clear();
        self.object_data.clear();

        collect_draw_commands(commands, &mut self.object_data, &mut self.commands);

        let num_chunks = self.object_data.len().div_ceil(CHUNK_SIZE);

        if num_chunks > self.object_buffers.len() {
            self.object_buffers.extend(
                (self.object_buffers.len()..num_chunks).map(|_| {
                    create_object_bindings(ctx, &self.object_bind_group_layout, CHUNK_SIZE)
                }),
            )
        }

        for (objects, (_, buffer)) in self
            .object_data
            .chunks(CHUNK_SIZE)
            .zip(&self.object_buffers)
        {
            buffer.write(&ctx.gpu.queue, 0, objects);
        }

        ctx.mesh_buffer.bind(render_pass);

        self.commands.iter().for_each(|(chunk_index, cmd)| {
            let chunk = &self.object_buffers[*chunk_index];
            let shader = &cmd.draw_cmd.shader;
            let bind_group = &cmd.draw_cmd.bind_group;
            let shader = &self.store.shaders[shader];
            let bind_group = &self.store.bind_groups[bind_group];

            render_pass.set_pipeline(shader.pipeline());

            render_pass.set_bind_group(0, &ctx.globals_bind_group, &[]);
            render_pass.set_bind_group(1, &chunk.0, &[]);
            render_pass.set_bind_group(2, bind_group, &[]);

            let mesh = &cmd.draw_cmd.mesh;
            let first_index = mesh.ib().offset() as u32;

            render_pass.draw_indexed(
                first_index..(first_index + cmd.draw_cmd.index_count),
                0,
                cmd.first_instance..(cmd.first_instance + cmd.instance_count),
            )
        });

        Ok(())
    }
}

fn collect_draw_commands<'a>(
    entities: impl Iterator<Item = (DrawCommand, ObjectData)>,
    objects: &mut Vec<ObjectData>,
    draw_cmds: &mut Vec<(usize, InstancedDrawCommand)>,
) {
    let chunks = entities.chunks(CHUNK_SIZE);

    for (chunk_index, chunk) in (&chunks).into_iter().enumerate() {
        let iter = chunk
            .enumerate()
            .map(|(i, (draw_cmd, object))| {
                objects.push(object);
                // let first_instance = instance_index as u32;
                // instance_index += 1;
                // objects.push(*item.object_data);

                // let draw_cmd = item.draw_cmd;
                InstancedDrawCommand {
                    draw_cmd: draw_cmd.clone(),
                    first_instance: i as u32,
                    instance_count: 1,
                }
            })
            .coalesce(|prev, current| {
                if prev.draw_cmd == current.draw_cmd {
                    assert!(prev.first_instance + prev.instance_count == current.first_instance);
                    Ok(InstancedDrawCommand {
                        draw_cmd: prev.draw_cmd,
                        first_instance: prev.first_instance,
                        instance_count: prev.instance_count + 1,
                    })
                } else {
                    Err((prev, current))
                }
            })
            .map(move |cmd| (chunk_index, cmd));

        draw_cmds.extend(iter);
    }
}

#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
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

struct RendererIter<'a> {
    world: &'a World,
    queue: VecDeque<EntityRef<'a>>,
}

impl<'a> Iterator for RendererIter<'a> {
    type Item = EntityRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let entity = self.queue.pop_front()?;
        if let Ok(children) = entity.get(children()) {
            self.queue
                .extend(children.iter().map(|&id| self.world.entity(id).unwrap()));
        }

        Some(entity)
    }
}
