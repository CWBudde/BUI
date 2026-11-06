//! No leaks: thousands of decodes and drops return every byte, counted by a
//! Rust allocator and Blender's MEM counters (one test: the counter is global).

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::Arc;

use blenderui::{sys, Image};

/// Bytes the Rust side holds (alloc - dealloc).
struct Counting;
static LIVE: AtomicIsize = AtomicIsize::new(0);

// SAFETY: forwards to the system allocator; only counts.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        LIVE.fetch_add(l.size() as isize, Ordering::Relaxed);
        // SAFETY: same layout contract as the caller's.
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        LIVE.fetch_sub(l.size() as isize, Ordering::Relaxed);
        // SAFETY: `p` came from `alloc` with `l`.
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        LIVE.fetch_add(n as isize - l.size() as isize, Ordering::Relaxed);
        // SAFETY: as `dealloc` + `alloc`.
        unsafe { System.realloc(p, l, n) }
    }
}

#[global_allocator]
static A: Counting = Counting;

fn fixture(name: &str) -> String {
    format!("{}/../../../tests/images/{name}", env!("CARGO_MANIFEST_DIR")).replace('\\', "/")
}

#[test]
fn decode_and_drop_returns_all_memory() {
    let bytes = std::fs::read(fixture("quad.jpg")).unwrap();
    // warm up (thread-locals, first-use tables)
    drop(Image::decode(&bytes, 0, 0));
    let (rust0, c0) = (LIVE.load(Ordering::Relaxed), unsafe { sys::MEM_get_memory_blocks_in_use() });
    for i in 0..2000u32 {
        let img = Arc::new(Image::decode(&bytes, 8 - i % 4, 6).unwrap());
        assert!(img.width > 0);
    }
    assert_eq!(LIVE.load(Ordering::Relaxed), rust0, "Rust allocations returned");
    // SAFETY: a counter read.
    assert_eq!(unsafe { sys::MEM_get_memory_blocks_in_use() }, c0, "C (MEM) blocks returned");
}

