use std::path::PathBuf;

use super::{AssetCache, AssetKey};

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
/// Loads bytes from a file
pub struct BytesFromFile {
    path: PathBuf,
}

impl AssetKey for BytesFromFile {
    type Output = Vec<u8>;

    fn load(&self, _: &AssetCache) -> Self::Output {
        std::fs::read(&self.path).unwrap()
    }
}
