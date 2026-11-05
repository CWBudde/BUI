//! [`TableCore`]: a [`TableModel`] together with its data and the glue that
//! every framework adapter needs and would otherwise copy: calling the
//! delegate's hooks, answering the events the model queues, keeping the
//! model in step with the data, and the selection / scroll API that also has
//! to tell the framework to redraw.
//!
//! The framework enters through one trait, [`Host`], implemented once per
//! adapter on a marker type: its associated types name the framework's
//! window and context, and its functions forward to the framework's
//! delegate trait and to its notify / refresh calls. Everything else is
//! here, once.
//!
//! An adapter's `TableState` is a struct holding a `TableCore<ItsHost>` (and
//! what only it needs, like a focus handle) that implements `Deref` to it;
//! the core's methods then read as the state's own, and `TableCore` derefs
//! to the [`TableModel`] in turn.

use std::ops::{Deref, DerefMut, Range};

use crate::cell::CellSource;
use crate::column::{Column, ColumnGroup, ColumnSort};
use crate::event::{TableEvent, TableSelection};
use crate::geom::Point;
use crate::input::{Button, Key, Mods};
use crate::model::TableModel;

/// What a framework adapter provides: its types and how to reach its
/// delegate's hooks and redraw calls. Reads take the context immutably,
/// hooks take the window and context mutably.
pub trait Host: Sized {
    type Delegate: CellSource;
    /// The framework's window.
    type Window;
    /// The context of the table's entity (it can notify and read the app).
    type Cx<'a>;

