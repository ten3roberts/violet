use std::{
    any::{Any, TypeId},
    borrow::Borrow,
    convert::Infallible,
    hash::Hash,
    path::Path,
    sync::Arc,
};

use dashmap::DashMap;

pub mod cell;
pub mod fs;
mod handle;
pub mod map;
pub use handle::Asset;
use image::{DynamicImage, ImageError, ImageResult};

use self::{cell::AssetCell, handle::WeakHandle};

slotmap::new_key_type! {
    pub struct AssetId;
}

#[derive(Clone)]
pub struct AssetCache {
    inner: Arc<AssetCacheInner>,
}

impl std::fmt::Debug for AssetCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssetCache").finish()
    }
}

pub type KeyMap<K, V> = DashMap<K, WeakHandle<V>>;

/// Stores assets which are accessible through handles
struct AssetCacheInner {
    keys: DashMap<TypeId, Box<dyn Any + Send + Sync>>,
    cells: DashMap<TypeId, Box<dyn Any + Send + Sync>>,
    providers: DashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl AssetCache {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AssetCacheInner {
                keys: DashMap::new(),
                cells: DashMap::new(),
                providers: DashMap::new(),
            }),
        }
    }

    pub fn insert_provider<P: 'static + Send + Sync>(&mut self, provider: P) -> &mut Self {
        self.inner
            .providers
            .insert(TypeId::of::<P>(), Box::new(provider));
        self
    }

    pub fn try_load<K, V>(&self, key: &K) -> Result<Asset<V>, K::Error>
    where
        K: ?Sized + AssetKey<V>,
        V: 'static + Send + Sync,
    {
        let _span = tracing::debug_span!("AssetCache::try_load", key = std::any::type_name::<K>())
            .entered();
        if let Some(handle) = self.get(key) {
            return Ok(handle);
        }

        // Load the asset and insert it to get a handle
        let value = key.load(self)?;

        self.inner
            .keys
            .entry(TypeId::of::<K::Stored>())
            .or_insert_with(|| Box::<KeyMap<K::Stored, V>>::default())
            .downcast_mut::<KeyMap<K::Stored, V>>()
            .unwrap()
            .insert(key.to_stored(), value.downgrade());

        Ok(value)
    }

    pub fn load<K, V>(&self, key: &K) -> Asset<V>
    where
        K: ?Sized + AssetKey<V>,
        V: 'static + Send + Sync,
    {
        match self.try_load(key) {
            Ok(v) => v,
            Err(_) => {
                unreachable!()
            }
        }
    }

    pub fn get<K, V>(&self, key: &K) -> Option<Asset<V>>
    where
        K: ?Sized + AssetKey<V>,
        V: 'static + Send + Sync,
    {
        // Keys of K
        let keys = self.inner.keys.get(&TypeId::of::<K::Stored>())?;

        let handle = keys
            .downcast_ref::<KeyMap<K::Stored, V>>()
            .unwrap()
            .get(key)?
            .upgrade()?;

        Some(handle)
    }

    pub fn insert<V: 'static + Send + Sync>(&self, value: V) -> Asset<V> {
        self.inner
            .cells
            .entry(TypeId::of::<V>())
            .or_insert_with(|| Box::new(AssetCell::<V>::new()))
            .downcast_mut::<AssetCell<V>>()
            .unwrap()
            .insert(value)
    }
}

impl Default for AssetCache {
    fn default() -> Self {
        Self::new()
    }
}

pub trait StoredKey: 'static + Send + Sync + Hash + Eq {
    type Stored: 'static + Send + Sync + Hash + Eq + Borrow<Self>;
    fn to_stored(&self) -> Self::Stored;
}

impl<K> StoredKey for K
where
    K: 'static + Send + Sync + ?Sized + Hash + Eq + ToOwned,
    K::Owned: 'static + Send + Sync + Hash + Eq,
{
    type Stored = K::Owned;

    fn to_stored(&self) -> Self::Stored {
        self.to_owned()
    }
}

/// A key or descriptor which can be used to load an asset.
///
/// This trait is implemented for `Path`, `str` and `String` by default to load assets from the
/// filesystem using the provided [`FsProvider`].
pub trait AssetKey<V>: StoredKey {
    type Error: 'static;

    fn load(&self, assets: &AssetCache) -> Result<Asset<V>, Self::Error>;
}

impl AssetKey<DynamicImage> for Path {
    type Error = ImageError;

    fn load(&self, assets: &AssetCache) -> ImageResult<Asset<DynamicImage>> {
        Ok(assets.insert(image::open(self)?))
    }
}

impl AssetKey<()> for Path {
    type Error = Infallible;

    fn load(&self, assets: &AssetCache) -> Result<Asset<()>, Infallible> {
        Ok(assets.insert(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_cache() {
        let assets = AssetCache::new();

        let content: Asset<()> = assets.load(&"Foo");
        let content2: Asset<()> = assets.load(&"Foo".to_string());
        let bar: Asset<()> = assets.load(&"Bar".to_string());
        let content4: Asset<()> = assets.load(&"Foo");

        assert_eq!(content, content2);

        assert!(Arc::ptr_eq(content.as_arc(), content2.as_arc()));
        assert!(!Arc::ptr_eq(content.as_arc(), bar.as_arc()));
        assert_eq!(content, content4);

        assert!(assets.get::<_, ()>(&"Bar".to_string()).is_some());

        drop(bar);

        assert!(assets.get::<_, ()>(&"Bar".to_string()).is_none());
    }
}
