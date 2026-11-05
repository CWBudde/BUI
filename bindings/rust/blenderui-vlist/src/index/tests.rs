//! Randomized tests of the index against a model: a `Vec<Option<u32>>` of
//! the heights the test set (`None`: the index estimates). Every item the
//! model knows must read back exactly, and the index must be consistent
//! with itself for all the others: offsets are the running sums of
//! heights, `locate` inverts `offset_of`, totals add up.

use super::*;
use blenderui_paint::testing::Rng;

fn est() -> Estimator {
    Estimator::new(12.0, 19.0, 7.0, 400.0)
}

fn fx(rng: &mut Rng) -> Fx {
    Fx::from_units(1 + rng.below(40 * 64))
}

fn verify(idx: &HeightIndex, model: &[Option<u32>], rng: &mut Rng) {
    idx.check();
    assert_eq!(idx.len(), model.len());
    let n = idx.len();
    let full = n <= 5000;
    let mut run = 0u64;
    let samples: Vec<usize> = if full { (0..n).collect() } else { (0..300).map(|_| rng.below(n as u64) as usize).chain([0, n - 1]).collect() };
    for &i in &samples {
        let (h, exact) = idx.height_of(i).unwrap();
        if let Some(m) = model[i] {
            assert_eq!((h.units(), exact), (m, true), "item {i}");
        }
        assert!(h.units() > 0);
        let off = idx.offset_of(i);
        if full {
            assert_eq!(off, run, "offset of {i}");
            run += u64::from(h.units());
        }
        // consecutive offsets differ by the height
        assert_eq!(idx.offset_of(i + 1) - off, u64::from(h.units()), "span of {i}");
        // locate inverts offset_of, inside and at the top of the item
        let l = idx.locate(off).unwrap();
        assert_eq!((l.item, l.within), (i, 0), "locate top of {i}");
        let mid = u64::from(h.units()) / 2;
        let l = idx.locate(off + mid).unwrap();
        assert_eq!((l.item, l.within), (i, mid), "locate mid of {i}");
    }
    if full {
        assert_eq!(run, idx.total());
    }
    assert_eq!(idx.offset_of(n), idx.total());
    if n > 0 {
        assert_eq!(idx.locate(u64::MAX / 4).unwrap().item, n - 1);
    } else {
        assert!(idx.locate(0).is_none());
    }
}

fn run(n: usize, ops: usize, seed: u64, uniform_start: bool) {
    let mut rng = Rng::new(seed);
    let mut idx = if uniform_start {
        HeightIndex::uniform(n, Fx::from_px(20.0), est()).unwrap()
    } else {
        HeightIndex::estimated(n, est(), |r| r.len() as u64 * 60).unwrap()
    };
    let mut model: Vec<Option<u32>> = vec![if uniform_start { Some(Fx::from_px(20.0).units()) } else { None }; n];
    verify(&idx, &model, &mut rng);
    for op in 0..ops {
        let len = model.len();
        let which = rng.below(9);
        match which {
            0 | 1 if len > 0 => {
                let i = rng.below(len as u64) as usize;
                let h = fx(&mut rng);
                assert!(idx.set_height(i, h, true));
                model[i] = Some(h.units());
            }
            2 if len > 0 => {
                let k = 1 + rng.below(40) as usize;
                let mut ups: Vec<(usize, Fx)> = (0..k).map(|_| (rng.below(len as u64) as usize, fx(&mut rng))).collect();
                // duplicates: the last wins
                let mut last = std::collections::HashMap::new();
                for &(i, h) in &ups {
                    last.insert(i, h);
                }
                for (&i, &h) in &last {
                    model[i] = Some(h.units());
                }
                idx.set_heights(&mut ups);
            }
            3 => {
                // append or prepend (one item or a page), or insert in the middle
                let at = match rng.below(3) {
                    0 => len,
                    1 => 0,
                    _ => rng.below(len as u64 + 1) as usize,
                };
                let count = if rng.below(8) == 0 { 1 + rng.below(1800) as usize } else { 1 + rng.below(5) as usize };
                idx.insert_items(at, count, count as u64 * 50).unwrap();
                model.splice(at..at, std::iter::repeat_n(None, count));
            }
            4 if len > 0 => {
                let at = rng.below(len as u64) as usize;
                let big = rng.below(6) == 0;
                let count = 1 + rng.below(if big { 1500 } else { 4 }) as usize;
                let removed = idx.remove_items(at, count, count as u64 * 50);
                model.drain(at..at + removed);
            }
            5 if !uniform_start => {
                let w = 200.0 + rng.below(600) as f64;
                idx.set_estimator(est().with_text_width(w));
                model.iter_mut().for_each(|m| *m = None);
            }
            6 if len > 0 && !uniform_start => {
                idx.trim(rng.below(len as u64) as usize, 600);
                // trimmed blocks forget their heights: only what the index still reports exact counts
                for (i, m) in model.iter_mut().enumerate() {
                    if m.is_some() && !idx.height_of(i).unwrap().1 {
                        *m = None;
                    }
                }
            }
            7 => idx.observe(rng.below(400), 20.0 + rng.below(100) as f64),
            _ => {}
        }
        // observe may re-sum estimates, which cannot change known heights
        if op % 5 == 0 || len < 3000 {
            verify(&idx, &model, &mut rng);
        }
    }
    verify(&idx, &model, &mut rng);
}

