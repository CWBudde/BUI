//! Helpers the cores' tests share: a seeded generator and a counting
//! allocator. A test binary installs the allocator itself
//! (`#[global_allocator] static A: Counting = Counting;`) and then asks
//! [`count_allocs`] how often a closure hit the heap.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

/// A xorshift generator: seeded, so failures reproduce.
pub struct Rng(u64);

impl Rng {
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    #[allow(clippy::should_implement_trait)]
    pub const fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// Uniform-ish in `0..n` (`n > 0`).
    pub const fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// The counting allocator (see the module docs).
pub struct Counting;

thread_local! {
    static ARMED: Cell<bool> = const { Cell::new(false) };
    static COUNT: Cell<usize> = const { Cell::new(0) };
}

// SAFETY: forwards to the system allocator; the counters are const-initialised
// thread-locals without destructors, so counting never allocates.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if ARMED.get() {
            COUNT.set(COUNT.get() + 1);
        }
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        if ARMED.get() {
            COUNT.set(COUNT.get() + 1);
        }
        unsafe { System.realloc(p, l, n) }
    }
}

/// Allocations made by this thread while `f` runs (needs [`Counting`] installed).
pub fn count_allocs(f: impl FnOnce()) -> usize {
    COUNT.set(0);
    ARMED.set(true);
    f();
    ARMED.set(false);
    COUNT.get()
}
