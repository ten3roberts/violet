use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use flax::Entity;
use glam::{vec2, vec3, vec4, Mat4, Quat, Vec4};
use image::DynamicImage;
use itertools::Itertools;
use palette::{num::Powi, Hsva, IntoColor};
use violet_core::{
    assets::Asset,
    components::screen_rect,
    layout::cache::LayoutUpdate,
    stored::{self, Handle},
    Frame,
};
use wgpu::{BindGroup, BindGroupLayout, SamplerDescriptor, ShaderStages, TextureFormat};

use crate::{
    graphics::{
        shader::ShaderDesc, texture::Texture, BindGroupBuilder, BindGroupLayoutBuilder, Shader,
        Vertex, VertexDesc,
    },
    mesh_buffer::MeshHandle,
    rect_renderer::ImageFromColor,
    renderer::RendererContext,
    widget_renderer::{srgba_to_vec4, DrawCommand, ObjectData, RendererStore},
};

pub struct DebugRenderer {
    white_image: Asset<DynamicImage>,
    layout: BindGroupLayout,
    bind_group: Handle<BindGroup>,
    sampler: wgpu::Sampler,

    mesh: Arc<MeshHandle>,

    shader: stored::Handle<Shader>,

    layout_changes_rx: flume::Receiver<(Entity, LayoutUpdate)>,
    layout_changes: BTreeMap<(Entity, LayoutUpdate), usize>,
    objects: Vec<(DrawCommand, ObjectData)>,
}

impl DebugRenderer {
    pub fn new(
        ctx: &mut RendererContext,
        frame: &Frame,
        color_format: TextureFormat,
        object_bind_group_layout: &BindGroupLayout,
        store: &mut RendererStore,
        layout_changes_rx: flume::Receiver<(Entity, LayoutUpdate)>,
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

        let shader = store.shaders.insert(Shader::new(
            &ctx.gpu,
            &ShaderDesc {
                label: "ShapeRenderer::shader",
                source: include_str!("../../assets/shaders/debug_indicator.wgsl"),
                format: color_format,
                vertex_layouts: &[Vertex::layout()],
                layouts: &[&ctx.globals_layout, &object_bind_group_layout, &layout],
            },
        ));
        Self {
            white_image,
            layout,
            bind_group,
            sampler,
            mesh,
            shader,
            layout_changes_rx,
            layout_changes: BTreeMap::new(),
            objects: Vec::new(),
        }
    }

    pub fn update(&mut self, frame: &mut Frame) {
        self.layout_changes.extend(
            self.layout_changes_rx
                .try_iter()
                .map(|(entity, layout)| ((entity, layout), 60)),
        );

        self.objects.clear();

        let mut index = 0;
        self.layout_changes.retain(|(id, layout), lifetime| {
            *lifetime -= 1;

            *lifetime > 0 && frame.world.has(*id, screen_rect())
        });
        let groups = self.layout_changes.iter().group_by(|v| v.0 .0);

        let objects = groups.into_iter().filter_map(|(id, group)| {
            let color: Vec4 = group
                .map(|((_, update), lifetime)| {
                    let opacity = *lifetime as f32 / 60.0;
                    indicator_color(update) * vec4(1.0, 1.0, 1.0, opacity.powi(8))
                })
                .sum();

            let entity = frame.world.entity(id).ok()?;

            let screen_rect = entity.get(screen_rect()).ok()?;

            let model_matrix = Mat4::from_scale_rotation_translation(
                screen_rect.size().extend(1.0),
                Quat::IDENTITY,
                screen_rect.pos().extend(0.2),
            );

            let object_data = ObjectData {
                model_matrix,
                color,
            };

            Some((
                DrawCommand {
                    shader: self.shader.clone(),
                    bind_group: self.bind_group.clone(),
                    mesh: self.mesh.clone(),
                    index_count: 6,
                },
                object_data,
            ))
        });

        self.objects.clear();
        self.objects.extend(objects);
    }

    pub fn draw_commands(&self) -> &[(DrawCommand, ObjectData)] {
        &self.objects
    }
}

fn indicator_color(layout: &LayoutUpdate) -> Vec4 {
    match layout {
        LayoutUpdate::Explicit => vec4(1.0, 0.0, 0.0, 1.0),
        LayoutUpdate::SizeQueryUpdate => vec4(0.0, 1.0, 0.0, 1.0),
        LayoutUpdate::LayoutUpdate => vec4(0.0, 0.0, 1.0, 1.0),
    }
}
