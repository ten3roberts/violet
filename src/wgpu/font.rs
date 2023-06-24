use std::collections::BTreeMap;

use glam::{ivec2, uvec2, IVec2, Vec2};
use guillotiere::{size2, AtlasAllocator};
use image::{ImageBuffer, Luma};
use wgpu::{util::DeviceExt, Extent3d, TextureDescriptor, TextureDimension, TextureUsages};

use crate::assets::{fs::BytesFromFile, AssetCache, AssetKey, Handle};

use super::{graphics::texture::Texture, Gpu};

/// Loads a font from memory
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct FontFromBytes {
    pub bytes: Handle<Vec<u8>>,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct FontFromFile {
    pub path: BytesFromFile,
}

impl AssetKey for FontFromFile {
    type Output = Font;

    fn load(&self, assets: &AssetCache) -> Self::Output {
        let bytes = assets.load(&self.path);

        FontFromBytes { bytes }.load(assets)
    }
}

pub struct Font {
    pub(crate) font: fontdue::Font,
}

impl AssetKey for FontFromBytes {
    type Output = Font;

    fn load(&self, assets: &crate::assets::AssetCache) -> Self::Output {
        let bytes = &*self.bytes;
        let font = fontdue::Font::from_bytes(bytes.as_ref(), fontdue::FontSettings::default())
            .map_err(|v| anyhow::anyhow!("Error loading font: {v:?}"))
            .unwrap();

        Font { font }
    }
}

pub struct GlyphLocation {
    pub min: Vec2,
    pub max: Vec2,
}

pub struct FontAtlas {
    pub texture: Texture,
    pub glyphs: BTreeMap<u16, GlyphLocation>,
}

impl FontAtlas {
    pub fn new(
        assets: &AssetCache,
        gpu: &Gpu,
        font: &Font,
        px: f32,
        glyphs: impl IntoIterator<Item = char>,
    ) -> anyhow::Result<Self> {
        let size = uvec2(512, 512);
        let mut atlas = AtlasAllocator::new(size2(size.x as _, size.y as _));

        let mut image = ImageBuffer::from_pixel(size.x as _, size.y as _, Luma([0]));

        let glyphs = glyphs
            .into_iter()
            .map(|glyph| {
                let (metrics, pixels) = font.font.rasterize(glyph, px);

                let padding = 10;

                let v = atlas
                    .allocate(size2(
                        metrics.width as i32 + padding * 2,
                        metrics.height as i32 + padding * 2,
                    ))
                    .unwrap();

                let min = ivec2(v.rectangle.min.x + padding, v.rectangle.min.y + padding);
                let max = ivec2(v.rectangle.max.x - padding, v.rectangle.max.y - padding);

                tracing::debug!("Glyph: {:?} {:?}", glyph, metrics);
                if metrics.width > 0 {
                    blit_to_image(&pixels, &mut image, min, metrics.width, size.x as usize);
                }

                Ok((
                    font.font.lookup_glyph_index(glyph),
                    GlyphLocation {
                        min: min.as_vec2(),
                        max: max.as_vec2(),
                    },
                )) as anyhow::Result<_>
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;

        let texture = gpu.device.create_texture_with_data(
            &gpu.queue,
            &TextureDescriptor {
                label: Some("FontAtlas"),
                size: Extent3d {
                    width: size.x,
                    height: size.y,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                view_formats: &[],
            },
            &image,
        );
        //
        // let texture = assets.insert(Texture::from_texture(texture));

        Ok(Self {
            texture: Texture::from_texture(texture),
            glyphs,
        })
    }
}

pub fn blit_to_image(
    src: &[u8],
    dst: &mut [u8],
    position: IVec2,
    src_width: usize,
    dst_width: usize,
) {
    for (row_index, row) in src.chunks_exact(src_width).enumerate() {
        let dst_index = position.x as usize + (position.y as usize + row_index) * dst_width;

        dst[dst_index..(dst_index + src_width)].copy_from_slice(row);
    }
}
