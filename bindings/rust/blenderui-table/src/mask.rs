//! Row masks: Blender's `IndexMask` for tables. Sorting and filtering never
//! copy rows, they only produce the list of row indices to show, in order
//! (`spreadsheet_row_filter.cc` builds an `IndexMask` the same way).
//!
//! A mask is either the identity (every row, no storage) or a permutation /
//! subset in a `Vec<u32>`. The sorts that build one live in [`crate::sort`].

use std::cmp::Ordering;

/// The data rows shown, in display order.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum RowMask {
    /// Rows `0..n` in order: no storage.
    #[default]
    Identity,
    /// Explicit data row indices.
    Indices(Vec<u32>),
}

impl RowMask {
    /// The data row shown at display row `view_row`.
    #[inline]
    #[must_use]
    pub fn get(&self, view_row: usize) -> usize {
        match self {
            Self::Identity => view_row,
            Self::Indices(v) => v[view_row] as usize,
        }
    }

    /// Number of rows shown given `tot` data rows.
    #[inline]
    #[must_use]
    pub const fn len(&self, tot: usize) -> usize {
        match self {
            Self::Identity => tot,
            Self::Indices(v) => v.len(),
        }
    }

    /// Whether no row is shown given `tot` data rows.
    #[must_use]
    pub const fn is_empty(&self, tot: usize) -> bool {
        self.len(tot) == 0
    }

    /// The row indices, `None` for the identity.
    #[must_use]
    pub fn indices(&self) -> Option<&[u32]> {
        match self {
            Self::Identity => None,
            Self::Indices(v) => Some(v),
        }
    }

    /// The display row showing data row `row` (linear for an index list).
    #[must_use]
    pub fn position_of(&self, row: usize, tot: usize) -> Option<usize> {
        match self {
            Self::Identity => (row < tot).then_some(row),
            Self::Indices(v) => v.iter().position(|&r| r as usize == row),
        }
    }

    /// The rows of `self` (of `tot` data rows) that satisfy `keep`, in order
    /// (Blender's `IndexMask::from_predicate`).
    #[must_use]
    pub fn filtered(&self, tot: usize, keep: impl Fn(usize) -> bool) -> Self {
        let n = self.len(tot);
        Self::Indices((0..n).map(|i| self.get(i)).filter(|&r| keep(r)).map(|r| r as u32).collect())
    }

    /// The rows of `self` ordered by `cmp` (ties keep their order). Large
    /// masks sort on every core.
    #[must_use]
    pub fn sorted(&self, tot: usize, cmp: impl Fn(u32, u32) -> Ordering + Sync) -> Self {
        let mut v = self.to_vec(tot);
        crate::sort::sort_stable(&mut v, &|&a, &b| cmp(a, b));
        Self::Indices(v)
    }

    /// The row indices as a vector (`0..tot` for the identity).
    #[must_use]
    pub fn to_vec(&self, tot: usize) -> Vec<u32> {
        match self {
            Self::Identity => (0..tot as u32).collect(),
            Self::Indices(v) => v.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Rng;

    #[test]
    fn sort_equals_naive_reference() {
        for (seed, n) in [(1u64, 0usize), (2, 1), (3, 100), (4, 5000), (5, 70_000), (6, 200_001)] {
            let mut rng = Rng::new(seed);
            let keys: Vec<u32> = (0..n).map(|_| (rng.next() % 1000) as u32).collect(); // many ties
            let cmp = |a: u32, b: u32| keys[a as usize].cmp(&keys[b as usize]);
            let mask = RowMask::Identity.sorted(n, cmp);
            let mut naive: Vec<u32> = (0..n as u32).collect();
            naive.sort_by(|&a, &b| cmp(a, b));
            assert_eq!(mask.indices().unwrap(), &naive[..], "seed {seed} n {n}");
        }
    }

    #[test]
    fn filter_then_sort_equals_naive() {
        let n = 50_000;
        let mut rng = Rng::new(77);
        let keys: Vec<u32> = (0..n).map(|_| (rng.next() % 5000) as u32).collect();
        let m = RowMask::Identity.filtered(n, |r| keys[r].is_multiple_of(3));
        let m = m.sorted(n, |a, b| keys[b as usize].cmp(&keys[a as usize]));
        let mut naive: Vec<u32> = (0..n as u32).filter(|&r| keys[r as usize].is_multiple_of(3)).collect();
        naive.sort_by(|&a, &b| keys[b as usize].cmp(&keys[a as usize]));
        assert_eq!(m.indices().unwrap(), &naive[..]);
        assert_eq!(m.len(n), naive.len());
        assert_eq!(m.get(0), naive[0] as usize);
    }

    #[test]
    fn identity_is_free() {
        let m = RowMask::Identity;
        assert_eq!(m.get(41), 41);
        assert_eq!(m.len(7), 7);
        assert!(m.indices().is_none());
        assert_eq!(m.position_of(3, 7), Some(3));
        assert_eq!(m.position_of(7, 7), None);
        assert!(RowMask::Indices(vec![]).is_empty(9));
    }

    #[test]
    fn position_of_finds_the_display_row() {
        let m = RowMask::Indices(vec![5, 2, 9]);
        assert_eq!(m.position_of(9, 10), Some(2));
        assert_eq!(m.position_of(3, 10), None);
    }
}
