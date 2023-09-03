use std::collections::{btree_map, BTreeMap};

use flax::{
    entity_ids,
    fetch::{Modified, TransformFetch},
    filter::{All, With},
    CommandBuffer, Component, Debuggable, EntityIds, Fetch, FetchExt, Mutable, Opt, OptOr, Query,
};
use fontdue::layout::{Layout, TextStyle};
use glam::{vec2, vec3, Mat4, Quat, Vec2, Vec3};
use itertools::Itertools;
use wgpu::{BindGroup, BindGroupLayout, Sampler, SamplerDescriptor, ShaderStages, TextureFormat};

use crate::{
    assets::{map::HandleMap, AssetCache, Handle},
    components::{font_size, rect, screen_position, text, Rect},
    wgpu::{
        font::FontAtlas,
        graphics::{allocator::Allocation, BindGroupBuilder},
        shape_renderer::DrawCommand,
    },
    Frame,
};

use super::{
    components::{draw_cmd, font, mesh_handle, model_matrix},
    font::Font,
    graphics::{shader::ShaderDesc, BindGroupLayoutBuilder, Shader, Vertex, VertexDesc},
    mesh_buffer::MeshHandle,
    renderer::RendererContext,
    Gpu,
};

flax::component! {
    text_mesh: Allocation => [ Debuggable ],
}

#[derive(Fetch)]
struct ObjectQuery {
    rect: Component<Rect>,
    pos: Component<Vec2>,
    model_matrix: Mutable<Mat4>,
}

impl ObjectQuery {
    pub fn new() -> Self {
        Self {
            rect: rect(),
            pos: screen_position(),
            model_matrix: model_matrix().as_mut(),
        }
    }
}

struct MeshGenerator {
    fonts: HandleMap<Font, RenderFont>,
    shader: Handle<Shader>,
    text_layout: BindGroupLayout,

    sampler: Handle<Sampler>,
}

impl MeshGenerator {
    fn new(
        ctx: &mut RendererContext,
        frame: &mut Frame,
        color_format: TextureFormat,
        object_layout: &BindGroupLayout,
    ) -> Self {
        let text_layout = BindGroupLayoutBuilder::new("TextRenderer::text_layout")
            .bind_sampler(ShaderStages::FRAGMENT)
            .bind_texture(ShaderStages::FRAGMENT)
            .build(&ctx.gpu);

        let shader = frame.assets.insert(Shader::new(
            &ctx.gpu,
            &ShaderDesc {
                label: "ShapeRenderer::shader",
                source: include_str!("../../assets/shaders/text.wgsl"),
                format: color_format,
                vertex_layouts: &[Vertex::layout()],
                layouts: &[&ctx.globals_layout, object_layout, &text_layout],
            },
        ));

        let sampler = frame
            .assets
            .insert(ctx.gpu.device.create_sampler(&SamplerDescriptor {
                label: Some("ShapeRenderer::sampler"),
                anisotropy_clamp: 1,
                mag_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,

                ..Default::default()
            }));

        Self {
            fonts: HandleMap::new(),
            shader,
            text_layout,
            sampler,
        }
    }
}

#[derive(Fetch, Debug, Clone)]
#[fetch(transforms = [Modified])]
/// Query text entities in the world and allocate them a slot in the mesh and atlas
pub struct TextMeshQuery {
    #[fetch(ignore)]
    id: EntityIds,
    #[fetch(ignore)]
    mesh: Opt<Mutable<MeshHandle>>,

    rect: Component<Rect>,
    text: Component<String>,
    font: Component<Handle<Font>>,
    font_size: OptOr<Component<f32>, f32>,
}

impl TextMeshQuery {
    fn new() -> Self {
        Self {
            id: entity_ids(),
            mesh: mesh_handle().as_mut().opt(),
            rect: rect(),
            text: text(),
            font: font(),
            font_size: font_size().opt_or(16.0),
        }
    }
}

pub struct RasterizedFont {
    /// Stored to retrieve *where* the character is located
    atlas: FontAtlas,
    bind_group: Handle<BindGroup>,
}

pub struct RenderFont {
    font: Handle<Font>,
    sampler: Handle<Sampler>,
    rasterized: BTreeMap<u32, RasterizedFont>,
}

impl RenderFont {
    fn get(
        &mut self,
        gpu: &Gpu,
        assets: &AssetCache,
        px: u32,
        text: &str,
        text_layout: &BindGroupLayout,
    ) -> &mut RasterizedFont {
        let rasterize = || {
            let atlas = FontAtlas::new(assets, gpu, &self.font, px as f32, text.chars()).unwrap();

            let bind_group = assets.insert(
                BindGroupBuilder::new("TextRenderer::bind_group")
                    .bind_sampler(&self.sampler)
                    .bind_texture(&atlas.texture.view(&Default::default()))
                    .build(gpu, text_layout),
            );

            RasterizedFont { atlas, bind_group }
        };

        match self.rasterized.entry(px) {
            btree_map::Entry::Vacant(slot) => slot.insert(rasterize()),
            btree_map::Entry::Occupied(slot) => {
                let rasterized = slot.into_mut();
                if !text.chars().all(|c| rasterized.atlas.contains_char(c)) {
                    *rasterized = rasterize();
                }
                rasterized
            }
        }
    }
}

