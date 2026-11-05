//! The data side of a table for gpui: [`TableDelegate`], with the method
//! names of gpui-component's trait and Blender's data-source shape.
//!
//! The cells themselves come from the [`CellSource`] supertrait (shared by
//! every framework adapter): typed, lazy, called for visible cells only.
//! Where gpui-component's `render_td` builds an element for every visible
//! cell, here a cell is *painted* from its [`Cell`](blenderui_table::Cell)
//! value; [`render_td`](TableDelegate::render_td) is the escape hatch for the
//! few cells that need rich content.

use std::ops::Range;

use blenderui_table::{CellSource, Column, ColumnGroup, ColumnSort, VecTable};
use gpui::{AnyElement, App, Context, IntoElement as _, Pixels, Window, div};

use crate::state::TableState;

/// Provides a table's columns and the hooks around it.
#[allow(unused_variables)]
pub trait TableDelegate: CellSource + Sized + 'static {
    /// Number of columns.
    fn columns_count(&self, cx: &App) -> usize;

    /// Number of rows.
    fn rows_count(&self, cx: &App) -> usize;

    /// The column at `col_ix`. Called when the table prepares or refreshes.
    fn column(&self, col_ix: usize, cx: &App) -> Column;

    /// Whether a header click sorts the table's row index list (typed keys,
    /// parallel, no data moved: see [`CellSource::sort_kind`]) rather than
    /// leave sorting to [`perform_sort`](Self::perform_sort).
    fn sort_by_mask(&self) -> bool {
        true
    }

    /// The column was sorted. With [`sort_by_mask`](Self::sort_by_mask) the
    /// table has already reordered its row index list and this is a
    /// notification; otherwise sort your own data here.
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
    #[allow(clippy::unnecessary_literal_bound)]
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
    fn visible_rows_changed(&mut self, visible_range: Range<usize>, window: &mut Window, cx: &mut Context<TableState<Self>>) {}

    /// The visible columns (view indices) changed. Keep it fast.
    fn visible_columns_changed(&mut self, visible_range: Range<usize>, window: &mut Window, cx: &mut Context<TableState<Self>>) {}

    // -- the escape hatch: rich cells, off the fast path ---------------------------------------------

    /// Whether any cell is a custom element. `false` (the default) skips
    /// the per-cell question below altogether.
    fn has_custom_cells(&self) -> bool {
        false
    }

    /// Whether the cell at (`row_ix`, `col_ix`) is built by
    /// [`render_td`](Self::render_td) instead of painted from its value
    /// (return [`Cell::Empty`](blenderui_table::Cell::Empty) from `cell` for it).
    /// Asked only for visible cells.
    fn is_custom_cell(&self, row_ix: usize, col_ix: usize) -> bool {
        false
    }

    /// The element of a custom cell (gpui-component's `render_td`), laid out
    /// in the cell's box. Called every frame for each visible custom cell.
    fn render_td(&mut self, row_ix: usize, col_ix: usize, window: &mut Window, cx: &mut Context<TableState<Self>>) -> AnyElement {
        div().into_any_element()
    }
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
