use std::{collections::BTreeMap, sync::Arc};

use flax::Entity;
use glam::{vec2, vec3, vec4, Mat4, Quat, Vec4};
use itertools::Itertools;
use violet_core::{
    components::{rect, screen_clip_mask, screen_transform},
    layout::cache::LayoutUpdateEvent,
    stored::{self, Handle},
    Frame,
};
use wgpu::{BindGroup, BindGroupLayout, SamplerDescriptor, ShaderStages, TextureFormat};

use super::{rect_renderer::ImageFromColor, ObjectData, RendererContext, RendererStore};
use crate::{
    graphics::{
        shader::ShaderDesc, texture::Texture, BindGroupBuilder, BindGroupLayoutBuilder, Shader,
        Vertex, VertexDesc,
    },
    mesh_buffer::MeshHandle,
    renderer::ComputedDrawCommand,
};

pub struct DebugRenderer {
    bind_group: Handle<BindGroup>,

    mesh: Arc<MeshHandle>,

    border_shader: stored::Handle<Shader>,

    layout_changes_rx: flume::Receiver<(Entity, LayoutUpdateEvent)>,
    layout_changes: BTreeMap<(Entity, LayoutUpdateEvent), usize>,
    objects: Vec<(ComputedDrawCommand, ObjectData)>,
}

impl DebugRenderer {
    pub fn new(
        ctx: &mut RendererContext,
        frame: &Frame,
        color_format: TextureFormat,
        object_bind_group_layout: &BindGroupLayout,
        store: &mut RendererStore,
        layout_changes_rx: flume::Receiver<(Entity, LayoutUpdateEvent)>,
    ) -> Self {
        let layout = BindGroupLayoutBuilder::new("RectRenderer::layout")
            .bind_sampler(ShaderStages::FRAGMENT)
            .bind_texture(ShaderStages::FRAGMENT)
            .build(&ctx.gpu);

        let white_image = frame.assets.load(&ImageFromColor([255, 255, 255, 255]));
        let texture = Texture::from_image(&ctx.gpu, &white_image);

        let sampler = ctx.gpu.device.create_sampler(&SamplerDescriptor {
            label: Some("ShapeRenderer::sampler"),
            anisotropy_clamp: 16,
            mag_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group = store.bind_groups.insert(
            BindGroupBuilder::new("DebugRenderer::textured_bind_group")
                .bind_sampler(&sampler)
                .bind_texture(&texture.view(&Default::default()))
                .build(&ctx.gpu, &layout),
        );

        let vertices = [
            Vertex::new(vec3(0.0, 0.0, 0.0), Vec4::ONE, vec2(0.0, 0.0)),
            Vertex::new(vec3(1.0, 0.0, 0.0), Vec4::ONE, vec2(1.0, 0.0)),
            Vertex::new(vec3(1.0, 1.0, 0.0), Vec4::ONE, vec2(1.0, 1.0)),
            Vertex::new(vec3(0.0, 1.0, 0.0), Vec4::ONE, vec2(0.0, 1.0)),
        ];

        let indices = [0, 1, 2, 2, 3, 0];

        let mesh = Arc::new(ctx.mesh_buffer.insert(&ctx.gpu, &vertices, &indices));

        // let corner_shader = store.shaders.insert(Shader::new(
        //     &ctx.gpu,
        //     &ShaderDesc {
        //         label: "ShapeRenderer::shader",
        //         source: include_str!("../../../assets/shaders/debug_indicator.wgsl"),
        //         format: color_format,
        //         vertex_layouts: &[Vertex::layout()],
        //         layouts: &[&ctx.globals_layout, object_bind_group_layout, &layout],
        //     },
        // ));

        let border_shader = store.shaders.insert(Shader::new(
            &ctx.gpu,
            &ShaderDesc {
                label: "ShapeRenderer::shader",
                source: include_str!("../../../assets/shaders/border_shader.wgsl"),
                format: color_format,
                vertex_layouts: &[Vertex::layout()],
                layouts: &[&ctx.globals_layout, object_bind_group_layout, &layout],
            },
        ));
        Self {
            bind_group,
            mesh,
            border_shader,
            layout_changes_rx,
            layout_changes: BTreeMap::new(),
            objects: Vec::new(),
        }
    }

    pub fn update(&mut self, frame: &mut Frame) {
        puffin::profile_function!();
        self.layout_changes.extend(
            self.layout_changes_rx
                .try_iter()
                .map(|(entity, layout)| ((entity, layout), 30)),
        );

        self.objects.clear();

        let groups = self.layout_changes.iter().chunk_by(|v| v.0 .0);

        let objects = groups.into_iter().filter_map(|(id, group)| {
            let color: Vec4 = group
                .map(|((_, update), lifetime)| {
                    let opacity = (*lifetime) as f32 / 30.0;
                    indicator_color(update) * vec4(1.0, 1.0, 1.0, opacity.powi(8) * 0.5)
                })
                .sum();
            let entity = frame.world.entity(id).ok()?;

            Some((entity, &self.border_shader, color))
        });

        let objects = objects.filter_map(|(entity, shader, color)| {
            let rect = entity.get_copy(rect()).ok()?.align_to_grid();
            let transform = entity.get_copy(screen_transform()).ok()?;
            let clip_mask = entity.get_copy(screen_clip_mask()).ok()?;

            let model_matrix = transform
                * Mat4::from_scale_rotation_translation(
                    rect.size().extend(1.0),
                    Quat::IDENTITY,
                    rect.pos().extend(0.2),
                );

            let object_data = ObjectData {
                model_matrix,
                color,
                corner_radius: 0.0,
                _padding: Default::default(),
            };

            Some((
                ComputedDrawCommand {
                    shader: shader.clone(),
                    bind_group: self.bind_group.clone(),
                    mesh: self.mesh.clone(),
                    index_count: 6,
                    clip_mask,
                },
                object_data,
            ))
        });

        self.objects.clear();
        self.objects.extend(objects);

        // self.layout_changes.clear();
        self.layout_changes.retain(|_, lifetime| {
            *lifetime -= 1;

            *lifetime > 0
        });
    }

    pub fn draw_commands(&self) -> &[(ComputedDrawCommand, ObjectData)] {
        &self.objects
    }
}

fn indicator_color(layout: &LayoutUpdateEvent) -> Vec4 {
    match layout {
        LayoutUpdateEvent::Explicit => vec4(1.0, 0.0, 0.0, 1.0),
        LayoutUpdateEvent::SizeQueryUpdate => vec4(0.0, 1.0, 0.0, 1.0),
        LayoutUpdateEvent::LayoutUpdate => vec4(0.0, 0.0, 1.0, 1.0),
    }
}
