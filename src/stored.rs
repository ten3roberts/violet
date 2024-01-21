use std::{fmt::Debug, marker::PhantomData};

use slotmap::{new_key_type, SlotMap};

new_key_type! {
    pub struct HandleIndex;
}

/// Allows storing non-send and non-sync types through handles
pub struct Store<T> {
    values: SlotMap<HandleIndex, T>,
    free_tx: flume::Sender<HandleIndex>,
    free_rx: flume::Receiver<HandleIndex>,
}

impl<T> Default for Store<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Debug> Debug for Store<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.values.fmt(f)
    }
}

impl<T> Store<T> {
    pub fn new() -> Self {
        let (free_tx, free_rx) = flume::unbounded();
        Self {
            values: SlotMap::with_key(),
            free_tx,
            free_rx,
        }
    }

    pub fn reclaim(&mut self) {
        for index in self.free_rx.try_iter() {
            self.values.remove(index);
        }
    }

    pub fn insert(&mut self, value: T) -> Handle<T> {
        let index = self.values.insert(value);
        Handle {
            index,
            free_tx: self.free_tx.clone(),
            _marker: PhantomData,
        }
    }

    pub fn get(&self, handle: &Handle<T>) -> Option<&T> {
        self.values.get(handle.index)
    }

    pub fn get_mut(&mut self, handle: &Handle<T>) -> Option<&mut T> {
        self.values.get_mut(handle.index)
    }

    pub fn remove(&mut self, handle: &Handle<T>) -> Option<T> {
        self.values.remove(handle.index)
    }

    pub fn iter(&self) -> impl Iterator<Item = (HandleIndex, &T)> {
        self.values.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (HandleIndex, &mut T)> {
        self.values.iter_mut()
    }
}

impl<T> std::ops::Index<&Handle<T>> for Store<T> {
    type Output = T;

    fn index(&self, handle: &Handle<T>) -> &Self::Output {
        &self.values[handle.index]
    }
}

impl<T> std::ops::IndexMut<&Handle<T>> for Store<T> {
    fn index_mut(&mut self, handle: &Handle<T>) -> &mut Self::Output {
        &mut self.values[handle.index]
    }
}

impl<T> std::ops::Index<&WeakHandle<T>> for Store<T> {
    type Output = T;

    fn index(&self, handle: &WeakHandle<T>) -> &Self::Output {
        &self.values[handle.index]
    }
}

impl<T> std::ops::IndexMut<&WeakHandle<T>> for Store<T> {
    fn index_mut(&mut self, handle: &WeakHandle<T>) -> &mut Self::Output {
        &mut self.values[handle.index]
    }
}

/// Cheap to clone handle to a value in a store
///
/// When the handle is dropped, the value is removed from the store
pub struct Handle<T> {
    index: HandleIndex,
    free_tx: flume::Sender<HandleIndex>,
    _marker: PhantomData<T>,
}

impl<T> Handle<T> {
    pub fn downgrade(&self) -> WeakHandle<T> {
        WeakHandle {
            index: self.index,
            _marker: PhantomData,
        }
    }
}

impl<T> Drop for Handle<T> {
    fn drop(&mut self) {
        self.free_tx.send(self.index).ok();
    }
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            free_tx: self.free_tx.clone(),
            _marker: PhantomData,
        }
    }
}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl<T> Eq for Handle<T> {}
impl<T> Debug for Handle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Handle").field(&self.index).finish()
    }
}
pub struct WeakHandle<T> {
    index: HandleIndex,
    _marker: PhantomData<T>,
}

impl<T> WeakHandle<T> {
    pub fn upgrade(&self, store: &Store<T>) -> Option<Handle<T>> {
        if store.values.contains_key(self.index) {
            Some(Handle {
                index: self.index,
                free_tx: store.free_tx.clone(),
                _marker: PhantomData,
            })
        } else {
            None
        }
    }
}

impl<T> Copy for WeakHandle<T> {}

impl<T> Clone for WeakHandle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> PartialEq for WeakHandle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl<T> Eq for WeakHandle<T> {}
impl<T> Debug for WeakHandle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Handle").field(&self.index).finish()
    }
}
