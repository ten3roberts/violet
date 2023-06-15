use std;

use std::hash::Hash;

use ulid::Ulid;

use std::sync::Arc;

use super::{AssetCache, AssetKey};

/// Keep-alive handle to an asset
#[derive(Debug)]
pub struct Handle<T> {
    pub(crate) value: Arc<T>,
    pub(crate) id: Ulid,
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            id: self.id.clone(),
        }
    }
}

impl<T> Handle<T> {
    pub(crate) fn new_dangling(value: T) -> Self {
        Self {
            value: Arc::new(value),
            id: Ulid::new(),
        }
    }

    pub fn get(&self) -> &Arc<T> {
        &self.value
    }
}

impl<T> Hash for Handle<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> Eq for Handle<T> {}

impl<T: 'static + Send + Sync> AssetKey for Handle<T> {
    type Output = T;

    fn load(&self, _: &AssetCache) -> Self::Output {
        panic!("Invalid handle")
    }
}
