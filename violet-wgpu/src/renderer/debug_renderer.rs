use std::{collections::BTreeMap, sync::Arc};

use flax::{fetch::entity_refs, Entity, Query};
use glam::{vec2, vec3, vec4, Mat4, Quat, Vec2, Vec3, Vec4};
use image::DynamicImage;
use itertools::Itertools;
use violet_core::{
    assets::Asset,
    components::{layout_args, rect, screen_clip_mask, screen_transform},
    layout::{
        cache::{layout_cache, LayoutUpdateEvent},
        Direction,
    },
    stored::{self, Handle},
    Frame, Rect,
};
use wgpu::{BindGroup, BindGroupLayout, SamplerDescriptor, ShaderStages, TextureFormat};

use crate::{
    graphics::{
        shader::ShaderDesc, texture::Texture, BindGroupBuilder, BindGroupLayoutBuilder, Shader,
        Vertex, VertexDesc,
    },
    mesh_buffer::MeshHandle,
};

use super::{
    rect_renderer::ImageFromColor, DrawCommand, ObjectData, RendererContext, RendererStore,
};

pub struct DebugRenderer {
    white_image: Asset<DynamicImage>,
    layout: BindGroupLayout,
    bind_group: Handle<BindGroup>,
    sampler: wgpu::Sampler,

    mesh: Arc<MeshHandle>,

    corner_shader: stored::Handle<Shader>,
    border_shader: stored::Handle<Shader>,
    solid_shader: stored::Handle<Shader>,

    layout_changes_rx: flume::Receiver<(Entity, LayoutUpdateEvent)>,
    layout_changes: BTreeMap<(Entity, LayoutUpdateEvent), usize>,
    objects: Vec<(DrawCommand, ObjectData)>,
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

        let corner_shader = store.shaders.insert(Shader::new(
            &ctx.gpu,
            &ShaderDesc {
                label: "ShapeRenderer::shader",
                source: include_str!("../../../assets/shaders/debug_indicator.wgsl"),
                format: color_format,
                vertex_layouts: &[Vertex::layout()],
                layouts: &[&ctx.globals_layout, &object_bind_group_layout, &layout],
            },
        ));
        let border_shader = store.shaders.insert(Shader::new(
            &ctx.gpu,
            &ShaderDesc {
                label: "ShapeRenderer::shader",
                source: include_str!("../../../assets/shaders/border_shader.wgsl"),
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
            corner_shader,
            border_shader,
            layout_changes_rx,
            layout_changes: BTreeMap::new(),
            objects: Vec::new(),
            solid_shader: todo!(),
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

        let mut overflow = Vec::new();

        let mut query = Query::new((entity_refs(), layout_args(), rect()));
        let mut query = query.borrow(&frame.world);

        query
            .iter()
            .filter_map(|(entity, &args, &rect)| {
                let diff = (rect.size() - args.overflow_limit).max(Vec2::ZERO);

                let transform = entity.get_copy(screen_transform()).ok()?;
                let clip_mask = entity.get_copy(screen_clip_mask()).ok()?;

                // let model_matrix = Mat4::from_scale_rotation_translation(
                //     screen_rect.size().extend(1.0),
                //     Quat::IDENTITY,
                //     screen_rect.pos().extend(0.2),
                // );

                let mut draw = |rect: Rect| {
                    let model_matrix = transform
                        * Mat4::from_scale_rotation_translation(
                            rect.size().extend(1.0),
                            Quat::IDENTITY,
                            rect.pos().extend(0.2),
                        );

                    let object_data = ObjectData {
                        model_matrix,
                        color: vec4(1.0, 0.0, 0.0, 0.5),
                    };

                    overflow.push((
                        DrawCommand {
                            shader: self.solid_shader.clone(),
                            bind_group: self.bind_group.clone(),
                            mesh: self.mesh.clone(),
                            index_count: 6,
                            clip_mask,
                        },
                        object_data,
                    ));
                };

                if diff.x > 0.0 {
                    // tracing::error!(%entity, %diff, ?args, %rect, "horizontal overflow detected");
                    let rect = Rect::new(
                        vec2(args.overflow_limit.x, rect.min.y),
                        vec2(rect.max.x, rect.max.y),
                    );

                    draw(rect);
                }

                if diff.y > 0.0 {
                    // tracing::error!(%entity, %diff, ?args, %rect, "vertical overflow detected");
                    let rect = Rect::new(
                        vec2(rect.min.x, args.overflow_limit.y),
                        vec2(rect.max.x, rect.max.y),
                    );

                    draw(rect);
                }

                Some(())
            })
            .for_each(|_| {});

        let mut query = Query::new((entity_refs(), layout_cache()));
        let mut query = query.borrow(&frame.world);

        let clamped_indicators = query.iter().filter_map(|(entity, v)| {
            let can_grow_vert = if v
                .get_query(Direction::Vertical)
                .iter()
                .any(|v| v.value.hints.can_grow.any())
            {
                vec3(0.5, 0.0, 0.0)
            } else {
                Vec3::ZERO
            };

            let can_grow_hor = if v
                .get_query(Direction::Horizontal)
                .iter()
                .any(|v| v.value.hints.can_grow.any())
            {
                vec3(0.0, 0.5, 0.0)
            } else {
                Vec3::ZERO
            };

            let can_grow = if v.layout().is_some_and(|v| v.value.can_grow.any()) {
                vec3(0.0, 0.0, 0.5)
            } else {
                Vec3::ZERO
            };

            let color: Vec3 = [can_grow_vert, can_grow_hor, can_grow].into_iter().sum();

            if color == Vec3::ZERO {
                None
            } else {
                Some((entity, &self.corner_shader, color.extend(1.0)))
            }
        });

        // let mut query = Query::new((entity_refs(), layout_cache()));
        // let mut query = query.borrow(&frame.world);

        // let fixed_indicators = query.iter().filter_map(|(entity, v)| {
        //     let color = if v.fixed_size() {
        //         vec4(1.0, 1.0, 0.0, 1.0)
        //     } else {
        //         return None;
        //     };

        //     Some((entity, color))
        // });

        let groups = self.layout_changes.iter().group_by(|v| v.0 .0);

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

        let objects = clamped_indicators
            .chain(objects)
            .filter_map(|(entity, shader, color)| {
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
                };

                Some((
                    DrawCommand {
                        shader: shader.clone(),
                        bind_group: self.bind_group.clone(),
                        mesh: self.mesh.clone(),
                        index_count: 6,
                        clip_mask,
                    },
                    object_data,
                ))
            })
            .chain(overflow);

        self.objects.clear();
        self.objects.extend(objects);

        // self.layout_changes.clear();
        self.layout_changes.retain(|_, lifetime| {
            *lifetime -= 1;

            *lifetime > 0
        });
    }

    pub fn draw_commands(&self) -> &[(DrawCommand, ObjectData)] {
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
