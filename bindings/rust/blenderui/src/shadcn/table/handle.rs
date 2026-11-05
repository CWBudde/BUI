//! [`TableHandle`]: what `useReactTable` returns. A `Copy` handle to a table
//! that the helper components ([`DataTablePagination`](super::DataTablePagination),
//! [`DataTableViewOptions`](super::DataTableViewOptions) ...) read and drive
//! without owning it, so they can sit anywhere in the page.
//!
//! The table itself lives in the [`TableState`](crate::table::TableState) entity
//! the [`DataTable`](super::DataTable) creates. Reading goes through a
//! [`Snapshot`] (a small copy of what the UI shows: counts, page, column
//! list), republished whenever it changes; writing sends a [`Command`] to the
//! entity, which needs the `cx` every event handler has.

use std::rc::Rc;

use blenderui_table::tanstack::{ColumnFilter, ColumnInfo, PaginationState, SortingState, Table};
use blenderui_table::{ColumnSort, Text};

use crate::hooks::{use_signal, Signal};
use crate::App;

/// What the UI reads from a table (`table.getState()`, `getPageCount()` ...).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Snapshot {
    /// `getAllColumns()`.
    pub columns: Vec<ColumnInfo>,
    pub sorting: SortingState,
    pub column_filters: Vec<ColumnFilter>,
    pub global_filter: String,
    pub pagination: PaginationState,
    /// `getPageCount()`.
    pub page_count: usize,
    /// Rows after filtering (`getPrePaginationRowModel().rows.length`).
    pub row_count: usize,
    /// `getCoreRowModel().rows.length`.
    pub core_row_count: usize,
    /// `getFilteredSelectedRowModel().rows.length`.
    pub selected_count: usize,
    /// Rows on the current page.
    pub page_rows: usize,
}

impl Snapshot {
    pub(crate) fn of<T: Send + Sync>(t: &Table<T>) -> Self {
        Self {
            columns: t.column_infos(),
            sorting: t.sorting().clone(),
            column_filters: t.column_filters().clone(),
            global_filter: t.global_filter().to_owned(),
            pagination: *t.pagination(),
            page_count: t.page_count(),
            row_count: t.row_count(),
            core_row_count: t.core_row_count(),
            selected_count: t.filtered_selected_row_count(),
            page_rows: t.row_model().len(),
        }
    }
}

/// A change to make to a table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    PageIndex(usize),
    PageSize(usize),
    FirstPage,
    PreviousPage,
    NextPage,
    LastPage,
    ColumnVisibility(Text, Option<bool>),
    AllColumnsVisible(bool),
    ColumnFilter(Text, String),
    GlobalFilter(String),
    /// `Some(desc)`, or `None` to cycle ascending, descending, none.
    ColumnSorting(Text, Option<bool>),
    ClearColumnSorting(Text),
    /// The checkbox of page row `n` (the row's position on the page).
    TogglePageRow(usize),
    /// The select-all checkbox: selects the page, or deselects it when it is all selected.
    TogglePage,
    AllPageRowsSelected(bool),
    AllRowsSelected(bool),
}

impl Command {
    /// Do this to `t`.
    pub(crate) fn apply<T: Send + Sync>(self, t: &mut Table<T>) {
        match self {
            Self::PageIndex(i) => t.set_page_index(i),
            Self::PageSize(n) => t.set_page_size(n),
            Self::FirstPage => t.first_page(),
            Self::PreviousPage => t.previous_page(),
            Self::NextPage => t.next_page(),
            Self::LastPage => t.last_page(),
            Self::ColumnVisibility(id, v) => t.toggle_column_visibility(&id, v),
            Self::AllColumnsVisible(v) => t.toggle_all_columns_visible(v),
            Self::ColumnFilter(id, v) => t.set_column_filter_value(&id, v),
            Self::GlobalFilter(v) => t.set_global_filter(v),
            Self::ColumnSorting(id, desc) => t.toggle_column_sorting(&id, desc),
            Self::ClearColumnSorting(id) => t.set_column_sort(&id, ColumnSort::Default),
            Self::TogglePageRow(r) => {
                if let Some(row) = t.data_row(r) {
                    t.toggle_row_selected(row, None);
                }
            }
            Self::TogglePage => {
                let all = t.is_all_page_rows_selected();
                t.toggle_all_page_rows_selected(!all);
            }
            Self::AllPageRowsSelected(v) => t.toggle_all_page_rows_selected(v),
            Self::AllRowsSelected(v) => t.toggle_all_rows_selected(v),
        }
    }
}

/// The table behind a handle: what the [`DataTable`](super::DataTable) element
/// installs once it has created the entity.
pub(crate) trait Ops {
    /// Apply `cmd`, then publish the snapshot and report the changes.
    fn run(&self, cmd: Command, cx: &mut App);
}

/// The open column-header menu: which column, and where (table coordinates).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct HeaderMenu {
    pub column: Text,
    pub x: f32,
    pub y: f32,
}

