//! [`TableModel`]: everything a table remembers (columns, row mask, sort,
//! selection, scroll, size) and the operations on it, with no framework in
//! sight. Adapters own a model next to their data, feed it input
//! ([`crate::interact`]) and size ([`TableModel::begin_frame`]), and draw it
//! ([`crate::paint`]).
//!
//! Where a framework must be called (a delegate hook, the clipboard), the
//! model queues a [`TableEvent`] and the adapter answers it after the call:
//! the model never calls out.

use std::ops::Range;

use crate::cell::CellSource;
use crate::column::{Column, ColumnGroup, ColumnLayout, ColumnSort};
use crate::event::{TableEvent, TableOptions, TableSelection, TableVisibleRange};
use crate::mask::RowMask;
use crate::selection::RowSelection;
use crate::sort::sorted_by_column;
use crate::visible::{clamp_scroll, scroll_to_show};

/// A pointer drag in progress.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Drag {
    None,
    /// Resizing view column `v` from its width when grabbed.
    Resize { v: usize, x: f32, w: f32 },
    /// A header pressed; becomes a column move after a few pixels.
    Header { v: usize, x: f32, moved: bool, to: usize },
    VThumb { grab: f32 },
    HThumb { grab: f32 },
}

/// What [`TableModel::begin_frame`] found changed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FrameReport {
    /// The visible display rows, when they changed.
    pub rows_changed: Option<Range<usize>>,
    /// The visible view columns, when they changed.
    pub cols_changed: Option<Range<usize>>,
    /// Rows below the last visible one (for "load more").
    pub rows_left: usize,
}

/// The table. See the module docs.
pub struct TableModel {
    pub(crate) layout: ColumnLayout,
    pub(crate) mask: RowMask,
    filter: RowMask,
    /// Rows in the data source.
    pub(crate) total: usize,
    /// Settings; change them freely between frames.
    pub opts: TableOptions,
    pub(crate) sel: RowSelection,
    pub(crate) sel_col: Option<usize>,
    pub(crate) sel_cell: Option<(usize, usize)>,
    pub(crate) right_clicked: Option<(usize, Option<usize>)>,
    pub(crate) sort: Option<(usize, ColumnSort)>,
    /// Scroll offset in pixels (x, y), `f64` so a billion rows scroll exactly.
    pub(crate) scroll: (f64, f64),
    /// Width and height.
    pub(crate) size: (f32, f32),
    pub(crate) unit: f32,
    pub(crate) scale: f32,
    pub(crate) hover: Option<usize>,
    pub(crate) drag: Drag,
    visible: TableVisibleRange,
    pub(crate) groups: Option<Vec<Vec<ColumnGroup>>>,
    pub(crate) loading: bool,
    pub(crate) events: Vec<TableEvent>,
}

impl Default for TableModel {
    fn default() -> Self {
        Self::new()
    }
}

impl TableModel {
    #[must_use]
    pub fn new() -> Self {
        Self {
            layout: ColumnLayout::default(),
            mask: RowMask::Identity,
            filter: RowMask::Identity,
            total: 0,
            opts: TableOptions::default(),
            sel: RowSelection::default(),
            sel_col: None,
            sel_cell: None,
            right_clicked: None,
            sort: None,
            scroll: (0.0, 0.0),
            size: (0.0, 0.0),
            unit: 20.0,
            scale: 1.0,
            hover: None,
            drag: Drag::None,
            visible: TableVisibleRange::default(),
            groups: None,
            loading: false,
            events: Vec::new(),
        }
    }

    // -- data -----------------------------------------------------------------------------------

    /// Replace the column list (the data changed): widths of columns with the
    /// same key survive, and so does their order and the sort indicator.
    pub fn set_columns(&mut self, cols: Vec<Column>, groups: Option<Vec<Vec<ColumnGroup>>>) {
        self.layout.reset(cols);
        if let Some((c, s)) = self.sort {
            self.layout.mark_sorted(c, s);
        }
        self.groups = groups;
    }

