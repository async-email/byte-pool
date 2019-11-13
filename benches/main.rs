#![feature(test)]

extern crate test;

use test::Bencher;

use byte_pool::BytePool;

fn touch_obj(buf: &mut [u8], size: usize) {
    assert_eq!(buf.len(), size);
    buf[0] = 1;
}

macro_rules! benches_for_size {
    ($size:expr, $name1:ident, $name2:ident) => {
        #[bench]
        fn $name1(b: &mut Bencher) {
            b.bytes = $size as u64;

            b.iter(|| {
                // alloc
                let mut buf = vec![0u8; $size];
                touch_obj(&mut buf, $size);
            });
        }

        #[bench]
        fn $name2(b: &mut Bencher) {
            b.bytes = $size as u64;
            let pool = BytePool::new();

            b.iter(|| {
                // alloc
                let mut buf = pool.alloc($size);
                touch_obj(&mut buf, $size);
            });
        }
    };
}

benches_for_size!(256, base_line_vec_256b, pool_256b);
benches_for_size!(1 * 1024, base_line_vec_1k, pool_1k);
benches_for_size!(4 * 1024, base_line_vec_4k, pool_4k);
benches_for_size!(8 * 1024, base_line_vec_8k, pool_8k);

#[bench]
fn base_line_vec_mixed(b: &mut Bencher) {
    let mut i = 0;

    b.iter(|| {
        // alternate between two sizes
        let size = if i % 2 == 0 { 1024 } else { 4096 };
        let mut buf = vec![0u8; size];
        touch_obj(&mut buf, size);

        i += 1;
    });
}

#[bench]
fn pool_mixed(b: &mut Bencher) {
    let mut i = 0;

    let pool = BytePool::new();

    b.iter(|| {
        // alternate between two sizes
        let size = if i % 2 == 0 { 1024 } else { 4096 };
        let mut buf = pool.alloc(size);
        touch_obj(&mut buf, size);

        i += 1;
    });
}

#[bench]
fn base_vec_grow(b: &mut Bencher) {
    let mut size = 16;

    b.iter(|| {
        let mut buf = vec![0u8; size];
        touch_obj(&mut buf, size);

        size = (size * 2).min(4 * 1024);
        buf.resize(size, 0);
        touch_obj(&mut buf, size);
    });
}

#[bench]
fn pool_grow(b: &mut Bencher) {
    let mut size = 16;
    let pool = BytePool::new();

    b.iter(|| {
        let mut buf = pool.alloc(size);
        touch_obj(&mut buf, size);

        size = (size * 2).min(4 * 1024);
        buf.realloc(size);
        touch_obj(&mut buf, size);
    });
}
