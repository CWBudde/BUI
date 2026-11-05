//! [`Table`]: TanStack's table instance. It owns the data, the columns and
//! the state, keeps the row models (index lists) up to date, and is the
//! [`CellSource`] of the current page.

use std::cmp::Ordering;
use std::fmt;
use std::sync::Arc;

use super::column::{Accessor, ColumnDef};
use super::state::{
    Change, ColumnFilter, ColumnFiltersState, PaginationState, PartialState, RowSelectionState, SortingRule, SortingState, TableState,
    VisibilityState,
};
use crate::cell::{Cell, CellSource, Check, SortKind};
use crate::column::{BadgeVariant, CellRender, Column, ColumnSort, Text};
use crate::fmt::CellBuf;
use crate::mask::RowMask;
use crate::sort::sorted_by_column;

/// What is wrong with a table's options, found when they are given (the
/// boundary): nothing later has to check again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TableError {
    /// An `accessor_key` names a field the row type does not have.
    UnknownKey(Text),
    /// Two columns have the same id.
    DuplicateId(Text),
    /// A page size of zero.
    ZeroPageSize,
    /// More rows than a row id (`u32`) can name.
    TooManyRows(usize),
}

impl fmt::Display for TableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownKey(k) => write!(f, "accessor_key({k:?}): the row type has no such field"),
            Self::DuplicateId(i) => write!(f, "two columns have the id {i:?}"),
            Self::ZeroPageSize => f.write_str("the page size must be at least 1"),
            Self::TooManyRows(n) => write!(f, "{n} rows do not fit in a u32 row id"),
        }
    }
}

impl std::error::Error for TableError {}

/// TanStack's table options.
#[allow(clippy::struct_excessive_bools)]
pub struct TableOptions<T> {
    pub columns: Arc<[ColumnDef<T>]>,
    pub data: Arc<[T]>,
    /// `initialState`: where an uncontrolled field starts.
    pub initial_state: TableState,
    /// `state`: the fields the parent controls.
    pub state: PartialState,
    /// `getPaginationRowModel()`: show one page of the rows at a time.
    pub paginate: bool,
    /// `manualSorting`: the data arrives sorted (a server did it).
    pub manual_sorting: bool,
    /// `manualFiltering`: the data arrives filtered.
    pub manual_filtering: bool,
    /// `manualPagination`: the data is one page already; with `page_count`.
    pub manual_pagination: bool,
    /// `pageCount` (with `manual_pagination`).
    pub page_count: Option<usize>,
    /// `autoResetPageIndex`: go back to the first page when the sorting or
    /// filters change.
    pub auto_reset_page_index: bool,
}

impl<T> TableOptions<T> {
    /// Options for `columns` over `data` with TanStack's defaults.
    pub fn new(columns: impl Into<Arc<[ColumnDef<T>]>>, data: impl Into<Arc<[T]>>) -> Self {
        Self {
            columns: columns.into(),
            data: data.into(),
            initial_state: TableState::default(),
            state: PartialState::default(),
            paginate: false,
            manual_sorting: false,
            manual_filtering: false,
            manual_pagination: false,
            page_count: None,
            auto_reset_page_index: true,
        }
    }

    /// Show one page at a time (`getPaginationRowModel`).
    #[must_use]
    pub const fn paginate(mut self, on: bool) -> Self {
        self.paginate = on;
        self
    }

    #[must_use]
    pub fn initial_state(mut self, state: TableState) -> Self {
        self.initial_state = state;
        self
    }

    /// The controlled state.
    #[must_use]
    pub fn state(mut self, state: PartialState) -> Self {
        self.state = state;
        self
    }
}

/// What the UI needs to know about a column (header menus, the view-options
/// dropdown).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColumnInfo {
    pub id: Text,
    pub title: Text,
    pub can_sort: bool,
    pub can_hide: bool,
    pub visible: bool,
    pub sort: ColumnSort,
}

/// Pieces of the row models, in the order they depend on each other.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Stage {
    Filter,
    Sort,
    Page,
    Selection,
}

/// TanStack's table instance over `data`.
pub struct Table<T> {
    columns: Arc<[ColumnDef<T>]>,
    data: Arc<[T]>,
    flags: Flags,
    internal: TableState,
    controlled: PartialState,
    changes: Vec<Change>,
    filtered: RowMask,
    sorted: RowMask,
    page: Vec<u32>,
    filtered_selected: usize,
    dirty: bool,
}

