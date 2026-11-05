//! [`DataTable`]: shadcn's `<DataTable columns={columns} data={data} />`.
//!
//! One component that renders the whole recipe: the toolbar (filter input and
//! view options), the table, and the pagination footer. It is generic over the
//! row type, which `#[component]` cannot be, so its props builder is written
//! out: `DataTable::__builder()` is what `rsx!` calls, `DataTable::new` is the
//! same thing for builder chains, and the type parameters `C` and `D` are
//! `()` until `columns` and `data` are given (a missing one is the usual
//! "a required prop is missing" compile error).

use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;

use blenderui_table::tanstack::{ColumnDef, PaginationState, PartialState, RowSelectionState, SortingState, Table, TableOptions, TableState, VisibilityState};
use blenderui_table::Text;

use super::column_header::DataTableColumnHeader;
use super::handle::{use_table, Ops, TableHandle};
use super::link::{Callbacks, Link};
use super::pagination::DataTablePagination;
use super::toolbar::DataTableToolbar;
use crate::css::Decl;
use crate::hooks::{build, use_signal, Complete, Signal};
use crate::prelude::*;
use crate::table::{DataTable as TableElement, TableState as TableEntity};

/// Everything that is not the columns or the data.
struct Extras {
    base: Div,
    handle: Option<TableHandle>,
    initial_state: TableState,
    state: PartialState,
    filter_column: Option<Text>,
    filter_placeholder: Option<String>,
    toolbar: Option<bool>,
    view_options: bool,
    pagination: bool,
    page_sizes: Option<Vec<usize>>,
    callbacks: Callbacks,
}

/// The Data Table recipe over rows of type `T`.
///
/// ```ignore
/// // rsx!
/// rsx! { <DataTable columns={columns} data={data} filter_column="email" h={px(420.0)} /> }
/// // builder
/// DataTable::new(columns, data).filter_column("email").h(px(420.0))
/// ```
///
/// `columns` and `data` are compared by identity (an `Arc` clone is the same
/// table; a fresh `Vec` is new data), like TanStack's memoized references:
/// build them once (`use_memo`, or a field of your view) and pass them again.
pub struct DataTable<T, C = Arc<[ColumnDef<T>]>, D = Arc<[T]>> {
    columns: C,
    data: D,
    extras: Extras,
    row: PhantomData<fn() -> T>,
}

impl<T> DataTable<T, (), ()> {
    /// A table of `data` with `columns`.
    pub fn new(columns: impl Into<Arc<[ColumnDef<T>]>>, data: impl Into<Arc<[T]>>) -> DataTable<T> {
        Self::__builder().columns(columns).data(data)
    }

    #[doc(hidden)]
    #[must_use]
    pub fn __builder() -> Self {
        Self {
            columns: (),
            data: (),
            extras: Extras {
                base: div().flex().flex_col().gap_2(),
                handle: None,
                initial_state: TableState::default(),
                state: PartialState::default(),
                filter_column: None,
                filter_placeholder: None,
                toolbar: None,
                view_options: true,
                pagination: true,
                page_sizes: None,
                callbacks: Callbacks::default(),
            },
            row: PhantomData,
        }
    }
}

impl<T, D> DataTable<T, (), D> {
    /// `columns`.
    pub fn columns(self, columns: impl Into<Arc<[ColumnDef<T>]>>) -> DataTable<T, Arc<[ColumnDef<T>]>, D> {
        DataTable { columns: columns.into(), data: self.data, extras: self.extras, row: PhantomData }
    }
}

impl<T, C> DataTable<T, C, ()> {
    /// `data`.
    pub fn data(self, data: impl Into<Arc<[T]>>) -> DataTable<T, C, Arc<[T]>> {
        DataTable { columns: self.columns, data: data.into(), extras: self.extras, row: PhantomData }
    }
}

impl<T, C, D> DataTable<T, C, D> {
    /// A handle made with [`use_table`], so helper components elsewhere in
    /// the page (`DataTablePagination`, `DataTableViewOptions` ...) drive this table.
    #[must_use]
    pub fn table(mut self, handle: TableHandle) -> Self {
        self.extras.handle = Some(handle);
        self
    }

    /// `initialState`: where the uncontrolled state starts.
    #[must_use]
    pub fn initial_state(mut self, state: TableState) -> Self {
        self.extras.initial_state = state;
        self
    }

