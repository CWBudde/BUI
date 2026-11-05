//! Sorting a table by a column into a [`RowMask`]: never reorders data.
//!
//! * A column whose cells are all numbers of one variant, or all text,
//!   sorts through **typed keys**: one pass extracts an order-preserving
//!   `u64` per row (see [`Cell::number_key`], [`Cell::text_prefix`]), then
//!   `(key, position)` pairs sort with no cell access and no dynamic call per
//!   comparison. Text columns resolve equal 8-byte prefixes with a stable
//!   full-text sort of just those runs.
//! * Anything else (mixed variants, [`SortKind::Custom`]) sorts with a
//!   comparator on the row index list: the generic path the typed one must
//!   agree with (it is the reference in the tests).
//! * Both are stable and run on every core above [`PARALLEL_MIN`] rows: chunks
//!   sort on scoped threads, then merge pairwise between two buffers.

use std::cmp::Ordering;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};

use crate::cell::{Cell, CellKind, CellSource, SortKind};
use crate::mask::RowMask;

/// Below this many rows one thread beats spawning.
pub const PARALLEL_MIN: usize = 1 << 18;

fn threads() -> usize {
    std::thread::available_parallelism().map_or(1, usize::from).min(16)
}

/// Stable parallel merge sort of `v` by `cmp`.
pub fn sort_stable<T: Copy + Send + Sync>(v: &mut Vec<T>, cmp: &(impl Fn(&T, &T) -> Ordering + Sync)) {
    sort_impl(v, cmp, true);
}

/// Parallel sort by a *total* order (no two elements compare equal unless
/// identical), so the faster unstable sort gives the stable result.
pub fn sort_total<T: Copy + Send + Sync>(v: &mut Vec<T>, cmp: &(impl Fn(&T, &T) -> Ordering + Sync)) {
    sort_impl(v, cmp, false);
}

fn sort_impl<T: Copy + Send + Sync>(v: &mut Vec<T>, cmp: &(impl Fn(&T, &T) -> Ordering + Sync), stable: bool) {
    let n = v.len();
    let threads = threads();
    let chunk_sort = move |part: &mut [T]| {
        if stable {
            part.sort_by(cmp);
        } else {
            part.sort_unstable_by(cmp);
        }
    };
    if n < PARALLEL_MIN || threads < 2 {
        chunk_sort(v);
        return;
    }
    let chunk = n.div_ceil(threads);
    std::thread::scope(|s| {
        for part in v.chunks_mut(chunk) {
            s.spawn(move || chunk_sort(part));
        }
    });
    let mut src = std::mem::take(v);
    let mut dst = src.clone();
    let mut width = chunk;
    while width < n {
        std::thread::scope(|s| {
            let mut rest = &mut dst[..];
            let mut at = 0;
            while at < n {
                let end = (at + 2 * width).min(n);
                let (out, tail) = rest.split_at_mut(end - at);
                rest = tail;
                let (a, b) = src[at..end].split_at((at + width).min(end) - at);
                s.spawn(move || merge(a, b, out, cmp));
                at = end;
            }
        });
        std::mem::swap(&mut src, &mut dst);
        width *= 2;
    }
    *v = src;
}

fn merge<T: Copy>(a: &[T], b: &[T], out: &mut [T], cmp: &impl Fn(&T, &T) -> Ordering) {
    let (mut i, mut j) = (0, 0);
    for o in out.iter_mut() {
        // stable: take from `a` unless `b` is strictly smaller
        if j >= b.len() || (i < a.len() && cmp(&b[j], &a[i]) != Ordering::Less) {
            *o = a[i];
            i += 1;
        } else {
            *o = b[j];
            j += 1;
        }
    }
}

/// A `(key, position)` pair: positions index the mask being sorted.
type Pair = (u64, u32);

/// What typed extraction found.
struct Extracted {
    pairs: Vec<Pair>,
    /// Text keys that are the whole text (every string fits the prefix).
    exact: bool,
    text: bool,
}

