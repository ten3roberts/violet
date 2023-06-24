use std::collections::{BTreeSet, HashMap};

use flax::{entity_ids, All, And, CommandBuffer, Component, Mutable, Query, With};
use fontdue::layout::TextStyle;
use glam::{vec2, vec3, Mat4, Quat, Vec2, Vec3};
use itertools::Itertools;
use wgpu::{BindGroup, BindGroupLayout, SamplerDescriptor, ShaderStages, TextureFormat};

use crate::{
    assets::Handle,
    components::{rect, screen_position, text, Rect},
    wgpu::{
        font::FontAtlas,
        graphics::{BindGroupBuilder, Mesh, Vertex2d},
        shape_renderer::DrawCommand,
    },
    Frame,
};

use super::{
    components::{draw_cmd, font_from_file, model_matrix},
    font::{Font, FontFromFile},
    graphics::{shader::ShaderDesc, BindGroupLayoutBuilder, Shader, Vertex, VertexDesc},
    Gpu,
};

pub struct TextRenderer {
    fonts: HashMap<FontFromFile, (FontAtlas, Handle<BindGroup>)>,
    shader: Handle<Shader>,
    text_layout: BindGroupLayout,

    object_query: Query<(Component<Rect>, Component<Vec2>, Mutable<Mat4>), And<All, With>>,
}

impl TextRenderer {
    pub fn new(
        gpu: &Gpu,
        frame: &mut Frame,
        color_format: TextureFormat,
        global_layout: &BindGroupLayout,
        object_bind_group_layout: &BindGroupLayout,
    ) -> Self {
        let text_layout = BindGroupLayoutBuilder::new("TextRenderer::text_layout")
            .bind_sampler(ShaderStages::FRAGMENT)
            .bind_texture(ShaderStages::FRAGMENT)
            .build(&gpu);

        let shader = frame.assets.insert(Shader::new(
            gpu,
            &ShaderDesc {
                label: "ShapeRenderer::shader",
                source: include_str!("../../assets/shaders/solid.wgsl").into(),
                format: color_format,
                vertex_layouts: &[Vertex::layout()],
                layouts: &[global_layout, object_bind_group_layout, &text_layout],
            },
        ));

        Self {
            fonts: HashMap::new(),
            text_layout,
            shader,
            object_query: Query::new((rect(), screen_position(), model_matrix().as_mut()))
                .with(text()),
        }
    }
    pub fn update_text_meshes(&mut self, gpu: &Gpu, frame: &mut Frame) {
        let mut query = Query::new((entity_ids(), font_from_file(), text()));

        let mut cmd = CommandBuffer::new();

        for (id, font_from_file, text) in &mut query.borrow(frame.world()) {
            tracing::info!("Updating mesh for text {id}");

            let font = frame.assets.load(font_from_file);

            // Create a bind group with the bound font atlas
            let (font_atlas, bind_group) = self
                .fonts
                .entry(font_from_file.clone())
                .or_insert_with_key(|key| {
                    let font_atlas = FontAtlas::new(
                        &frame.assets,
                        gpu,
                        &font,
                        96.0,
                        text.chars().collect::<BTreeSet<_>>(),
                    )
                    .unwrap();

                    let sampler = gpu.device.create_sampler(&SamplerDescriptor {
                        label: Some("ShapeRenderer::sampler"),
                        anisotropy_clamp: 16,
                        mag_filter: wgpu::FilterMode::Linear,
                        mipmap_filter: wgpu::FilterMode::Linear,
                        min_filter: wgpu::FilterMode::Linear,

                        ..Default::default()
                    });

                    let bind_group = BindGroupBuilder::new("TextRenderer::bind_group")
                        .bind_sampler(&sampler)
                        .bind_texture(&font_atlas.texture.view(&Default::default()))
                        .build(gpu, &self.text_layout);

                    (font_atlas, frame.assets.insert(bind_group))
                });

            let mut layout = fontdue::layout::Layout::<()>::new(
                fontdue::layout::CoordinateSystem::PositiveYDown,
            );

            layout.append(
                &[&font.font],
                &TextStyle {
                    text,
                    px: 96.0,
                    font_index: 0,
                    user_data: (),
                },
            );

            let glyph_count = layout.glyphs().len();

            let vertices = layout
                .glyphs()
                .iter()
                .flat_map(|glyph| {
                    tracing::info!("Glyph: {:?}", glyph);

                    let atlas_glyph = font_atlas.glyphs.get(&glyph.key.glyph_index).unwrap();

                    let glyph_width = glyph.width as f32;
                    let glyph_height = glyph.height as f32;

                    let atlas_size = font_atlas.texture.size();
                    let atlas_size = vec2(atlas_size.width as f32, atlas_size.height as f32);

                    let uv_min = atlas_glyph.min / atlas_size;
                    let uv_max = atlas_glyph.max / atlas_size;

                    tracing::debug!(?glyph.x, glyph_width, ?glyph_height, "Glyph");

                    [
                        // Bottom left
                        Vertex::new(
                            vec3(glyph.x, glyph.y + glyph_height, 0.0),
                            vec2(uv_min.x, uv_max.y),
                        ),
                        Vertex::new(
                            vec3(glyph.x + glyph_width, glyph.y + glyph_height, 0.0),
                            vec2(uv_max.x, uv_max.y),
                        ),
                        Vertex::new(
                            vec3(glyph.x + glyph_width, glyph.y, 0.0),
                            vec2(uv_max.x, uv_min.y),
                        ),
                        Vertex::new(vec3(glyph.x, glyph.y, 0.0), vec2(uv_min.x, uv_min.y)),
                    ]
                })
                .collect_vec();

            let indices = (0..)
                .step_by(4)
                .take(glyph_count)
                .flat_map(|i| [i, 1 + i, 2 + i, 2 + i, 3 + i, i])
                .collect_vec();

            let mesh = frame.assets.insert(Mesh::new(gpu, &vertices, &indices));

            cmd.set(
                id,
                draw_cmd(),
                DrawCommand {
                    mesh,
                    bind_group: bind_group.clone(),
                    shader: self.shader.clone(),
                    index_count: indices.len() as u32,
                },
            );
        }

        cmd.apply(&mut frame.world).unwrap();
    }

    pub fn update(&mut self, _: &Gpu, frame: &mut Frame) {
        self.object_query
            .borrow(&frame.world)
            .iter()
            .for_each(|(rect, &pos, model)| {
                let pos = pos + rect.pos();
                // tracing::info!("Updating text rect: {rect:?} at {pos}");
                *model = Mat4::from_scale_rotation_translation(
                    Vec3::ONE,
                    Quat::IDENTITY,
                    pos.extend(0.1),
                );
            })
    }
}
