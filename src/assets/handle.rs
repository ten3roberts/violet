use std::cmp::Ordering;
use std::{self, marker::PhantomData};

use std::hash::Hash;

use ulid::Ulid;

use std::sync::{Arc, Weak};

use super::{AssetCache, AssetId, AssetKey};

pub struct WeakHandle<T> {
    pub(crate) value: Weak<T>,
    pub(crate) id: AssetId,
}

impl<T> Clone for WeakHandle<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            id: self.id.clone(),
        }
    }
}

impl<T> WeakHandle<T> {
    pub fn upgrade(&self) -> Option<Handle<T>> {
        self.value.upgrade().map(|count| Handle {
            value: count,
            id: self.id,
        })
    }

    pub fn strong_count(&self) -> usize {
        self.value.strong_count()
    }
}

impl<T> PartialEq for WeakHandle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> Eq for WeakHandle<T> {}

/// Keep-alive handle to an asset
#[derive(Debug)]
pub struct Handle<T> {
    pub(crate) value: Arc<T>,
    pub(crate) id: AssetId,
}

impl<T> std::ops::Deref for Handle<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            id: self.id,
        }
    }
}

impl<T> Handle<T> {
    pub fn downgrade(&self) -> WeakHandle<T> {
        WeakHandle {
            value: Arc::downgrade(&self.value),
            id: self.id,
        }
    }
}

impl<T> Hash for Handle<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T> PartialOrd for Handle<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.id.partial_cmp(&other.id)
    }
}

impl<T> Ord for Handle<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> Eq for Handle<T> {}
