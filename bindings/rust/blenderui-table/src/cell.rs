//! Cell values: Blender's lazily evaluated, typed column values
//! (`blender::bke::GVArray` read through `spreadsheet_data_source.hh`) for a
//! table. A cell is fetched only when drawn or compared, as a borrowed
//! `&str` or a number; numbers are formatted into a stack [`CellBuf`].

use std::cmp::Ordering;

use crate::column::{BadgeVariant, NumberFormat};
use crate::fmt::CellBuf;

/// One cell's value.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Cell<'a> {
    /// Nothing is drawn.
    Empty,
    Str(&'a str),
    I64(i64),
    U64(u64),
    /// A float shown with a fixed number of decimals (at most 9).
    F64(f64, u8),
}

/// The kind of a cell, for typed sort keys: columns whose cells all have the
/// same kind sort without comparing cells.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CellKind {
    Empty,
    Number,
    Text,
}

impl Cell<'_> {
    #[must_use]
    pub const fn kind(&self) -> CellKind {
        match self {
            Cell::Empty => CellKind::Empty,
            Cell::Str(_) => CellKind::Text,
            _ => CellKind::Number,
        }
    }

    /// The text shown: borrowed, or formatted into `buf`.
    #[must_use]
    pub fn text<'b>(&'b self, buf: &'b mut CellBuf) -> &'b str {
        match *self {
            Cell::Empty => "",
            Cell::Str(s) => s,
            Cell::I64(v) => {
                buf.clear();
                buf.push_i64(v);
                buf.as_str()
            }
            Cell::U64(v) => {
                buf.clear();
                buf.push_u64(v);
                buf.as_str()
            }
            Cell::F64(v, d) => {
                buf.clear();
                buf.push_f64(v, d);
                buf.as_str()
            }
        }
    }

    /// The text shown with a number format (a currency sign, grouped
    /// digits): borrowed, or formatted into `buf`. Text is shown as it is.
    #[must_use]
    pub fn text_fmt<'b>(&'b self, buf: &'b mut CellBuf, fmt: &NumberFormat) -> &'b str {
        if fmt.is_plain() || self.kind() != CellKind::Number {
            return self.text(buf);
        }
        let mut digits = CellBuf::new();
        let shown = self.text(&mut digits);
        buf.clear();
        buf.push_formatted(shown, fmt);
        buf.as_str()
    }

    /// Whether the cell counts as "on": a non-zero number, or non-empty text
    /// other than `false` and `0`.
    #[must_use]
    pub fn as_bool(&self) -> bool {
        match *self {
            Cell::Empty => false,
            Cell::Str(s) => !matches!(s, "" | "false" | "0"),
            Cell::I64(v) => v != 0,
            Cell::U64(v) => v != 0,
            Cell::F64(v, _) => v != 0.0,
        }
    }

    /// A numeric cell's value (`None` for text and empty cells).
    #[must_use]
    pub const fn number(&self) -> Option<f64> {
        match *self {
            Cell::I64(v) => Some(v as f64),
            Cell::U64(v) => Some(v as f64),
            Cell::F64(v, _) => Some(v),
            _ => None,
        }
    }

    /// An order-preserving `u64` for a numeric cell (`None` for text and
    /// empty cells): integers keep their order, floats follow `f64::total_cmp`.
    /// Only meaningful between cells of the same variant.
    #[must_use]
    pub const fn number_key(&self) -> Option<u64> {
        match *self {
            Cell::I64(v) => Some((v as u64) ^ (1 << 63)),
            Cell::U64(v) => Some(v),
            Cell::F64(v, _) => {
                let bits = v.to_bits();
                Some(if bits >> 63 == 1 { !bits } else { bits | (1 << 63) })
            }
            _ => None,
        }
    }

    /// The first 8 bytes of a text cell, big endian, zero padded: orders like
    /// the text up to ties.
    #[must_use]
    pub fn text_prefix(&self) -> Option<u64> {
        if let Cell::Str(s) = self {
            let b = s.as_bytes();
            let mut p = [0u8; 8];
            let n = b.len().min(8);
            p[..n].copy_from_slice(&b[..n]);
            Some(u64::from_be_bytes(p))
        } else {
            None
        }
    }

    /// The table's total order over cells: empty cells first, then numbers
    /// (by value; mixed integer and float compare as floats), then text
    /// (byte order).
    #[must_use]
    pub fn sort_cmp(&self, other: &Cell<'_>) -> Ordering {
        match (self, other) {
            (Cell::Str(a), Cell::Str(b)) => a.cmp(b),
            (Cell::I64(a), Cell::I64(b)) => a.cmp(b),
            (Cell::U64(a), Cell::U64(b)) => a.cmp(b),
            (Cell::F64(a, _), Cell::F64(b, _)) => a.total_cmp(b),
            (Cell::I64(a), Cell::U64(b)) => {
                if *a < 0 { Ordering::Less } else { (*a as u64).cmp(b) }
            }
            (Cell::U64(_), Cell::I64(_)) => other.sort_cmp(self).reverse(),
            _ => match (self.kind(), other.kind()) {
                (CellKind::Number, CellKind::Number) => self.as_f64().total_cmp(&other.as_f64()),
                (a, b) => rank(a).cmp(&rank(b)),
            },
        }
    }

    const fn as_f64(&self) -> f64 {
        match *self {
            Cell::I64(v) => v as f64,
            Cell::U64(v) => v as f64,
            Cell::F64(v, _) => v,
            _ => 0.0,
        }
    }
}