#[derive(Copy, Clone)]
#[allow(clippy::struct_excessive_bools)]
struct Flags {
    paginate: bool,
    manual_sorting: bool,
    manual_filtering: bool,
    manual_pagination: bool,
    page_count: Option<usize>,
    auto_reset_page_index: bool,
}

impl<T: Send + Sync> Table<T> {
    /// A table, or what is wrong with the options.
    pub fn new(opts: TableOptions<T>) -> Result<Self, TableError> {
        let TableOptions { columns, data, initial_state, state, paginate, manual_sorting, manual_filtering, manual_pagination, page_count, auto_reset_page_index } =
            opts;
        validate(&columns, data.len(), initial_state.pagination, &state)?;
        let mut t = Self {
            columns,
            data,
            flags: Flags { paginate, manual_sorting, manual_filtering, manual_pagination, page_count, auto_reset_page_index },
            internal: initial_state,
            controlled: state,
            changes: Vec::new(),
            filtered: RowMask::Identity,
            sorted: RowMask::Identity,
            page: Vec::new(),
            filtered_selected: 0,
            dirty: true,
        };
        t.recompute(Stage::Filter);
        Ok(t)
    }

    /// Take new options (a parent re-rendered): data and columns are
    /// compared by identity (`Arc::ptr_eq`, like TanStack's memoized
    /// references), the controlled state by value. Returns whether anything
    /// the view shows changed.
    pub fn set_options(&mut self, opts: TableOptions<T>) -> Result<bool, TableError> {
        let TableOptions { columns, data, state, paginate, manual_sorting, manual_filtering, manual_pagination, page_count, auto_reset_page_index, initial_state: _ } = opts;
        let pagination = state.pagination.unwrap_or_else(|| *self.pagination());
        validate(&columns, data.len(), pagination, &state)?;
        let flags = Flags { paginate, manual_sorting, manual_filtering, manual_pagination, page_count, auto_reset_page_index };
        let mut from = None;
        let mut note = |s: Stage| from = Some(from.map_or(s, |f: Stage| f.min(s)));
        if !Arc::ptr_eq(&self.columns, &columns) {
            self.columns = columns;
            note(Stage::Filter);
        }
        if !Arc::ptr_eq(&self.data, &data) {
            self.data = data;
            self.internal.row_selection.clamp(self.data.len());
            note(Stage::Filter);
        }
        if state != self.controlled {
            let old = std::mem::replace(&mut self.controlled, state);
            note(if old.column_filters != self.controlled.column_filters || old.global_filter != self.controlled.global_filter {
                Stage::Filter
            } else if old.sorting != self.controlled.sorting {
                Stage::Sort
            } else if old.pagination != self.controlled.pagination {
                Stage::Page
            } else {
                Stage::Selection
            });
        }
        if (flags.paginate, flags.manual_sorting, flags.manual_filtering, flags.manual_pagination, flags.page_count)
            != (self.flags.paginate, self.flags.manual_sorting, self.flags.manual_filtering, self.flags.manual_pagination, self.flags.page_count)
        {
            note(Stage::Filter);
        }
        self.flags = flags;
        Ok(from.is_some_and(|s| {
            self.recompute(s);
            true
        }))
    }

    // -- state ---------------------------------------------------------------------------------------

    /// The state as shown: the controlled fields over the internal ones.
    #[must_use]
    pub fn state(&self) -> TableState {
        TableState {
            sorting: self.sorting().clone(),
            column_filters: self.column_filters().clone(),
            global_filter: self.global_filter().to_owned(),
            column_visibility: self.column_visibility().clone(),
            row_selection: self.row_selection().clone(),
            pagination: *self.pagination(),
        }
    }

    #[must_use]
    pub fn sorting(&self) -> &SortingState {
        self.controlled.sorting.as_ref().unwrap_or(&self.internal.sorting)
    }

    #[must_use]
    pub fn column_filters(&self) -> &ColumnFiltersState {
        self.controlled.column_filters.as_ref().unwrap_or(&self.internal.column_filters)
    }

    #[must_use]
    pub fn global_filter(&self) -> &str {
        self.controlled.global_filter.as_deref().unwrap_or(&self.internal.global_filter)
    }

