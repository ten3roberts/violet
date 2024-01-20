use std::{
    any::{Any, TypeId},
    borrow::Borrow,
    convert::Infallible,
    hash::Hash,
    sync::Arc,
};

use dashmap::DashMap;

pub mod cell;
pub mod fs;
mod handle;
pub mod map;
mod provider;
pub use handle::Asset;

use self::{cell::AssetCell, handle::WeakHandle, provider::AssetProvider};

slotmap::new_key_type! {
    pub struct AssetId;
}

type KeyMap<K, V> = DashMap<K, WeakHandle<V>>;

#[derive(Clone)]
pub struct AssetCache {
    inner: Arc<AssetCacheInner>,
}

impl std::fmt::Debug for AssetCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssetCache").finish()
    }
}

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
                cells: DashMap::new(),
                providers: DashMap::new(),
                keys: DashMap::new(),
            }),
        }
    }

    pub fn insert_provider<P: 'static + Send + Sync>(&mut self, provider: P) -> &mut Self {
        self.inner
            .providers
            .insert(TypeId::of::<P>(), Box::new(provider));
        self
    }

    pub fn try_load<K, V>(&self, key: &K) -> Result<Asset<V>, V::Error>
    where
        K: AssetKey + Clone,
        V: Loadable<K>,
    {
        let _span = tracing::debug_span!("AssetCache::try_load", key=?key).entered();
        if let Some(handle) = self.get(key) {
            return Ok(handle);
        }

        // Load the asset and insert it to get a handle
        let value = V::load(key.clone(), self)?;

        let handle = self.insert(value);

        self.inner
            .cells
            .entry(TypeId::of::<K>())
            .or_insert_with(|| Box::<KeyMap<K, V>>::default())
            .downcast_mut::<KeyMap<K, V>>()
            .unwrap()
            .insert(key.clone(), handle.downgrade());

        Ok(handle)
    }

    pub fn load<K, V>(&self, key: &K) -> Asset<V>
    where
        K: AssetKey + Clone,
        V: 'static + Send + Sync + Loadable<K>,
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
        K: AssetKey,
        V: Loadable<K>,
    {
        // Keys of K
        let keys = self.inner.cells.get(&TypeId::of::<K>())?;

        let handle = keys
            .downcast_ref::<KeyMap<K, V>>()
            .unwrap()
            .get(key)?
            .upgrade()?;

        Some(handle)
    }

    fn insert<V: 'static + Send + Sync>(&self, value: V) -> Asset<V> {
        let ty = std::any::type_name::<V>();
        let _span = tracing::debug_span!("AssetCache::insert", ty).entered();
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

/// Marker trait for a type which can be used as an asset key.
///
/// An asset key is any plain type which can be compared and hashed.
pub trait AssetKey: 'static + Send + Sync + Hash + Eq + std::fmt::Debug {}

impl<K> AssetKey for K where K: 'static + Send + Sync + Hash + Eq + std::fmt::Debug {}

/// An asset which is loaded from a key, such as a file path, or a descriptor structure
pub trait Loadable<Key: AssetKey>: 'static + Send + Sync {
    /// The error type returned when loading fails
    type Error: 'static + Send + Sync;

    fn load(key: Key, assets: &AssetCache) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use super::*;

    #[test]
    fn asset_cache() {
        #[derive(Hash, Eq, PartialEq, Clone, Debug)]
        struct Key(String);

        impl Loadable<Key> for String {
            type Error = Infallible;

            fn load(key: Key, _: &AssetCache) -> Result<String, Infallible> {
                Ok(key.0)
            }
        }

        let assets = AssetCache::new();

        let content: Asset<String> = assets.load(&Key("Foo".to_string()));
        let content2: Asset<String> = assets.load(&Key("Foo".to_string()));
        let _content3: Asset<String> = assets.load(&Key("Bar".to_string()));

        assert_eq!(&content, &content2);

        assert!(assets.get::<_, String>(&Key("Foo".to_string())).is_some());

        drop((content, content2));

        assert!(assets.get::<_, String>(&Key("Foo".to_string())).is_none());
        assert!(assets.get::<_, String>(&Key("Bar".to_string())).is_some());
    }
}