/// `useReactTable`'s result.
///
/// ```ignore
/// let table = use_table();
/// rsx! {
///     <DataTable table={table} columns={columns} data={data} pagination={false} />
///     <DataTablePagination table={table} />
/// }
/// ```
#[derive(Clone, Copy)]
pub struct TableHandle {
    pub(crate) snap: Signal<Snapshot>,
    pub(crate) ops: Signal<Option<Rc<dyn Ops>>>,
    pub(crate) menu: Signal<Option<HeaderMenu>>,
}

/// A table handle for the calling component (the same one on every render).
#[track_caller]
#[must_use]
pub fn use_table() -> TableHandle {
    TableHandle { snap: use_signal(Snapshot::default()), ops: use_signal(None), menu: use_signal(None) }
}

impl TableHandle {
    /// A copy of what the table shows now (as of the last frame).
    #[must_use]
    pub fn snapshot(self) -> Snapshot {
        self.snap.get()
    }

    /// Send `cmd` to the table (nothing happens before the table's first frame).
    pub fn dispatch(self, cmd: Command, cx: &mut App) {
        if let Some(ops) = self.ops.get() {
            ops.run(cmd, cx);
            cx.notify_all();
        }
    }

    // -- reading (`table.getState()` and friends) --------------------------------------------------------------

    #[must_use]
    pub fn page_index(self) -> usize {
        self.snap.with(|s| s.pagination.page_index)
    }

    #[must_use]
    pub fn page_size(self) -> usize {
        self.snap.with(|s| s.pagination.page_size)
    }

    #[must_use]
    pub fn page_count(self) -> usize {
        self.snap.with(|s| s.page_count)
    }

    #[must_use]
    pub fn can_previous_page(self) -> bool {
        self.page_index() > 0
    }

    #[must_use]
    pub fn can_next_page(self) -> bool {
        self.snap.with(|s| s.pagination.page_index + 1 < s.page_count)
    }

    /// `getFilteredSelectedRowModel().rows.length`.
    #[must_use]
    pub fn selected_row_count(self) -> usize {
        self.snap.with(|s| s.selected_count)
    }

    /// `getFilteredRowModel().rows.length`.
    #[must_use]
    pub fn filtered_row_count(self) -> usize {
        self.snap.with(|s| s.row_count)
    }

    /// `getAllColumns()`.
    #[must_use]
    pub fn columns(self) -> Vec<ColumnInfo> {
        self.snap.with(|s| s.columns.clone())
    }

    /// `column.getFilterValue()`.
    #[must_use]
    pub fn column_filter_value(self, id: &str) -> String {
        self.snap.with(|s| s.column_filters.iter().find(|f| f.id == id).map(|f| f.value.clone()).unwrap_or_default())
    }

    #[must_use]
    pub fn global_filter(self) -> String {
        self.snap.with(|s| s.global_filter.clone())
    }

    // -- writing -------------------------------------------------------------------------------------------------

    pub fn set_page_index(self, index: usize, cx: &mut App) {
        self.dispatch(Command::PageIndex(index), cx);
    }

    pub fn set_page_size(self, size: usize, cx: &mut App) {
        self.dispatch(Command::PageSize(size), cx);
    }

    pub fn first_page(self, cx: &mut App) {
        self.dispatch(Command::FirstPage, cx);
    }

    pub fn previous_page(self, cx: &mut App) {
        self.dispatch(Command::PreviousPage, cx);
    }

    pub fn next_page(self, cx: &mut App) {
        self.dispatch(Command::NextPage, cx);
    }

    pub fn last_page(self, cx: &mut App) {
        self.dispatch(Command::LastPage, cx);
    }

    /// `column.toggleVisibility(value)`; `None` flips it.
    pub fn toggle_column_visibility(self, id: impl Into<Text>, visible: Option<bool>, cx: &mut App) {
        self.dispatch(Command::ColumnVisibility(id.into(), visible), cx);
    }

    /// `column.setFilterValue(value)`: an empty value clears the filter.
    pub fn set_column_filter_value(self, id: impl Into<Text>, value: impl Into<String>, cx: &mut App) {
        self.dispatch(Command::ColumnFilter(id.into(), value.into()), cx);
    }

    pub fn set_global_filter(self, value: impl Into<String>, cx: &mut App) {
        self.dispatch(Command::GlobalFilter(value.into()), cx);
    }

    /// `column.toggleSorting(desc)`; `None` cycles ascending, descending, none.
    pub fn toggle_column_sorting(self, id: impl Into<Text>, desc: Option<bool>, cx: &mut App) {
        self.dispatch(Command::ColumnSorting(id.into(), desc), cx);
    }

    /// `table.toggleAllPageRowsSelected(value)`.
    pub fn toggle_all_page_rows_selected(self, selected: bool, cx: &mut App) {
        self.dispatch(Command::AllPageRowsSelected(selected), cx);
    }

    /// `table.toggleAllRowsSelected(value)`.
    pub fn toggle_all_rows_selected(self, selected: bool, cx: &mut App) {
        self.dispatch(Command::AllRowsSelected(selected), cx);
    }
}
