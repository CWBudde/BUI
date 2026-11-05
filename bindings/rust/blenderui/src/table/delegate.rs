//! The data side of a table: [`TableDelegate`] (gpui-component's trait,
//! with Blender's data-source shape) and the core's [`VecTable`] as a delegate over
//! `Vec<Vec<String>>`.
//!
//! Blender's `DataSource` (`spreadsheet_data_source.hh`) answers
//! `tot_rows()` and hands out lazily evaluated columns; a cell is fetched
//! only when drawn. [`CellSource::cell`] (the supertrait, shared by every
//! framework adapter) is that contract: it is called for visible cells only,
//! and returns a borrowed `&str` or a number that the table formats into a
//! stack buffer, so drawing allocates nothing.

use blenderui_table::{CellSource, Column, ColumnGroup, ColumnSort, VecTable};

use super::state::TableState;
use crate::{App, Context, Pixels, Window};

/// Provides a table's columns and the hooks around it. Names and meaning
/// follow gpui-component's `TableDelegate`; the cells themselves come from
/// the [`CellSource`] supertrait (text drawn by the table: custom element
/// cells are not supported, which is what makes drawing O(visible cells)
/// with no element tree).
#[allow(unused_variables)]
pub trait TableDelegate: CellSource + Sized + 'static {
    /// Number of columns.
    fn columns_count(&self, cx: &App) -> usize;

    /// Number of rows.
    fn rows_count(&self, cx: &App) -> usize;

    /// The column at `col_ix`. Called when the table prepares or refreshes.
    fn column(&self, col_ix: usize, cx: &App) -> Column;

    /// Whether the table sorts a row index list ([`CellSource::sort_kind`],
    /// typed keys: no data is copied or moved) rather than leave sorting to
    /// [`perform_sort`](Self::perform_sort).
    fn sort_by_mask(&self) -> bool {
        true
    }

    /// The column was sorted (`sort`). With [`sort_by_mask`](Self::sort_by_mask)
    /// the table has already reordered its row index list; this is a
    /// notification. Otherwise sort your own data here.
    fn perform_sort(&mut self, col_ix: usize, sort: ColumnSort, window: &mut Window, cx: &mut Context<TableState<Self>>) {}

    /// The width column `col_ix` needs to show its content whole, for a
    /// double click on its header border. `None` changes nothing.
    fn fit_column(&mut self, col_ix: usize, window: &mut Window, cx: &mut Context<TableState<Self>>) -> Option<Pixels> {
        None
    }

    /// Column `col_ix` was dragged to index `to_ix` (the table keeps its own
    /// order; this is a notification, e.g. to persist it).
    fn move_column(&mut self, col_ix: usize, to_ix: usize, window: &mut Window, cx: &mut Context<TableState<Self>>) {}

    /// Group header rows above the column headers (outermost first).
    fn group_headers(&self, cx: &App) -> Option<Vec<Vec<ColumnGroup>>> {
        None
    }

    /// Show the loading skeleton instead of rows.
    fn loading(&self, cx: &App) -> bool {
        false
    }

    /// The text shown when there are no rows.
    fn empty_text(&self) -> &str {
        "No data"
    }

    /// More rows can be loaded when scrolling reaches the bottom.
    fn has_more(&self, cx: &App) -> bool {
        false
    }

    /// Remaining rows below the view that trigger [`load_more`](Self::load_more).
    fn load_more_threshold(&self) -> usize {
        20
    }

    /// Load more rows (called when near the bottom and `has_more`).
    fn load_more(&mut self, window: &mut Window, cx: &mut Context<TableState<Self>>) {}

    /// The visible display rows changed. Keep it fast.
    fn visible_rows_changed(&mut self, visible_range: std::ops::Range<usize>, window: &mut Window, cx: &mut Context<TableState<Self>>) {}

    /// The visible columns (view indices) changed. Keep it fast.
    fn visible_columns_changed(&mut self, visible_range: std::ops::Range<usize>, window: &mut Window, cx: &mut Context<TableState<Self>>) {}
}

impl TableDelegate for VecTable {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }
    fn rows_count(&self, _: &App) -> usize {
        self.rows.len()
    }
    fn column(&self, col_ix: usize, _: &App) -> Column {
        self.columns[col_ix].clone()
    }
}