    /// Control the sorting (`state.sorting`); see `on_sorting_change`.
    #[must_use]
    pub fn sorting(mut self, v: SortingState) -> Self {
        self.extras.state.sorting = Some(v);
        self
    }

    /// Control the column filters (`state.columnFilters`).
    #[must_use]
    pub fn column_filters(mut self, v: Vec<blenderui_table::tanstack::ColumnFilter>) -> Self {
        self.extras.state.column_filters = Some(v);
        self
    }

    /// Control the global filter (`state.globalFilter`).
    #[must_use]
    pub fn global_filter(mut self, v: impl Into<String>) -> Self {
        self.extras.state.global_filter = Some(v.into());
        self
    }

    /// Control the column visibility (`state.columnVisibility`).
    #[must_use]
    pub fn column_visibility(mut self, v: VisibilityState) -> Self {
        self.extras.state.column_visibility = Some(v);
        self
    }

    /// Control the row selection (`state.rowSelection`).
    #[must_use]
    pub fn row_selection(mut self, v: RowSelectionState) -> Self {
        self.extras.state.row_selection = Some(v);
        self
    }

    /// Control the pagination (`state.pagination`).
    #[must_use]
    pub fn pagination(mut self, v: PaginationState) -> Self {
        self.extras.state.pagination = Some(v);
        self
    }

    /// Page the rows and show the pagination footer (`getPaginationRowModel`; on by default).
    #[must_use]
    pub const fn enable_pagination(mut self, on: bool) -> Self {
        self.extras.pagination = on;
        self
    }

    /// The choices of the "Rows per page" menu.
    #[must_use]
    pub fn page_sizes(mut self, sizes: Vec<usize>) -> Self {
        self.extras.page_sizes = Some(sizes);
        self
    }

    /// Show the toolbar (the default when there is a filter column or the view options).
    #[must_use]
    pub const fn toolbar(mut self, on: bool) -> Self {
        self.extras.toolbar = Some(on);
        self
    }

    /// The column the toolbar's filter input filters (`column.setFilterValue`);
    /// without one the input is the global filter.
    #[must_use]
    pub fn filter_column(mut self, id: impl Into<Text>) -> Self {
        self.extras.filter_column = Some(id.into());
        self
    }

    #[must_use]
    pub fn filter_placeholder(mut self, text: impl Into<String>) -> Self {
        self.extras.filter_placeholder = Some(text.into());
        self
    }

    /// The "View" column-visibility menu in the toolbar (on by default).
    #[must_use]
    pub const fn view_options(mut self, on: bool) -> Self {
        self.extras.view_options = on;
        self
    }

    /// `onSortingChange`.
    #[must_use]
    pub fn on_sorting_change(mut self, f: impl Fn(&SortingState) + 'static) -> Self {
        self.extras.callbacks.sorting = Some(Box::new(f));
        self
    }

    /// `onColumnFiltersChange`.
    #[must_use]
    pub fn on_column_filters_change(mut self, f: impl Fn(&Vec<blenderui_table::tanstack::ColumnFilter>) + 'static) -> Self {
        self.extras.callbacks.column_filters = Some(Box::new(f));
        self
    }

    /// `onGlobalFilterChange`.
    #[must_use]
    pub fn on_global_filter_change(mut self, f: impl Fn(&String) + 'static) -> Self {
        self.extras.callbacks.global_filter = Some(Box::new(f));
        self
    }

    /// `onColumnVisibilityChange`.
    #[must_use]
    pub fn on_column_visibility_change(mut self, f: impl Fn(&VisibilityState) + 'static) -> Self {
        self.extras.callbacks.column_visibility = Some(Box::new(f));
        self
    }

    /// `onRowSelectionChange`.
    #[must_use]
    pub fn on_row_selection_change(mut self, f: impl Fn(&RowSelectionState) + 'static) -> Self {
        self.extras.callbacks.row_selection = Some(Box::new(f));
        self
    }

    /// `onPaginationChange`.
    #[must_use]
    pub fn on_pagination_change(mut self, f: impl Fn(&PaginationState) + 'static) -> Self {
        self.extras.callbacks.pagination = Some(Box::new(f));
        self
    }

