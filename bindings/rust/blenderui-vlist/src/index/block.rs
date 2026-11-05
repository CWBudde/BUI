//! One block of the height index: up to [`MAX_BLOCK`] consecutive items in
//! one of three representations, cheapest first.
//!
//! * `Uniform`: every item has one height. A list of equal rows costs a
//!   count and a height however long it is.
//! * `Estimated`: nothing is known per item, only the item count and the
//!   text bytes they hold; the height is [`Estimator::block`]. A block the
//!   user never scrolled to stays this size forever (about 40 bytes).
//! * `Measured`: per-item prefix sums (`u32`, 4 bytes per item) plus an
//!   exactness bit per item. Offset in block -> item is a binary search,
//!   item -> offset a load; a height change adds a delta to a suffix, a
//!   loop the compiler vectorizes. It is Blender's `OffsetIndices`
//!   (`BLI_offset_indices.hh`) for one block, made updatable.

use crate::estimate::Estimator;
use crate::fx::Fx;

/// The most items a block holds.
pub const MAX_BLOCK: usize = 1024;
/// The size new blocks get.
pub const TARGET_BLOCK: usize = 512;

#[derive(Clone)]
pub struct Block {
    n: u32,
    bytes: u64,
    kind: Kind,
}

#[derive(Clone)]
enum Kind {
    Uniform(Fx),
    Estimated,
    Measured(Measured),
}

/// `prefix[i]` = height of items `..i` (`prefix.len() == n + 1`); bit `i` of
/// `exact` says item `i` was really measured.
#[derive(Clone)]
struct Measured {
    prefix: Vec<u32>,
    exact: Vec<u64>,
}

impl Measured {
    fn from_items(items: impl Iterator<Item = (u32, bool)>, cap: usize) -> Self {
        let mut prefix = Vec::with_capacity(cap + 1);
        let mut exact = Vec::with_capacity(cap.div_ceil(64));
        prefix.push(0);
        let mut acc = 0u32;
        for (i, (h, e)) in items.enumerate() {
            acc += h;
            prefix.push(acc);
            if i % 64 == 0 {
                exact.push(0);
            }
            if e {
                exact[i / 64] |= 1 << (i % 64);
            }
        }
        Self { prefix, exact }
    }

    const fn n(&self) -> usize {
        self.prefix.len() - 1
    }

    fn is_exact(&self, i: usize) -> bool {
        (self.exact[i / 64] >> (i % 64)) & 1 == 1
    }

    fn height(&self, i: usize) -> u32 {
        self.prefix[i + 1] - self.prefix[i]
    }

    fn items(&self) -> impl Iterator<Item = (u32, bool)> + '_ {
        (0..self.n()).map(|i| (self.height(i), self.is_exact(i)))
    }

    fn set_exact(&mut self, i: usize, e: bool) {
        let (w, b) = (i / 64, 1u64 << (i % 64));
        if e {
            self.exact[w] |= b;
        } else {
            self.exact[w] &= !b;
        }
    }

    /// Replace items `at..at + remove` by `count` items from `new`.
    fn splice(&mut self, at: usize, remove: usize, count: usize, new: impl Iterator<Item = (u32, bool)>) {
        let old: Vec<(u32, bool)> = self.items().collect();
        let it = old[..at].iter().copied().chain(new).chain(old[at + remove..].iter().copied());
        *self = Self::from_items(it, old.len() - remove + count);
    }
}

impl Block {
    #[must_use]
    pub const fn uniform(n: u32, h: Fx) -> Self {
        Self { n, bytes: 0, kind: Kind::Uniform(h) }
    }

    #[must_use]
    pub const fn estimated(n: u32, bytes: u64) -> Self {
        Self { n, bytes, kind: Kind::Estimated }
    }

    #[must_use]
    pub const fn n(&self) -> usize {
        self.n as usize
    }

    #[must_use]
    pub const fn bytes(&self) -> u64 {
        self.bytes
    }

    #[must_use]
    pub const fn is_estimated(&self) -> bool {
        matches!(self.kind, Kind::Estimated)
    }

    #[must_use]
    pub const fn is_measured(&self) -> bool {
        matches!(self.kind, Kind::Measured(_))
    }

    /// Total height, fixed-point units.
    #[must_use]
    pub fn height(&self, est: &Estimator) -> u64 {
        match &self.kind {
            Kind::Uniform(h) => u64::from(self.n) * u64::from(h.units()),
            Kind::Estimated => est.block(u64::from(self.n), self.bytes),
            Kind::Measured(m) => u64::from(m.prefix[m.prefix.len() - 1]),
        }
    }

