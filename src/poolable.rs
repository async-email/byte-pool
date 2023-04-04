use std::collections::HashMap;
use std::hash::{BuildHasher, Hash};

/// The trait required to be able to use a type in `BytePool`.
pub trait Poolable {
    fn empty(&self) -> bool;
    fn len(&self) -> usize;
    fn capacity(&self) -> usize;
    fn resize(&mut self, count: usize);
    fn reset(&mut self);
    fn alloc(size: usize) -> Self;
    fn alloc_and_fill(size: usize) -> Self;
}

impl<T: Default + Clone> Poolable for Vec<T> {
    fn empty(&self) -> bool {
        self.len() == 0
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn capacity(&self) -> usize {
        self.capacity()
    }

    fn resize(&mut self, count: usize) {
        self.resize(count, T::default());
    }

    fn reset(&mut self) {
        self.clear();
    }

    fn alloc(size: usize) -> Self {
        Vec::<T>::with_capacity(size)
    }

    fn alloc_and_fill(size: usize) -> Self {
        vec![T::default(); size]
    }
}

impl<K, V, S> Poolable for HashMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher + Default,
{
    fn empty(&self) -> bool {
        self.len() == 0
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn capacity(&self) -> usize {
        self.capacity()
    }

    fn resize(&mut self, _count: usize) {
        // do thing
    }

    fn reset(&mut self) {
        self.clear();
    }

    fn alloc(size: usize) -> Self {
        Self::alloc_and_fill(size)
    }

    fn alloc_and_fill(size: usize) -> Self {
        // not actually filling the HaspMap though
        HashMap::with_capacity_and_hasher(size, Default::default())
    }
}

/// A trait allowing for efficient reallocation.
pub trait Realloc {
    fn realloc(&mut self, new_size: usize);
}

impl<T: Default + Clone> Realloc for Vec<T> {
    fn realloc(&mut self, new_size: usize) {
        use std::cmp::Ordering::*;

        assert!(new_size > 0);
        match new_size.cmp(&self.capacity()) {
            Greater => self.reserve(new_size - self.capacity()),
            Less => {
                self.truncate(new_size);
                self.shrink_to_fit();
            }
            Equal => {}
        }
    }
}

impl<K, V, S> Realloc for HashMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    fn realloc(&mut self, new_size: usize) {
        use std::cmp::Ordering::*;

        assert!(new_size > 0);
        match new_size.cmp(&self.capacity()) {
            Greater => {
                let current = self.capacity();
                let diff = new_size - current;
                self.reserve(diff);
            }
            Less => {
                self.shrink_to_fit();
            }
            Equal => {}
        }
    }
}