    /// A click on a row's "more" button ([`actions_column`](super::actions_column)):
    /// the index of the row in the data.
    #[must_use]
    pub fn on_row_action(mut self, f: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.extras.callbacks.row_action = Some(Box::new(f));
        self
    }
}

impl<T, C, D> Styled for DataTable<T, C, D> {
    fn decls_mut(&mut self) -> &mut Vec<Decl> {
        self.extras.base.decls_mut()
    }
}

impl<T> Complete for DataTable<T> {
    type Props = Self;
    fn finish(self) -> Self {
        self
    }
}

/// The element that fills in the table once the window can create its entity.
type EntitySlot<T> = Signal<Option<Entity<TableEntity<Table<T>>>>>;

impl<T: Send + Sync + 'static> IntoElement for DataTable<T> {
    fn into_any_element(self) -> AnyElement {
        let Self { columns, data, extras, .. } = self;
        let Extras { base, handle, initial_state, state, filter_column, filter_placeholder, toolbar, view_options, pagination, page_sizes, callbacks } = extras;
        // hooks: the same calls in the same order on every render
        let own = use_table();
        let entity: EntitySlot<T> = use_signal(None);
        let handle = handle.unwrap_or(own);

        let mut opts = TableOptions::new(columns, data).initial_state(initial_state).state(state);
        opts.paginate = pagination;

        let mut root = base;
        if toolbar.unwrap_or(filter_column.is_some() || view_options) {
            let mut b = DataTableToolbar::__builder().table(handle).view_options(view_options);
            if let Some(c) = filter_column {
                b = b.filter_column(c.to_string());
            }
            if let Some(p) = filter_placeholder {
                b = b.placeholder(p);
            }
            root = root.child(build(b));
        }
        root = root.child(
            div().relative().flex_1().min_h(px(240.0)).overflow_hidden().defer(move |window, cx, el| {
                *el = fill(std::mem::replace(el, Div::new("div")), entity, handle, opts, callbacks, window, cx);
            }),
        );
        if pagination {
            let mut b = DataTablePagination::__builder().table(handle);
            if let Some(s) = page_sizes {
                b = b.page_sizes(s);
            }
            root = root.child(build(b));
        }
        root.into_any_element()
    }
}

/// The table region: the painted table (an entity made on first use) and the
/// open header menu over it.
fn fill<T: Send + Sync + 'static>(
    slot: Div,
    entity: EntitySlot<T>,
    handle: TableHandle,
    opts: TableOptions<T>,
    callbacks: Callbacks,
    window: &mut Window,
    cx: &mut App,
) -> Div {
    let state = match entity.get() {
        Some(state) => {
            let link = Link { state, handle, callbacks };
            return match link.set_options(opts, cx) {
                Ok(()) => table_region(slot, Rc::new(link)),
                Err(e) => error(slot, &e),
            };
        }
        None => match Table::new(opts) {
            Ok(table) => cx.new(|cx| TableEntity::new(table, window, cx).row_selectable(false).col_movable(false)),
            Err(e) => return error(slot, &e),
        },
    };
    entity.set(Some(state));
    let link = Link { state, handle, callbacks };
    link.settle(cx);
    table_region(slot, Rc::new(link))
}

fn error(slot: Div, e: &dyn std::fmt::Display) -> Div {
    slot.child(format!("DataTable: {e}"))
}

fn table_region<T: Send + Sync + 'static>(slot: Div, link: Rc<Link<T>>) -> Div {
    let (state, handle) = (link.state, link.handle);
    handle.ops.update_quiet(|o| *o = Some(Rc::clone(&link) as Rc<dyn Ops>));
    let listener = Rc::clone(&link);
    let table = TableElement::new(&state).on_event(move |e, w, cx| listener.on_event(e, w, cx));
    let menu = handle.menu.get().map(|m| {
        let close = move |_: &ClickEvent, _: &mut Window, _: &mut App| handle.menu.set(None);
        div().absolute().left(px(0.0)).top(px(0.0)).w_full().h_full().on_click(close).child(
            div().absolute().left(px(m.x)).top(px(m.y)).w(px(150.0)).z_index(20).child(build(DataTableColumnHeader::__builder().table(handle).column(m.column.to_string()))),
        )
    });
    slot.child(table).children(menu)
}
