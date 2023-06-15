use std::sync::Arc;

use image::DynamicImage;
use ulid::Ulid;

use crate::assets::AssetKey;

use super::graphics::texture::Texture;

/// Load a texture from in memory data
pub struct TextureFromImage {
    id: Ulid,
    image: Arc<DynamicImage>,
}

impl PartialEq for TextureFromImage {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for TextureFromImage {}

impl std::hash::Hash for TextureFromImage {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl AssetKey for TextureFromImage {
    type Output = TextureHandle;

    fn load(&self) -> Self::Output {
        let texture = Texture::from_image(&todo!(), &self.image);
        TextureHandle {
            id: self.id,
            texture: Arc::new(texture),
        }
    }
}

pub struct TextureHandle {
    id: Ulid,
    texture: Arc<Texture>,
}
