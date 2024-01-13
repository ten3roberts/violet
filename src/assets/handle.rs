use std::cmp::Ordering;
use std::{self};

use std::hash::Hash;

use std::sync::{Arc, Weak};

use super::AssetId;

#[derive(Debug)]
pub struct WeakHandle<T: ?Sized> {
    pub(crate) id: AssetId,
    pub(crate) value: Weak<T>,
}

impl<T: ?Sized> Clone for WeakHandle<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            id: self.id,
        }
    }
}

impl<T: ?Sized> WeakHandle<T> {
    pub fn upgrade(&self) -> Option<Asset<T>> {
        self.value.upgrade().map(|count| Asset {
            value: count,
            id: self.id,
        })
    }

    pub fn strong_count(&self) -> usize {
        self.value.strong_count()
    }
}

impl<T: ?Sized> PartialEq for WeakHandle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: ?Sized> Eq for WeakHandle<T> {}

/// Keep-alive handle to an asset
#[derive(Debug)]
pub struct Asset<T: ?Sized> {
    pub(crate) id: AssetId,
    pub(crate) value: Arc<T>,
}

impl<T: ?Sized> std::ops::Deref for Asset<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: ?Sized> Clone for Asset<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            id: self.id,
        }
    }
}

impl<T: ?Sized> Asset<T> {
    pub fn downgrade(&self) -> WeakHandle<T> {
        WeakHandle {
            value: Arc::downgrade(&self.value),
            id: self.id,
        }
    }

    #[inline]
    pub fn id(&self) -> AssetId {
        self.id
    }
}

impl<T: ?Sized> Hash for Asset<T> {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T: ?Sized> PartialOrd for Asset<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: ?Sized> Ord for Asset<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl<T: ?Sized> PartialEq for Asset<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: ?Sized> Eq for Asset<T> {}
