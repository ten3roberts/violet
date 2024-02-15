use std::{collections::VecDeque, sync::Arc};

use bytemuck::Zeroable;
use flax::{
    component,
    components::child_of,
    entity_ids,
    fetch::{entity_refs, nth_relation, EntityRefs, NthRelation},
    CommandBuffer, Component, EntityRef, Fetch, Query, QueryBorrow, World,
};
use glam::{vec4, Mat4, Vec4};
use itertools::Itertools;
use palette::Srgba;
use parking_lot::Mutex;
use wgpu::{BindGroup, BindGroupLayout, BufferUsages, RenderPass, ShaderStages, TextureFormat};

use violet::{
    components::{children, draw_shape},
    stored::{self, Store},
    Frame,
};

use super::{
    components::{draw_cmd, object_data},
    graphics::{BindGroupBuilder, BindGroupLayoutBuilder, Mesh, Shader, TypedBuffer},
    mesh_buffer::MeshHandle,
    rect_renderer::RectRenderer,
    renderer::RendererContext,
    text_renderer::{TextRenderer, TextSystem},
};

/// Specifies what to use when drawing a single entity
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DrawCommand {
    pub(crate) shader: stored::Handle<Shader>,
    pub(crate) bind_group: stored::Handle<BindGroup>,

    pub(crate) mesh: Arc<MeshHandle>,
    /// TODO: generate inside renderer
    pub(crate) index_count: u32,
    pub(crate) vertex_offset: i32,
}

/// Compatible draw commands are given an instance in the object buffer and merged together
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
) -> (TypedBuffer<ObjectData>, BindGroup) {
    let object_buffer = TypedBuffer::new_uninit(
        &ctx.gpu,
        "ShapeRenderer::object_buffer",
        BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        object_count,
    );

    let bind_group = BindGroupBuilder::new("ShapeRenderer::object_bind_group")
        .bind_buffer(object_buffer.buffer())
        .build(&ctx.gpu, bind_group_layout);

    (object_buffer, bind_group)
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
    object_bind_group_layout: BindGroupLayout,
}

impl WidgetRenderer {
    pub(crate) fn new(
        frame: &mut Frame,
        ctx: &mut RendererContext,
        text_system: Arc<Mutex<TextSystem>>,
        color_format: TextureFormat,
    ) -> Self {
        let object_bind_group_layout =
            BindGroupLayoutBuilder::new("ShapeRenderer::object_bind_group_layout")
                .bind_storage_buffer(ShaderStages::VERTEX)
                .build(&ctx.gpu);

        let (object_buffer, bind_group) = create_object_bindings(ctx, &object_bind_group_layout, 8);

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
                text_system,
                color_format,
                &object_bind_group_layout,
                &mut store,
            ),
            store,
            register_objects,
            object_bind_group_layout,
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

        let query = DrawQuery::new();

        self.objects.clear();

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
            let instance_index = self.objects.len() as u32;

            self.objects.push(*item.object_data);

            let draw_cmd = item.draw_cmd;
            Some(InstancedDrawCommand {
                draw_cmd: draw_cmd.clone(),
                first_instance: instance_index,
                instance_count: 1,
            })
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
        .map(|cmd| InstancedDrawCommand {
            draw_cmd: cmd.draw_cmd.clone(),
            first_instance: cmd.first_instance,
            instance_count: cmd.instance_count,
        });

        self.commands.clear();
        self.commands.extend(commands);

        if self.object_buffer.len() < self.objects.len()
            || self.object_buffer.len() > (self.objects.len() * 2).max(8)
        {
            let len = self.objects.len().next_power_of_two();
            tracing::info!(len, "resizing object buffer");

            let (object_buffer, bind_group) =
                create_object_bindings(ctx, &self.object_bind_group_layout, len);

            self.object_buffer = object_buffer;
            self.bind_group = bind_group;
        }

        self.object_buffer.write(&ctx.gpu.queue, 0, &self.objects);

        ctx.mesh_buffer.bind(render_pass);

        self.commands.iter().for_each(|cmd| {
            let shader = &cmd.draw_cmd.shader;
            let bind_group = &cmd.draw_cmd.bind_group;
            let shader = &self.store.shaders[shader];
            let bind_group = &self.store.bind_groups[bind_group];

            render_pass.set_pipeline(shader.pipeline());

            render_pass.set_bind_group(0, &ctx.globals_bind_group, &[]);
            render_pass.set_bind_group(1, &self.bind_group, &[]);
            render_pass.set_bind_group(2, bind_group, &[]);

            let mesh = &cmd.draw_cmd.mesh;
            let first_index = mesh.ib().offset() as u32;

            render_pass.draw_indexed(
                first_index..(first_index + cmd.draw_cmd.index_count),
                cmd.draw_cmd.vertex_offset + mesh.vb().offset() as i32,
                cmd.first_instance..(cmd.first_instance + cmd.instance_count),
            )
        });

        Ok(())
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