    #[must_use]
    pub fn column_visibility(&self) -> &VisibilityState {
        self.controlled.column_visibility.as_ref().unwrap_or(&self.internal.column_visibility)
    }

    #[must_use]
    pub fn row_selection(&self) -> &RowSelectionState {
        self.controlled.row_selection.as_ref().unwrap_or(&self.internal.row_selection)
    }

    #[must_use]
    pub fn pagination(&self) -> &PaginationState {
        self.controlled.pagination.as_ref().unwrap_or(&self.internal.pagination)
    }

    /// The changes the user made since the last call, in order (what the
    /// `onXChange` callbacks receive).
    pub fn take_changes(&mut self) -> Vec<Change> {
        std::mem::take(&mut self.changes)
    }

    /// Whether the view needs to redraw; clears the flag.
    pub const fn take_dirty(&mut self) -> bool {
        std::mem::replace(&mut self.dirty, false)
    }

    // -- setters (`setSorting`, `setPageIndex` ...): apply to an uncontrolled field, always report --------

    /// `setSorting`.
    pub fn set_sorting(&mut self, sorting: SortingState) {
        if sorting == *self.sorting() {
            return;
        }
        self.changes.push(Change::Sorting(sorting.clone()));
        if self.controlled.sorting.is_none() {
            self.internal.sorting = sorting;
            self.reset_page_index();
            self.recompute(Stage::Sort);
        }
    }

    /// `setColumnFilters`.
    pub fn set_column_filters(&mut self, filters: ColumnFiltersState) {
        if filters == *self.column_filters() {
            return;
        }
        self.changes.push(Change::ColumnFilters(filters.clone()));
        if self.controlled.column_filters.is_none() {
            self.internal.column_filters = filters;
            self.reset_page_index();
            self.recompute(Stage::Filter);
        }
    }

    /// `setGlobalFilter`.
    pub fn set_global_filter(&mut self, value: impl Into<String>) {
        let value = value.into();
        if value == self.global_filter() {
            return;
        }
        self.changes.push(Change::GlobalFilter(value.clone()));
        if self.controlled.global_filter.is_none() {
            self.internal.global_filter = value;
            self.reset_page_index();
            self.recompute(Stage::Filter);
        }
    }

    /// `setColumnVisibility`.
    pub fn set_column_visibility(&mut self, visibility: VisibilityState) {
        if visibility == *self.column_visibility() {
            return;
        }
        self.changes.push(Change::ColumnVisibility(visibility.clone()));
        if self.controlled.column_visibility.is_none() {
            self.internal.column_visibility = visibility;
            self.dirty = true;
        }
    }

    /// `setRowSelection`.
    pub fn set_row_selection(&mut self, selection: RowSelectionState) {
        if selection == *self.row_selection() {
            return;
        }
        self.changes.push(Change::RowSelection(selection.clone()));
        if self.controlled.row_selection.is_none() {
            self.internal.row_selection = selection;
            self.recompute(Stage::Selection);
        }
    }

    /// `setPagination`.
    pub fn set_pagination(&mut self, pagination: PaginationState) {
        let pagination = PaginationState { page_size: pagination.page_size.max(1), ..pagination };
        if pagination == *self.pagination() {
            return;
        }
        self.changes.push(Change::Pagination(pagination));
        if self.controlled.pagination.is_none() {
            self.internal.pagination = pagination;
            self.recompute(Stage::Page);
        }
    }

    fn reset_page_index(&mut self) {
        if self.flags.auto_reset_page_index && !self.flags.manual_pagination && self.pagination().page_index != 0 {
            self.set_pagination(PaginationState { page_index: 0, ..*self.pagination() });
        }
    }

    // -- columns ----------------------------------------------------------------------------------------

    /// `getAllColumns`.
    #[must_use]
    pub fn column_defs(&self) -> &[ColumnDef<T>] {
        &self.columns
    }

    /// The index of the column `id`.
    #[must_use]
    pub fn column_index(&self, id: &str) -> Option<usize> {
        self.columns.iter().position(|c| c.id == id)
    }

    /// `column.getIsVisible()`.
    #[must_use]
    pub fn column_is_visible(&self, id: &str) -> bool {
        self.column_visibility().get(id).copied().unwrap_or(true)
    }