    /// The item height of an estimated block: items `0..n-1` have it, the
    /// last takes the remainder, so the items sum to the block exactly.
    fn avg(&self, est: &Estimator) -> u64 {
        self.height(est) / u64::from(self.n.max(1))
    }

    /// Offsets of item `local`'s top and bottom inside the block.
    #[must_use]
    pub fn item_range(&self, local: usize, est: &Estimator) -> (u64, u64) {
        match &self.kind {
            Kind::Uniform(h) => {
                let h = u64::from(h.units());
                (local as u64 * h, (local as u64 + 1) * h)
            }
            Kind::Measured(m) => (u64::from(m.prefix[local]), u64::from(m.prefix[local + 1])),
            Kind::Estimated => {
                let avg = self.avg(est);
                let end = if local + 1 >= self.n() { self.height(est) } else { (local as u64 + 1) * avg };
                (local as u64 * avg, end)
            }
        }
    }

    #[must_use]
    pub fn item_height(&self, local: usize, est: &Estimator) -> Fx {
        let (a, b) = self.item_range(local, est);
        Fx::from_units(b - a)
    }

    /// Whether item `local`'s height was measured (a uniform block is exact).
    #[must_use]
    pub fn is_exact(&self, local: usize) -> bool {
        match &self.kind {
            Kind::Uniform(_) => true,
            Kind::Estimated => false,
            Kind::Measured(m) => m.is_exact(local),
        }
    }

    /// The item containing offset `rem` in the block, and the offset inside
    /// it (saturating at the last item).
    #[must_use]
    pub fn locate(&self, rem: u64, est: &Estimator) -> (usize, u64) {
        let last = self.n().saturating_sub(1);
        match &self.kind {
            Kind::Uniform(h) => {
                let h = u64::from(h.units()).max(1);
                let l = ((rem / h) as usize).min(last);
                (l, rem - l as u64 * h)
            }
            Kind::Measured(m) => {
                let key = u32::try_from(rem).unwrap_or(u32::MAX);
                let l = m.prefix.partition_point(|&p| p <= key).saturating_sub(1).min(last);
                (l, rem - u64::from(m.prefix[l]))
            }
            Kind::Estimated => {
                let avg = self.avg(est).max(1);
                let l = ((rem / avg) as usize).min(last);
                (l, rem.saturating_sub(l as u64 * avg))
            }
        }
    }

    fn make_measured(&mut self, est: &Estimator) -> &mut Measured {
        if !matches!(self.kind, Kind::Measured(_)) {
            let items: Vec<(u32, bool)> = match self.kind {
                Kind::Uniform(h) => vec![(h.units(), true); self.n()],
                _ => (0..self.n()).map(|l| (self.item_height(l, est).units(), false)).collect(),
            };
            self.kind = Kind::Measured(Measured::from_items(items.into_iter(), self.n()));
        }
        match &mut self.kind {
            Kind::Measured(m) => m,
            _ => unreachable!("just made measured"),
        }
    }

    /// Call `f(local, height, exact)` for items `from..` while it returns true; returns
    /// whether it never said stop. The block's kind is matched once, not per item.
    pub fn walk(&self, from: usize, est: &Estimator, mut f: impl FnMut(usize, Fx, bool) -> bool) -> bool {
        match &self.kind {
            Kind::Uniform(h) => (from..self.n()).all(|l| f(l, *h, true)),
            Kind::Measured(m) => m.prefix[from..].windows(2).enumerate().all(|(k, w)| f(from + k, Fx::from_units(u64::from(w[1] - w[0])), m.is_exact(from + k))),
            Kind::Estimated => (from..self.n()).all(|l| f(l, self.item_height(l, est), false)),
        }
    }

    /// Set item `local`'s height (`exact`: it was measured).
    pub fn set(&mut self, local: usize, h: Fx, exact: bool, est: &Estimator) {
        if let Kind::Uniform(u) = self.kind
            && exact
            && u == h
        {
            return;
        }
        let m = self.make_measured(est);
        let d = h.units().wrapping_sub(m.height(local));
        for p in &mut m.prefix[local + 1..] {
            *p = p.wrapping_add(d);
        }
        m.set_exact(local, exact);
    }