const fn rank(k: CellKind) -> u8 {
    match k {
        CellKind::Empty => 0,
        CellKind::Number => 1,
        CellKind::Text => 2,
    }
}

/// A checkbox's state (the header's select-all box has three).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum Check {
    #[default]
    Unchecked,
    Indeterminate,
    Checked,
}

/// How a column is sorted.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum SortKind {
    /// By the cells' own order ([`Cell::sort_cmp`]); the table extracts typed
    /// keys when the column's cells are all numbers of one variant or all text.
    #[default]
    Cells,
    /// By [`CellSource::compare`], called per comparison (slower).
    Custom,
}

/// Where a table's cells come from: Blender's `DataSource`. Implementations
/// compute or borrow a cell on demand; nothing is stored per cell.
///
/// `Sync` because sorting reads cells from several threads.
pub trait CellSource: Sync {
    /// The cell at (`row`, `col`), both source indices (rows are data rows,
    /// whatever the display order).
    fn cell(&self, row: usize, col: usize) -> Cell<'_>;

    /// How column `col` sorts; override with [`SortKind::Custom`] and
    /// [`compare`](Self::compare) for an order the cells do not express.
    fn sort_kind(&self, col: usize) -> SortKind {
        let _ = col;
        SortKind::Cells
    }

    /// The variant of the [`CellRender::Badge`](crate::CellRender::Badge) cell at (`row`, `col`).
    fn badge(&self, row: usize, col: usize) -> BadgeVariant {
        let _ = (row, col);
        BadgeVariant::default()
    }

    /// The state of the select-all checkbox in the header of column `col`
    /// ([`CellRender::Checkbox`](crate::CellRender::Checkbox)).
    fn header_check(&self, col: usize) -> Check {
        let _ = col;
        Check::Unchecked
    }

    /// Order rows `a` and `b` by column `col` (used for [`SortKind::Custom`]).
    fn compare(&self, col: usize, a: usize, b: usize) -> Ordering {
        self.cell(a, col).sort_cmp(&self.cell(b, col))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_formats_into_the_buffer() {
        let mut b = CellBuf::new();
        assert_eq!(Cell::Empty.text(&mut b), "");
        assert_eq!(Cell::Str("hi").text(&mut b), "hi");
        assert_eq!(Cell::I64(-42).text(&mut b), "-42");
        assert_eq!(Cell::U64(7).text(&mut b), "7");
        assert_eq!(Cell::F64(12.5, 1).text(&mut b), "12.5");
    }

    #[test]
    fn keys_preserve_order() {
        let ints = [i64::MIN, -5, -1, 0, 1, 7, i64::MAX];
        for w in ints.windows(2) {
            assert!(Cell::I64(w[0]).number_key() < Cell::I64(w[1]).number_key());
        }
        let floats = [f64::NEG_INFINITY, -1e9, -1.5, -0.0, 0.0, 1e-300, 2.5, f64::INFINITY, f64::NAN];
        for w in floats.windows(2) {
            let (a, b) = (Cell::F64(w[0], 2), Cell::F64(w[1], 2));
            assert!(a.number_key() < b.number_key(), "{w:?}");
            assert_eq!(a.sort_cmp(&b), Ordering::Less);
        }
        assert!(Cell::Str("abcdefgh").text_prefix() < Cell::Str("abcdefgi").text_prefix());
        assert_eq!(Cell::Str("abcdefghX").text_prefix(), Cell::Str("abcdefghY").text_prefix());
    }

    #[test]
    fn total_order_across_kinds() {
        use Cell::{Empty, F64, I64, Str, U64};
        let order = [Empty, I64(-1), F64(0.5, 1), U64(1), I64(2), Str("a"), Str("b")];
        for i in 0..order.len() {
            for j in 0..order.len() {
                assert_eq!(order[i].sort_cmp(&order[j]), i.cmp(&j), "{:?} {:?}", order[i], order[j]);
            }
        }
        assert_eq!(I64(-3).sort_cmp(&U64(0)), Ordering::Less);
        assert_eq!(U64(u64::MAX).sort_cmp(&I64(i64::MAX)), Ordering::Greater);
    }
}
