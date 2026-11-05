//! The top level of the height index: a Fenwick tree over blocks that
//! keeps, per node, the summed height and the summed item count together
//! (one cache line answers both questions). Slots left of the first block
//! and right of the last one are spare and hold zero, so putting a block at
//! either end is a point update, not a shift; the owner rebuilds into a
//! bigger tree (doubling) only when a side is full.
//!
//! Blender has no such tree: its `OffsetIndices` are static prefix sums
//! (`BLI_offset_indices.hh`, `accumulate_counts_to_offsets`) that must be
//! recomputed whole. This is the dynamic version of the same idea, over
//! blocks instead of items, so it stays tiny: about 10k nodes (160 KB) for
//! 10M items.

#[derive(Copy, Clone, Default, Debug)]
pub struct Node {
    pub h: u64,
    pub n: u64,
}

/// Sums over `cap` slots (`cap` a power of two, 1-based nodes).
pub struct Fenwick {
    nodes: Vec<Node>,
    cap: usize,
}

/// What a descent found: the slot, and the sums of every slot before it.
#[derive(Copy, Clone, Debug)]
pub struct Found {
    pub slot: usize,
    pub h_before: u64,
    pub n_before: u64,
}

impl Fenwick {
    /// A tree of `cap` slots (rounded up to a power of two), all zero.
    #[must_use]
    pub fn new(cap: usize) -> Self {
        let cap = cap.max(1).next_power_of_two();
        Self { nodes: vec![Node::default(); cap + 1], cap }
    }

    /// A tree whose slots `first..first + values.len()` hold `values`, built in O(cap).
    #[must_use]
    pub fn from_values(cap: usize, first: usize, values: impl Iterator<Item = Node>) -> Self {
        let mut t = Self::new(cap);
        for (i, v) in values.enumerate() {
            t.nodes[first + i + 1] = v;
        }
        for i in 1..=t.cap {
            let j = i + (i & i.wrapping_neg());
            if j <= t.cap {
                let c = t.nodes[i];
                t.nodes[j].h += c.h;
                t.nodes[j].n += c.n;
            }
        }
        t
    }

    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.cap
    }

    #[must_use]
    pub const fn heap_bytes(&self) -> usize {
        self.nodes.capacity() * size_of::<Node>()
    }

    /// Add a signed delta to slot `slot`.
    pub fn add(&mut self, slot: usize, dh: i64, dn: i64) {
        let mut i = slot + 1;
        while i <= self.cap {
            let n = &mut self.nodes[i];
            n.h = n.h.wrapping_add_signed(dh);
            n.n = n.n.wrapping_add_signed(dn);
            i += i & i.wrapping_neg();
        }
    }

    /// Sums over slots `0..slot`.
    #[must_use]
    pub fn prefix(&self, slot: usize) -> Node {
        let (mut i, mut s) = (slot.min(self.cap), Node::default());
        while i > 0 {
            s.h += self.nodes[i].h;
            s.n += self.nodes[i].n;
            i &= i - 1;
        }
        s
    }

    /// The slot whose height range contains `target` (the first slot whose
    /// running height exceeds it; `cap` when `target` is past the total).
    #[must_use]
    pub fn find_by_height(&self, target: u64) -> Found {
        self.descend(target, |n| n.h)
    }

    /// The slot holding item number `target`.
    #[must_use]
    pub fn find_by_count(&self, target: u64) -> Found {
        self.descend(target, |n| n.n)
    }

    fn descend(&self, mut target: u64, key: impl Fn(&Node) -> u64) -> Found {
        let (mut pos, mut hb, mut nb) = (0usize, 0u64, 0u64);
        let mut step = self.cap;
        while step > 0 {
            let next = pos + step;
            if next <= self.cap {
                let n = &self.nodes[next];
                let k = key(n);
                if k <= target {
                    target -= k;
                    pos = next;
                    hb += n.h;
                    nb += n.n;
                }
            }
            step >>= 1;
        }
        Found { slot: pos, h_before: hb, n_before: nb }
    }
}
