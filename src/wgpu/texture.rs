use bytes::Bytes;
use image::DynamicImage;

use crate::assets::{AssetCache, AssetKey, Loadable};

impl<K> Loadable<K> for DynamicImage
where
    K: AssetKey,
    Bytes: Loadable<K>,
    <Bytes as Loadable<K>>::Error: Into<anyhow::Error>,
{
    type Error = anyhow::Error;

    fn load(key: K, _: &AssetCache) -> anyhow::Result<Self> {
        Ok(image::load_from_memory(
            &Bytes::load(key, &AssetCache::new()).map_err(|v| v.into())?,
        )?)
    }
}

// #[derive(PartialEq, Eq, Hash, Debug, Clone)]
// struct SolidTextureKey {
//     color: Rgba<u8>,
//     gpu: Asset<Gpu>,
// }

// impl AssetKey for SolidTextureKey {
//     type Output = Texture;
//     type Error = Infallible;

//     fn load(self, _: &AssetCache) -> Result<Self::Output, Infallible> {
//         let contents = ImageBuffer::from_pixel(256, 256, self.color);

//         Ok(Texture::from_image(
//             &self.gpu,
//             &DynamicImage::ImageRgba8(contents),
//         ))
//     }
// }