    /// `column.toggleVisibility(value)`; `None` flips it. Columns with
    /// `enable_hiding` off ignore it.
    pub fn toggle_column_visibility(&mut self, id: &str, visible: Option<bool>) {
        let Some(def) = self.columns.iter().find(|c| c.id == id) else { return };
        if !def.enable_hiding {
            return;
        }
        let want = visible.unwrap_or_else(|| !self.column_is_visible(id));
        let mut v = self.column_visibility().clone();
        v.insert(def.id.clone(), want);
        self.set_column_visibility(v);
    }

    /// `table.toggleAllColumnsVisible(value)`.
    pub fn toggle_all_columns_visible(&mut self, visible: bool) {
        let mut v = self.column_visibility().clone();
        for c in self.columns.iter().filter(|c| c.enable_hiding) {
            v.insert(c.id.clone(), visible);
        }
        self.set_column_visibility(v);
    }

    /// `column.getIsSorted()`.
    #[must_use]
    pub fn column_sort(&self, id: &str) -> ColumnSort {
        self.sorting().iter().find(|r| r.id == id).map_or(ColumnSort::Default, |r| if r.desc { ColumnSort::Descending } else { ColumnSort::Ascending })
    }

    /// `column.toggleSorting(desc)` for one column, replacing other sorting
    /// (`multi` off): `Some(false)` ascending, `Some(true)` descending,
    /// `None` cycles ascending, descending, none (the header click).
    pub fn toggle_column_sorting(&mut self, id: &str, desc: Option<bool>) {
        let Some(def) = self.columns.iter().find(|c| c.id == id) else { return };
        if !def.enable_sorting {
            return;
        }
        let next = match desc {
            Some(d) => ColumnSort::from_desc(d),
            None => self.column_sort(id).next(),
        };
        self.set_column_sort(id, next);
    }

    /// Sort by `id` alone in direction `sort` (`Default` clears the sorting).
    pub fn set_column_sort(&mut self, id: &str, sort: ColumnSort) {
        let Some(def) = self.columns.iter().find(|c| c.id == id) else { return };
        let v: SortingState = match sort {
            ColumnSort::Default => Vec::new(),
            s => vec![SortingRule::new(def.id.clone(), s == ColumnSort::Descending)],
        };
        self.set_sorting(v);
    }

    /// `column.setFilterValue`: an empty value clears the filter.
    pub fn set_column_filter_value(&mut self, id: &str, value: impl Into<String>) {
        let Some(def) = self.columns.iter().find(|c| c.id == id) else { return };
        if !def.enable_column_filter {
            return;
        }
        let value = value.into();
        let mut f: ColumnFiltersState = self.column_filters().iter().filter(|f| f.id != id).cloned().collect();
        if !value.is_empty() {
            f.push(ColumnFilter::new(def.id.clone(), value));
        }
        self.set_column_filters(f);
    }

    /// `column.getFilterValue()`.
    #[must_use]
    pub fn column_filter_value(&self, id: &str) -> Option<&str> {
        self.column_filters().iter().find(|f| f.id == id).map(|f| f.value.as_str())
    }

    /// The columns as the UI lists them.
    #[must_use]
    pub fn column_infos(&self) -> Vec<ColumnInfo> {
        self.columns
            .iter()
            .map(|c| ColumnInfo {
                id: c.id.clone(),
                title: Text::Owned(c.header.title().to_owned()),
                can_sort: c.enable_sorting,
                can_hide: c.enable_hiding,
                visible: self.column_is_visible(&c.id),
                sort: self.column_sort(&c.id),
            })
            .collect()
    }

    /// Column `ix` as the table core draws it: a hidden one is flagged
    /// invisible, so every column keeps its index.
    #[must_use]
    pub fn core_column(&self, ix: usize) -> Column {
        let d = &self.columns[ix];
        let mut c = Column::new(d.id.clone(), Text::Owned(d.header.title().to_owned())).width(d.size).min_width(d.min_size).max_width(d.max_size);
        c.align = d.meta.align;
        c.format = d.meta.format.clone();
        c.render = d.cell;
        c.header_menu = matches!(d.header, super::column::Header::Menu(_)) && d.enable_sorting;
        c.resizable = d.enable_resizing;
        c.movable = false;
        c.selectable = false;
        c.visible = self.column_is_visible(&d.id);
        if d.meta.pinned {
            c = c.fixed_left();
        }
        if d.enable_sorting {
            c.sort = Some(self.column_sort(&d.id));
        }
        c
    }

