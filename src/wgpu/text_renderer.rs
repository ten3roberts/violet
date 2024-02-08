use std::sync::Arc;

use cosmic_text::{Buffer, CacheKey, FontSystem, Placement, SwashCache};
use flax::{
    entity_ids,
    fetch::{Modified, TransformFetch},
    filter::{All, With},
    CommandBuffer, Component, EntityIds, Fetch, FetchExt, Mutable, Opt, OptOr, Query,
};
use glam::{vec2, vec3, Mat4, Quat, Vec2, Vec3, Vec4};
use itertools::Itertools;
use palette::Srgba;
use parking_lot::Mutex;
use wgpu::{BindGroup, BindGroupLayout, Sampler, SamplerDescriptor, ShaderStages, TextureFormat};

use crate::{
    assets::AssetCache,
    components::{color, draw_shape, font_size, layout_bounds, rect, screen_position, text, Rect},
    shape::shape_text,
    stored::{self, Handle},
    text::TextSegment,
    wgpu::{font::FontAtlas, graphics::BindGroupBuilder, shape_renderer::DrawCommand},
    Frame,
};

use super::{
    components::{draw_cmd, object_data, text_buffer_state, text_mesh, TextBufferState},
    font::GlyphLocation,
    graphics::{shader::ShaderDesc, BindGroupLayoutBuilder, Shader, Vertex, VertexDesc},
    mesh_buffer::MeshHandle,
    renderer::RendererContext,
    shape_renderer::{srgba_to_vec4, ObjectData, RendererStore},
    Gpu,
};

pub(crate) struct TextSystem {
    pub(crate) font_system: FontSystem,
    pub(crate) swash_cache: SwashCache,
}

impl TextSystem {
    pub fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
            swash_cache: SwashCache::new(),
        }
    }
}

#[derive(Fetch)]
struct ObjectQuery {
    draw_shape: With,
    rect: Component<Rect>,
    pos: Component<Vec2>,
    object_data: Mutable<ObjectData>,
    color: OptOr<Component<Srgba>, Srgba>,
}

impl ObjectQuery {
    pub fn new() -> Self {
        Self {
            draw_shape: draw_shape(shape_text()).with(),
            rect: rect(),
            pos: screen_position(),
            object_data: object_data().as_mut(),
            color: color().opt_or(Srgba::new(1.0, 1.0, 1.0, 1.0)),
        }
    }
}

struct FontRasterizer {
    rasterized: RasterizedFont,
    sampler: Arc<Sampler>,
    text_layout: BindGroupLayout,
}

impl FontRasterizer {
    fn new(
        gpu: &Gpu,
        sampler: Arc<Sampler>,
        text_layout: BindGroupLayout,
        store: &mut RendererStore,
    ) -> Self {
        let atlas = FontAtlas::empty(gpu);

        let bind_group = BindGroupBuilder::new("TextRenderer::bind_group")
            .bind_sampler(&sampler)
            .bind_texture(&atlas.texture.view(&Default::default()))
            .build(gpu, &text_layout);

        Self {
            rasterized: RasterizedFont {
                atlas,
                bind_group: store.bind_groups.insert(bind_group),
            },
            sampler,
            text_layout,
        }
    }

    pub fn add_glyphs(
        &mut self,
        assets: &AssetCache,
        ctx: &mut RendererContext,
        text_system: &mut TextSystem,
        new_glyphs: &[CacheKey],
        store: &mut RendererStore,
    ) -> anyhow::Result<()> {
        let glyphs = self
            .rasterized
            .atlas
            .glyphs
            .keys()
            .chain(new_glyphs)
            .copied();

        let atlas = FontAtlas::new(assets, &ctx.gpu, text_system, glyphs)?;

        let bind_group = store.bind_groups.insert(
            BindGroupBuilder::new("TextRenderer::bind_group")
                .bind_sampler(&self.sampler)
                .bind_texture(&atlas.texture.view(&Default::default()))
                .build(&ctx.gpu, &self.text_layout),
        );

        self.rasterized = RasterizedFont { atlas, bind_group };

        Ok(())
    }

    pub fn get_glyph(&self, glyph: CacheKey) -> Option<&(Placement, GlyphLocation)> {
        self.rasterized.atlas.glyphs.get(&glyph)
    }
}

struct MeshGenerator {
    rasterizer: FontRasterizer,
    shader: Handle<Shader>,
}