pub struct TextRenderer {
    mesh_generator: MeshGenerator,

    object_query: Query<ObjectQuery, (All, With)>,
    mesh_query: Query<<TextMeshQuery as TransformFetch<Modified>>::Output, All>,
}

impl TextRenderer {
    pub fn new(
        ctx: &mut RendererContext,
        frame: &mut Frame,
        color_format: TextureFormat,
        object_layout: &BindGroupLayout,
    ) -> Self {
        let mesh_generator = MeshGenerator::new(ctx, frame, color_format, object_layout);
        Self {
            object_query: Query::new(ObjectQuery::new()).with(text()),
            mesh_generator,
            mesh_query: Query::new(TextMeshQuery::new().transform_fetch(Modified)),
        }
    }

    pub fn update_meshes(&mut self, ctx: &mut RendererContext, frame: &mut Frame) {
        let mut cmd = CommandBuffer::new();

        (self.mesh_query.borrow(&frame.world)).for_each(|item| {
            tracing::debug!(%item.id, "updating mesh for {:?}", item.text);

            let render_font = self
                .mesh_generator
                .fonts
                .entry(item.font)
                .or_insert_with_key(|key| RenderFont {
                    rasterized: BTreeMap::new(),
                    font: key.clone(),
                    sampler: self.mesh_generator.sampler.clone(),
                });

            let rasterized = render_font.get(
                &ctx.gpu,
                &frame.assets,
                *item.font_size as _,
                item.text,
                &self.mesh_generator.text_layout,
            );

            let mut layout = Layout::<()>::new(fontdue::layout::CoordinateSystem::PositiveYDown);

            let size = item.rect.size();
            layout.reset(&fontdue::layout::LayoutSettings {
                x: item.rect.min.x,
                y: item.rect.min.x,
                max_width: Some(size.x),
                max_height: Some(size.y),
                horizontal_align: fontdue::layout::HorizontalAlign::Left,
                vertical_align: fontdue::layout::VerticalAlign::Top,
                line_height: 1.0,
                wrap_style: fontdue::layout::WrapStyle::Word,
                wrap_hard_breaks: true,
            });

            layout.append(
                &[&item.font.font],
                &TextStyle {
                    text: item.text,
                    px: *item.font_size,
                    font_index: 0,
                    user_data: (),
                },
            );

            let glyph_count = layout.glyphs().len();

            let vertices = layout
                .glyphs()
                .iter()
                .flat_map(|glyph| {
                    let atlas_glyph = rasterized.atlas.glyphs.get(&glyph.key.glyph_index).unwrap();

                    let glyph_width = glyph.width as f32;
                    let glyph_height = glyph.height as f32;

                    let atlas_size = rasterized.atlas.size();
                    let atlas_size = vec2(atlas_size.width as f32, atlas_size.height as f32);

                    let uv_min = atlas_glyph.min.as_vec2() / atlas_size;
                    let uv_max = atlas_glyph.max.as_vec2() / atlas_size;

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

            let mesh = match item.mesh {
                Some(mesh) => {
                    ctx.mesh_buffer
                        .reallocate(&ctx.gpu, mesh, vertices.len(), indices.len());

                    *mesh
                }
                None => {
                    let mesh = ctx
                        .mesh_buffer
                        .allocate(&ctx.gpu, vertices.len(), indices.len());
                    cmd.set(item.id, mesh_handle(), mesh);
                    mesh
                }
            };

            ctx.mesh_buffer.write(&ctx.gpu, &mesh, &vertices, &indices);

            cmd.set(
                item.id,
                draw_cmd(),
                DrawCommand {
                    mesh,
                    bind_group: rasterized.bind_group.clone(),
                    shader: self.mesh_generator.shader.clone(),
                    index_count: indices.len() as u32,
                    vertex_offset: mesh.vb().start() as i32,
                },
            );
        });

        cmd.apply(&mut frame.world).unwrap();
    }

    pub fn update(&mut self, _: &Gpu, frame: &Frame) {
        self.object_query
            .borrow(&frame.world)
            .iter()
            .for_each(|item| {
                let pos = *item.pos + item.rect.pos();
                // tracing::info!("Updating text rect: {rect:?} at {pos}");
                *item.model_matrix = Mat4::from_scale_rotation_translation(
                    Vec3::ONE,
                    Quat::IDENTITY,
                    pos.extend(0.1),
                );
            })
    }
}
