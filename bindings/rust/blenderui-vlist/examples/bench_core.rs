//! Micro-benchmarks of the height index against a plain flat prefix-sum
//! `Vec` (the simplest correct implementation) at n = 100, 10k, 1M, 10M.
//!
//! `cargo run --release -p blenderui-vlist --example bench_core`
//!
//! Heap numbers come from a counting global allocator (live bytes), not RSS.
//! Timings are medians of 5 runs of a batch; all inputs are seeded.

#![allow(clippy::many_single_char_names, clippy::too_many_lines, clippy::missing_const_for_fn)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::time::Instant;

use blenderui_paint::testing::Rng;
use blenderui_vlist::{Estimator, Fx, HeightIndex};

struct Tracking;
static LIVE: AtomicUsize = AtomicUsize::new(0);

// SAFETY: forwards to the system allocator, adding relaxed counters.
unsafe impl GlobalAlloc for Tracking {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        LIVE.fetch_add(l.size(), Relaxed);
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        LIVE.fetch_sub(l.size(), Relaxed);
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        LIVE.fetch_add(n, Relaxed);
        LIVE.fetch_sub(l.size(), Relaxed);
        unsafe { System.realloc(p, l, n) }
    }
}

#[global_allocator]
static A: Tracking = Tracking;

fn est() -> Estimator {
    Estimator::new(12.0, 19.0, 7.0, 400.0)
}

fn live() -> usize {
    LIVE.load(Relaxed)
}

/// Median over 5 runs of `f`, returning ns per op for `ops` operations per run.
fn ns_per_op(ops: usize, mut f: impl FnMut()) -> f64 {
    let mut runs: Vec<f64> = (0..5)
        .map(|_| {
            let t = Instant::now();
            f();
            t.elapsed().as_nanos() as f64 / ops as f64
        })
        .collect();
    runs.sort_by(f64::total_cmp);
    runs[2]
}

/// The baseline: one flat prefix-sum `Vec`, binary search for lookups, a
/// suffix update for a height change.
struct Flat {
    prefix: Vec<u64>,
}

impl Flat {
    fn new(heights: &[u32]) -> Self {
        let mut prefix = Vec::with_capacity(heights.len() + 1);
        let mut acc = 0u64;
        prefix.push(0);
        for &h in heights {
            acc += u64::from(h);
            prefix.push(acc);
        }
        Self { prefix }
    }
    fn locate(&self, off: u64) -> usize {
        self.prefix.partition_point(|&p| p <= off).saturating_sub(1).min(self.prefix.len() - 2)
    }
    fn append(&mut self, h: u32) {
        let last = self.prefix[self.prefix.len() - 1];
        self.prefix.push(last + u64::from(h));
    }
    fn prepend(&mut self, k: usize, h: u32) {
        let add = k as u64 * u64::from(h);
        let mut p: Vec<u64> = (0..=k).map(|i| i as u64 * u64::from(h)).collect();
        p.pop();
        p.extend(self.prefix.iter().map(|&x| x + add));
        self.prefix = p;
    }
    fn set(&mut self, i: usize, h: u32) {
        let d = u64::from(h).wrapping_sub(self.prefix[i + 1] - self.prefix[i]);
        for p in &mut self.prefix[i + 1..] {
            *p = p.wrapping_add(d);
        }
    }
}

fn heights(n: usize, rng: &mut Rng) -> Vec<u32> {
    (0..n).map(|_| 600 + rng.below(3000) as u32).collect()
}

fn measure_all(idx: &mut HeightIndex, h: &[u32]) {
    for (c, chunk) in h.chunks(1 << 20).enumerate() {
        let mut ups: Vec<(usize, Fx)> = chunk.iter().enumerate().map(|(i, &x)| (c * (1 << 20) + i, Fx::from_units(u64::from(x)))).collect();
        idx.set_heights(&mut ups);
    }
}

