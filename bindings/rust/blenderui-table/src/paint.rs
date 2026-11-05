//! Painting a table: a port of Blender's spreadsheet drawing
//! (`spreadsheet_draw.cc`) onto an abstract [`PaintSink`]. Only the visible
//! cells are touched, straight into the sink: no element per cell, no
//! allocation once the scratch strings have grown.
//!
//! * `draw_alternating_row_overlay`: [`draw_alternating_row_pairs`]
//! * header: `draw_top_row_bg` + `draw_column_headers` (sticky: drawn outside
//!   the scrolled body, clipped to its own strip)
//! * index column: `draw_left_column_bg` + `draw_left_column_content`
//! * cells: `draw_spreadsheet_cells`, with `get_visible_rows` and the
//!   `right_x >= left && left_x <= region_w` column culling
//!   ([`visible_rows`], [`visible_cols`])
//! * lines: `draw_separator_lines`
//! * column moving: `draw_column_move_overlay` (insert indicator)
//!
//! Blender's `UI_widgetbase_draw_cache` batches widget quads into instanced
//! draws; here the batching is in what the table emits: a selection of any
//! number of adjacent rows is **one** quad, a column's separator is one quad,
//! and a cell without a background emits none. Text goes through the sink,
//! which owns the glyph/shape cache ([`crate::TextCache`]).

use std::cell::Cell as StdCell;
use std::ops::Range;

use crate::cell::{Cell, CellSource, Check};
use crate::column::{BadgeVariant, CellRender, Column, ColumnSort, TextAlign};
use crate::fmt::CellBuf;
use crate::geom::{Rect, Rgba};
pub use blenderui_paint::{PaintSink, Palette};
use crate::geometry::Geometry;
use crate::model::{Drag, TableModel};
use crate::visible::{visible_cols, visible_rows};

/// Scratch strings reused every frame (cell formatting, truncation).
#[derive(Default)]
pub struct PaintScratch {
    cell: CellBuf,
    ell: String,
}

thread_local! {
    static SCRATCH: StdCell<Option<PaintScratch>> = const { StdCell::new(None) };
}

/// Run `f` with this thread's scratch (taken and put back, so painting can
/// nest no deeper than one table; there is no borrow to fail).
pub fn with_scratch<R>(f: impl FnOnce(&mut PaintScratch) -> R) -> R {
    let mut s = SCRATCH.take().unwrap_or_default();
    let r = f(&mut s);
    SCRATCH.set(Some(s));
    r
}

/// `s` cut with a trailing ellipsis to fit `avail` px (Blender clips cell
/// text with `UI_text_clip_middle`/`ui_text_clip_right_label`). Cells that
/// obviously fit (by an upper bound on their width) are not measured.
fn fit_text<'a>(s: &'a str, avail: f32, size: f32, sink: &mut dyn PaintSink, out: &'a mut String) -> &'a str {
    if s.len() as f32 * size * 0.4 <= avail * 0.8 {
        return s;
    }
    if s.len() as f32 * size * 1.2 < avail || sink.text_width(s, size) <= avail {
        return s;
    }
    // longest prefix whose width plus "..." fits (bisection on char boundaries)
    let ends = |k: usize| s.char_indices().nth(k).map_or(s.len(), |(i, _)| i);
    let (mut lo, mut hi) = (0usize, s.chars().count());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        out.clear();
        out.push_str(&s[..ends(mid)]);
        out.push('\u{2026}');
        if sink.text_width(out, size) <= avail {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    out.clear();
    out.push_str(&s[..ends(lo)]);
    out.push('\u{2026}');
    out
}

/// Blender's `draw_alternating_row_overlay`: one pass over the visible row
/// pairs, tinting the second row of each pair.
pub fn draw_alternating_row_pairs(sink: &mut dyn PaintSink, rows: &Range<usize>, row_rect: impl Fn(usize) -> Rect, color: Rgba) {
    let mut r = rows.start | 1;
    if r < rows.start {
        r += 2;
    }
    while r < rows.end {
        sink.fill(row_rect(r), color);
        r += 2;
    }
}

/// A chevron (two lines) of half-width `k` pointing up or down.
fn chevron(sink: &mut dyn PaintSink, cx: f32, cy: f32, k: f32, up: bool, color: Rgba) {
    let d = if up { -k } else { k };
    sink.line((cx - k, cy - d / 2.0), (cx, cy + d / 2.0), color, 1.5);
    sink.line((cx, cy + d / 2.0), (cx + k, cy - d / 2.0), color, 1.5);
}

/// The x of content in a column cell, by the column's alignment. The width of
/// the content is only asked for when the alignment needs it: measuring text
/// is the expensive part, and a left-aligned cell does not need it.
fn aligned(col: &Column, x: f32, w: f32, pad: f32, content_w: impl FnOnce() -> f32) -> f32 {
    match col.align {
        TextAlign::Right => x + w - pad - content_w(),
        TextAlign::Center => x + (w - content_w()) / 2.0,
        TextAlign::Left => x + pad,
    }
}

/// Everything one paint call needs besides the sink.
struct Frame<'a, S: CellSource + ?Sized> {
    t: &'a TableModel,
    src: &'a S,
    g: Geometry,
    pal: &'a Palette,
    size: f32,
    hair: f32,
}

