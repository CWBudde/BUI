//! [`HeightIndex`]: one structure for every list size.
//!
//! Items are grouped in blocks of up to [`MAX_BLOCK`]. The top level is a
//! Fenwick tree over blocks ([`fenwick`]) with spare slots on both sides, the
//! bottom level is per-block ([`block`]): a block is `Uniform`, `Estimated`
//! (a count and a byte total) or `Measured` (prefix sums). A list of up to
//! one block is a single block and the tree is bypassed: effectively a flat
//! array. A list of 10M items is about 20k blocks, a 320 KB tree, and the
//! blocks nobody scrolled to stay estimates, so memory follows what was
//! looked at, not the length.
//!
//! The cost model:
//!
//! | operation | cost |
//! |---|---|
//! | offset -> item, item -> offset | O(log blocks) + a binary search in one block |
//! | set one height | O(log blocks) + a suffix add in one block |
//! | `set_heights` (k at once) | one pass per touched block |
//! | append / prepend items | amortized O(1) per block, no shifting (spare slots) |
//! | insert / remove in the middle | one block, plus a tree rebuild when blocks change |
//! | new wrap width | O(blocks): measured blocks forget per-item heights |
//!
//! Blender keeps nothing like it: `textview_draw` walks and wraps lines
//! (`space_info/textview.cc`), `OffsetIndices` are static prefix sums
//! (`BLI_offset_indices.hh`).

pub mod block;
pub mod fenwick;

use std::collections::VecDeque;

use block::{Block, MAX_BLOCK, TARGET_BLOCK};
use fenwick::{Fenwick, Node};

use crate::estimate::Estimator;
use crate::fx::Fx;

/// The most items an index holds (hostile counts are refused, not wrapped).
pub const MAX_ITEMS: usize = 1 << 40;

/// Why an edit was refused.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum IndexError {
    /// More than [`MAX_ITEMS`] items.
    TooManyItems,
    /// A position past the end.
    OutOfRange,
}

/// Where an offset falls.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Located {
    pub item: usize,
    /// Offset inside the item, fixed-point units.
    pub within: u64,
}

/// See the module docs.
pub struct HeightIndex {
    blocks: VecDeque<Block>,
    tree: Fenwick,
    /// Slot of the first block.
    first: usize,
    len: usize,
    total: u64,
    est: Estimator,
}

fn node(b: &Block, est: &Estimator) -> Node {
    Node { h: b.height(est), n: b.n() as u64 }
}

impl HeightIndex {
    /// An empty index.
    #[must_use]
    pub fn new(est: Estimator) -> Self {
        let mut s = Self { blocks: VecDeque::new(), tree: Fenwick::new(4), first: 0, len: 0, total: 0, est };
        s.rebuild_tree();
        s
    }

    /// `n` items of the same height `h`: a handful of blocks, however big `n`.
    pub fn uniform(n: usize, h: Fx, est: Estimator) -> Result<Self, IndexError> {
        Self::from_chunks(n, est, |size, _| Block::uniform(size as u32, h))
    }

    /// `n` items, nothing measured; `bytes_of(range)` is the text bytes in
    /// an item range (O(1) for a source with an offsets table).
    pub fn estimated(n: usize, est: Estimator, mut bytes_of: impl FnMut(std::ops::Range<usize>) -> u64) -> Result<Self, IndexError> {
        Self::from_chunks(n, est, |size, start| Block::estimated(size as u32, bytes_of(start..start + size)))
    }

