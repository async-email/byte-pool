//! Pool of byte slices.
//!
//! # Example
//! ```rust
//! use byte_pool::BytePool;
//!
//! // Create a pool
//! let pool = BytePool::new();
//!
//! // Allocate a buffer
//! let mut buf = pool.alloc(1024);
//!
//! // write some data into it
//! for i in 0..100 {
//!   buf[i] = 12;
//! }
//!
//! // Check that we actually wrote sth.
//! assert_eq!(buf[55], 12);
//!
//! // Returns the underlying memory to the pool.
//! drop(buf);
//!
//! // Frees all memory in the pool.
//! drop(pool);
//! ```

use std::alloc::{alloc, dealloc, handle_alloc_error, Layout};
use std::mem;
use std::ops::{Deref, DerefMut};
use std::ptr::{self, NonNull};
use std::sync::Mutex;

/// A pool of byte slices, that reuses memory.
pub struct BytePool {
    list: Mutex<Vec<RawBlock>>,
}

pub struct RawBlock {
    ptr: NonNull<u8>,
    layout: Layout,
}

pub struct Block<'a> {
    data: mem::ManuallyDrop<RawBlock>,
    pool: &'a BytePool,
}

impl Default for BytePool {
    fn default() -> Self {
        BytePool {
            list: Mutex::new(Vec::new()),
        }
    }
}

fn layout_for_size(size: usize) -> Layout {
    let elem_size = mem::size_of::<u8>();
    let alloc_size = size.checked_mul(elem_size).unwrap();
    let align = mem::align_of::<u8>();
    Layout::from_size_align(alloc_size, align).unwrap()
}

impl BytePool {
    /// Constructs a new pool.
    pub fn new() -> Self {
        BytePool::default()
    }

    /// Allocates a new `Block`, which represents a fixed sice byte slice.
    /// If `Block` is dropped, the memory is _not_ freed, but rather it is returned into the pool.
    pub fn alloc(&self, size: usize) -> Block<'_> {
        assert!(size > 0, "Can not allocate empty blocks");

        // check the last 4 blocks
        let mut lock = self.list.lock().unwrap();
        let end = lock.len();
        let start = if end > 4 { end - 4 } else { 0 };

        for i in start..end {
            if lock[i].layout.size() == size {
                // found one, reuse it
                return Block::new(lock.remove(i), self);
            }
        }
        drop(lock);

        // allocate a new block
        let data = RawBlock::alloc(size);
        Block::new(data, self)
    }

    fn push_raw_block(&self, block: RawBlock) {
        self.list.lock().unwrap().push(block);
    }
}

impl<'a> Drop for Block<'a> {
    fn drop(&mut self) {
        let data = mem::ManuallyDrop::into_inner(unsafe { ptr::read(&self.data) });
        self.pool.push_raw_block(data);
    }
}

impl RawBlock {
    pub fn alloc(size: usize) -> Self {
        // TODO: consider caching the layout
        let layout = layout_for_size(size);
        debug_assert!(layout.size() > 0);

        let ptr = unsafe { alloc(layout) };
        RawBlock {
            ptr: NonNull::new(ptr).unwrap_or_else(|| handle_alloc_error(layout)),
            layout,
        }
    }
}

impl Drop for RawBlock {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.ptr.as_mut(), self.layout);
        }
    }
}

impl Deref for RawBlock {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.layout.size()) }
    }
}

impl DerefMut for RawBlock {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_mut(), self.layout.size()) }
    }
}

impl<'a> Block<'a> {
    pub fn new(data: RawBlock, pool: &'a BytePool) -> Self {
        Block {
            data: mem::ManuallyDrop::new(data),
            pool,
        }
    }
}

impl<'a> Deref for Block<'a> {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.data.deref()
    }
}

impl<'a> DerefMut for Block<'a> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data.deref_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basics() {
        let pool = BytePool::new();

        for i in 0..100 {
            let mut block_1k = pool.alloc(1 * 1024);
            let mut block_4k = pool.alloc(4 * 1024);

            for el in block_1k.deref_mut() {
                *el = i as u8;
            }

            for el in block_4k.deref_mut() {
                *el = i as u8;
            }

            for el in block_1k.deref() {
                assert_eq!(*el, i as u8);
            }

            for el in block_4k.deref() {
                assert_eq!(*el, i as u8);
            }
        }
    }
}
