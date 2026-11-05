//! [`TableState`]: the entity around a [`TableCore`], like gpui-component's
//! `TableState`. The core (in the `blenderui-table` crate) holds the model
//! (scroll, widths, order, sort, selection ...), the delegate, and the glue
//! shared by every adapter; what is here is what needs gpui: the [`Host`]
//! that reaches the delegate's hooks, the window and the clipboard, the focus
//! handle, and the shaped-line cache.
//!
//! The core's API is reachable on the state through `Deref`
//! (`state.rows()`, `state.selected_row()`, `state.set_selected_row(r, cx)`,
//! `state.sort_by(..)` ...).

use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Range};

use blenderui_table::{table_option_builders, Column, ColumnGroup, ColumnSort, Host, TableCore, TextCache};
use gpui::{px, App, ClipboardItem, Context, FocusHandle, Focusable, Pixels, ShapedLine, Window};

use crate::delegate::TableDelegate;

/// Reaches the delegate trait, the window and the entity context.
pub struct GpuiHost<D>(PhantomData<fn() -> D>);

impl<D: TableDelegate> Host for GpuiHost<D> {
    type Delegate = D;
    type Window = Window;
    type Cx<'a> = Context<'a, TableState<D>>;

    fn columns(d: &D, cx: &Self::Cx<'_>) -> Vec<Column> {
        let app: &App = cx;
        (0..d.columns_count(app)).map(|i| d.column(i, app)).collect()
    }
    fn group_headers(d: &D, cx: &Self::Cx<'_>) -> Option<Vec<Vec<ColumnGroup>>> {
        d.group_headers(cx)
    }
    fn rows_count(d: &D, cx: &Self::Cx<'_>) -> usize {
        d.rows_count(cx)
    }
    fn loading(d: &D, cx: &Self::Cx<'_>) -> bool {
        d.loading(cx)
    }
    fn sort_by_mask(d: &D) -> bool {
        d.sort_by_mask()
    }
    fn perform_sort(d: &mut D, col: usize, sort: ColumnSort, window: &mut Window, cx: &mut Self::Cx<'_>) {
        d.perform_sort(col, sort, window, cx);
    }
    fn move_column(d: &mut D, col: usize, to: usize, window: &mut Window, cx: &mut Self::Cx<'_>) {
        d.move_column(col, to, window, cx);
    }
    fn fit_column(d: &mut D, col: usize, window: &mut Window, cx: &mut Self::Cx<'_>) -> Option<f32> {
        d.fit_column(col, window, cx).map(f32::from)
    }
    fn visible_rows_changed(d: &mut D, rows: Range<usize>, window: &mut Window, cx: &mut Self::Cx<'_>) {
        d.visible_rows_changed(rows, window, cx);
    }
    fn visible_columns_changed(d: &mut D, cols: Range<usize>, window: &mut Window, cx: &mut Self::Cx<'_>) {
        d.visible_columns_changed(cols, window, cx);
    }
    fn has_more(d: &D, cx: &Self::Cx<'_>) -> bool {
        d.has_more(cx)
    }
    fn load_more_threshold(d: &D) -> usize {
        d.load_more_threshold()
    }
    fn load_more(d: &mut D, window: &mut Window, cx: &mut Self::Cx<'_>) {
        d.load_more(window, cx);
    }
    fn write_clipboard(_: &mut Window, cx: &mut Self::Cx<'_>, text: &str) {
        cx.write_to_clipboard(ClipboardItem::new_string(text.to_owned()));
    }
    fn notify(cx: &mut Self::Cx<'_>) {
        cx.notify();
    }
    fn refresh(window: &mut Window) {
        window.refresh();
    }
}

/// How many shaped text runs the cache keeps at least (it grows to twice the
/// visible cells).
const TEXT_CACHE_CAPACITY: usize = 2048;

/// The table.
pub struct TableState<D: TableDelegate> {
    pub(crate) core: TableCore<GpuiHost<D>>,
    focus: FocusHandle,
    /// Shaped text runs kept across frames: scrolling moves the same strings.
    pub(crate) text: TextCache<ShapedLine>,
    /// The scale `text` was shaped at (it is dropped when this changes).
    pub(crate) text_scale: f32,
    /// Whether the pointer was last over a column border (for the cursor).
    pub(crate) over_border: bool,
}

impl<D: TableDelegate> Deref for TableState<D> {
    type Target = TableCore<GpuiHost<D>>;
    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

impl<D: TableDelegate> DerefMut for TableState<D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.core
    }
}

impl<D: TableDelegate> Focusable for TableState<D> {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl<D: TableDelegate> TableState<D> {
    pub fn new(delegate: D, _: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            core: TableCore::new(delegate, cx),
            focus: cx.focus_handle(),
            text: TextCache::new(TEXT_CACHE_CAPACITY),
            text_scale: 1.0,
            over_border: false,
        }
    }

    // gpui-component's builders
    table_option_builders!();

    /// The width of delegate column `col_ix`.
    pub fn col_width(&self, col_ix: usize) -> Option<Pixels> {
        self.core.col_width(col_ix).map(px)
    }
}