/// Paint `t` (its data from `src`) into `sink`, in the table's own size.
pub fn paint_table<S: CellSource + ?Sized>(t: &TableModel, src: &S, pal: &Palette, empty: &str, scratch: &mut PaintScratch, sink: &mut dyn PaintSink) {
    let g = t.geo();
    let f = Frame { t, src, g, pal, size: t.opts.text_size.unwrap_or_else(|| (g.unit * 0.6).round().max(8.0)), hair: 1.0 / t.scale.max(1.0) };
    let n = t.rows();
    let rows = visible_rows(t.scroll.1, f64::from(g.row_h), f64::from(g.body_h), n);
    sink.fill(Rect::new(0.0, 0.0, g.w, g.h), pal.back);

    // ---- body: stripes, selection, cells ----
    let body = Rect::new(0.0, g.top, g.w, g.body_h);
    sink.clipped(body, &mut |sink| {
        if t.loading {
            f.draw_loading(sink);
        } else if n == 0 {
            let w = sink.text_width(empty, f.size);
            sink.text((g.w - w) / 2.0, g.top + g.body_h * 0.35 + f.size * 0.5, empty, f.size, pal.dim);
        } else {
            f.draw_rows_bg(sink, &rows, 0.0, g.w);
            f.draw_scrolled_cells(sink, &rows, scratch);
            if g.pinned_w > 0.0 {
                // sticky columns: an opaque strip over the scrolled cells
                let strip = Rect::new(g.index_w, g.top, g.pinned_w, g.body_h);
                sink.clipped(strip, &mut |sink| {
                    sink.fill(strip, pal.back);
                    f.draw_rows_bg(sink, &rows, g.index_w, g.pinned_w);
                    for v in 0..t.layout.pinned() {
                        f.draw_column_cells(sink, &rows, v, f.t.col_x(&g, v), scratch);
                        f.vline(sink, f.t.col_x(&g, v) + t.layout.width(v), g.top, g.body_h);
                    }
                });
                if t.opts.fixed_cols_border {
                    sink.fill(Rect::new(g.x0 - f.hair, g.top, f.hair, g.body_h), pal.line);
                }
            }
            if t.opts.row_header {
                f.draw_index_column(sink, &rows, scratch);
            }
        }
    });

    // ---- headers (outside the scrolled body: sticky) ----
    f.draw_headers(sink, scratch);
    f.draw_scrollbars(sink);
    if t.opts.bordered {
        sink.stroke(Rect::new(0.0, 0.0, g.w, g.h), pal.line, f.hair.max(1.0));
    }
}