/// The typed `(key, position)` pairs of column `col` over `base`, or `None`
/// when the cells are not all of one typed kind. `descending` inverts the
/// keys so one ascending sort serves both directions.
fn extract<S: CellSource + ?Sized>(src: &S, col: usize, base: &RowMask, n: usize, descending: bool) -> Option<Extracted> {
    let first = src.cell(base.get(0), col);
    let kind = first.kind();
    if kind == CellKind::Empty {
        return None;
    }
    let disc = std::mem::discriminant(&first);
    let (mismatch, inexact) = (AtomicBool::new(false), AtomicBool::new(false));
    let mut pairs: Vec<Pair> = vec![(0, 0); n];
    let fill = |start: usize, part: &mut [Pair]| {
        for (k, slot) in part.iter_mut().enumerate() {
            let pos = start + k;
            let cell = src.cell(base.get(pos), col);
            let key = if std::mem::discriminant(&cell) == disc {
                match cell {
                    Cell::Str(s) => {
                        let b = s.as_bytes();
                        if b.len() > 8 || b.last() == Some(&0) {
                            inexact.store(true, AtomicOrdering::Relaxed);
                        }
                        cell.text_prefix()
                    }
                    _ => cell.number_key(),
                }
            } else {
                None
            };
            let Some(key) = key else {
                mismatch.store(true, AtomicOrdering::Relaxed);
                return;
            };
            *slot = (if descending { !key } else { key }, pos as u32);
        }
    };
    let threads = threads();
    if n < PARALLEL_MIN || threads < 2 {
        fill(0, &mut pairs);
    } else {
        let chunk = n.div_ceil(threads);
        std::thread::scope(|s| {
            for (i, part) in pairs.chunks_mut(chunk).enumerate() {
                let fill = &fill;
                s.spawn(move || fill(i * chunk, part));
            }
        });
    }
    (!mismatch.load(AtomicOrdering::Relaxed)).then(|| Extracted {
        pairs,
        exact: !inexact.load(AtomicOrdering::Relaxed),
        text: kind == CellKind::Text,
    })
}

/// `base` sorted by column `col` (`descending` reverses; ties keep the order
/// of `base`).
#[must_use]
pub fn sorted_by_column<S: CellSource + ?Sized>(src: &S, col: usize, base: &RowMask, tot: usize, descending: bool) -> RowMask {
    let n = base.len(tot);
    if n < 2 {
        return RowMask::Indices(base.to_vec(tot));
    }
    // typed keys; falls back when the cells turn out to be mixed
    if src.sort_kind(col) == SortKind::Cells
        && let Some(e) = extract(src, col, base, n, descending) {
            return finish_typed(src, col, base, e, descending);
        }
    sorted_by_comparator(src, col, base, tot, descending)
}

/// The generic path: a comparator on the row index list. Reference for the
/// typed path; also the path for custom comparators and mixed columns.
#[must_use]
pub fn sorted_by_comparator<S: CellSource + ?Sized>(src: &S, col: usize, base: &RowMask, tot: usize, descending: bool) -> RowMask {
    let mut v = base.to_vec(tot);
    let custom = src.sort_kind(col) == SortKind::Custom;
    sort_stable(&mut v, &|&a: &u32, &b: &u32| {
        let o = if custom {
            src.compare(col, a as usize, b as usize)
        } else {
            src.cell(a as usize, col).sort_cmp(&src.cell(b as usize, col))
        };
        if descending { o.reverse() } else { o }
    });
    RowMask::Indices(v)
}

/// Equal 8-byte prefixes: stable full-text order within each run of `pairs`
/// (runs are in position order).
fn fix_prefix_runs<S: CellSource + ?Sized>(src: &S, col: usize, base: &RowMask, descending: bool, pairs: &mut [Pair]) {
    let mut i = 0;
    while i < pairs.len() {
        let mut j = i + 1;
        while j < pairs.len() && pairs[j].0 == pairs[i].0 {
            j += 1;
        }
        if j - i > 1 {
            pairs[i..j].sort_by(|a, b| {
                let (x, y) = (src.cell(base.get(a.1 as usize), col), src.cell(base.get(b.1 as usize), col));
                let o = x.sort_cmp(&y);
                if descending { o.reverse() } else { o }
            });
        }
        i = j;
    }
}

