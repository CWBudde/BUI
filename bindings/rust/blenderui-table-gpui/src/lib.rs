//! A gpui-component-style `DataTable` for gpui that paints only what is
//! visible, on the framework-independent `blenderui-table` core (Blender's
//! spreadsheet virtualization, index-mask sorting, typed lazy cells, batched
//! quads and a shaped-line cache).
//!
//! * [`TableDelegate`]: your data and hooks (gpui-component's method names).
//!   Cells come from `CellSource::cell` as typed values and are painted, not
//!   built as elements; [`TableDelegate::render_td`] is the escape hatch for
//!   rich cells.
//! * [`TableState`]: the entity: scroll, columns, sort, selection.
//! * [`DataTable`]: the element: a custom gpui `Element`, not a tree of cells.
//!
//! ```ignore
//! use blenderui_table_gpui::*;
//!
//! struct People;
//! impl CellSource for People {
//!     fn cell(&self, row: usize, col: usize) -> Cell<'_> {
//!         if col == 0 { Cell::U64(row as u64) } else { Cell::U64(row as u64 * 7) }
//!     }
//! }
//! impl TableDelegate for People {
//!     fn columns_count(&self, _: &App) -> usize { 2 }
//!     fn rows_count(&self, _: &App) -> usize { 1_000_000 }
//!     fn column(&self, ix: usize, _: &App) -> Column {
//!         if ix == 0 { Column::new("id", "ID").sortable().width(px(80.0)) } else { Column::new("v", "Value").sortable() }
//!     }
//! }
//!
//! let table = cx.new(|cx| TableState::new(People, window, cx));
//! // in render
//! div().size_full().child(DataTable::new(&table).stripe(true))
//! ```

mod data_table;
mod delegate;
mod state;

pub use blenderui_table::{
    Cell, CellBuf, CellSource, Column, ColumnFixed, ColumnGroup, ColumnLayout, ColumnSort, Palette, RowMask, RowSelection,
    Size, SortKind, TableEvent, TableModel, TableOptions, TableSelection, TableVisibleRange, TextAlign, VecTable,
};
pub use data_table::{DataTable, TablePalette};
pub use delegate::TableDelegate;
pub use state::{GpuiHost, TableState};
