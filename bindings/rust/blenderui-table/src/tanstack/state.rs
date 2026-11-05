//! TanStack's table state: the six pieces shadcn's Data Table uses.

use std::collections::BTreeMap;

use crate::column::Text;

/// One sorted column (`{ id, desc }`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SortingRule {
    pub id: Text,
    pub desc: bool,
}

impl SortingRule {
    pub fn new(id: impl Into<Text>, desc: bool) -> Self {
        Self { id: id.into(), desc }
    }
}

/// `SortingState`: the sorted columns, the first one most significant.
pub type SortingState = Vec<SortingRule>;

/// One column filter (`{ id, value }`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColumnFilter {
    pub id: Text,
    pub value: String,
}

impl ColumnFilter {
    pub fn new(id: impl Into<Text>, value: impl Into<String>) -> Self {
        Self { id: id.into(), value: value.into() }
    }
}

/// `ColumnFiltersState`.
pub type ColumnFiltersState = Vec<ColumnFilter>;

/// `VisibilityState`: `false` hides a column; a column that is not listed
/// is visible.
pub type VisibilityState = BTreeMap<Text, bool>;

/// `PaginationState`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PaginationState {
    pub page_index: usize,
    pub page_size: usize,
}

impl Default for PaginationState {
    /// TanStack's default: page 0, ten rows.
    fn default() -> Self {
        Self { page_index: 0, page_size: 10 }
    }
}

/// `RowSelectionState`: the selected rows, by row id (the index of the row
/// in the data), as a bitset: selecting a million rows is 125 KB.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RowSelectionState {
    words: Vec<u64>,
    count: usize,
}

impl RowSelectionState {
    /// How many rows are selected.
    #[must_use]
    pub const fn count(&self) -> usize {
        self.count
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    #[must_use]
    pub fn is_selected(&self, id: usize) -> bool {
        self.words.get(id / 64).is_some_and(|w| w >> (id % 64) & 1 == 1)
    }

    /// Select or deselect row `id`. Returns whether it changed.
    pub fn set(&mut self, id: usize, on: bool) -> bool {
        if self.is_selected(id) == on {
            return false;
        }
        if on {
            if self.words.len() <= id / 64 {
                self.words.resize(id / 64 + 1, 0);
            }
            self.words[id / 64] |= 1 << (id % 64);
            self.count += 1;
        } else {
            self.words[id / 64] &= !(1 << (id % 64));
            self.count -= 1;
            while self.words.last() == Some(&0) {
                self.words.pop();
            }
        }
        true
    }

    /// Deselect everything.
    pub fn clear(&mut self) {
        self.words.clear();
        self.count = 0;
    }

    /// Select rows `0..n`.
    pub fn select_all(&mut self, n: usize) {
        self.clear();
        for id in 0..n {
            self.set(id, true);
        }
    }

    /// Drop selected rows at or after `n` (the data got shorter).
    pub fn clamp(&mut self, n: usize) {
        let ids: Vec<usize> = self.iter().filter(|&i| i >= n).collect();
        for id in ids {
            self.set(id, false);
        }
    }

    /// The selected row ids, ascending.
    pub fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.words.iter().enumerate().flat_map(|(w, &bits)| {
            let mut rest = bits;
            std::iter::from_fn(move || {
                (rest != 0).then(|| {
                    let b = rest.trailing_zeros() as usize;
                    rest &= rest - 1;
                    w * 64 + b
                })
            })
        })
    }
}

/// `TableState`: everything a user can change.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TableState {
    pub sorting: SortingState,
    pub column_filters: ColumnFiltersState,
    pub global_filter: String,
    pub column_visibility: VisibilityState,
    pub row_selection: RowSelectionState,
    pub pagination: PaginationState,
}

/// The state a parent controls (TanStack's `state` option): a field that is
/// `Some` is owned by the parent; the table only reports changes to it
/// ([`Change`]) and shows what the parent passes back.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PartialState {
    pub sorting: Option<SortingState>,
    pub column_filters: Option<ColumnFiltersState>,
    pub global_filter: Option<String>,
    pub column_visibility: Option<VisibilityState>,
    pub row_selection: Option<RowSelectionState>,
    pub pagination: Option<PaginationState>,
}

/// A state change the user made (the argument of `onSortingChange` and
/// friends). Raised whether or not the field is controlled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    Sorting(SortingState),
    ColumnFilters(ColumnFiltersState),
    GlobalFilter(String),
    ColumnVisibility(VisibilityState),
    RowSelection(RowSelectionState),
    Pagination(PaginationState),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_bits() {
        let mut s = RowSelectionState::default();
        assert!(s.set(3, true) && s.set(130, true) && !s.set(3, true));
        assert_eq!(s.count(), 2);
        assert_eq!(s.iter().collect::<Vec<_>>(), vec![3, 130]);
        assert!(s.set(130, false));
        assert_eq!(s, {
            let mut t = RowSelectionState::default();
            t.set(3, true);
            t
        });
        s.select_all(70);
        assert_eq!(s.count(), 70);
        s.clamp(10);
        assert_eq!(s.count(), 10);
        s.clear();
        assert!(s.is_empty() && !s.is_selected(0));
    }
}
