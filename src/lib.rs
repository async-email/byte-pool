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

use std::alloc::{alloc, dealloc, handle_alloc_error, realloc, Layout};
use std::fmt;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::ptr::{self, NonNull};
use std::sync::Mutex;

/// A pool of byte slices, that reuses memory.
#[derive(Debug)]
pub struct BytePool {
    list: Mutex<Vec<RawBlock>>,
}

pub struct RawBlock {
    ptr: NonNull<u8>,
    layout: Layout,
}

unsafe impl Sync for RawBlock {}
unsafe impl Send for RawBlock {}

#[cfg(feature = "stable_deref")]
unsafe impl stable_deref_trait::StableDeref for RawBlock {}

pub struct Block<'a> {
    data: mem::ManuallyDrop<RawBlock>,
    pool: &'a BytePool,
}

impl fmt::Debug for Block<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Block").field("data", &self.data).finish()
    }
}

impl fmt::Debug for RawBlock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RawBlock({:?})", self.deref())
    }
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
    fn alloc(size: usize) -> Self {
        // TODO: consider caching the layout
        let layout = layout_for_size(size);
        debug_assert!(layout.size() > 0);

        let ptr = unsafe { alloc(layout) };
        RawBlock {
            ptr: NonNull::new(ptr).unwrap_or_else(|| handle_alloc_error(layout)),
            layout,
        }
    }

    fn grow(&mut self, new_size: usize) {
        // TODO: use grow_in_place once it stablizies and possibly via a flag.
        assert!(new_size > 0);
        let new_layout = Layout::from_size_align(new_size, self.layout.align()).unwrap();
        let new_ptr = unsafe { realloc(self.ptr.as_mut(), self.layout, new_layout.size()) };
        self.ptr = NonNull::new(new_ptr).unwrap_or_else(|| handle_alloc_error(self.layout));
        self.layout = new_layout;
    }

    fn shrink(&mut self, new_size: usize) {
        // TODO: use shrink_in_place once it stablizies and possibly via a flag.
        assert!(new_size > 0);
        let new_layout = Layout::from_size_align(new_size, self.layout.align()).unwrap();
        let new_ptr = unsafe { realloc(self.ptr.as_mut(), self.layout, new_layout.size()) };
        self.ptr = NonNull::new(new_ptr).unwrap_or_else(|| handle_alloc_error(self.layout));
        self.layout = new_layout;
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
    fn new(data: RawBlock, pool: &'a BytePool) -> Self {
        Block {
            data: mem::ManuallyDrop::new(data),
            pool,
        }
    }

    /// Resizes a block to a new size
    pub fn realloc(&mut self, new_size: usize) {
        use std::cmp::Ordering::*;

        match new_size.cmp(&self.size()) {
            Greater => self.data.grow(new_size),
            Less => self.data.shrink(new_size),
            Equal => {}
        }
    }

    /// Returns the amount of bytes this block has.
    pub fn size(&self) -> usize {
        self.data.layout.size()
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

    #[test]
    fn realloc() {
        let pool = BytePool::new();

        let mut buf = pool.alloc(10);
        assert_eq!(buf.len(), 10);
        for el in buf.iter_mut() {
            *el = 1;
        }

        buf.realloc(512);
        assert_eq!(buf.len(), 512);
        for el in buf.iter().take(10) {
            assert_eq!(*el, 1);
        }

        buf.realloc(5);
        assert_eq!(buf.len(), 5);
        for el in buf.iter() {
            assert_eq!(*el, 1);
        }
    }
}
