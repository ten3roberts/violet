use anyhow::Context;
use fontdue::Font;

use crate::{
    assets::{AssetCache, Handle},
    components::FontFamily,
};

use super::font::FontFromFile;

pub struct FontMap {
    assets: AssetCache,
    provider: Box<dyn Provider>,
}

impl FontMap {
    pub fn new<P: 'static + Provider>(assets: AssetCache, provider: P) -> Self {
        Self {
            assets,
            provider: Box::new(provider),
        }
    }

    pub fn get(&self, name: &FontFamily) -> anyhow::Result<Handle<Font>> {
        self.provider.load(&self.assets, name)
    }
}

pub trait Provider: Send + Sync {
    fn load(&self, assets: &AssetCache, family: &FontFamily) -> anyhow::Result<Handle<Font>>;
}

pub struct FsProvider {
    root: std::path::PathBuf,
}

impl FsProvider {
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

impl Provider for FsProvider {
    fn load(&self, assets: &AssetCache, family: &FontFamily) -> anyhow::Result<Handle<Font>> {
        let path = self.root.join(family.as_ref());

        assets
            .try_load(&FontFromFile { path })
            .with_context(|| anyhow::anyhow!("Failed to load font {family}"))
    }
}
