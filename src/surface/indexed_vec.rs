//! File: src/surface/indexed_vec.rs

#![allow(dead_code)]

use std::{collections::HashMap, hash::Hash};

/// A value that exposes a lookup key.
pub(crate) trait Keyed<K> {
    /// Returns the value's lookup key.
    fn key(&self) -> &K;
}

/// A dense vec-backed store with O(1) average lookup by key and O(n) iteration.
pub(crate) struct IndexedVec<K, V>
where
    K: Eq + Hash + Clone,
    V: Keyed<K>,
{
    index: HashMap<K, usize>, // Mapped `K` to index.
    items: Vec<V>,            // Items `V` stored.
}

impl<K, V> IndexedVec<K, V>
where
    K: Eq + Hash + Clone,
    V: Keyed<K>,
{
    /// Creates an empty `IndexedVec`.
    pub fn new() -> Self {
        Self {
            index: HashMap::new(),
            items: Vec::new(),
        }
    }

    /// Returns the number of stored items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Inserts `value`.
    ///
    /// Replaces and returns the existing value if its key is already present.
    /// Otherwise inserts the value and returns `None`.
    pub fn insert(&mut self, value: V) -> Option<V> {
        let key = value.key().clone();

        if let Some(&idx) = self.index.get(&key) {
            return Some(std::mem::replace(&mut self.items[idx], value));
        }

        let idx = self.items.len();
        self.items.push(value);
        self.index.insert(key, idx);
        None
    }

    /// Inserts `value` at `index`.
    ///
    /// Panics if `index` is out of bounds or if the value's key already exists.
    pub fn insert_at(&mut self, index: usize, value: V) {
        assert!(index <= self.items.len(), "insert index out of bounds");
        assert!(
            !self.index.contains_key(value.key()),
            "duplicate key inserted into IndexedVec"
        );

        self.items.insert(index, value);
        self.rebuild_index();
    }

    /// Locates the value identified by `key`.
    pub fn get(&self, key: &K) -> Option<&V> {
        let idx = *self.index.get(key)?;
        Some(&self.items[idx])
    }

    /// Locates the mutable value identified by `key`.
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let idx = *self.index.get(key)?;
        Some(&mut self.items[idx])
    }

    /// Removes and returns the value identified by `key`.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let idx = self.index.remove(key)?;
        let removed = self.items.swap_remove(idx);

        if idx < self.items.len() {
            let moved_key = self.items[idx].key().clone();
            self.index.insert(moved_key, idx);
        }

        Some(removed)
    }

    /// Returns an iterator over stored values.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &V> + ExactSizeIterator + '_ {
        self.items.iter()
    }

    /// Returns a mutable iterator over stored values.
    pub fn iter_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut V> + ExactSizeIterator + '_ {
        self.items.iter_mut()
    }

    /// Returns the stored values as a slice.
    pub fn as_slice(&self) -> &[V] {
        &self.items
    }

    /// Rebuilds the lookup index from the current item order.
    fn rebuild_index(&mut self) {
        self.index.clear();
        self.index.reserve(self.items.len());

        for (idx, value) in self.items.iter().enumerate() {
            self.index.insert(value.key().clone(), idx);
        }
    }
}
