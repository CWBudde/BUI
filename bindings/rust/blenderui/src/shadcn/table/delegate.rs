//! A TanStack [`Table`] as the delegate of the data table: the engine paints
//! the table's current page straight from it, so there is one source of
//! truth for the data and nothing is copied into the table.

use blenderui_table::tanstack::Table;
use blenderui_table::{Column, ColumnSort};

use crate::table::{TableDelegate, TableState};
use crate::{App, Context, Window};

impl<T: Send + Sync + 'static> TableDelegate for Table<T> {
    fn columns_count(&self, _: &App) -> usize {
        self.column_defs().len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.row_model().len()
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        self.core_column(col_ix)
    }

    /// The table sorts its own row models (typed keys, stable, multi-column),
    /// so the engine's index list stays the identity.
    fn sort_by_mask(&self) -> bool {
        false
    }

    /// A header click: `setSorting` on the table (which may be controlled).
    fn perform_sort(&mut self, col_ix: usize, sort: ColumnSort, _: &mut Window, _: &mut Context<TableState<Self>>) {
        let Some(id) = self.column_defs().get(col_ix).map(|c| c.id.clone()) else { return };
        self.set_column_sort(&id, sort);
    }

    fn empty_text(&self) -> &str {
        "No results."
    }
}