    /// Set many items of this block (`updates` sorted by item, in range,
    /// measured): one pass over the prefix sums however many items change.
    pub fn set_many(&mut self, updates: &[(usize, Fx)], est: &Estimator) {
        let Some(&(first, _)) = updates.first() else { return };
        let m = self.make_measured(est);
        let mut carry = 0u32;
        let mut u = updates.iter().peekable();
        let mut prev = m.prefix[first];
        for i in first..m.n() {
            let next = m.prefix[i + 1];
            let old = next - prev;
            prev = next;
            let mut new = old;
            while let Some(&&(j, h)) = u.peek() {
                if j > i {
                    break;
                }
                u.next();
                if j == i {
                    m.set_exact(i, true);
                    new = h.units();
                }
            }
            carry = carry.wrapping_add(new.wrapping_sub(old));
            m.prefix[i + 1] = next.wrapping_add(carry);
        }
    }

    /// Insert `count` new items before `local`, total text `bytes`; they
    /// are inexact (a uniform block's new items share its height).
    pub fn insert(&mut self, local: usize, count: usize, bytes: u64, est: &Estimator) {
        self.bytes += bytes;
        if let Kind::Measured(m) = &mut self.kind {
            let h = est.item(bytes / count.max(1) as u64).units();
            if local == m.n() {
                let mut acc = m.prefix[m.n()];
                let n = m.n();
                m.exact.resize((n + count).div_ceil(64), 0);
                for _ in 0..count {
                    acc += h;
                    m.prefix.push(acc);
                }
            } else {
                m.splice(local, 0, count, std::iter::repeat_n((h, false), count));
            }
        }
        self.n += count as u32;
    }

    /// Text bytes of one item went from `old` to `new`.
    pub const fn change_bytes(&mut self, old: u64, new: u64) {
        self.bytes = self.bytes.saturating_sub(old).saturating_add(new);
    }

    /// Remove `count` items at `local`, which held `bytes` of text.
    pub fn remove(&mut self, local: usize, count: usize, bytes: u64) {
        self.bytes = self.bytes.saturating_sub(bytes);
        self.n -= count as u32;
        if let Kind::Measured(m) = &mut self.kind {
            m.splice(local, count, 0, std::iter::empty());
        }
    }

    /// Cut the block at `k`, returning the items from `k` on.
    #[must_use]
    pub fn split_off(&mut self, k: usize) -> Self {
        let right_n = self.n() - k;
        let right_bytes = (u128::from(self.bytes) * right_n as u128 / u128::from(self.n.max(1))) as u64;
        let kind = match &mut self.kind {
            Kind::Uniform(h) => Kind::Uniform(*h),
            Kind::Estimated => Kind::Estimated,
            Kind::Measured(m) => {
                let items: Vec<(u32, bool)> = m.items().collect();
                *m = Measured::from_items(items[..k].iter().copied(), k);
                Kind::Measured(Measured::from_items(items[k..].iter().copied(), right_n))
            }
        };
        self.n = k as u32;
        self.bytes -= right_bytes;
        Self { n: right_n as u32, bytes: right_bytes, kind }
    }

    /// Append `other`'s items when both are the same kind and fit in one block.
    pub fn try_merge(&mut self, other: &Self) -> bool {
        if self.n() + other.n() > MAX_BLOCK {
            return false;
        }
        match (&mut self.kind, &other.kind) {
            (Kind::Uniform(a), Kind::Uniform(b)) if a == b => {}
            (Kind::Estimated, Kind::Estimated) => {}
            (Kind::Measured(a), Kind::Measured(b)) => {
                let items: Vec<(u32, bool)> = a.items().chain(b.items()).collect();
                *a = Measured::from_items(items.into_iter(), self.n as usize + other.n());
            }
            _ => return false,
        }
        self.n += other.n;
        self.bytes += other.bytes;
        true
    }

    /// Forget per-item heights (the wrap width changed): back to an
    /// estimate. Returns whether anything changed. Uniform blocks do not
    /// depend on the width.
    pub fn demote(&mut self) -> bool {
        if matches!(self.kind, Kind::Measured(_)) {
            self.kind = Kind::Estimated;
            return true;
        }
        false
    }

    /// Heap bytes held.
    #[must_use]
    pub const fn heap_bytes(&self) -> usize {
        match &self.kind {
            Kind::Measured(m) => m.prefix.capacity() * 4 + m.exact.capacity() * 8,
            _ => 0,
        }
    }
}