fn main() {
    println!("| n | what | HeightIndex | flat Vec prefix sums |");
    println!("|---|---|---|---|");
    for n in [100usize, 10_000, 1_000_000, 10_000_000] {
        let mut rng = Rng::new(n as u64);
        let q = 200_000usize;

        // build estimated, no per-item data (the source answers bytes in O(1))
        let before = live();
        let build = ns_per_op(1, || {
            black_box(HeightIndex::estimated(n, est(), |r| r.len() as u64 * 60).unwrap());
        });
        let idx0 = HeightIndex::estimated(n, est(), |r| r.len() as u64 * 60).unwrap();
        let est_bytes = live() - before;
        println!("| {n} | build, nothing measured | {:.2} ms, {:.3} B/item | n/a (needs every height) |", build / 1e6, est_bytes as f64 / n as f64);

        // queries on the untouched estimate
        let total = idx0.total();
        let offs: Vec<u64> = (0..q).map(|_| rng.below(total)).collect();
        let t = ns_per_op(q, || {
            for &o in &offs {
                black_box(idx0.locate(black_box(o)));
            }
        });
        println!("| {n} | offset -> item, estimated blocks | {t:.0} ns | n/a |");
        drop(idx0);

        // only the tail touched (open at the newest message)
        let h = heights(n, &mut rng);
        let before = live();
        let mut tail = HeightIndex::estimated(n, est(), |r| r.len() as u64 * 60).unwrap();
        let touched = 60.min(n);
        let mut ups: Vec<(usize, Fx)> = (n - touched..n).map(|i| (i, Fx::from_units(u64::from(h[i])))).collect();
        tail.set_heights(&mut ups);
        println!("| {n} | heap after touching only the last {touched} items | {:.3} B/item ({} KB) | n/a (needs all heights) |", (live() - before) as f64 / n as f64, (live() - before) / 1024);
        drop(tail);

        // fully measured
        let before = live();
        let mut idx = HeightIndex::estimated(n, est(), |r| r.len() as u64 * 60).unwrap();
        let t0 = Instant::now();
        measure_all(&mut idx, &h);
        let t_all = t0.elapsed().as_nanos() as f64 / n as f64;
        let idx_bytes = live() - before;
        let flat_before = live();
        let t0 = Instant::now();
        let mut flat = Flat::new(&h);
        let t_flat_build = t0.elapsed().as_nanos() as f64 / n as f64;
        let flat_bytes = live() - flat_before;
        println!("| {n} | build, all measured (per item) | {t_all:.1} ns, {:.2} B/item | {t_flat_build:.1} ns, {:.2} B/item |", idx_bytes as f64 / n as f64, flat_bytes as f64 / n as f64);

        let total = idx.total();
        let offs: Vec<u64> = (0..q).map(|_| rng.below(total)).collect();
        let items: Vec<usize> = (0..q).map(|_| rng.below(n as u64) as usize).collect();
        let a = ns_per_op(q, || {
            for &o in &offs {
                black_box(idx.locate(black_box(o)));
            }
        });
        let b = ns_per_op(q, || {
            for &o in &offs {
                black_box(flat.locate(black_box(o)));
            }
        });
        println!("| {n} | offset -> item (random) | {a:.0} ns | {b:.0} ns |");
        let a = ns_per_op(q, || {
            for &i in &items {
                black_box(idx.offset_of(black_box(i)));
            }
        });
        let b = ns_per_op(q, || {
            for &i in &items {
                black_box(flat.prefix[black_box(i)]);
            }
        });
        println!("| {n} | item -> offset (random) | {a:.0} ns | {b:.1} ns |");

        // a frame's worth: the item at a random offset and the next 30 heights
        let a = ns_per_op(q / 10, || {
            for &o in offs.iter().take(q / 10) {
                let l = idx.locate(black_box(o)).unwrap();
                let mut acc = 0u64;
                let mut k = 0;
                idx.walk_forward(l.item, |_, h, _| {
                    acc += u64::from(h.units());
                    k += 1;
                    k < 30
                });
                black_box(acc);
            }
        });
        let b = ns_per_op(q / 10, || {
            for &o in offs.iter().take(q / 10) {
                let i = flat.locate(black_box(o));
                let end = (i + 30).min(flat.prefix.len() - 1);
                let mut acc = 0u64;
                for w in flat.prefix[i..=end].windows(2) {
                    acc += black_box(w[1] - w[0]);
                }
                black_box(acc);
            }
        });
        println!("| {n} | a frame's lookups: locate + walk 30 items | {a:.0} ns | {b:.0} ns |");

        let k = if n >= 10_000_000 { 2_000 } else { 20_000 };
        let a = ns_per_op(k, || {
            for (j, &i) in items.iter().take(k).enumerate() {
                idx.set_height(black_box(i), Fx::from_units(700 + (j % 50) as u64), true);
            }
        });
        let kf = if n >= 1_000_000 { 200 } else { k };
        let b = ns_per_op(kf, || {
            for (j, &i) in items.iter().take(kf).enumerate() {
                flat.set(black_box(i), 700 + (j % 50) as u32);
            }
        });
        println!("| {n} | set one height, random item | {a:.0} ns | {b:.0} ns |");
        let a = ns_per_op(k, || {
            for j in 0..k {
                idx.set_height(n - 1, Fx::from_units(700 + (j % 50) as u64), true);
            }
        });
        let b = ns_per_op(kf, || {
            for j in 0..kf {
                flat.set(n - 1, 700 + (j % 50) as u32);
            }
        });
        println!("| {n} | set the last item (streaming) | {a:.0} ns | {b:.1} ns |");

        // structure: append one, prepend a page
        let a = ns_per_op(10_000, || {
            for _ in 0..10_000 {
                let len = idx.len();
                idx.insert_items(len, 1, 60).unwrap();
            }
        });
        let a2 = ns_per_op(20, || {
            for _ in 0..20 {
                idx.insert_items(0, 500, 500 * 60).unwrap();
            }
        });
        let fa = ns_per_op(10_000, || {
            for _ in 0..10_000 {
                flat.append(700);
            }
        });
        let fp = ns_per_op(if n >= 1_000_000 { 2 } else { 20 }, || {
            for _ in 0..(if n >= 1_000_000 { 2 } else { 20 }) {
                flat.prepend(500, 700);
            }
        });
        println!("| {n} | append 1 item / prepend a 500-item page | {a:.0} ns / {:.1} us | {fa:.0} ns / {:.1} us |", a2 / 1e3, fp / 1e3);
        black_box(&idx);
        black_box(&flat);
        println!("|  |  |  |  |");
    }
}