    /// All columns as the table core draws them.
    #[must_use]
    pub fn core_columns(&self) -> Vec<Column> {
        (0..self.columns.len()).map(|i| self.core_column(i)).collect()
    }

    // -- row models ---------------------------------------------------------------------------------------

    /// `getCoreRowModel`: every row of the data, in order.
    #[must_use]
    pub const fn core_row_model(&self) -> RowMask {
        RowMask::Identity
    }

    /// `getFilteredRowModel`.
    #[must_use]
    pub const fn filtered_row_model(&self) -> &RowMask {
        &self.filtered
    }

    /// `getSortedRowModel`: the filtered rows, sorted.
    #[must_use]
    pub const fn sorted_row_model(&self) -> &RowMask {
        &self.sorted
    }

    /// `getRowModel` (`getPaginationRowModel` when paging): the data rows of the current page.
    #[must_use]
    pub fn row_model(&self) -> &[u32] {
        &self.page
    }

    /// The data row shown at page row `r`.
    #[must_use]
    pub fn data_row(&self, r: usize) -> Option<usize> {
        self.page.get(r).map(|&d| d as usize)
    }

    /// The data (`table.options.data`).
    #[must_use]
    pub fn data(&self) -> &[T] {
        &self.data
    }

    /// `getPrePaginationRowModel().rows.length`: the rows after filtering.
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.sorted.len(self.data.len())
    }

    /// `getCoreRowModel().rows.length`.
    #[must_use]
    pub fn core_row_count(&self) -> usize {
        self.data.len()
    }

    /// `getFilteredRowModel().rows.length`.
    #[must_use]
    pub fn filtered_row_count(&self) -> usize {
        self.filtered.len(self.data.len())
    }

    // -- pagination ---------------------------------------------------------------------------------------

    /// `getPageCount`.
    #[must_use]
    pub fn page_count(&self) -> usize {
        if self.flags.manual_pagination {
            return self.flags.page_count.unwrap_or(1);
        }
        if !self.flags.paginate {
            return 1;
        }
        self.row_count().div_ceil(self.pagination().page_size)
    }

    #[must_use]
    pub fn can_previous_page(&self) -> bool {
        self.pagination().page_index > 0
    }

    #[must_use]
    pub fn can_next_page(&self) -> bool {
        self.pagination().page_index + 1 < self.page_count()
    }

    /// `setPageIndex`, clamped to the pages there are.
    pub fn set_page_index(&mut self, index: usize) {
        let last = self.page_count().saturating_sub(1);
        self.set_pagination(PaginationState { page_index: index.min(last), ..*self.pagination() });
    }

    pub fn first_page(&mut self) {
        self.set_page_index(0);
    }

    pub fn previous_page(&mut self) {
        self.set_page_index(self.pagination().page_index.saturating_sub(1));
    }

    pub fn next_page(&mut self) {
        self.set_page_index(self.pagination().page_index + 1);
    }

    pub fn last_page(&mut self) {
        self.set_page_index(self.page_count().saturating_sub(1));
    }

    /// `setPageSize`: keeps the first row of the page in view.
    pub fn set_page_size(&mut self, size: usize) {
        let size = size.max(1);
        let p = *self.pagination();
        let top = p.page_index * p.page_size;
        self.set_pagination(PaginationState { page_index: top / size, page_size: size });
    }

    // -- selection -----------------------------------------------------------------------------------------

    /// `row.getIsSelected()`.
    #[must_use]
    pub fn is_row_selected(&self, data_row: usize) -> bool {
        self.row_selection().is_selected(data_row)
    }

    /// `row.toggleSelected(value)`; `None` flips it.
    pub fn toggle_row_selected(&mut self, data_row: usize, selected: Option<bool>) {
        if data_row >= self.data.len() {
            return;
        }
        let mut s = self.row_selection().clone();
        s.set(data_row, selected.unwrap_or_else(|| !s.is_selected(data_row)));
        self.set_row_selection(s);
    }

    /// `getFilteredSelectedRowModel().rows.length`.
    #[must_use]
    pub const fn filtered_selected_row_count(&self) -> usize {
        self.filtered_selected
    }

    /// `getIsAllPageRowsSelected`.
    #[must_use]
    pub fn is_all_page_rows_selected(&self) -> bool {
        !self.page.is_empty() && self.page.iter().all(|&r| self.is_row_selected(r as usize))
    }

    /// `getIsSomePageRowsSelected`.
    #[must_use]
    pub fn is_some_page_rows_selected(&self) -> bool {
        !self.is_all_page_rows_selected() && self.page.iter().any(|&r| self.is_row_selected(r as usize))
    }

    /// `toggleAllPageRowsSelected(value)`.
    pub fn toggle_all_page_rows_selected(&mut self, selected: bool) {
        let mut s = self.row_selection().clone();
        for &r in &self.page {
            s.set(r as usize, selected);
        }
        self.set_row_selection(s);
    }

    /// `toggleAllRowsSelected(value)`: every row of the data.
    pub fn toggle_all_rows_selected(&mut self, selected: bool) {
        let mut s = RowSelectionState::default();
        if selected {
            s.select_all(self.data.len());
        }
        self.set_row_selection(s);
    }

    /// `getSelectedRowModel().rows`: the selected rows of the data.
    #[must_use]
    pub fn selected_rows(&self) -> Vec<&T> {
        self.row_selection().iter().filter_map(|i| self.data.get(i)).collect()
    }

    // -- recomputing ----------------------------------------------------------------------------------------

    fn recompute(&mut self, from: Stage) {
        self.dirty = true;
        let n = self.data.len();
        let grid = Grid { columns: &self.columns, data: &self.data };
        if from <= Stage::Filter {
            self.filtered = if self.flags.manual_filtering { RowMask::Identity } else { filter_rows(&grid, self.column_filters(), self.global_filter()) };
        }
        if from <= Stage::Sort {
            self.sorted = if self.flags.manual_sorting { self.filtered.clone() } else { sort_rows(&grid, self.sorting(), &self.filtered) };
        }
        if from <= Stage::Page {
            let total = self.sorted.len(n);
            let (start, end) = if self.flags.paginate && !self.flags.manual_pagination {
                let p = self.pagination();
                let start = p.page_index.saturating_mul(p.page_size).min(total);
                (start, (start + p.page_size).min(total))
            } else {
                (0, total)
            };
            let sorted = &self.sorted;
            self.page.clear();
            self.page.extend((start..end).map(|i| sorted.get(i) as u32));
        }
        // the selected rows among the filtered ones (`getFilteredSelectedRowModel`)
        self.filtered_selected = match &self.filtered {
            RowMask::Identity => self.row_selection().iter().filter(|&i| i < n).count(),
            RowMask::Indices(v) => {
                let sel = self.row_selection();
                v.iter().filter(|&&r| sel.is_selected(r as usize)).count()
            }
        };
    }
}

