//! The counting allocator for this crate's tests (shared with the other
//! cores: it lives in `blenderui-paint`).

pub use blenderui_paint::testing::count_allocs;

#[global_allocator]
static A: blenderui_paint::testing::Counting = blenderui_paint::testing::Counting;