#[test]
fn matches_the_model_at_every_size() {
    for (k, &n) in [0usize, 1, 2, 511, 512, 513, 1024, 1025, 4096, 10_000].iter().enumerate() {
        run(n, 300, 0x1000 + k as u64, false);
        run(n, 150, 0x2000 + k as u64, true);
    }
}

#[test]
fn matches_the_model_at_a_million() {
    run(1_000_000, 60, 77, false);
    run(1_000_000, 40, 78, true);
}

#[test]
fn single_block_bypasses_the_tree() {
    let mut idx = HeightIndex::estimated(100, est(), |_| 5000).unwrap();
    assert_eq!(idx.block_count(), 1);
    for i in 0..100 {
        idx.set_height(i, Fx::from_px(10.0 + i as f64), true);
    }
    assert_eq!(idx.offset_of(3), (10 + 11 + 12) * 64);
    assert_eq!(idx.locate(30 * 64 + 5).unwrap().item, 2);
    idx.check();
}

#[test]
fn prepend_and_append_do_not_rebuild_per_block() {
    let mut idx = HeightIndex::estimated(0, est(), |_| 0).unwrap();
    for _ in 0..200 {
        idx.insert_items(0, 700, 700 * 40).unwrap();
        let n = idx.len();
        idx.insert_items(n, 700, 700 * 40).unwrap();
    }
    idx.check();
    assert_eq!(idx.len(), 280_000);
    assert!(idx.block_count() < 2000);
}

#[test]
fn uniform_ten_million_costs_nothing() {
    let idx = HeightIndex::uniform(10_000_000, Fx::from_px(24.0), est()).unwrap();
    assert_eq!(idx.len(), 10_000_000);
    assert_eq!(idx.total(), 10_000_000 * 24 * 64);
    assert!(idx.heap_bytes() < 2_200_000, "{}", idx.heap_bytes());
    assert_eq!(idx.measured_items(), 0);
    let l = idx.locate(5_000_000 * 24 * 64 + 10).unwrap();
    assert_eq!((l.item, l.within), (5_000_000, 10));
}

#[test]
fn a_different_height_expands_one_block_only() {
    let mut idx = HeightIndex::uniform(1_000_000, Fx::from_px(24.0), est()).unwrap();
    idx.set_height(123_456, Fx::from_px(48.0), true);
    assert_eq!(idx.measured_items(), idx.block_span(123_456).unwrap().0.len());
    assert_eq!(idx.total(), 1_000_000 * 24 * 64 + 24 * 64);
    idx.check();
}

#[test]
fn hostile_inputs_are_refused_or_clamped() {
    assert_eq!(HeightIndex::uniform(usize::MAX, Fx::from_px(1.0), est()).err(), Some(IndexError::TooManyItems));
    let mut idx = HeightIndex::estimated(10, est(), |_| 10).unwrap();
    assert_eq!(idx.insert_items(11, 1, 0), Err(IndexError::OutOfRange));
    assert_eq!(idx.insert_items(0, usize::MAX, 0), Err(IndexError::TooManyItems));
    assert_eq!(idx.remove_items(100, 5, 0), 0);
    assert_eq!(idx.remove_items(8, usize::MAX, 0), 2);
    assert!(!idx.set_height(99, Fx::from_px(5.0), true));
    assert_eq!(Fx::from_px(f64::NAN), Fx::ZERO);
    assert_eq!(Fx::from_px(f64::INFINITY), Fx::MAX);
    assert_eq!(Fx::from_px(-5.0), Fx::ZERO);
    idx.check();
    assert_eq!(idx.len(), 8);
    // empty list queries
    let e = HeightIndex::new(est());
    assert!(e.locate(5).is_none() && e.height_of(0).is_none() && e.block_span(0).is_none());
}

#[test]
fn estimates_converge_to_reality() {
    // real messages are 1.5x the first guess: after observing, estimated blocks follow
    let mut idx = HeightIndex::estimated(100_000, est(), |r| r.len() as u64 * 60).unwrap();
    let before = idx.total();
    for _ in 0..200 {
        idx.observe(60, 1.5 * est().item_raw(60));
    }
    let after = idx.total();
    let ratio = after as f64 / before as f64;
    assert!((ratio - 1.5).abs() < 0.08, "{ratio}");
}

#[test]
fn walks_visit_items_in_order() {
    let mut idx = HeightIndex::estimated(3000, est(), |r| r.len() as u64 * 40).unwrap();
    for i in (0..3000).step_by(7) {
        idx.set_height(i, Fx::from_px(30.0), true);
    }
    let mut seen = Vec::new();
    idx.walk_forward(1000, |i, h, _| {
        seen.push((i, h));
        i < 1100
    });
    assert_eq!(seen.first().unwrap().0, 1000);
    assert_eq!(seen.last().unwrap().0, 1100);
    let mut back = Vec::new();
    idx.walk_backward(1100, |i, h, _| {
        back.push((i, h));
        i > 1000
    });
    back.reverse();
    assert_eq!(back, seen);
    // across a block boundary, backwards to the start
    let mut n = 0;
    idx.walk_backward(2999, |_, _, _| {
        n += 1;
        true
    });
    assert_eq!(n, 3000);
}