fn finish_typed<S: CellSource + ?Sized>(src: &S, col: usize, base: &RowMask, e: Extracted, descending: bool) -> RowMask {
    let Extracted { mut pairs, exact, text } = e;
    sort_total(&mut pairs, &|a: &Pair, b: &Pair| a.cmp(b));
    if text && !exact {
        let (n, threads) = (pairs.len(), threads());
        if n < PARALLEL_MIN || threads < 2 {
            fix_prefix_runs(src, col, base, descending, &mut pairs);
        } else {
            // runs are independent: cut the pairs at run boundaries, one piece per thread
            let target = n.div_ceil(threads);
            std::thread::scope(|s| {
                let mut rest: &mut [Pair] = &mut pairs;
                while !rest.is_empty() {
                    let mut cut = target.min(rest.len());
                    while cut < rest.len() && rest[cut].0 == rest[cut - 1].0 {
                        cut += 1;
                    }
                    let (head, tail) = rest.split_at_mut(cut);
                    rest = tail;
                    s.spawn(move || fix_prefix_runs(src, col, base, descending, head));
                }
            });
        }
    }
    RowMask::Indices(pairs.iter().map(|p| base.get(p.1 as usize) as u32).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Rng;

    /// A table of generated columns: ints with many ties, floats with NaN and
    /// infinities, short and long text, mixed numbers, and a custom order.
    struct Grid {
        n: usize,
        seed: u64,
    }

    impl Grid {
        fn h(&self, r: usize, c: usize) -> u64 {
            let mut x = (r as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (c as u64 + self.seed).wrapping_mul(0xD6E8_FEB8_6659_FD93);
            x ^= x >> 29;
            x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
            x ^ (x >> 32)
        }
    }

    const WORDS: [&str; 6] = ["alpha", "beta", "gamma", "delta-long-word-over-8", "delta-long-word-over-9", ""];

    impl CellSource for Grid {
        fn cell(&self, r: usize, c: usize) -> Cell<'_> {
            let h = self.h(r, c);
            match c {
                0 => Cell::I64((h % 50) as i64 - 25),
                1 => Cell::U64(h % 1000),
                2 => match h % 11 {
                    0 => Cell::F64(f64::NAN, 2),
                    1 => Cell::F64(f64::INFINITY, 2),
                    2 => Cell::F64(f64::NEG_INFINITY, 2),
                    3 => Cell::F64(-0.0, 2),
                    _ => Cell::F64((h % 20_000) as f64 / 100.0 - 100.0, 2),
                },
                3 => Cell::Str(WORDS[(h % 6) as usize]),
                4 => if h.is_multiple_of(7) { Cell::F64(0.5, 1) } else { Cell::I64((h % 9) as i64) }, // mixed variants
                5 => Cell::I64((h % 100) as i64),                                          // custom: by |v - 50|
                _ => Cell::Empty,
            }
        }
        fn sort_kind(&self, col: usize) -> SortKind {
            if col == 5 { SortKind::Custom } else { SortKind::Cells }
        }
        fn compare(&self, col: usize, a: usize, b: usize) -> Ordering {
            let k = |r: usize| (self.h(r, col) % 100).abs_diff(50);
            k(a).cmp(&k(b))
        }
    }

    #[test]
    fn typed_path_equals_generic_path() {
        for (n, seed) in [(2usize, 1u64), (3, 2), (1000, 3), (40_000, 4), (100_003, 5)] {
            let g = Grid { n, seed };
            for col in 0..7 {
                for desc in [false, true] {
                    let typed = sorted_by_column(&g, col, &RowMask::Identity, g.n, desc);
                    let generic = sorted_by_comparator(&g, col, &RowMask::Identity, g.n, desc);
                    assert!(typed == generic, "n {n} col {col} desc {desc}");
                }
            }
        }
    }

    #[test]
    fn parallel_typed_paths_equal_generic_path() {
        // above PARALLEL_MIN: threaded key extraction, threaded sort, threaded prefix fix-up
        let g = Grid { n: PARALLEL_MIN + 12_345, seed: 11 };
        for col in [0usize, 2, 3] {
            for desc in [false, true] {
                let typed = sorted_by_column(&g, col, &RowMask::Identity, g.n, desc);
                let generic = sorted_by_comparator(&g, col, &RowMask::Identity, g.n, desc);
                assert!(typed == generic, "col {col} desc {desc}");
            }
        }
    }

    #[test]
    fn sorting_a_filtered_mask_keeps_its_order_on_ties() {
        let g = Grid { n: 60_000, seed: 8 };
        let mut rng = Rng::new(3);
        let mut rows: Vec<u32> = (0..g.n as u32).filter(|_| !rng.next().is_multiple_of(3)).collect();
        // an arbitrary (not ascending) base order: ties must follow it
        for i in (1..rows.len()).rev() {
            rows.swap(i, (rng.next() % (i as u64 + 1)) as usize);
        }
        let base = RowMask::Indices(rows);
        for col in [0usize, 1, 2, 3] {
            for desc in [false, true] {
                let typed = sorted_by_column(&g, col, &base, g.n, desc);
                let generic = sorted_by_comparator(&g, col, &base, g.n, desc);
                assert!(typed == generic, "col {col} desc {desc}");
            }
        }
    }

    #[test]
    fn sorted_output_is_ordered_and_stable() {
        let g = Grid { n: 50_000, seed: 9 };
        let m = sorted_by_column(&g, 1, &RowMask::Identity, g.n, false);
        let v = m.indices().unwrap();
        for w in v.windows(2) {
            let (a, b) = (g.cell(w[0] as usize, 1), g.cell(w[1] as usize, 1));
            match a.sort_cmp(&b) {
                Ordering::Greater => panic!("out of order"),
                Ordering::Equal => assert!(w[0] < w[1], "unstable"),
                Ordering::Less => {}
            }
        }
        let mut seen = v.to_vec();
        seen.sort_unstable();
        assert!(seen.iter().enumerate().all(|(i, &r)| i as u32 == r), "a permutation");
    }

    #[test]
    fn degenerate_sizes() {
        for n in [0usize, 1] {
            let g = Grid { n, seed: 1 };
            assert_eq!(sorted_by_column(&g, 0, &RowMask::Identity, n, false).len(n), n);
        }
        // all-empty column keeps the order
        let g = Grid { n: 10, seed: 1 };
        let m = sorted_by_column(&g, 6, &RowMask::Identity, 10, true);
        assert_eq!(m.indices().unwrap(), (0..10u32).collect::<Vec<_>>().as_slice());
    }

    #[test]
    fn nan_and_negative_zero_are_totally_ordered() {
        let g = Grid { n: 5000, seed: 2 };
        let m = sorted_by_column(&g, 2, &RowMask::Identity, g.n, false);
        let first = g.cell(m.get(0), 2);
        let last = g.cell(m.get(g.n - 1), 2);
        // -inf first (or a -NaN would be, none generated); NaN (positive) last
        assert!(matches!(first, Cell::F64(v, _) if v == f64::NEG_INFINITY));
        assert!(matches!(last, Cell::F64(v, _) if v.is_nan()));
    }

    #[test]
    fn parallel_merge_equals_serial_for_every_width() {
        let mut rng = Rng::new(42);
        for n in [PARALLEL_MIN - 1, PARALLEL_MIN, PARALLEL_MIN + 1, PARALLEL_MIN * 3 + 5] {
            let keys: Vec<u8> = (0..n).map(|_| (rng.next() % 5) as u8).collect();
            let mut v: Vec<u32> = (0..n as u32).collect();
            sort_stable(&mut v, &|&a: &u32, &b: &u32| keys[a as usize].cmp(&keys[b as usize]));
            let mut naive: Vec<u32> = (0..n as u32).collect();
            naive.sort_by_key(|&r| keys[r as usize]);
            assert_eq!(v, naive, "n {n}");
        }
    }

    #[test]
    fn huge_row_index_does_not_overflow_the_key() {
        // positions are u32: a mask is limited to u32::MAX rows by construction
        let p: Pair = (u64::MAX, u32::MAX);
        assert!(p > (u64::MAX, 0));
    }
}
