#![feature(test)]

extern crate test;

use test::Bencher;

use byte_pool::BytePool;

#[bench]
fn base_line_vec_4k(b: &mut Bencher) {
    let size = 4 * 1024;

    b.bytes = size as u64;

    b.iter(|| {
        // alloc
        let mut buf = vec![0u8; size];
        // write manual, simulating a Write::write call
        for i in 0..size {
            buf[i] = 1;
        }
    });
}

#[bench]
fn byte_pool_4k(b: &mut Bencher) {
    let size = 4 * 1024;

    b.bytes = size as u64;
    let pool = BytePool::new();

    b.iter(|| {
        // alloc
        let mut buf = pool.alloc(size);
        // write manual, simulating a Write::write call
        for i in 0..size {
            buf[i] = 1;
        }
    });
}
