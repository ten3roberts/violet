use std::{
    any::{Any, TypeId},
    collections::HashMap,
    hash::Hash,
    sync::{Arc, Weak},
};

use dashmap::DashMap;

pub mod cell;
mod handle;
pub use handle::Handle;
use tracing::info_span;

use self::{cell::AssetCell, handle::WeakHandle};

slotmap::new_key_type! {
    pub(crate) struct AssetId;
}

type KeyMap<K> = HashMap<K, WeakHandle<<K as AssetKey>::Output>>;

/// Stores assets which are accessible through handles
pub struct AssetCache {
    keys: HashMap<TypeId, Box<dyn Any>>,
    cells: HashMap<TypeId, Box<dyn Any>>,
}

impl AssetCache {
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            cells: HashMap::new(),
        }
    }

    pub fn load<K: AssetKey>(&mut self, key: K) -> Handle<K::Output> {
        if let Some(handle) = self.get(&key) {
            return handle;
        }

        // Load the asset and insert it to get a handle
        let value = key.load(self);

        let handle = self.insert(value);

        self.keys
            .entry(TypeId::of::<K>())
            .or_insert_with(|| Box::<HashMap<K, WeakHandle<<K as AssetKey>::Output>>>::default())
            .downcast_mut::<KeyMap<K>>()
            .unwrap()
            .insert(key, handle.downgrade());

        handle
    }

    pub fn get<K: AssetKey>(&self, key: &K) -> Option<Handle<K::Output>> {
        // Keys of K
        let keys = self.keys.get(&TypeId::of::<K>())?;

        let handle = keys
            .downcast_ref::<KeyMap<K>>()
            .unwrap()
            .get(key)?
            .upgrade()?;

        Some(handle)
    }

    pub fn insert<T: 'static + Send + Sync>(&mut self, value: T) -> Handle<T> {
        self.cell_mut::<T>().insert(value)
    }

    fn cell_mut<V: 'static>(&mut self) -> &mut AssetCell<V> {
        self.cells
            .entry(TypeId::of::<V>())
            .or_insert_with(|| Box::new(AssetCell::<V>::new()))
            .downcast_mut::<AssetCell<V>>()
            .unwrap()
    }

    fn cell<V: 'static>(&self) -> Option<&AssetCell<V>> {
        Some(
            self.cells
                .get(&TypeId::of::<V>())?
                .downcast_ref::<AssetCell<V>>()
                .unwrap(),
        )
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
        #[derive(Hash, Eq, PartialEq)]
        struct Key(String);

        impl AssetKey for Key {
            type Output = String;

            fn load(&self, _: &AssetCache) -> Self::Output {
                self.0.clone()
            }
        }

        let mut assets = AssetCache::new();

        let content = assets.load(Key("Foo".to_string()));
        let content2 = assets.load(Key("Foo".to_string()));
        let _content3 = assets.load(Key("Bar".to_string()));

        assert_eq!(&content, &content2);

        assert!(assets.get(&Key("Foo".to_string())).is_some());

        drop((content, content2));

        assert!(assets.get(&Key("Foo".to_string())).is_none());
        assert!(assets.get(&Key("Bar".to_string())).is_some());
    }
}
