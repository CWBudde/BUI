//! [`VecTable`]: a table's data as rows of strings, the convenience for
//! small tables and examples. Adapters implement their delegate trait for it.

use std::cmp::Ordering;

use crate::cell::{Cell, CellSource, SortKind};
use crate::column::Column;

/// Columns and rows of strings. Sortable columns sort by text, as numbers
/// when both cells parse as numbers.
#[derive(Clone, Debug, Default)]
pub struct VecTable {
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<String>>,
}

impl VecTable {
    #[must_use]
    pub const fn new(columns: Vec<Column>, rows: Vec<Vec<String>>) -> Self {
        Self { columns, rows }
    }
}

impl CellSource for VecTable {
    fn cell(&self, row: usize, col: usize) -> Cell<'_> {
        self.rows.get(row).and_then(|r| r.get(col)).map_or(Cell::Empty, |s| Cell::Str(s))
    }

    fn sort_kind(&self, _: usize) -> SortKind {
        SortKind::Custom
    }

    fn compare(&self, col: usize, a: usize, b: usize) -> Ordering {
        let (x, y) = (self.cell(a, col), self.cell(b, col));
        let (Cell::Str(p), Cell::Str(q)) = (x, y) else { return x.sort_cmp(&y) };
        match (p.trim().parse::<f64>(), q.trim().parse::<f64>()) {
            (Ok(m), Ok(n)) => m.total_cmp(&n),
            _ => p.cmp(q),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mask::RowMask;
    use crate::sort::sorted_by_column;

    #[test]
    fn numbers_sort_numerically_and_text_lexically() {
        let order = |cells: &[&str]| {
            let rows = cells.iter().map(|s| vec![(*s).to_owned()]).collect();
            let t = VecTable::new(vec![Column::new("a", "A")], rows);
            let m = sorted_by_column(&t, 0, &RowMask::Identity, cells.len(), false);
            m.indices().unwrap().iter().map(|&r| t.rows[r as usize][0].clone()).collect::<Vec<_>>()
        };
        assert_eq!(order(&["10", "9", "100", "2", " 3 "]), vec!["2", " 3 ", "9", "10", "100"]);
        assert_eq!(order(&["b", "a", "B", "ab"]), vec!["B", "a", "ab", "b"]);
    }

    #[test]
    fn missing_cells_are_empty() {
        let t = VecTable::new(vec![], vec![vec!["a".into()]]);
        assert_eq!(t.cell(0, 5), Cell::Empty);
        assert_eq!(t.cell(9, 0), Cell::Empty);
    }
}
