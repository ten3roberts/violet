use std::{
    any::{Any, TypeId},
    collections::HashMap,
    hash::Hash,
    process::Output,
    sync::{Arc, Weak},
};

use dashmap::{mapref::one::MappedRef, DashMap};
use ulid::Ulid;

mod handle;
pub use handle::Handle;

pub struct AssetCache {
    cells: DashMap<TypeId, Arc<dyn Any>>,
}

impl AssetCache {
    pub fn new() -> Self {
        Self {
            cells: DashMap::new(),
        }
    }

    pub fn insert<V: 'static + Send + Sync>(&self, value: V) -> handle::Handle<T> {
        let mut cell = self.cell::<Handle<T>>();

        let handle = Handle::new_dangling(value);

        let cell = cell.downcast_ref::<AssetCell<Handle<V>>>().unwrap();

        cell.loaded
            .insert(handle.clone(), Arc::downgrade(handle.get()));

        value
    }

    pub fn get<K: AssetKey>(&self, key: K) -> Arc<K::Output> {
        let mut cell = self.cell();

        if let Some(value) = cell.loaded.get(&key).and_then(|v| v.upgrade()) {
            value
        } else {
            let value = key.load(self);
            let value = Arc::new(value);
            cell.loaded.insert(key, Arc::downgrade(&value));
            value
        }
    }

    fn cell<K: AssetKey>(&self) -> Arc<AssetCell<K>> {
        let mut cell = self.cells.entry(TypeId::of::<K>()).or_insert_with(|| {
            Arc::new(AssetCell::<K> {
                loaded: DashMap::new(),
            })
        });

        cell.downcast_mut::<AssetCell<K>>().unwrap())
    }

    pub fn is_loaded<K: AssetKey>(&self, key: &K) -> bool {
        let mut cell = self.cells.entry(TypeId::of::<K>()).or_insert_with(|| {
            Box::new(AssetCell::<K> {
                loaded: HashMap::new(),
            })
        });

        let cell = cell.downcast_mut::<AssetCell<K>>().unwrap();
        cell.loaded.get(key).is_some_and(|v| v.strong_count() > 0)
    }
}

impl Default for AssetCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Stores assets for a single key type
pub struct AssetCell<K: AssetKey> {
    loaded: DashMap<K, Weak<K::Output>>,
}

pub trait AssetKey: 'static + Send + Sync + Hash + Eq {
    type Output;
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

            fn load(&self, assets: &AssetCache) -> Self::Output {
                self.0.clone()
            }
        }

        let assets = AssetCache::new();

        let content = assets.get(Key("Foo".to_string()));
        let content2 = assets.get(Key("Foo".to_string()));
        let _content3 = assets.get(Key("Bar".to_string()));

        assert!(Arc::ptr_eq(&content, &content2));

        assert!(assets.is_loaded(&Key("Foo".to_string())));

        drop((content, content2));

        assert!(!assets.is_loaded(&Key("Foo".to_string())));
        assert!(assets.is_loaded(&Key("Bar".to_string())));
    }
}