    fn columns(delegate: &Self::Delegate, cx: &Self::Cx<'_>) -> Vec<Column>;
    fn group_headers(delegate: &Self::Delegate, cx: &Self::Cx<'_>) -> Option<Vec<Vec<ColumnGroup>>>;
    fn rows_count(delegate: &Self::Delegate, cx: &Self::Cx<'_>) -> usize;
    fn loading(delegate: &Self::Delegate, cx: &Self::Cx<'_>) -> bool;
    /// Whether a header click sorts the row mask (false: the delegate sorts its data).
    fn sort_by_mask(delegate: &Self::Delegate) -> bool;
    fn perform_sort(delegate: &mut Self::Delegate, col: usize, sort: ColumnSort, window: &mut Self::Window, cx: &mut Self::Cx<'_>);
    fn move_column(delegate: &mut Self::Delegate, col: usize, to: usize, window: &mut Self::Window, cx: &mut Self::Cx<'_>);
    /// The width column `col` needs to show its content whole.
    fn fit_column(delegate: &mut Self::Delegate, col: usize, window: &mut Self::Window, cx: &mut Self::Cx<'_>) -> Option<f32>;
    fn visible_rows_changed(delegate: &mut Self::Delegate, rows: Range<usize>, window: &mut Self::Window, cx: &mut Self::Cx<'_>);
    fn visible_columns_changed(delegate: &mut Self::Delegate, cols: Range<usize>, window: &mut Self::Window, cx: &mut Self::Cx<'_>);
    fn has_more(delegate: &Self::Delegate, cx: &Self::Cx<'_>) -> bool;
    fn load_more_threshold(delegate: &Self::Delegate) -> usize;
    fn load_more(delegate: &mut Self::Delegate, window: &mut Self::Window, cx: &mut Self::Cx<'_>);
    fn write_clipboard(window: &mut Self::Window, cx: &mut Self::Cx<'_>, text: &str);
    /// Ask for a redraw from state changes made outside an input handler.
    fn notify(cx: &mut Self::Cx<'_>);
    /// Ask the window to redraw (the table paints from its state, so this is
    /// all a scroll needs).
    fn refresh(window: &mut Self::Window);
}

/// See the module docs.
pub struct TableCore<H: Host> {
    model: TableModel,
    delegate: H::Delegate,
    /// Events for the element's listeners (what the model raised, minus the
    /// ones that only ask the adapter to act).
    out: Vec<TableEvent>,
}

impl<H: Host> Deref for TableCore<H> {
    type Target = TableModel;
    fn deref(&self) -> &TableModel {
        &self.model
    }
}

impl<H: Host> DerefMut for TableCore<H> {
    fn deref_mut(&mut self) -> &mut TableModel {
        &mut self.model
    }
}

impl<H: Host> TableCore<H> {
    pub fn new(delegate: H::Delegate, cx: &H::Cx<'_>) -> Self {
        let mut s = Self { model: TableModel::new(), delegate, out: Vec::new() };
        s.reload(cx);
        s
    }

    // -- data ----------------------------------------------------------------------------------
    pub const fn delegate(&self) -> &H::Delegate {
        &self.delegate
    }

    pub const fn delegate_mut(&mut self) -> &mut H::Delegate {
        &mut self.delegate
    }

    /// Re-read the delegate's columns and row count (call after changing
    /// them). Keeps widths, order, scroll and sort where the columns still
    /// exist; the row filter is dropped when the row count changed.
    pub fn refresh(&mut self, cx: &mut H::Cx<'_>) {
        self.reload(cx);
        H::notify(cx);
    }

    fn reload(&mut self, cx: &H::Cx<'_>) {
        self.model.set_columns(H::columns(&self.delegate, cx), H::group_headers(&self.delegate, cx));
        self.model.set_loading(H::loading(&self.delegate, cx));
        self.model.set_rows(H::rows_count(&self.delegate, cx), &self.delegate);
    }

    /// Show only the rows `keep` accepts (an index mask: nothing is copied),
    /// keeping the current sort. Selection is cleared.
    pub fn set_filter(&mut self, keep: impl Fn(&H::Delegate, usize) -> bool, cx: &mut H::Cx<'_>) {
        let d = &self.delegate;
        self.model.set_filter(d, |r| keep(d, r));
        H::notify(cx);
    }

    /// Show every row again.
    pub fn clear_filter(&mut self, cx: &mut H::Cx<'_>) {
        self.model.clear_filter(&self.delegate);
        H::notify(cx);
    }

    /// Sort by source column `col` (what a header click does).
    pub fn sort_by(&mut self, col: usize, sort: ColumnSort, window: &mut H::Window, cx: &mut H::Cx<'_>) {
        self.model.sort_by(&self.delegate, col, sort, H::sort_by_mask(&self.delegate));
        self.answer_events(window, cx);
        H::notify(cx);
    }

    /// Answer what the model asked for (delegate hooks, the clipboard) and
    /// queue the rest for the element's listeners.
    fn answer_events(&mut self, window: &mut H::Window, cx: &mut H::Cx<'_>) {
        for e in self.model.take_events() {
            match &e {
                TableEvent::Sorted(col, sort) => H::perform_sort(&mut self.delegate, *col, *sort, window, cx),
                TableEvent::MoveColumn(_, to) => {
                    let src = self.model.layout().src(*to);
                    H::move_column(&mut self.delegate, src, *to, window, cx);
                }
                TableEvent::FitColumn(col) => {
                    self.fit_col(*col, window, cx);
                    continue;
                }
                TableEvent::Copied(text) => H::write_clipboard(window, cx, text),
                _ => {}
            }
            self.out.push(e);
        }
    }

    /// Events raised since the last call (the element hands them to its
    /// listeners; programmatic changes are collected here too).
    pub fn take_events(&mut self) -> Vec<TableEvent> {
        let mut v = std::mem::take(&mut self.out);
        v.extend(self.model.take_events());
        v
    }

    // -- frame -----------------------------------------------------------------------------------
    /// Called while rendering with the table's size: picks up data changes,
    /// keeps the scroll in range, tells the delegate what became visible and
    /// asks for more rows near the bottom.
    pub fn sync(&mut self, width: f32, height: f32, unit: f32, scale: f32, window: &mut H::Window, cx: &mut H::Cx<'_>) {
        self.model.set_loading(H::loading(&self.delegate, cx));
        if self.model.set_rows(H::rows_count(&self.delegate, cx), &self.delegate) {
            self.reload(cx);
        }
        let report = self.model.begin_frame(width, height, unit, scale);
        let changed = report.rows_changed.is_some() || report.cols_changed.is_some();
        if let Some(rows) = report.rows_changed {
            H::visible_rows_changed(&mut self.delegate, rows, window, cx);
        }
        if let Some(cols) = report.cols_changed {
            H::visible_columns_changed(&mut self.delegate, cols, window, cx);
        }
        let near_end = report.rows_left <= H::load_more_threshold(&self.delegate);
        if changed && near_end && H::has_more(&self.delegate, cx) && !self.model.loading() {
            H::load_more(&mut self.delegate, window, cx);
        }
    }

    // -- scrolling and selection (gpui-component's names) ---------------------------------------------
    /// Scroll so source row `row_ix` is in view.
    pub fn scroll_to_row(&mut self, row_ix: usize, cx: &mut H::Cx<'_>) {
        if self.model.scroll_to_row(row_ix) {
            H::notify(cx);
        }
    }

    /// Scroll so source column `col_ix` is in view.
    pub fn scroll_to_col(&mut self, col_ix: usize, cx: &mut H::Cx<'_>) {
        if self.model.scroll_to_col(col_ix) {
            H::notify(cx);
        }
    }

    pub fn set_right_clicked_row(&mut self, row: Option<usize>, cx: &mut H::Cx<'_>) {
        self.model.set_right_clicked_row(row);
        H::notify(cx);
    }

    pub fn set_selected_row(&mut self, row_ix: usize, cx: &mut H::Cx<'_>) {
        if self.model.set_selected_row(row_ix) {
            H::notify(cx);
        }
    }

    pub fn set_selected_col(&mut self, col_ix: usize, cx: &mut H::Cx<'_>) {
        self.model.set_selected_col(col_ix);
        H::notify(cx);
    }

    pub fn set_selected_cell(&mut self, row_ix: usize, col_ix: usize, cx: &mut H::Cx<'_>) {
        if self.model.set_selected_cell(row_ix, col_ix) {
            H::notify(cx);
        }
    }

    pub fn set_selection(&mut self, selection: TableSelection, cx: &mut H::Cx<'_>) {
        if self.model.set_selection(selection) {
            H::notify(cx);
        }
    }

    pub fn clear_selection(&mut self, cx: &mut H::Cx<'_>) {
        self.model.clear_selection();
        H::notify(cx);
    }

    /// Fit a column to its content through the delegate (double click on its
    /// border). Returns whether the width changed.
    pub fn fit_col(&mut self, col_ix: usize, window: &mut H::Window, cx: &mut H::Cx<'_>) -> bool {
        let Some(w) = H::fit_column(&mut self.delegate, col_ix, window, cx) else { return false };
        let changed = self.model.set_col_width(col_ix, w);
        if changed {
            H::notify(cx);
        }
        changed
    }

    // -- text ------------------------------------------------------------------------------------------
    /// The whole table as text (headers, rows in display order).
    pub fn dump(&self) -> (Vec<String>, Vec<Vec<String>>) {
        self.model.dump(&self.delegate)
    }

    /// Display rows `range` as text, with the headers.
    pub fn dump_range(&self, range: Range<usize>) -> (Vec<String>, Vec<Vec<String>>) {
        (self.model.headers(), self.model.dump_range(&self.delegate, range))
    }

    /// The selection as tab-separated text (what Ctrl+C copies).
    pub fn selection_text(&self) -> String {
        self.model.selection_text(&self.delegate)
    }

    /// State changed by input: re-render the view and repaint.
    fn redraw(window: &mut H::Window, cx: &mut H::Cx<'_>) {
        H::notify(cx);
        H::refresh(window);
    }

    // -- input (positions relative to the table) ----------------------------------------------------------
    /// Pointer pressed; `clicks` is the click count.
    pub fn pointer_down(&mut self, p: Point, button: Button, mods: Mods, clicks: usize, window: &mut H::Window, cx: &mut H::Cx<'_>) {
        self.model.pointer_down(p, button, mods, clicks);
        self.answer_events(window, cx);
        Self::redraw(window, cx);
    }

    /// Pointer moved. Returns whether the table redrew.
    pub fn pointer_move(&mut self, p: Point, window: &mut H::Window, cx: &mut H::Cx<'_>) -> bool {
        let redraw = self.model.pointer_move(p);
        if redraw {
            self.answer_events(window, cx);
            Self::redraw(window, cx);
        }
        redraw
    }

    /// Pointer released: finishes drags, and a header press that didn't
    /// move is a click (sort, or select the column).
    pub fn pointer_up(&mut self, window: &mut H::Window, cx: &mut H::Cx<'_>) {
        self.model.pointer_up(&self.delegate, H::sort_by_mask(&self.delegate));
        self.answer_events(window, cx);
        Self::redraw(window, cx);
    }

    /// Wheel: `lines` > 0 scrolls up. Shift scrolls sideways. Returns whether it moved.
    pub fn wheel(&mut self, lines: f32, shift: bool, window: &mut H::Window) -> bool {
        let px = self.model.wheel_pixels(lines);
        if shift { self.scroll_by_px(px, 0.0, window) } else { self.scroll_by_px(0.0, px, window) }
    }

    /// Scroll by pixels (positive: down / right; a touchpad's deltas). Returns whether it moved.
    pub fn scroll_by_px(&mut self, dx: f64, dy: f64, window: &mut H::Window) -> bool {
        let moved = self.model.scroll_by(dx, dy);
        if moved {
            H::refresh(window);
        }
        moved
    }

    /// A key while the table has focus. Returns whether it was handled.
    pub fn key_down(&mut self, key: Key, mods: Mods, window: &mut H::Window, cx: &mut H::Cx<'_>) -> bool {
        let handled = self.model.key_down(key, mods, &self.delegate);
        if handled {
            self.answer_events(window, cx);
            Self::redraw(window, cx);
        }
        handled
    }
}