impl<S: CellSource + ?Sized> Frame<'_, S> {
    /// A one-pixel vertical separator ending at `right`.
    fn vline(&self, sink: &mut dyn PaintSink, right: f32, y: f32, h: f32) {
        sink.fill(Rect::new(right - self.hair, y, self.hair, h), self.pal.line);
    }

    fn pad(&self) -> f32 {
        (self.g.unit * 0.3).round()
    }

    fn draw_loading(&self, sink: &mut dyn PaintSink) {
        let g = &self.g;
        let n = (g.body_h / g.row_h).ceil() as usize;
        let cols = visible_cols(self.t.layout.scroll_edges(), 0.0, g.view_w);
        for r in 0..n {
            let y = g.top + r as f32 * g.row_h + g.row_h * 0.3;
            for c in cols.clone() {
                let x = g.x0 + self.t.layout.scroll_edges()[c];
                let w = self.t.layout.width(c + self.t.layout.pinned()) * 0.6;
                sink.round(Rect::new(x + g.unit * 0.3, y, w, g.row_h * 0.4), 3.0, self.pal.text.opacity(0.1));
            }
        }
    }

    /// Blender's `draw_alternating_row_overlay` (pairs of rows: the second row
    /// of each pair is tinted), then selection (one quad per run of selected
    /// rows) and hover, over `x..x+w`.
    fn draw_rows_bg(&self, sink: &mut dyn PaintSink, rows: &Range<usize>, x: f32, w: f32) {
        let (g, t) = (&self.g, self.t);
        if t.opts.stripe {
            draw_alternating_row_pairs(sink, rows, |r| Rect::new(x, t.row_top(g, r), w, g.row_h), self.pal.alt);
        }
        let ranges = t.sel.ranges();
        let first = ranges.partition_point(|r| r.end <= rows.start);
        for run in &ranges[first..] {
            if run.start >= rows.end {
                break;
            }
            let (lo, hi) = (run.start.max(rows.start), run.end.min(rows.end));
            let top = t.row_top(g, lo);
            sink.fill(Rect::new(x, top, w, (hi - lo) as f32 * g.row_h), self.pal.sel);
        }
        if let Some(h) = t.hover.filter(|h| rows.contains(h) && !t.sel.contains(*h)) {
            sink.fill(Rect::new(x, t.row_top(g, h), w, g.row_h), self.pal.hover);
        }
    }

    fn draw_scrolled_cells(&self, sink: &mut dyn PaintSink, rows: &Range<usize>, scratch: &mut PaintScratch) {
        let (g, t) = (&self.g, self.t);
        let sx = t.scroll.0 as f32;
        sink.clipped(Rect::new(g.x0, g.top, g.view_w, g.body_h), &mut |sink| {
            for c in visible_cols(t.layout.scroll_edges(), sx, sx + g.view_w) {
                let v = c + t.layout.pinned();
                let x = t.col_x(g, v);
                self.draw_column_cells(sink, rows, v, x, scratch);
                self.vline(sink, x + t.layout.width(v), g.top, g.body_h);
            }
        });
    }

    /// One column's visible cells (Blender's inner loop of
    /// `draw_spreadsheet_cells`: rows `first_row .. first_row + max_visible`).
    fn draw_column_cells(&self, sink: &mut dyn PaintSink, rows: &Range<usize>, v: usize, x: f32, scratch: &mut PaintScratch) {
        let (g, t) = (&self.g, self.t);
        let col = t.layout.column(v);
        let source = t.layout.src(v);
        let w = t.layout.width(v);
        let pad = self.pad();
        let avail = (w - 2.0 * pad).max(0.0);
        let selected_cell = t.sel_cell.filter(|&(_, c)| c == source);
        for r in rows.clone() {
            let PaintScratch { cell: buf, ell } = &mut *scratch;
            let top = t.row_top(g, r);
            let row = t.mask.get(r);
            let cell = self.src.cell(row, source);
            let at = Rect::new(x, top, w, g.row_h);
            match col.render {
                CellRender::Text => {
                    let text = cell.text_fmt(buf, &col.format);
                    if !text.is_empty() {
                        let shown = fit_text(text, avail, self.size, sink, ell);
                        let tx = aligned(col, x, w, pad, || sink.text_width(shown, self.size));
                        sink.text(tx, top + g.row_h / 2.0, shown, self.size, self.pal.text);
                    }
                }
                CellRender::Checkbox => self.draw_checkbox(sink, at, col, if cell.as_bool() { Check::Checked } else { Check::Unchecked }),
                CellRender::Badge => self.draw_badge(sink, at, col, &cell, self.src.badge(row, source), scratch),
                CellRender::Progress => self.draw_progress(sink, at, pad, &cell),
                CellRender::Actions => self.draw_dots(sink, at),
            }
            if selected_cell.is_some_and(|(sr, _)| sr == row) {
                sink.stroke(Rect::new(x, top, w, g.row_h), self.pal.accent, 1.5);
            }
        }
        if t.sel_col == Some(source) {
            sink.fill(Rect::new(x, g.top, w, g.body_h), self.pal.accent.opacity(0.18));
        }
    }

    /// The fill of checked boxes, default badges and progress bars: Blender's
    /// selection blue, lifted so it shows on a selected row too.
    fn primary(&self) -> Rgba {
        Rgba { a: 1.0, ..self.pal.sel }.shade(25.0)
    }

    /// A checkbox in a cell or header: Blender-sized (about 60% of the row),
    /// at the column's alignment.
    fn draw_checkbox(&self, sink: &mut dyn PaintSink, at: Rect, col: &Column, state: Check) {
        let s = (at.h * 0.6).round().max(8.0);
        let x = aligned(col, at.x, at.w, self.pad(), || s);
        let b = Rect::new(x, at.y + (at.h - s) / 2.0, s, s);
        let pal = self.pal;
        if state == Check::Unchecked {
            sink.round(b, 3.0, pal.text.opacity(0.5));
            sink.round(Rect::new(b.x + 1.0, b.y + 1.0, s - 2.0, s - 2.0), 2.0, pal.back);
            return;
        }
        let white = Rgba::new(1.0, 1.0, 1.0, 1.0);
        sink.round(b, 3.0, self.primary());
        let w = (s * 0.1).max(1.5);
        if state == Check::Checked {
            let p = |fx: f32, fy: f32| (b.x + s * fx, b.y + s * fy);
            sink.line(p(0.24, 0.52), p(0.42, 0.7), white, w);
            sink.line(p(0.42, 0.7), p(0.76, 0.3), white, w);
        } else {
            sink.line((b.x + s * 0.25, b.y + s * 0.5), (b.x + s * 0.75, b.y + s * 0.5), white, w);
        }
    }

    /// shadcn's Badge: the cell's text in a pill.
    fn draw_badge(&self, sink: &mut dyn PaintSink, at: Rect, col: &Column, cell: &Cell<'_>, variant: BadgeVariant, scratch: &mut PaintScratch) {
        let PaintScratch { cell: buf, ell } = scratch;
        let text = cell.text_fmt(buf, &col.format);
        if text.is_empty() {
            return;
        }
        let pal = self.pal;
        let (pad_x, h) = (self.pad() * 0.9, (at.h * 0.72).round());
        let shown = fit_text(text, (at.w - 2.0 * self.pad() - 2.0 * pad_x).max(0.0), self.size, sink, ell);
        let tw = sink.text_width(shown, self.size);
        let r = Rect::new(aligned(col, at.x, at.w, self.pad(), || tw + 2.0 * pad_x), at.y + (at.h - h) / 2.0, tw + 2.0 * pad_x, h);
        let white = Rgba::new(1.0, 1.0, 1.0, 1.0);
        let ink = match variant {
            BadgeVariant::Default => {
                sink.round(r, h / 2.0, self.primary());
                white
            }
            BadgeVariant::Secondary => {
                sink.round(r, h / 2.0, pal.text.opacity(0.14));
                pal.text
            }
            BadgeVariant::Destructive => {
                sink.round(r, h / 2.0, pal.danger);
                white
            }
            BadgeVariant::Outline => {
                sink.round(r, h / 2.0, pal.text.opacity(0.4));
                sink.round(Rect::new(r.x + 1.0, r.y + 1.0, r.w - 2.0, r.h - 2.0), h / 2.0 - 1.0, pal.back);
                pal.text
            }
        };
        sink.text(r.x + pad_x, at.y + at.h / 2.0, shown, self.size, ink);
    }

    /// shadcn's Progress: a track and a fill, the cell a number 0..=100.
    fn draw_progress(&self, sink: &mut dyn PaintSink, at: Rect, pad: f32, cell: &Cell<'_>) {
        let (w, h) = ((at.w - 2.0 * pad).max(0.0), (at.h * 0.28).round().max(3.0));
        let frac = (cell.number().unwrap_or(0.0) / 100.0).clamp(0.0, 1.0) as f32;
        let y = at.y + (at.h - h) / 2.0;
        sink.round(Rect::new(at.x + pad, y, w, h), h / 2.0, self.pal.text.opacity(0.15));
        if frac > 0.0 {
            sink.round(Rect::new(at.x + pad, y, (w * frac).max(h), h), h / 2.0, self.primary());
        }
    }

    /// The row-actions button: three dots (lucide's `MoreHorizontal`).
    fn draw_dots(&self, sink: &mut dyn PaintSink, at: Rect) {
        let d = (at.h * 0.14).round().max(2.0);
        let (cx, cy) = (at.x + at.w / 2.0, at.y + at.h / 2.0);
        for k in [-1.0f32, 0.0, 1.0] {
            sink.round(Rect::new(cx + k * d * 2.0 - d / 2.0, cy - d / 2.0, d, d), d / 2.0, self.pal.text);
        }
    }

    /// Blender's left index column.
    fn draw_index_column(&self, sink: &mut dyn PaintSink, rows: &Range<usize>, scratch: &mut PaintScratch) {
        let (g, t) = (&self.g, self.t);
        let strip = Rect::new(0.0, g.top, g.index_w, g.body_h);
        sink.clipped(strip, &mut |sink| {
            sink.fill(strip, self.pal.header);
            for r in rows.clone() {
                let top = t.row_top(g, r);
                scratch.cell.clear();
                scratch.cell.push_u64(t.mask.get(r) as u64);
                let text = scratch.cell.as_str();
                let w = sink.text_width(text, self.size);
                sink.text(g.index_w - w - g.unit * 0.3, top + g.row_h / 2.0, text, self.size, self.pal.dim);
            }
            self.vline(sink, g.index_w, g.top, g.body_h);
        });
    }

    fn draw_headers(&self, sink: &mut dyn PaintSink, scratch: &mut PaintScratch) {
        let (g, t, pal) = (&self.g, self.t, self.pal);
        let hair = self.hair;
        sink.fill(Rect::new(0.0, 0.0, g.w, g.top), pal.header);
        // group rows
        if let Some(levels) = &t.groups {
            for (lvl, groups) in levels.iter().enumerate() {
                let y = lvl as f32 * g.header_h;
                let mut at = 0usize;
                for gr in groups {
                    let end = (at + gr.span).min(t.layout.len());
                    if at >= end {
                        break;
                    }
                    let (l, r) = (t.col_x(g, at), t.col_x(g, end - 1) + t.layout.width(end - 1));
                    let (l, r) = (l.max(g.index_w), r.min(g.w));
                    if r > l {
                        sink.clipped(Rect::new(l, y, r - l, g.header_h), &mut |sink| {
                            let w = sink.text_width(&gr.label, self.size);
                            sink.text(l + ((r - l) - w) / 2.0, y + g.header_h / 2.0, &gr.label, self.size, pal.text);
                            sink.fill(Rect::new(r - hair, y, hair, g.header_h), pal.line);
                        });
                    }
                    at = end;
                }
                sink.fill(Rect::new(0.0, y + g.header_h - hair, g.w, hair), pal.line);
            }
        }
        let hy = g.groups_h;
        let sx = t.scroll.0 as f32;
        sink.clipped(Rect::new(g.x0, hy, g.view_w, g.header_h), &mut |sink| {
            for c in visible_cols(t.layout.scroll_edges(), sx, sx + g.view_w) {
                self.draw_header_cell(sink, c + t.layout.pinned(), scratch);
            }
        });
        if g.pinned_w > 0.0 {
            let strip = Rect::new(g.index_w, hy, g.pinned_w, g.header_h);
            sink.clipped(strip, &mut |sink| {
                sink.fill(strip, pal.header);
                for v in 0..t.layout.pinned() {
                    self.draw_header_cell(sink, v, scratch);
                }
            });
        }
        sink.fill(Rect::new(0.0, g.top - hair, g.w, hair), pal.line);
        // Blender's column move: dim the grabbed column, mark where it lands
        if let Drag::Header { v, moved: true, to, .. } = t.drag {
            let x = t.col_x(g, v);
            sink.fill(Rect::new(x, hy, t.layout.width(v), g.header_h), pal.back.shade(-20.0).opacity(0.5));
            let tx = t.col_x(g, to) + if to > v { t.layout.width(to) } else { 0.0 };
            sink.fill(Rect::new(tx - 1.0, hy, 2.0, g.header_h + g.body_h), pal.text.shade(20.0));
        }
    }

    fn draw_header_cell(&self, sink: &mut dyn PaintSink, v: usize, scratch: &mut PaintScratch) {
        let (g, t, pal) = (&self.g, self.t, self.pal);
        let col = t.layout.column(v);
        let x = t.col_x(g, v);
        let w = t.layout.width(v);
        let hy = g.groups_h;
        let pad = self.pad();
        let source = t.layout.src(v);
        if col.render == CellRender::Checkbox {
            self.draw_checkbox(sink, Rect::new(x, hy, w, g.header_h), col, self.src.header_check(source));
            if t.sel_col == Some(source) {
                sink.fill(Rect::new(x, hy, w, g.header_h), pal.accent.opacity(0.25));
            }
            self.vline(sink, x + w, hy, g.header_h);
            return;
        }
        let arrow = if col.header_menu || (col.sort.is_some() && t.opts.sortable) { g.unit * 0.7 } else { 0.0 };
        let text = fit_text(&col.name, (w - 2.0 * pad - arrow).max(0.0), self.size, sink, &mut scratch.ell);
        let tx = match col.align {
            TextAlign::Right => x + w - pad - arrow - sink.text_width(text, self.size),
            TextAlign::Center => x + (w - arrow - sink.text_width(text, self.size)) / 2.0,
            TextAlign::Left => x + pad,
        };
        sink.text(tx, hy + g.header_h / 2.0, text, self.size, pal.text);
        let (cy, k) = (hy + g.header_h / 2.0, g.unit * 0.15);
        if col.header_menu {
            // shadcn's DataTableColumnHeader: the icon follows the title (ArrowUp, ArrowDown or ChevronsUpDown)
            let cx = if col.align == TextAlign::Left { (tx + sink.text_width(text, self.size) + pad + k).min(x + w - pad - k) } else { x + w - pad - g.unit * 0.25 };
            let s = col.sort.unwrap_or_default();
            if s == ColumnSort::Default {
                chevron(sink, cx, cy - k * 0.9, k * 0.8, true, pal.dim);
                chevron(sink, cx, cy + k * 0.9, k * 0.8, false, pal.dim);
            } else {
                chevron(sink, cx, cy, k, s == ColumnSort::Ascending, pal.text);
            }
        } else if let (Some(s @ (ColumnSort::Ascending | ColumnSort::Descending)), true) = (col.sort, t.opts.sortable) {
            chevron(sink, x + w - pad - g.unit * 0.25, cy, k, s == ColumnSort::Ascending, pal.text);
        }
        if t.sel_col == Some(t.layout.src(v)) {
            sink.fill(Rect::new(x, hy, w, g.header_h), pal.accent.opacity(0.25));
        }
        self.vline(sink, x + w, hy, g.header_h);
    }

    fn draw_scrollbars(&self, sink: &mut dyn PaintSink) {
        let (g, t) = (&self.g, self.t);
        let thumb = self.pal.text.opacity(0.35);
        if let Some((ty, th)) = t.vthumb(g) {
            sink.round(Rect::new(g.w - g.bar + 1.0, ty, g.bar - 2.0, th), (g.bar - 2.0) / 2.0, thumb);
        }
        if let Some((tx, tw)) = t.hthumb(g) {
            sink.round(Rect::new(tx, g.h - g.bar + 1.0, tw, g.bar - 2.0), (g.bar - 2.0) / 2.0, thumb);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::Cell;
    use crate::column::Column;

    /// Records what the table paints.
    #[derive(Default)]
    struct Rec {
        fills: Vec<Rect>,
        texts: Vec<String>,
        depth: i32,
    }

    impl PaintSink for Rec {
        fn fill(&mut self, r: Rect, _: Rgba) {
            self.fills.push(r);
        }
        fn stroke(&mut self, _: Rect, _: Rgba, _: f32) {}
        fn round(&mut self, _: Rect, _: f32, _: Rgba) {}
        fn line(&mut self, _: (f32, f32), _: (f32, f32), _: Rgba, _: f32) {}
        fn text(&mut self, _: f32, _: f32, text: &str, _: f32, _: Rgba) {
            self.texts.push(text.to_owned());
        }
        fn text_width(&mut self, text: &str, size: f32) -> f32 {
            text.len() as f32 * size * 0.5
        }
        fn clipped(&mut self, _: Rect, f: &mut dyn FnMut(&mut dyn PaintSink)) {
            self.depth += 1;
            f(self);
            self.depth -= 1;
        }
    }

    struct Data;
    impl CellSource for Data {
        fn cell(&self, r: usize, c: usize) -> Cell<'_> {
            Cell::U64((r * 10 + c) as u64)
        }
    }

    fn model(rows: usize) -> (TableModel, Data) {
        let d = Data;
        let mut t = TableModel::new();
        t.set_columns((0..6).map(|i| Column::new(format!("c{i}"), format!("C{i}")).width(80.0_f32)).collect(), None);
        t.set_rows(rows, &d);
        t.begin_frame(300.0, 200.0, 20.0, 1.0);
        (t, d)
    }

    fn pal() -> Palette {
        let c = |v: f32| Rgba::new(v, v, v, 1.0);
        Palette::new(c(0.2), c(0.9), c(0.3), c(0.5), c(0.7))
    }

    fn paint(t: &TableModel, d: &Data) -> Rec {
        let mut rec = Rec::default();
        with_scratch(|s| paint_table(t, d, &pal(), "No data", s, &mut rec));
        assert_eq!(rec.depth, 0, "clips are balanced");
        rec
    }

    #[test]
    fn paints_only_visible_cells() {
        let (t, d) = model(1_000_000);
        let rec = paint(&t, &d);
        // header names + visible cells: (200 - 22) / 20 rows, 300 px of 80 px columns
        let cells = rec.texts.iter().filter(|s| s.parse::<u64>().is_ok()).count();
        assert!(cells <= 10 * 5, "{cells}");
        assert!(cells >= 8 * 3);
    }

    #[test]
    fn a_selection_of_many_rows_is_one_quad() {
        let (mut t, d) = model(1000);
        let base = paint(&t, &d).fills.len();
        t.sel.select_all(1000);
        let all = paint(&t, &d).fills.len();
        assert_eq!(all, base + 1, "select-all costs a single fill");
        t.sel.clear();
        t.sel.select_only(3);
        t.sel.add(5..8);
        let two = paint(&t, &d).fills.len();
        assert_eq!(two, base + 2);
    }

    #[test]
    fn right_alignment_measures_through_the_sink() {
        let (mut t, d) = model(10);
        t.set_columns(vec![Column::new("a", "A").width(100.0_f32).text_right()], None);
        t.begin_frame(300.0, 200.0, 20.0, 1.0);
        let rec = paint(&t, &d);
        assert!(rec.texts.iter().any(|s| s == "0"));
    }

    #[test]
    fn steady_state_paint_does_not_allocate_beyond_the_sink() {
        use crate::alloc_probe::count_allocs;
        let (mut t, d) = model(1_000_000);
        t.scroll_by(0.0, 5_000_000.0);
        t.begin_frame(300.0, 200.0, 20.0, 1.0);
        t.opts.stripe = true;
        t.opts.row_header = true;
        // a sink that records nothing: only the table's own allocations count
        struct Null;
        impl PaintSink for Null {
            fn fill(&mut self, _: Rect, _: Rgba) {}
            fn stroke(&mut self, _: Rect, _: Rgba, _: f32) {}
            fn round(&mut self, _: Rect, _: f32, _: Rgba) {}
            fn line(&mut self, _: (f32, f32), _: (f32, f32), _: Rgba, _: f32) {}
            fn text(&mut self, _: f32, _: f32, _: &str, _: f32, _: Rgba) {}
            fn text_width(&mut self, text: &str, size: f32) -> f32 {
                text.len() as f32 * size * 0.5
            }
            fn clipped(&mut self, _: Rect, f: &mut dyn FnMut(&mut dyn PaintSink)) {
                f(self);
            }
        }
        let p = pal();
        with_scratch(|s| {
            for _ in 0..3 {
                paint_table(&t, &d, &p, "", s, &mut Null);
            }
            let n = count_allocs(|| paint_table(&t, &d, &p, "", s, &mut Null));
            assert_eq!(n, 0, "a steady-state frame allocated");
        });
    }

    /// Every cell kind in one table: text with a currency format, a checkbox,
    /// a badge, a progress bar and the actions dots.
    struct Kinds;
    impl CellSource for Kinds {
        fn cell(&self, r: usize, c: usize) -> Cell<'_> {
            match c {
                0 => Cell::U64((r % 2) as u64),
                1 => Cell::Str(["paid", "failed"][r % 2]),
                2 => Cell::F64(1234.5 + r as f64, 2),
                3 => Cell::U64((r * 7 % 101) as u64),
                _ => Cell::Empty,
            }
        }
        fn badge(&self, r: usize, _: usize) -> BadgeVariant {
            if r.is_multiple_of(2) { BadgeVariant::Default } else { BadgeVariant::Destructive }
        }
        fn header_check(&self, _: usize) -> Check {
            Check::Indeterminate
        }
    }

    fn kinds_model(rows: usize) -> TableModel {
        use crate::column::NumberFormat;
        let mut t = TableModel::new();
        t.set_columns(
            vec![
                Column::new("sel", "").width(40.0_f32).render(CellRender::Checkbox).text_center(),
                Column::new("status", "Status").width(90.0_f32).render(CellRender::Badge),
                Column::new("amount", "Amount").width(110.0_f32).text_right().format(NumberFormat::usd()),
                Column::new("pct", "Done").width(100.0_f32).render(CellRender::Progress),
                Column::new("act", "").width(48.0_f32).render(CellRender::Actions),
            ],
            None,
        );
        t.set_rows(rows, &Kinds);
        t.begin_frame(400.0, 200.0, 20.0, 1.0);
        t
    }

    #[test]
    fn typed_cell_kinds_paint_without_elements() {
        let t = kinds_model(1000);
        let mut rec = Rec::default();
        with_scratch(|s| paint_table(&t, &Kinds, &pal(), "", s, &mut rec));
        assert_eq!(rec.depth, 0);
        assert!(rec.texts.iter().any(|s| s == "paid") && rec.texts.iter().any(|s| s == "failed"), "badge text");
        assert!(rec.texts.iter().any(|s| s == "$1,234.50"), "currency format: {:?}", &rec.texts[..8]);
        assert!(!rec.texts.iter().any(|s| s == "1234.50"));
    }

    #[test]
    fn typed_cell_kinds_do_not_allocate_in_a_steady_frame() {
        use crate::alloc_probe::count_allocs;
        let mut t = kinds_model(1_000_000);
        t.scroll_by(0.0, 5_000_000.0);
        t.begin_frame(400.0, 200.0, 20.0, 1.0);
        struct Null;
        impl PaintSink for Null {
            fn fill(&mut self, _: Rect, _: Rgba) {}
            fn stroke(&mut self, _: Rect, _: Rgba, _: f32) {}
            fn round(&mut self, _: Rect, _: f32, _: Rgba) {}
            fn line(&mut self, _: (f32, f32), _: (f32, f32), _: Rgba, _: f32) {}
            fn text(&mut self, _: f32, _: f32, _: &str, _: f32, _: Rgba) {}
            fn text_width(&mut self, text: &str, size: f32) -> f32 {
                text.len() as f32 * size * 0.5
            }
            fn clipped(&mut self, _: Rect, f: &mut dyn FnMut(&mut dyn PaintSink)) {
                f(self);
            }
        }
        let p = pal();
        with_scratch(|s| {
            for _ in 0..3 {
                paint_table(&t, &Kinds, &p, "", s, &mut Null);
            }
            assert_eq!(count_allocs(|| paint_table(&t, &Kinds, &p, "", s, &mut Null)), 0, "a steady-state frame allocated");
        });
    }

    #[test]
    fn clicks_on_typed_cells_raise_events() {
        use crate::event::TableEvent;
        use crate::input::{Button, Mods};
        let mut t = kinds_model(100);
        let g = t.geo();
        let row = |r: f32| g.top + g.row_h * (r + 0.5);
        // checkbox cell, row 2: neither selects the row nor changes the sort
        t.pointer_down(crate::geom::Point { x: 20.0, y: row(2.0) }, Button::Left, Mods::default(), 1);
        assert_eq!(t.take_events(), vec![TableEvent::CheckRow(2, 0)]);
        assert!(t.selected_rows().next().is_none());
        // the actions cell, row 1 (x past 40 + 90 + 110 + 100)
        t.pointer_down(crate::geom::Point { x: 360.0, y: row(1.0) }, Button::Left, Mods::default(), 1);
        assert_eq!(t.take_events(), vec![TableEvent::CellAction(1, 4)]);
        // the checkbox header: select all
        t.pointer_down(crate::geom::Point { x: 20.0, y: g.groups_h + g.header_h / 2.0 }, Button::Left, Mods::default(), 1);
        t.pointer_up(&Kinds, false);
        assert_eq!(t.take_events(), vec![TableEvent::CheckAll(0)]);
    }
}