impl MeshGenerator {
    fn new(
        ctx: &mut RendererContext,
        frame: &mut Frame,
        color_format: TextureFormat,
        object_layout: &BindGroupLayout,
        store: &mut RendererStore,
    ) -> Self {
        let text_layout = BindGroupLayoutBuilder::new("TextRenderer::text_layout")
            .bind_sampler(ShaderStages::FRAGMENT)
            .bind_texture(ShaderStages::FRAGMENT)
            .build(&ctx.gpu);

        let shader = store.shaders.insert(Shader::new(
            &ctx.gpu,
            &ShaderDesc {
                label: "ShapeRenderer::shader",
                source: include_str!("../../assets/shaders/text.wgsl"),
                format: color_format,
                vertex_layouts: &[Vertex::layout()],
                layouts: &[&ctx.globals_layout, object_layout, &text_layout],
            },
        ));

        let sampler = Arc::new(ctx.gpu.device.create_sampler(&SamplerDescriptor {
            label: Some("ShapeRenderer::sampler"),
            anisotropy_clamp: 1,
            mag_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        }));

        Self {
            rasterizer: FontRasterizer::new(&ctx.gpu, sampler, text_layout, store),
            shader,
        }
    }

    fn update_mesh(
        &mut self,
        ctx: &mut RendererContext,
        assets: &AssetCache,
        text_system: &mut TextSystem,
        buffer: &mut Buffer,
        // text: &str,
        mesh: &mut Arc<MeshHandle>,
        store: &mut RendererStore,
    ) -> u32 {
        // let rasterized = self
        //     .rasterizer
        //     .get(ctx, assets, font.clone(), font_size, text);

        let mut vertices = Vec::new();

        // let color = cosmic_text::Color::rgb(0xFF, 0xFF, 0xFF);

        let mut missing = Vec::new();
        loop {
            for run in buffer.layout_runs() {
                for glyph in run.glyphs.iter() {
                    let physical_glyph = glyph.physical((0., 0.), 1.0);
                    let Some((placement, loc)) =
                        self.rasterizer.get_glyph(physical_glyph.cache_key)
                    else {
                        missing.push(physical_glyph.cache_key);
                        continue;
                    };

                    let atlas_size = self.rasterizer.rasterized.atlas.size();
                    let atlas_size = vec2(atlas_size.width as f32, atlas_size.height as f32);

                    let uv_min = loc.min.as_vec2() / atlas_size;
                    let uv_max = loc.max.as_vec2() / atlas_size;

                    let x = placement.left as f32 + physical_glyph.x as f32;
                    let y = run.line_y - placement.top as f32 + physical_glyph.y as f32;
                    let color = glyph
                        .color_opt
                        .map(|v| {
                            srgba_to_vec4(Srgba::new(v.r(), v.g(), v.b(), v.a()).into_format())
                        })
                        .unwrap_or(Vec4::ONE);

                    vertices.extend_from_slice(&[
                        // Bottom left
                        Vertex::new(
                            vec3(x, y + placement.height as f32, 0.0),
                            color,
                            vec2(uv_min.x, uv_max.y),
                        ),
                        Vertex::new(
                            vec3(x + placement.width as f32, y + placement.height as f32, 0.0),
                            color,
                            vec2(uv_max.x, uv_max.y),
                        ),
                        Vertex::new(
                            vec3(x + placement.width as f32, y, 0.0),
                            color,
                            vec2(uv_max.x, uv_min.y),
                        ),
                        Vertex::new(vec3(x, y, 0.0), color, vec2(uv_min.x, uv_min.y)),
                    ]);
                }
            }

            if missing.is_empty() {
                break;
            } else {
                tracing::debug!(?missing, "Adding missing glyphs");
                vertices.clear();
                self.rasterizer
                    .add_glyphs(assets, ctx, text_system, &missing, store)
                    .unwrap();

                missing.clear();
            }
        }
        let glyph_count = vertices.len() / 4;

        let indices = (0..)
            .step_by(4)
            .take(glyph_count)
            .flat_map(|i| [i, 1 + i, 2 + i, 2 + i, 3 + i, i])
            .collect_vec();

        if mesh.vb().size() >= vertices.len() && mesh.ib().size() >= indices.len() {
            ctx.mesh_buffer.write(&ctx.gpu, mesh, &vertices, &indices);
        } else {
            tracing::info!(?glyph_count, "Allocating new mesh for text");
            *mesh = Arc::new(ctx.mesh_buffer.insert(&ctx.gpu, &vertices, &indices));
        }

        indices.len() as u32
    }
}