    pub const fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
    }

    /// Whether the loading skeleton is shown.
    #[must_use]
    pub const fn loading(&self) -> bool {
        self.loading
    }

    /// The data has `n` rows now. Returns whether that changed anything: the
    /// filter is dropped, the sort redone and the selection clamped.
    pub fn set_rows<S: CellSource + ?Sized>(&mut self, n: usize, src: &S) -> bool {
        if n == self.total {
            return false;
        }
        self.total = n;
        self.filter = RowMask::Identity;
        self.mask = RowMask::Identity;
        if let Some((c, s)) = self.sort {
            self.resort(src, c, s);
        }
        self.sel.clamp(self.rows());
        true
    }

    /// Rows shown (after filtering).
    #[must_use]
    pub const fn rows(&self) -> usize {
        self.mask.len(self.total)
    }

    /// The data row shown at display row `view_row`.
    #[must_use]
    pub fn row_at(&self, view_row: usize) -> usize {
        self.mask.get(view_row)
    }

    /// The display row showing data row `row` (linear when sorted).
    #[must_use]
    pub fn view_row_of(&self, row: usize) -> Option<usize> {
        self.mask.position_of(row, self.total)
    }

    /// Show only the rows `keep` accepts (an index mask: nothing is copied),
    /// keeping the current sort. Selection is cleared.
    pub fn set_filter<S: CellSource + ?Sized>(&mut self, src: &S, keep: impl Fn(usize) -> bool) {
        self.filter = RowMask::Identity.filtered(self.total, keep);
        self.mask = self.filter.clone();
        if let Some((c, s)) = self.sort {
            self.resort(src, c, s);
        }
        self.clear_selection_quiet();
        self.scroll.1 = 0.0;
    }

    /// Show every row again.
    pub fn clear_filter<S: CellSource + ?Sized>(&mut self, src: &S) {
        self.filter = RowMask::Identity;
        self.mask = RowMask::Identity;
        if let Some((c, s)) = self.sort {
            self.resort(src, c, s);
        }
        self.clear_selection_quiet();
    }

    /// The current sort (source column, sort).
    #[must_use]
    pub const fn sorted_by(&self) -> Option<(usize, ColumnSort)> {
        self.sort
    }

    fn resort<S: CellSource + ?Sized>(&mut self, src: &S, col: usize, sort: ColumnSort) {
        self.mask = if sort == ColumnSort::Default {
            self.filter.clone()
        } else {
            sorted_by_column(src, col, &self.filter, self.total, sort == ColumnSort::Descending)
        };
    }

    /// Sort by source column `col` (what a header click does). The mask is
    /// rebuilt unless `by_mask` is false (the data sorts itself, see the
    /// adapter's `perform_sort`); a [`TableEvent::Sorted`] is queued either way.
    pub fn sort_by<S: CellSource + ?Sized>(&mut self, src: &S, col: usize, sort: ColumnSort, by_mask: bool) {
        self.sort = (sort != ColumnSort::Default).then_some((col, sort));
        if by_mask {
            self.resort(src, col, sort);
        } else {
            self.mask = self.filter.clone();
        }
        self.clear_selection_quiet();
        self.events.push(TableEvent::Sorted(col, sort));
        // the sort indicator lives in the layout's column description
        self.layout.mark_sorted(col, sort);
    }

    // -- columns ----------------------------------------------------------------------------------

    /// The live column layout.
    #[must_use]
    pub const fn layout(&self) -> &ColumnLayout {
        &self.layout
    }

    /// The width of source column `col`.
    #[must_use]
    pub fn col_width(&self, col: usize) -> Option<f32> {
        self.layout.view_of(col).map(|v| self.layout.width(v))
    }

    /// Set the width of source column `col` (a fit). Returns whether it changed.
    pub fn set_col_width(&mut self, col: usize, width: f32) -> bool {
        let Some(v) = self.layout.view_of(col) else { return false };
        let changed = self.layout.set_width(v, width);
        if changed {
            self.widths_changed();
        }
        changed
    }

    pub(crate) fn widths_changed(&mut self) {
        self.events.push(TableEvent::ColumnWidthsChanged(self.layout.widths()));
        self.clamp_scroll();
    }

    /// Header names, in view order.
    #[must_use]
    pub fn headers(&self) -> Vec<String> {
        (0..self.layout.len()).map(|v| self.layout.column(v).name.to_string()).collect()
    }

    // -- scrolling --------------------------------------------------------------------------------

    pub(crate) fn clamp_scroll(&mut self) {
        let g = self.geo();
        self.scroll.1 = clamp_scroll(self.scroll.1, self.rows(), f64::from(g.row_h), f64::from(g.body_h));
        self.scroll.0 = if self.scroll.0.is_nan() {
            0.0
        } else {
            self.scroll.0.clamp(0.0, f64::from((self.layout.scroll_width() - g.view_w).max(0.0)))
        };
    }

    /// Scroll by pixels (positive: down / right). Returns whether it moved.
    pub fn scroll_by(&mut self, dx: f64, dy: f64) -> bool {
        let before = self.scroll;
        self.scroll.0 += dx;
        self.scroll.1 += dy;
        self.clamp_scroll();
        self.scroll != before
    }

    /// The scroll offset in pixels (x, y).
    #[must_use]
    pub const fn scroll_offset(&self) -> (f64, f64) {
        self.scroll
    }

    /// Scroll so display row `v` is in view (nothing until the table has a size).
    pub fn show_view_row(&mut self, v: usize) {
        let g = self.geo();
        if g.body_h <= 0.0 {
            return;
        }
        self.scroll.1 = scroll_to_show(self.scroll.1, v, f64::from(g.row_h), f64::from(g.body_h));
        self.clamp_scroll();
    }

    /// Scroll so view column `v` (not a pinned one) is in view.
    pub fn show_view_col(&mut self, v: usize) {
        let g = self.geo();
        let l = f64::from(self.layout.left(v));
        let w = f64::from(self.layout.width(v));
        let view = f64::from(g.view_w);
        self.scroll.0 = if l < self.scroll.0 {
            l
        } else if l + w > self.scroll.0 + view {
            l + w - view
        } else {
            self.scroll.0
        };
        self.clamp_scroll();
    }

    /// Scroll so source row `row` is in view. Returns whether the row exists.
    pub fn scroll_to_row(&mut self, row: usize) -> bool {
        self.view_row_of(row).map(|v| self.show_view_row(v)).is_some()
    }

    /// Scroll so source column `col` is in view. Returns whether it is a
    /// scrolling column.
    pub fn scroll_to_col(&mut self, col: usize) -> bool {
        match self.layout.view_of(col).filter(|&v| v >= self.layout.pinned()) {
            Some(v) => {
                self.show_view_col(v);
                true
            }
            None => false,
        }
    }

    /// Rows and columns in view now (display rows, view columns).
    #[must_use]
    pub const fn visible_range(&self) -> &TableVisibleRange {
        &self.visible
    }

    /// Record the table's size for this frame, keep the scroll in range and
    /// report what became visible. Call once per frame before painting.
    pub fn begin_frame(&mut self, width: f32, height: f32, unit: f32, scale: f32) -> FrameReport {
        self.size = (width, height);
        self.unit = unit;
        self.scale = scale;
        self.clamp_scroll();
        let g = self.geo();
        let vis = self.compute_visible(&g);
        let mut report = FrameReport { rows_left: self.rows().saturating_sub(vis.rows.end), ..FrameReport::default() };
        if vis != self.visible {
            if vis.rows != self.visible.rows {
                report.rows_changed = Some(vis.rows.clone());
            }
            if vis.cols != self.visible.cols {
                report.cols_changed = Some(vis.cols.clone());
            }
            self.visible = vis;
        }
        report
    }

    /// The table's size (width, height) as last given to [`begin_frame`](Self::begin_frame).
    #[must_use]
    pub const fn size(&self) -> (f32, f32) {
        self.size
    }

    /// The UI scale (device pixels per logical pixel) the table was last laid out with.
    #[must_use]
    pub const fn scale(&self) -> f32 {
        self.scale
    }

    /// The UI unit the table was last laid out with.
    #[must_use]
    pub const fn unit(&self) -> f32 {
        self.unit
    }

    // -- selection ----------------------------------------------------------------------------------

    #[must_use]
    pub fn selection(&self) -> TableSelection {
        if let Some((r, c)) = self.sel_cell {
            TableSelection::Cell(r, c)
        } else if let Some(c) = self.sel_col {
            TableSelection::Column(c)
        } else if let Some(r) = self.selected_row() {
            TableSelection::Row(r)
        } else {
            TableSelection::None
        }
    }

    /// Highlight exactly the display rows `rows` (a selection the owner of
    /// the data keeps, like a checkbox column's), without raising events.
    pub fn set_row_highlights(&mut self, rows: impl IntoIterator<Item = usize>) {
        self.sel.clear();
        for r in rows {
            self.sel.add(r..r + 1);
        }
        self.sel.clamp(self.rows());
    }

    /// Show `sorted` as the sort indicator (the owner of the data sorts).
    pub const fn set_sort_indicator(&mut self, sorted: Option<(usize, ColumnSort)>) {
        self.sort = sorted;
    }

    /// The active selected row (source index).
    #[must_use]
    pub fn selected_row(&self) -> Option<usize> {
        self.sel.active().filter(|&a| self.sel.contains(a)).map(|v| self.mask.get(v))
    }

    /// Every selected row (source indices, display order).
    pub fn selected_rows(&self) -> impl Iterator<Item = usize> + '_ {
        self.sel.iter().map(|v| self.mask.get(v))
    }

    /// The selected display-row ranges.
    #[must_use]
    pub fn selected_ranges(&self) -> &[Range<usize>] {
        self.sel.ranges()
    }

    #[must_use]
    pub const fn selected_col(&self) -> Option<usize> {
        self.sel_col
    }

    #[must_use]
    pub const fn selected_cell(&self) -> Option<(usize, usize)> {
        self.sel_cell
    }

    #[must_use]
    pub fn right_clicked_row(&self) -> Option<usize> {
        self.right_clicked.map(|(r, _)| r)
    }

    #[must_use]
    pub fn right_clicked_cell(&self) -> Option<(usize, usize)> {
        self.right_clicked.and_then(|(r, c)| c.map(|c| (r, c)))
    }

    pub fn set_right_clicked_row(&mut self, row: Option<usize>) {
        self.right_clicked = row.map(|r| (r, None));
    }

    /// Select only source row `row` and show it. Returns whether it exists.
    pub fn set_selected_row(&mut self, row: usize) -> bool {
        let Some(v) = self.view_row_of(row) else { return false };
        self.sel.select_only(v);
        self.sel_col = None;
        self.sel_cell = None;
        self.show_view_row(v);
        self.events.push(TableEvent::SelectRow(row));
        true
    }

    pub fn set_selected_col(&mut self, col: usize) {
        self.sel.clear();
        self.sel_cell = None;
        self.sel_col = Some(col);
        self.events.push(TableEvent::SelectColumn(col));
    }

    /// Select one cell and show its row. Returns whether the row exists.
    pub fn set_selected_cell(&mut self, row: usize, col: usize) -> bool {
        let Some(v) = self.view_row_of(row) else { return false };
        self.sel.select_only(v);
        self.sel_col = None;
        self.sel_cell = Some((row, col));
        self.show_view_row(v);
        self.events.push(TableEvent::SelectCell(row, col));
        true
    }

    /// Apply a [`TableSelection`]. Returns whether anything was selected or cleared.
    pub fn set_selection(&mut self, selection: TableSelection) -> bool {
        match selection {
            TableSelection::None => {
                self.clear_selection();
                true
            }
            TableSelection::Row(r) => self.set_selected_row(r),
            TableSelection::Column(c) => {
                self.set_selected_col(c);
                true
            }
            TableSelection::Cell(r, c) => self.set_selected_cell(r, c),
        }
    }

    pub fn clear_selection(&mut self) {
        self.clear_selection_quiet();
        self.events.push(TableEvent::ClearSelection);
    }

    pub(crate) fn clear_selection_quiet(&mut self) {
        self.sel.clear();
        self.sel_col = None;
        self.sel_cell = None;
    }

    /// Events raised since the last call.
    pub fn take_events(&mut self) -> Vec<TableEvent> {
        std::mem::take(&mut self.events)
    }

    /// Whether a pointer drag is in progress.
    #[must_use]
    pub fn dragging(&self) -> bool {
        self.drag != Drag::None
    }

    /// The drag in progress.
    #[must_use]
    pub const fn drag(&self) -> Drag {
        self.drag
    }

    /// The hovered display row.
    #[must_use]
    pub const fn hover(&self) -> Option<usize> {
        self.hover
    }

    // -- text -----------------------------------------------------------------------------------------

    /// Display rows `range` as text (headers aside).
    #[must_use]
    pub fn dump_range<S: CellSource + ?Sized>(&self, src: &S, range: Range<usize>) -> Vec<Vec<String>> {
        let mut buf = crate::fmt::CellBuf::new();
        range
            .filter(|&v| v < self.rows())
            .map(|v| {
                let r = self.mask.get(v);
                (0..self.layout.len()).map(|c| src.cell(r, self.layout.src(c)).text(&mut buf).to_owned()).collect()
            })
            .collect()
    }

    /// The whole table as text (headers, rows in display order).
    #[must_use]
    pub fn dump<S: CellSource + ?Sized>(&self, src: &S) -> (Vec<String>, Vec<Vec<String>>) {
        (self.headers(), self.dump_range(src, 0..self.rows()))
    }

    /// The selection as tab-separated text, one line per selected row in
    /// display order, columns in view order (what Ctrl+C copies). A selected
    /// cell copies that cell, a selected column that column. Tabs and line
    /// breaks inside a cell become spaces.
    #[must_use]
    pub fn selection_text<S: CellSource + ?Sized>(&self, src: &S) -> String {
        let mut out = String::new();
        let mut buf = crate::fmt::CellBuf::new();
        let push = |out: &mut String, s: &str| out.extend(s.chars().map(|c| if matches!(c, '\t' | '\n' | '\r') { ' ' } else { c }));
        if let Some((r, c)) = self.sel_cell {
            push(&mut out, src.cell(r, c).text(&mut buf));
        } else if let Some(c) = self.sel_col {
            for v in 0..self.rows() {
                push(&mut out, src.cell(self.mask.get(v), c).text(&mut buf));
                out.push('\n');
            }
        } else {
            for v in self.sel.iter() {
                let r = self.mask.get(v);
                for c in 0..self.layout.len() {
                    if c > 0 {
                        out.push('\t');
                    }
                    push(&mut out, src.cell(r, self.layout.src(c)).text(&mut buf));
                }
                out.push('\n');
            }
        }
        out
    }
}
