use std::path::PathBuf;

use super::{AssetCache, AssetKey};

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
/// Loads bytes from a file
pub struct BytesFromFile(pub PathBuf);

impl std::ops::Deref for BytesFromFile {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AssetKey for BytesFromFile {
    type Output = Vec<u8>;

    fn load(&self, _: &AssetCache) -> Self::Output {
        std::fs::read(&self.0).unwrap()
    }
}