fn validate<T>(columns: &[ColumnDef<T>], rows: usize, pagination: PaginationState, state: &PartialState) -> Result<(), TableError> {
    if rows > u32::MAX as usize {
        return Err(TableError::TooManyRows(rows));
    }
    if pagination.page_size == 0 || state.pagination.is_some_and(|p| p.page_size == 0) {
        return Err(TableError::ZeroPageSize);
    }
    for (i, c) in columns.iter().enumerate() {
        if let Accessor::Missing(k) = &c.accessor {
            return Err(TableError::UnknownKey(k.clone()));
        }
        if columns[..i].iter().any(|o| o.id == c.id) {
            return Err(TableError::DuplicateId(c.id.clone()));
        }
    }
    Ok(())
}

impl ColumnSort {
    const fn from_desc(desc: bool) -> Self {
        if desc { Self::Descending } else { Self::Ascending }
    }
}

// -- the data rows as a CellSource (what sorting and filtering read) --------------------------------------------

/// The data rows: `cell(row, col)` is the value of data row `row`.
struct Grid<'a, T> {
    columns: &'a [ColumnDef<T>],
    data: &'a [T],
}

impl<T: Send + Sync> CellSource for Grid<'_, T> {
    fn cell(&self, row: usize, col: usize) -> Cell<'_> {
        match (self.data.get(row), self.columns.get(col)) {
            (Some(r), Some(c)) => c.read(r),
            _ => Cell::Empty,
        }
    }

    fn sort_kind(&self, _: usize) -> SortKind {
        SortKind::Cells
    }
}

