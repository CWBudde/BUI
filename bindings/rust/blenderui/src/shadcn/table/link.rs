//! The link between a table entity, its [`TableHandle`] and the callbacks:
//! applies commands, keeps the view and the snapshot in step with the
//! table, and reports changes (`onSortingChange` and friends).

use blenderui_table::tanstack::{Change, PaginationState, RowSelectionState, SortingState, Table, TableError, TableOptions};
use blenderui_table::{ColumnSort, TableEvent};

use super::handle::{Command, HeaderMenu, Ops, Snapshot, TableHandle};
use crate::table::TableState;
use crate::{App, Entity, Window};

type Callback<T> = Option<Box<dyn Fn(&T)>>;

/// `onXChange` callbacks, and the row-action handler.
#[derive(Default)]
pub(crate) struct Callbacks {
    pub sorting: Callback<SortingState>,
    pub column_filters: Callback<Vec<blenderui_table::tanstack::ColumnFilter>>,
    pub global_filter: Callback<String>,
    pub column_visibility: Callback<blenderui_table::tanstack::VisibilityState>,
    pub row_selection: Callback<RowSelectionState>,
    pub pagination: Callback<PaginationState>,
    /// The "more" button of a row (the index of the row in the data).
    pub row_action: Option<Box<dyn Fn(usize, &mut Window, &mut App)>>,
}

impl Callbacks {
    fn fire(&self, change: &Change) {
        match change {
            Change::Sorting(v) => self.sorting.iter().for_each(|f| f(v)),
            Change::ColumnFilters(v) => self.column_filters.iter().for_each(|f| f(v)),
            Change::GlobalFilter(v) => self.global_filter.iter().for_each(|f| f(v)),
            Change::ColumnVisibility(v) => self.column_visibility.iter().for_each(|f| f(v)),
            Change::RowSelection(v) => self.row_selection.iter().for_each(|f| f(v)),
            Change::Pagination(v) => self.pagination.iter().for_each(|f| f(v)),
        }
    }
}

/// One table entity with everything that drives it.
pub(crate) struct Link<T: Send + Sync + 'static> {
    pub state: Entity<TableState<Table<T>>>,
    pub handle: TableHandle,
    pub callbacks: Callbacks,
}

impl<T: Send + Sync + 'static> Link<T> {
    /// Hand new options to the table (a parent re-rendered).
    pub fn set_options(&self, opts: TableOptions<T>, cx: &mut App) -> Result<(), TableError> {
        let changed = self.state.update(cx, |s, _| s.delegate_mut().set_options(opts))?;
        if changed {
            self.settle(cx);
        }
        Ok(())
    }

    /// The table changed (or may have): refresh the engine's view of it,
    /// publish the snapshot, report the changes. Always from outside any
    /// borrow of the table.
    pub fn settle(&self, cx: &mut App) {
        let (changes, snap) = self.state.update(cx, |s, c| {
            let t = s.delegate_mut();
            let (changes, snap) = (t.take_changes(), Snapshot::of(t));
            let dirty = t.take_dirty();
            let sorted = t.sorting().first().and_then(|r| t.column_index(&r.id).map(|i| (i, if r.desc { ColumnSort::Descending } else { ColumnSort::Ascending })));
            let highlights: Vec<usize> = if dirty {
                (0..t.row_model().len()).filter(|&r| t.data_row(r).is_some_and(|d| t.is_row_selected(d))).collect()
            } else {
                Vec::new()
            };
            // the engine keeps its own sort indicator when a header is clicked: ours is the truth
            s.set_sort_indicator(sorted);
            s.refresh(c);
            if dirty {
                s.set_row_highlights(highlights);
            }
            (changes, snap)
        });
        if self.handle.snap.with(|old| *old != snap) {
            self.handle.snap.set(snap);
        }
        for change in &changes {
            self.callbacks.fire(change);
        }
    }

    /// A table event (after an input the table handled).
    pub fn on_event(&self, e: &TableEvent, window: &mut Window, cx: &mut App) {
        match e {
            TableEvent::CheckRow(row, _) => self.run(Command::TogglePageRow(*row), cx),
            TableEvent::CheckAll(_) => self.run(Command::TogglePage, cx),
            TableEvent::CellAction(row, _) => {
                if let Some(f) = &self.callbacks.row_action
                    && let Some(data_row) = self.state.read(cx).delegate().data_row(*row)
                {
                    f(data_row, window, cx);
                }
            }
            TableEvent::HeaderMenu(col) => self.open_menu(*col, cx),
            TableEvent::Sorted(..) => self.settle(cx),
            _ => {}
        }
        window.refresh();
    }

    fn open_menu(&self, col: usize, cx: &mut App) {
        let spot = {
            let s = self.state.read(cx);
            let (g, id) = (s.geo(), s.delegate().column_defs().get(col).map(|c| c.id.clone()));
            s.layout().view_of(col).zip(id).map(|(v, id)| HeaderMenu { column: id, x: s.col_x(&g, v), y: g.groups_h + g.header_h })
        };
        self.handle.menu.set(spot);
    }
}

impl<T: Send + Sync + 'static> Ops for Link<T> {
    fn run(&self, cmd: Command, cx: &mut App) {
        self.state.update(cx, |s, _| cmd.apply(s.delegate_mut()));
        self.settle(cx);
    }
}
