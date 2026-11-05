//! A fast, virtualized data table, drawn the way Blender's spreadsheet
//! editor draws (`source/blender/editors/space_spreadsheet`), with the API
//! of gpui-component's `DataTable`.
//!
//! * [`TableDelegate`]: your data. Cells are fetched only when drawn
//!   (Blender's `DataSource::get_column_values`), as typed [`Cell`]s.
//! * [`TableState`]: scroll, columns, sort, selection; an entity.
//! * [`DataTable`]: the element.
//!
//! Where it lives: everything that is not blenderui is in the
//! `blenderui-table` crate, once: the visible-row math, column culling, row
//! index masks and typed sorting, selection, keyboard navigation, column
//! layout and the painting itself ([`blenderui_table::paint_table`]). This
//! module is the adapter: the [`TableState`] entity around a
//! [`blenderui_table::TableModel`], the [`DataTable`] element that feeds it
//! input, and the paint sink that draws into the engine's display list
//! ([`sink`]). The gpui adapter (`blenderui-table-gpui`) shares all of that
//! core. The C kit's list/tree views (`kit_views.c`) are widget-tree views of
//! a few hundred rows with Blender's UI-list semantics and cannot share it
//! without rebuilding the engine on the C side.
//!
//! ```ignore
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
//!         if ix == 0 { Column::new("id", "ID").sortable().width(px(80.0)) } else { Column::new("name", "Name") }
//!     }
//! }
//! let table = cx.new(|cx| TableState::new(People, window, cx));
//! // in render
//! DataTable::new(&table).stripe(true)
//! ```

pub mod data_table;
pub mod delegate;
pub mod sink;
pub mod state;

pub use blenderui_table::{
    Cell, CellBuf, CellSource, Column, ColumnFixed, ColumnGroup, ColumnLayout, ColumnSort, RowMask, RowSelection, Size,
    SortKind, TableEvent, TableModel, TableOptions, TableSelection, TableVisibleRange, TextAlign, VecTable,
};
pub use blenderui_table::{clamp_scroll, max_scroll, scroll_to_show, visible_cols, visible_rows};
pub use data_table::DataTable;
pub use delegate::TableDelegate;
pub use state::TableState;