fn contains_ignore_case(hay: &str, needle_lower: &str) -> bool {
    if needle_lower.is_empty() {
        return true;
    }
    if hay.is_ascii() && needle_lower.is_ascii() {
        let (h, n) = (hay.as_bytes(), needle_lower.as_bytes());
        return h.len() >= n.len() && h.windows(n.len()).any(|w| w.eq_ignore_ascii_case(n));
    }
    hay.to_lowercase().contains(needle_lower)
}

/// The rows passing every column filter and the global filter
/// (`includesString`: case-insensitive contains, on the cell's text).
fn filter_rows<T: Send + Sync>(grid: &Grid<'_, T>, filters: &[ColumnFilter], global: &str) -> RowMask {
    let cols: Vec<(usize, String)> = filters
        .iter()
        .filter_map(|f| grid.columns.iter().position(|c| c.id == f.id && c.enable_column_filter).map(|i| (i, f.value.to_lowercase())))
        .collect();
    let global = global.to_lowercase();
    if cols.is_empty() && global.is_empty() {
        return RowMask::Identity;
    }
    let global_cols: Vec<usize> = if global.is_empty() {
        Vec::new()
    } else {
        grid.columns.iter().enumerate().filter(|(_, c)| c.enable_global_filter && c.has_accessor()).map(|(i, _)| i).collect()
    };
    let text_has = |row: usize, col: usize, needle: &str| {
        let mut buf = CellBuf::new();
        let cell = grid.cell(row, col);
        contains_ignore_case(cell.text(&mut buf), needle)
    };
    RowMask::Identity.filtered(grid.data.len(), |r| {
        cols.iter().all(|(c, v)| text_has(r, *c, v)) && (global.is_empty() || global_cols.iter().any(|&c| text_has(r, c, &global)))
    })
}

/// `base` sorted by every rule: the last rule first, then the earlier ones
/// stably on top, so the first rule is the most significant.
fn sort_rows<T: Send + Sync>(grid: &Grid<'_, T>, rules: &[SortingRule], base: &RowMask) -> RowMask {
    let n = grid.data.len();
    let mut mask: Option<RowMask> = None;
    for r in rules.iter().rev() {
        let Some(col) = grid.columns.iter().position(|c| c.id == r.id && c.enable_sorting) else { continue };
        mask = Some(sorted_by_column(grid, col, mask.as_ref().unwrap_or(base), n, r.desc));
    }
    mask.unwrap_or_else(|| base.clone())
}

// -- the current page as a CellSource (what the table paints) -------------------------------------------------------

impl<T: Send + Sync> CellSource for Table<T> {
    fn cell(&self, row: usize, col: usize) -> Cell<'_> {
        let (Some(&data_row), Some(def)) = (self.page.get(row), self.columns.get(col)) else { return Cell::Empty };
        match def.accessor {
            Accessor::Read(_) => def.read(&self.data[data_row as usize]),
            _ if def.cell == CellRender::Checkbox => Cell::U64(u64::from(self.is_row_selected(data_row as usize))),
            _ => Cell::Empty,
        }
    }

    fn sort_kind(&self, _: usize) -> SortKind {
        SortKind::Cells
    }

    fn compare(&self, _: usize, _: usize, _: usize) -> Ordering {
        Ordering::Equal
    }

    fn badge(&self, row: usize, col: usize) -> BadgeVariant {
        let (Some(def), true) = (self.columns.get(col), row < self.page.len()) else { return BadgeVariant::default() };
        let Some(f) = &def.badge else { return BadgeVariant::default() };
        let mut buf = CellBuf::new();
        let cell = self.cell(row, col);
        f(cell.text(&mut buf))
    }

    fn header_check(&self, col: usize) -> Check {
        if self.columns.get(col).is_none_or(|c| c.cell != CellRender::Checkbox) {
            Check::Unchecked
        } else if self.is_all_page_rows_selected() {
            Check::Checked
        } else if self.is_some_page_rows_selected() {
            Check::Indeterminate
        } else {
            Check::Unchecked
        }
    }
}

#[cfg(test)]
mod tests;
