use std::ops::{Deref, DerefMut};

pub struct BytePool {
    // list: Vec<Block>,
}

pub struct Block<'a> {
    pool: &'a BytePool,
    data: Box<[u8]>,
}

impl Default for BytePool {
    fn default() -> Self {
        BytePool { /*list: Vec::new()*/ }
    }
}

impl BytePool {
    pub fn new() -> Self {
        BytePool::default()
    }

    pub fn alloc(&self, size: usize) -> Block<'_> {
        // TODO: reuse blocks

        Block::new(size, &self)
    }
}

impl<'a> Block<'a> {
    fn new(size: usize, pool: &'a BytePool) -> Self {
        // TODO: use manual allocation

        Block {
            pool,
            data: vec![0u8; size].into_boxed_slice(),
        }
    }
}

impl<'a> Deref for Block<'a> {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &*self.data
    }
}

impl<'a> DerefMut for Block<'a> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.data
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