#[derive(Fetch)]
#[fetch(transforms = [Modified])]
/// Query text entities in the world and allocate them a slot in the mesh and atlas
pub(crate) struct TextMeshQuery {
    #[fetch(ignore)]
    draw_shape: With,
    #[fetch(ignore)]
    id: EntityIds,
    #[fetch(ignore)]
    text_mesh: Opt<Mutable<Arc<MeshHandle>>>,

    #[fetch(ignore)]
    state: Mutable<TextBufferState>,

    rect: Component<Rect>,
    text: Component<Vec<TextSegment>>,
    layout_bounds: Component<Vec2>,
    #[fetch(ignore)]
    font_size: OptOr<Component<f32>, f32>,
}

impl TextMeshQuery {
    fn new() -> Self {
        Self {
            draw_shape: draw_shape(shape_text()).with(),
            id: entity_ids(),
            text_mesh: text_mesh().as_mut().opt(),
            rect: rect(),
            text: text(),
            layout_bounds: layout_bounds(),
            font_size: font_size().opt_or(16.0),
            state: text_buffer_state().as_mut(),
        }
    }
}

pub struct RasterizedFont {
    /// Stored to retrieve *where* the character is located
    atlas: FontAtlas,
    bind_group: stored::Handle<BindGroup>,
}

pub struct TextRenderer {
    mesh_generator: MeshGenerator,
    text_system: Arc<Mutex<TextSystem>>,

    object_query: Query<ObjectQuery, (All, With)>,
    mesh_query: Query<<TextMeshQuery as TransformFetch<Modified>>::Output, All>,
}

impl TextRenderer {
    pub fn new(
        ctx: &mut RendererContext,
        frame: &mut Frame,
        text_system: Arc<Mutex<TextSystem>>,
        color_format: TextureFormat,
        object_layout: &BindGroupLayout,
        store: &mut RendererStore,
    ) -> Self {
        let mesh_generator = MeshGenerator::new(ctx, frame, color_format, object_layout, store);
        Self {
            object_query: Query::new(ObjectQuery::new()).with(text()),
            mesh_generator,
            mesh_query: Query::new(TextMeshQuery::new().modified()),
            text_system,
        }
    }

    pub fn update_meshes(
        &mut self,
        ctx: &mut RendererContext,
        frame: &mut Frame,
        store: &mut RendererStore,
    ) {
        let mut cmd = CommandBuffer::new();

        let text_system = &mut *self.text_system.lock();

        (self.mesh_query.borrow(&frame.world))
            .iter()
            .collect_vec()
            .into_iter()
            .rev()
            .for_each(|item| {
                let _span = tracing::info_span!( "update_mesh", %item.id).entered();
                // tracing::debug!(%item.id, "updating mesh for {:?}", item.text);

                // Update intrinsic sizes

                {
                    let mut buffer = item.state.buffer.borrow_with(&mut text_system.font_system);

                    buffer.set_size(item.layout_bounds.x + 15.0, item.layout_bounds.y);

                    buffer.shape_until_scroll(true);
                }

                let mut new_mesh = None;

                let text_mesh = match item.text_mesh {
                    Some(v) => v,
                    None => new_mesh.insert(Arc::new(ctx.mesh_buffer.allocate(&ctx.gpu, 0, 0))),
                };

                let index_count = self.mesh_generator.update_mesh(
                    ctx,
                    &frame.assets,
                    text_system,
                    &mut item.state.buffer,
                    text_mesh,
                    store,
                );

                cmd.set(
                    item.id,
                    draw_cmd(),
                    DrawCommand {
                        bind_group: self.mesh_generator.rasterizer.rasterized.bind_group.clone(),
                        shader: self.mesh_generator.shader.clone(),
                        mesh: text_mesh.clone(),
                        index_count,
                        vertex_offset: 0,
                    },
                );

                if let Some(text_mesh) = new_mesh {
                    cmd.set(item.id, crate::wgpu::components::text_mesh(), text_mesh);
                }
            });

        cmd.apply(&mut frame.world).unwrap();
    }

    pub fn update(&mut self, _: &Gpu, frame: &Frame) {
        self.object_query
            .borrow(&frame.world)
            .iter()
            .for_each(|item| {
                let rect = item.rect.translate(*item.pos).align_to_grid();
                let model_matrix = Mat4::from_scale_rotation_translation(
                    Vec3::ONE,
                    Quat::IDENTITY,
                    rect.pos().extend(0.1),
                );

                *item.object_data = ObjectData {
                    model_matrix,
                    color: srgba_to_vec4(*item.color),
                };
            })
    }
}
