//! Row selection as sorted, disjoint ranges of display rows: selecting all
//! a million rows is one range, a click is one range of one.

use std::ops::Range;

use crate::input::Mods;

/// Selected display rows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RowSelection {
    ranges: Vec<Range<usize>>,
    /// Where a shift-extension starts from.
    anchor: Option<usize>,
    /// The row keyboard navigation moves (the "active" row).
    active: Option<usize>,
}

impl RowSelection {
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }
    pub fn clear(&mut self) {
        self.ranges.clear();
        self.anchor = None;
        self.active = None;
    }
    /// Number of selected rows.
    pub fn count(&self) -> usize {
        self.ranges.iter().map(ExactSizeIterator::len).sum()
    }
    /// The selected ranges, ascending.
    #[must_use]
    pub fn ranges(&self) -> &[Range<usize>] {
        &self.ranges
    }
    #[must_use]
    pub const fn anchor(&self) -> Option<usize> {
        self.anchor
    }
    /// The row keyboard navigation moves.
    #[must_use]
    pub const fn active(&self) -> Option<usize> {
        self.active
    }
    /// Selected rows, ascending.
    pub fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.ranges.iter().flat_map(Clone::clone)
    }

    #[must_use]
    pub fn contains(&self, row: usize) -> bool {
        let i = self.ranges.partition_point(|r| r.end <= row);
        self.ranges.get(i).is_some_and(|r| r.start <= row)
    }

    /// Only `row`.
    pub fn select_only(&mut self, row: usize) {
        self.ranges.clear();
        self.ranges.push(row..row + 1);
        self.anchor = Some(row);
        self.active = Some(row);
    }

    /// Add `rows`, merging neighbours.
    pub fn add(&mut self, rows: Range<usize>) {
        if rows.is_empty() {
            return;
        }
        let lo = self.ranges.partition_point(|r| r.end < rows.start);
        let hi = self.ranges.partition_point(|r| r.start <= rows.end);
        let merged = self.ranges[lo..hi].iter().fold(rows, |acc, r| acc.start.min(r.start)..acc.end.max(r.end));
        self.ranges.splice(lo..hi, [merged]);
    }

    /// Remove `rows`.
    pub fn remove(&mut self, rows: Range<usize>) {
        if rows.is_empty() {
            return;
        }
        let lo = self.ranges.partition_point(|r| r.end <= rows.start);
        let hi = self.ranges.partition_point(|r| r.start < rows.end);
        let mut keep = Vec::with_capacity(2);
        if let Some(first) = self.ranges.get(lo).filter(|r| r.start < rows.start) {
            keep.push(first.start..rows.start);
        }
        if hi > lo
            && let Some(last) = self.ranges.get(hi - 1).filter(|r| r.end > rows.end) {
                keep.push(rows.end..last.end);
            }
        self.ranges.splice(lo..hi, keep);
    }

    pub fn toggle(&mut self, row: usize) {
        if self.contains(row) {
            self.remove(row..row + 1);
        } else {
            self.add(row..row + 1);
        }
        self.anchor = Some(row);
        self.active = Some(row);
    }

    pub fn select_all(&mut self, n: usize) {
        self.ranges.clear();
        if n > 0 {
            self.ranges.push(0..n);
            self.anchor = Some(0);
            self.active = Some(n - 1);
        }
    }

    /// Drop everything at or past `n` rows (the data shrank).
    pub fn clamp(&mut self, n: usize) {
        self.remove(n..usize::MAX);
        self.anchor = self.anchor.filter(|&a| a < n);
        self.active = self.active.filter(|&a| a < n);
    }

    /// A click on `row` with `mods` (single click semantics of Blender's
    /// list: plain replaces, ctrl toggles, shift extends from the anchor,
    /// ctrl+shift adds the range).
    pub fn click(&mut self, row: usize, mods: Mods) {
        match (mods.shift, mods.control) {
            (false, false) => self.select_only(row),
            (false, true) => self.toggle(row),
            (true, ctrl) => {
                let a = self.anchor.unwrap_or(row);
                if !ctrl {
                    self.ranges.clear();
                }
                self.add(a.min(row)..a.max(row) + 1);
                self.anchor = Some(a);
                self.active = Some(row);
            }
        }
    }

    /// Move the active row to `to` (keyboard): shift extends from the
    /// anchor, otherwise `to` becomes the only selected row.
    pub fn move_to(&mut self, to: usize, extend: bool) {
        if extend {
            self.click(to, Mods { shift: true, ..Mods::default() });
        } else {
            self.select_only(to);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: Mods = Mods { shift: false, control: false, alt: false };
    const SHIFT: Mods = Mods { shift: true, control: false, alt: false };
    const CTRL: Mods = Mods { shift: false, control: true, alt: false };

    /// The naive reference: a bool per row.
    fn naive(sel: &RowSelection, n: usize) -> Vec<bool> {
        (0..n).map(|r| sel.contains(r)).collect()
    }

    #[test]
    fn click_semantics() {
        let mut s = RowSelection::default();
        s.click(5, NONE);
        s.click(8, SHIFT);
        assert_eq!(s.iter().collect::<Vec<_>>(), vec![5, 6, 7, 8]);
        s.click(2, SHIFT); // re-extends from the anchor 5
        assert_eq!(s.iter().collect::<Vec<_>>(), vec![2, 3, 4, 5]);
        s.click(10, CTRL);
        assert_eq!(s.count(), 5);
        s.click(3, CTRL); // toggle off splits the range
        assert_eq!(s.ranges(), &[2..3, 4..6, 10..11]);
        s.click(7, NONE);
        assert_eq!(s.ranges().to_vec(), vec![7..8]);
        s.select_all(1_000_000);
        assert_eq!(s.count(), 1_000_000);
        assert!(s.contains(999_999) && !s.contains(1_000_000));
    }

    #[test]
    fn random_ops_match_reference() {
        let mut seed = 12345u64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let n = 60;
        let mut s = RowSelection::default();
        let mut reference = vec![false; n];
        for _ in 0..4000 {
            let a = (next() % n as u64) as usize;
            let b = a + (next() % 8) as usize;
            let b = b.min(n);
            if next() % 2 == 0 {
                s.add(a..b);
                reference[a..b].iter_mut().for_each(|x| *x = true);
            } else {
                s.remove(a..b);
                reference[a..b].iter_mut().for_each(|x| *x = false);
            }
            assert_eq!(naive(&s, n), reference);
            // sorted, disjoint, non-adjacent
            assert!(s.ranges().windows(2).all(|w| w[0].end < w[1].start));
        }
    }

    #[test]
    fn clamp_and_keyboard() {
        let mut s = RowSelection::default();
        s.select_all(100);
        s.clamp(10);
        assert_eq!(s.count(), 10);
        s.move_to(3, false);
        assert_eq!(s.iter().collect::<Vec<_>>(), vec![3]);
        s.move_to(6, true);
        assert_eq!(s.iter().collect::<Vec<_>>(), vec![3, 4, 5, 6]);
        s.move_to(1, true);
        assert_eq!(s.iter().collect::<Vec<_>>(), vec![1, 2, 3]);
    }
}
