//! Pool of byte slices.
//!
//! # Example
//! ```rust
//! use byte_pool::BytePool;
//!
//! // Create a pool
//! let pool = BytePool::new();
//!
//! // Allocate a buffer with capacity 1024.
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

use std::fmt;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::ptr;
use std::sync::Mutex;

/// A pool of byte slices, that reuses memory.
#[derive(Debug)]
pub struct BytePool {
    list: Mutex<Vec<Vec<u8>>>,
}

/// The value returned by an allocation of the pool.
/// When it is dropped the memory gets returned into the pool, and is not zeroed.
/// If that is a concern, you must clear the data yourself.
pub struct Block<'a> {
    data: mem::ManuallyDrop<Vec<u8>>,
    pool: &'a BytePool,
}

impl fmt::Debug for Block<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Block").field("data", &self.data).finish()
    }
}

impl Default for BytePool {
    fn default() -> Self {
        BytePool {
            list: Mutex::new(Vec::new()),
        }
    }
}

impl BytePool {
    /// Constructs a new pool.
    pub fn new() -> Self {
        BytePool::default()
    }

    /// Allocates a new `Block`, which represents a fixed sice byte slice.
    /// If `Block` is dropped, the memory is _not_ freed, but rather it is returned into the pool.
    /// The returned `Block` contains arbitrary data, and must be zeroed or overwritten,
    /// in cases this is needed.
    pub fn alloc(&self, size: usize) -> Block<'_> {
        assert!(size > 0, "Can not allocate empty blocks");

        // check the last 4 blocks
        let mut lock = self.list.lock().unwrap();
        let end = lock.len();
        let start = if end > 4 { end - 4 } else { 0 };

        for i in start..end {
            if lock[i].capacity() == size {
                // found one, reuse it
                let data = lock.remove(i);
                return Block::new(data, self);
            }
        }
        drop(lock);

        // allocate a new block
        let data = vec![0u8; size];
        Block::new(data, self)
    }

    fn push_raw_block(&self, block: Vec<u8>) {
        self.list.lock().unwrap().push(block);
    }
}

impl<'a> Drop for Block<'a> {
    fn drop(&mut self) {
        let data = mem::ManuallyDrop::into_inner(unsafe { ptr::read(&self.data) });
        self.pool.push_raw_block(data);
    }
}

impl<'a> Block<'a> {
    fn new(data: Vec<u8>, pool: &'a BytePool) -> Self {
        Block {
            data: mem::ManuallyDrop::new(data),
            pool,
        }
    }

    /// Resizes a block to a new size.
    pub fn realloc(&mut self, new_size: usize) {
        use std::cmp::Ordering::*;

        assert!(new_size > 0);
        match new_size.cmp(&self.size()) {
            Greater => self.data.resize(new_size, 0u8),
            Less => {
                self.data.truncate(new_size);
                self.shrink_to_fit();
            }
            Equal => {}
        }
    }

    /// Returns the amount of bytes this block has.
    pub fn size(&self) -> usize {
        self.data.capacity()
    }
}

impl<'a> Deref for Block<'a> {
    type Target = Vec<u8>;

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

        let _slice: &[u8] = &buf;

        assert_eq!(buf.capacity(), 10);
        for i in 0..10 {
            buf[i] = 1;
        }

        buf.realloc(512);
        assert_eq!(buf.capacity(), 512);
        for el in buf.iter().take(10) {
            assert_eq!(*el, 1);
        }

        buf.realloc(5);
        assert_eq!(buf.capacity(), 5);
        for el in buf.iter() {
            assert_eq!(*el, 1);
        }
    }

    #[test]
    fn multi_thread() {
        let pool = std::sync::Arc::new(BytePool::new());

        let pool1 = pool.clone();
        let h1 = std::thread::spawn(move || {
            for _ in 0..100 {
                let mut buf = pool1.alloc(64);
                buf[10] = 10;
            }
        });

        let pool2 = pool.clone();
        let h2 = std::thread::spawn(move || {
            for _ in 0..100 {
                let mut buf = pool2.alloc(64);
                buf[10] = 10;
            }
        });

        h1.join().unwrap();
        h2.join().unwrap();

        // two threads allocating in parallel will need 2 buffers
        assert_eq!(pool.list.lock().unwrap().len(), 2);
    }
}