    fn from_chunks(n: usize, est: Estimator, mut make: impl FnMut(usize, usize) -> Block) -> Result<Self, IndexError> {
        if n > MAX_ITEMS {
            return Err(IndexError::TooManyItems);
        }
        let mut s = Self::new(est);
        s.blocks.reserve(n.div_ceil(TARGET_BLOCK));
        let (mut start, k) = (0, n.div_ceil(TARGET_BLOCK));
        for i in 0..k {
            let size = n / k + usize::from(i < n % k);
            s.blocks.push_back(make(size, start));
            start += size;
        }
        s.rebuild_tree();
        Ok(s)
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Total height, fixed-point units.
    #[must_use]
    pub const fn total(&self) -> u64 {
        self.total
    }

    #[must_use]
    pub const fn estimator(&self) -> &Estimator {
        &self.est
    }

    #[must_use]
    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    /// Heap bytes the index holds.
    #[must_use]
    pub fn heap_bytes(&self) -> usize {
        self.tree.heap_bytes() + self.blocks.capacity() * size_of::<Block>() + self.blocks.iter().map(Block::heap_bytes).sum::<usize>()
    }

    /// Items whose heights are stored one by one.
    #[must_use]
    pub fn measured_items(&self) -> usize {
        self.blocks.iter().filter(|b| b.is_measured()).map(Block::n).sum()
    }

    // -- the two levels -------------------------------------------------------------------------------------

    /// Re-sum everything from the blocks (a structural change, a new estimator).
    fn rebuild_tree(&mut self) {
        let nb = self.blocks.len();
        // at least a spare slot each side, and about a quarter of the blocks spare each side
        let cap = (nb + nb / 2).max(nb + 2).next_power_of_two();
        self.first = (cap - nb) / 2;
        let est = self.est;
        self.tree = Fenwick::from_values(cap, self.first, self.blocks.iter().map(|b| node(b, &est)));
        self.len = self.blocks.iter().map(Block::n).sum();
        self.total = self.blocks.iter().map(|b| b.height(&est)).sum();
    }

    /// Make a spare slot on one side (the tree doubles when the side is full).
    fn ensure_room(&mut self, back: bool) {
        let full = if back { self.first + self.blocks.len() >= self.tree.capacity() } else { self.first == 0 };
        if full {
            self.rebuild_tree();
        }
    }

    /// Change block `bi` with `f`, and carry the change of its height and
    /// count up the tree.
    fn update_block(&mut self, bi: usize, f: impl FnOnce(&mut Block, &Estimator)) {
        let est = &self.est;
        let b = &mut self.blocks[bi];
        let (oh, on) = (b.height(est), b.n());
        f(b, est);
        let (nh, nn) = (b.height(est), b.n());
        let (dh, dn) = (nh.wrapping_sub(oh) as i64, nn as i64 - on as i64);
        self.tree.add(self.first + bi, dh, dn);
        self.total = self.total.wrapping_add_signed(dh);
        self.len = (self.len as i64 + dn) as usize;
    }

    /// The block holding `item` (`item < len`), the height before it and
    /// the items before it.
    fn find_item(&self, item: usize) -> (usize, u64, usize) {
        if self.blocks.len() == 1 {
            return (0, 0, 0);
        }
        let f = self.tree.find_by_count(item as u64);
        let bi = f.slot.saturating_sub(self.first).min(self.blocks.len() - 1);
        (bi, f.h_before, f.n_before as usize)
    }

    // -- queries --------------------------------------------------------------------------------------------

    /// The item at offset `off` (saturating at the last item); `None` when empty.
    #[must_use]
    pub fn locate(&self, off: u64) -> Option<Located> {
        if self.len == 0 {
            return None;
        }
        let (bi, hb, nb) = if self.blocks.len() == 1 {
            (0, 0, 0)
        } else {
            let f = self.tree.find_by_height(off);
            let bi = f.slot.saturating_sub(self.first).min(self.blocks.len() - 1);
            // past the end lands on the last slot: its before-sums are the totals
            if f.slot >= self.first + self.blocks.len() {
                let last = self.blocks.len() - 1;
                let b = &self.blocks[last];
                let before = self.total - b.height(&self.est);
                let (l, within) = b.locate(off - before, &self.est);
                return Some(Located { item: self.len - b.n() + l, within });
            }
            (bi, f.h_before, f.n_before as usize)
        };
        let (l, within) = self.blocks[bi].locate(off - hb, &self.est);
        Some(Located { item: nb + l, within })
    }

    /// Offset of `item`'s top (`len` gives the total).
    #[must_use]
    pub fn offset_of(&self, item: usize) -> u64 {
        if item >= self.len {
            return self.total;
        }
        let (bi, hb, nb) = self.find_item(item);
        hb + self.blocks[bi].item_range(item - nb, &self.est).0
    }

    /// `item`'s height and whether it was measured.
    #[must_use]
    pub fn height_of(&self, item: usize) -> Option<(Fx, bool)> {
        if item >= self.len {
            return None;
        }
        let (bi, _, nb) = self.find_item(item);
        let b = &self.blocks[bi];
        Some((b.item_height(item - nb, &self.est), b.is_exact(item - nb)))
    }

    /// The item range of the block holding `item`, and whether it is an
    /// estimate (the view materializes the blocks it looks at).
    #[must_use]
    pub fn block_span(&self, item: usize) -> Option<(std::ops::Range<usize>, bool)> {
        if item >= self.len {
            return None;
        }
        let (bi, _, nb) = self.find_item(item);
        let b = &self.blocks[bi];
        Some((nb..nb + b.n(), b.is_estimated()))
    }

    /// Call `f(item, height, exact)` for `from`, `from + 1` ... until it
    /// returns false or the end. One tree lookup, then block by block.
    pub fn walk_forward(&self, from: usize, mut f: impl FnMut(usize, Fx, bool) -> bool) {
        if from >= self.len {
            return;
        }
        let (mut bi, _, mut nb) = self.find_item(from);
        let mut local = from - nb;
        while bi < self.blocks.len() {
            let b = &self.blocks[bi];
            if !b.walk(local, &self.est, |l, h, e| f(nb + l, h, e)) {
                return;
            }
            nb += b.n();
            bi += 1;
            local = 0;
        }
    }

    /// Like [`Self::walk_forward`], towards item 0, starting at `from` (inclusive).
    pub fn walk_backward(&self, from: usize, mut f: impl FnMut(usize, Fx, bool) -> bool) {
        if self.len == 0 {
            return;
        }
        let from = from.min(self.len - 1);
        let (mut bi, _, nb) = self.find_item(from);
        let mut start = nb;
        let mut local = from - nb;
        loop {
            let b = &self.blocks[bi];
            for l in (0..=local).rev() {
                if !f(start + l, b.item_height(l, &self.est), b.is_exact(l)) {
                    return;
                }
            }
            if bi == 0 {
                return;
            }
            bi -= 1;
            start -= self.blocks[bi].n();
            local = self.blocks[bi].n() - 1;
        }
    }

    // -- height updates -------------------------------------------------------------------------------------

    /// Set `item`'s height (`exact`: it was measured). Returns whether `item` exists.
    pub fn set_height(&mut self, item: usize, h: Fx, exact: bool) -> bool {
        if item >= self.len {
            return false;
        }
        let (bi, _, nb) = self.find_item(item);
        self.update_block(bi, |b, est| b.set(item - nb, h, exact, est));
        true
    }

    /// Set many measured heights at once (any order, duplicates: the last
    /// wins; items past the end are ignored). Each touched block is updated
    /// in one pass, and the tree is rebuilt instead of patched when that is
    /// cheaper than one point update per block.
    pub fn set_heights(&mut self, updates: &mut [(usize, Fx)]) {
        updates.sort_by_key(|u| u.0);
        let end = updates.partition_point(|u| u.0 < self.len);
        let updates = &mut updates[..end];
        let mut i = 0;
        let mut touched = 0usize;
        let mut patched = Vec::new();
        while i < updates.len() {
            let (bi, _, nb) = self.find_item(updates[i].0);
            let stop = nb + self.blocks[bi].n();
            let j = i + updates[i..].partition_point(|u| u.0 < stop);
            for u in &mut updates[i..j] {
                u.0 -= nb;
            }
            let est = self.est;
            let slice = &updates[i..j];
            let b = &mut self.blocks[bi];
            let oh = b.height(&est);
            b.set_many(slice, &est);
            patched.push((bi, b.height(&est).wrapping_sub(oh) as i64));
            touched += 1;
            i = j;
        }
        let log = self.tree.capacity().trailing_zeros() as usize + 1;
        if touched * log > self.tree.capacity() {
            self.rebuild_tree();
        } else {
            for (bi, dh) in patched {
                self.tree.add(self.first + bi, dh, 0);
                self.total = self.total.wrapping_add_signed(dh);
            }
        }
    }

    /// Item `item` held `old` bytes of text and now holds `new`: only the
    /// aggregate of an estimated block changes.
    pub fn change_bytes(&mut self, item: usize, old: u64, new: u64) {
        if item >= self.len {
            return;
        }
        let (bi, _, _) = self.find_item(item);
        self.update_block(bi, |b, _| b.change_bytes(old, new));
    }

    /// Learn from a measurement of an item with `bytes` of text: when the
    /// estimate's calibration moved, every estimated block is re-summed.
    pub fn observe(&mut self, bytes: u64, real_px: f64) {
        if self.est.observe(bytes, real_px) {
            self.rebuild_tree();
        }
    }

    /// A new estimator (a new wrap width): measured blocks forget their
    /// per-item heights (they are for another width) and become estimates
    /// again, so memory is also released. Uniform blocks stay.
    pub fn set_estimator(&mut self, est: Estimator) {
        self.est = est;
        for b in &mut self.blocks {
            b.demote();
        }
        self.rebuild_tree();
    }

    /// Keep at most `max_items` items measured: the measured blocks farthest
    /// from `center` become estimates again. Memory is bounded by what was
    /// looked at *recently*, not by everything ever scrolled through.
    pub fn trim(&mut self, center: usize, max_items: usize) {
        let mut measured = self.measured_items();
        if measured <= max_items {
            return;
        }
        let mut far = Vec::new();
        let mut start = 0;
        for (bi, b) in self.blocks.iter().enumerate() {
            if b.is_measured() {
                let mid = start + b.n() / 2;
                far.push((mid.abs_diff(center), bi));
            }
            start += b.n();
        }
        far.sort_unstable_by(|a, b| b.cmp(a));
        for (_, bi) in far {
            if measured <= max_items {
                break;
            }
            measured -= self.blocks[bi].n();
            self.blocks[bi].demote();
        }
        self.rebuild_tree();
    }

    // -- structure ------------------------------------------------------------------------------------------

    /// Insert `count` items before item `at` (`at == len` appends, `at == 0`
    /// prepends) holding `bytes` of text. They are inexact. At either end
    /// this adds blocks without moving anything; a hostile count is refused.
    pub fn insert_items(&mut self, at: usize, count: usize, bytes: u64) -> Result<(), IndexError> {
        if count == 0 {
            return Ok(());
        }
        if at > self.len {
            return Err(IndexError::OutOfRange);
        }
        if self.len.checked_add(count).is_none_or(|t| t > MAX_ITEMS) {
            return Err(IndexError::TooManyItems);
        }
        if self.blocks.is_empty() {
            self.insert_chunks(0, count, bytes);
            return Ok(());
        }
        let (bi, local) = if at == self.len {
            (self.blocks.len() - 1, self.blocks[self.blocks.len() - 1].n())
        } else {
            let (bi, _, nb) = self.find_item(at);
            (bi, at - nb)
        };
        let n = self.blocks[bi].n();
        if count <= MAX_BLOCK - n {
            self.update_block(bi, |b, est| b.insert(local, count, bytes, est));
        } else if local == 0 {
            self.insert_chunks(bi, count, bytes);
        } else if local == n {
            self.insert_chunks(bi + 1, count, bytes);
        } else {
            let right = self.blocks[bi].split_off(local);
            self.blocks.insert(bi + 1, right);
            self.insert_chunks(bi + 1, count, bytes);
        }
        Ok(())
    }

    /// New estimated blocks before block position `pos`.
    fn insert_chunks(&mut self, pos: usize, count: usize, bytes: u64) {
        let k = count.div_ceil(TARGET_BLOCK);
        let (mut left, mut bytes_left) = (count, bytes);
        let mut chunks = Vec::with_capacity(k);
        for i in 0..k {
            let size = if i + 1 == k { left } else { count / k + usize::from(i < count % k) };
            let b = if i + 1 == k { bytes_left } else { (u128::from(bytes) * size as u128 / count as u128) as u64 };
            chunks.push(Block::estimated(size as u32, b));
            left -= size;
            bytes_left -= b;
        }
        let est = self.est;
        if pos == self.blocks.len() {
            for b in chunks {
                self.ensure_room(true);
                let nd = node(&b, &est);
                self.tree.add(self.first + self.blocks.len(), nd.h as i64, nd.n as i64);
                self.total += nd.h;
                self.len += nd.n as usize;
                self.blocks.push_back(b);
            }
        } else if pos == 0 {
            for b in chunks.into_iter().rev() {
                self.ensure_room(false);
                self.first -= 1;
                let nd = node(&b, &est);
                self.tree.add(self.first, nd.h as i64, nd.n as i64);
                self.total += nd.h;
                self.len += nd.n as usize;
                self.blocks.push_front(b);
            }
        } else {
            for (i, b) in chunks.into_iter().enumerate() {
                self.blocks.insert(pos + i, b);
            }
            self.rebuild_tree();
        }
    }

    /// Remove `count` items at `at` (clamped to the end) that held `bytes`
    /// of text. Returns how many went. Emptied blocks are dropped and small
    /// neighbours merged.
    pub fn remove_items(&mut self, at: usize, count: usize, bytes: u64) -> usize {
        if at >= self.len {
            return 0;
        }
        let count = count.min(self.len - at);
        let (mut remaining, mut bytes_left) = (count, bytes);
        while remaining > 0 {
            let (bi, _, nb) = self.find_item(at);
            let local = at - nb;
            let take = remaining.min(self.blocks[bi].n() - local);
            let part = if take == remaining { bytes_left } else { (u128::from(bytes_left) * take as u128 / remaining as u128) as u64 };
            self.update_block(bi, |b, _| b.remove(local, take, part));
            remaining -= take;
            bytes_left -= part;
            if self.blocks[bi].n() == 0 {
                // an empty block is skipped by the tree; drop it now so the next lookup is clean
                self.blocks.remove(bi);
                self.rebuild_tree();
            }
        }
        self.merge_small();
        count
    }

    /// Merge neighbouring blocks that are small and of the same kind.
    fn merge_small(&mut self) {
        let small = TARGET_BLOCK / 4;
        if !self.blocks.iter().any(|b| b.n() < small) || self.blocks.len() < 2 {
            return;
        }
        let mut out: VecDeque<Block> = VecDeque::with_capacity(self.blocks.len());
        for b in self.blocks.drain(..) {
            let merged = out.back_mut().is_some_and(|last| (last.n() < small || b.n() < small) && last.try_merge(&b));
            if !merged {
                out.push_back(b);
            }
        }
        self.blocks = out;
        self.rebuild_tree();
    }

    /// Check the invariants (tests): the tree equals the blocks.
    #[doc(hidden)]
    pub fn check(&self) {
        let est = &self.est;
        let mut acc = Node::default();
        for (i, b) in self.blocks.iter().enumerate() {
            let p = self.tree.prefix(self.first + i);
            assert_eq!((p.h, p.n), (acc.h, acc.n), "tree prefix at block {i}");
            acc.h += b.height(est);
            acc.n += b.n() as u64;
            assert!(b.n() > 0 && b.n() <= MAX_BLOCK, "block {i} has {} items", b.n());
        }
        assert_eq!(acc.h, self.total);
        assert_eq!(acc.n as usize, self.len);
        let all = self.tree.prefix(self.tree.capacity());
        assert_eq!((all.h, all.n), (acc.h, acc.n), "tree total");
    }
}

#[cfg(test)]
mod tests;
