use std::{
    collections::{BTreeMap, BTreeSet},
    convert::Infallible,
    path::PathBuf,
};

use fontdue::Font;
use glam::{uvec2, UVec2};
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
    pub path: PathBuf,
}

impl AssetKey for FontFromFile {
    type Output = Font;
    type Error = anyhow::Error;

    fn load(self, assets: &AssetCache) -> anyhow::Result<Self::Output> {
        let bytes = assets.try_load(&BytesFromFile(self.path))?;

        FontFromBytes { bytes }.load(assets)
    }
}

impl AssetKey for FontFromBytes {
    type Output = Font;
    type Error = anyhow::Error;

    fn load(self, _assets: &crate::assets::AssetCache) -> anyhow::Result<Self::Output> {
        let bytes = &*self.bytes;
        fontdue::Font::from_bytes(bytes.as_ref(), fontdue::FontSettings::default())
            .map_err(|v| anyhow::anyhow!("Error loading font: {v:?}"))
    }
}

pub struct GlyphLocation {
    pub min: UVec2,
    pub max: UVec2,
}

pub struct FontAtlas {
    pub texture: Texture,
    pub glyphs: BTreeMap<u16, GlyphLocation>,
    pub chars: BTreeSet<char>,
}

impl FontAtlas {
    pub fn new(
        _assets: &AssetCache,
        gpu: &Gpu,
        font: &Font,
        px: f32,
        glyphs: impl IntoIterator<Item = char>,
    ) -> anyhow::Result<Self> {
        let mut atlas = AtlasAllocator::new(size2(128, 128));

        let chars = glyphs.into_iter().collect::<BTreeSet<_>>();

        let glyphs = chars
            .iter()
            .map(|&c| {
                let index = font.lookup_glyph_index(c);

                let metrics = font.metrics_indexed(index, px);
                let padding = 10;

                let requested_size = size2(
                    metrics.width as i32 + padding * 2,
                    metrics.height as i32 + padding * 2,
                );
                let v = loop {
                    if let Some(v) = atlas.allocate(requested_size) {
                        break v;
                    } else {
                        atlas.grow(atlas.size() * 2)
                    }
                };
                let min = uvec2(
                    (v.rectangle.min.x + padding) as u32,
                    (v.rectangle.min.y + padding) as u32,
                );
                let max = uvec2(
                    (v.rectangle.max.x - padding) as u32,
                    (v.rectangle.max.y - padding) as u32,
                );

                (index, GlyphLocation { min, max })
            })
            .collect::<BTreeMap<_, _>>();

        let size = atlas.size();
        let size = uvec2(size.width as _, size.height as _);
        let mut image = ImageBuffer::from_pixel(size.x, size.y, Luma([0]));

        // Rasterize and blit without storing all the small bitmaps in memory at the same time
        glyphs.iter().for_each(|(&glyph, loc)| {
            let (metrics, pixels) = font.rasterize_indexed(glyph, px);

            if metrics.width > 0 {
                blit_to_image(
                    &pixels,
                    &mut image,
                    loc.min.x as i32,
                    loc.min.y as i32,
                    metrics.width as u32,
                    size.x,
                );
            }
        });

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

        Ok(Self {
            texture: Texture::from_texture(texture),
            glyphs,
            chars,
        })
    }

    pub(crate) fn size(&self) -> Extent3d {
        self.texture.size()
    }

    pub(crate) fn contains_char(&self, glyph: char) -> bool {
        self.chars.contains(&glyph)
    }
}

pub fn blit_to_image(src: &[u8], dst: &mut [u8], x: i32, y: i32, src_stride: u32, dst_stride: u32) {
    for (row_index, row) in src.chunks_exact(src_stride as usize).enumerate() {
        let dst_index = x as usize + (y as usize + row_index) * dst_stride as usize;

        dst[dst_index..(dst_index + src_stride as usize)].copy_from_slice(row);
    }
}
