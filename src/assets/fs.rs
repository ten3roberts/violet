use std::path::Path;

use bytes::Bytes;

use super::{AssetCache, AssetKey, Loadable};

impl<K> Loadable<K> for Bytes
where
    K: AssetKey,
    K: AsRef<Path>,
{
    type Error = std::io::Error;

    fn load(key: K, assets: &AssetCache) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(std::fs::read(key.as_ref())?.into())
    }
}
