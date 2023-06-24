use std::{
    any::{type_name, Any, TypeId},
    borrow::Borrow,
    collections::HashMap,
    hash::Hash,
    sync::Arc,
};

use dashmap::DashMap;

pub mod cell;
pub mod fs;
mod handle;
pub mod map;
pub use handle::Handle;

use self::{cell::AssetCell, handle::WeakHandle};

slotmap::new_key_type! {
    pub struct AssetId;
}

type KeyMap<K> = HashMap<K, WeakHandle<<K as AssetKey>::Output>>;

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
}

impl AssetCache {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AssetCacheInner {
                keys: DashMap::new(),
                cells: DashMap::new(),
            }),
        }
    }

    #[tracing::instrument(level = "info", skip(key), fields(key = type_name::<K>()))]
    pub fn load<K>(&self, key: &K) -> Handle<K::Output>
    where
        K: AssetKey + Clone,
    {
        let key = key.borrow();
        if let Some(handle) = self.get(key) {
            return handle;
        }

        // Load the asset and insert it to get a handle
        let value = key.load(self);

        let handle = self.insert(value);

        self.inner
            .keys
            .entry(TypeId::of::<K>())
            .or_insert_with(|| Box::<HashMap<K, WeakHandle<<K as AssetKey>::Output>>>::default())
            .downcast_mut::<KeyMap<K>>()
            .unwrap()
            .insert(key.clone(), handle.downgrade());

        handle
    }

    #[tracing::instrument(level = "info", skip(key), fields(key = type_name::<K>()))]
    pub fn get<K: AssetKey>(&self, key: &K) -> Option<Handle<K::Output>> {
        // Keys of K
        let keys = self.inner.keys.get(&TypeId::of::<K>())?;

        let handle = keys
            .downcast_ref::<KeyMap<K>>()
            .unwrap()
            .get(key)?
            .upgrade()?;

        Some(handle)
    }

    pub fn insert<T: 'static + Send + Sync>(&self, value: T) -> Handle<T> {
        self.inner
            .cells
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(AssetCell::<T>::new()))
            .downcast_mut::<AssetCell<T>>()
            .unwrap()
            .insert(value)
    }
}

impl Default for AssetCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Describes an asset sufficiently to load it
pub trait AssetKey: 'static + Send + Sync + Hash + Eq {
    type Output: 'static + Send + Sync;

    fn load(&self, assets: &AssetCache) -> Self::Output;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_cache() {
        #[derive(Hash, Eq, PartialEq, Clone)]
        struct Key(String);

        impl AssetKey for Key {
            type Output = String;

            fn load(&self, _: &AssetCache) -> Self::Output {
                self.0.clone()
            }
        }

        let assets = AssetCache::new();

        let content = assets.load(&Key("Foo".to_string()));
        let content2 = assets.load(&Key("Foo".to_string()));
        let _content3 = assets.load(&Key("Bar".to_string()));

        assert_eq!(&content, &content2);

        assert!(assets.get(&Key("Foo".to_string())).is_some());

        drop((content, content2));

        assert!(assets.get(&Key("Foo".to_string())).is_none());
        assert!(assets.get(&Key("Bar".to_string())).is_some());
    }
}
