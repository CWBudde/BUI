//! How pointer, wheel and keyboard input change a [`TableModel`]: the one
//! implementation of the table's interaction, shared by every adapter.
//! Positions are relative to the table's top-left. Handlers return whether
//! the table needs a redraw; results the framework must act on (hooks, the
//! clipboard) are queued as [`TableEvent`]s.

use crate::cell::CellSource;
use crate::column::{CellRender, ColumnSort};
use crate::event::TableEvent;
use crate::geom::Point;
use crate::geometry::Hit;
use crate::input::{Button, Key, Mods};
use crate::model::{Drag, TableModel};
use crate::visible::max_scroll;

/// Pointer travel before a header press becomes a column move.
const MOVE_SLOP: f32 = 4.0;

impl TableModel {
    /// Pointer pressed at `p`; `clicks` is the click count (2: double click).
    pub fn pointer_down(&mut self, p: Point, button: Button, mods: Mods, clicks: usize) {
        let hit = self.hit(p);
        let g = self.geo();
        if button == Button::Right {
            self.right_click(hit);
            return;
        }
        if button != Button::Left {
            return;
        }
        match hit {
            Hit::VBar(on) => {
                let (ty, th) = self.vthumb(&g).unwrap_or((0.0, 0.0));
                let grab = if on { p.y - ty } else { th / 2.0 };
                self.drag = Drag::VThumb { grab };
                self.drag_thumb(p);
            }
            Hit::HBar(on) => {
                let (tx, tw) = self.hthumb(&g).unwrap_or((0.0, 0.0));
                let grab = if on { p.x - tx } else { tw / 2.0 };
                self.drag = Drag::HThumb { grab };
                self.drag_thumb(p);
            }
            Hit::Border(v) => {
                if clicks >= 2 {
                    self.events.push(TableEvent::FitColumn(self.layout.src(v)));
                } else if self.opts.col_resizable && self.layout.column(v).resizable {
                    self.drag = Drag::Resize { v, x: p.x, w: self.layout.width(v) };
                }
            }
            Hit::Header(v) => self.drag = Drag::Header { v, x: p.x, moved: false, to: v },
            Hit::Index(r) => self.click_row(r, None, mods, clicks),
            Hit::Cell(r, v) => self.click_row(r, Some(v), mods, clicks),
            Hit::Group | Hit::None => {}
        }
    }

    fn right_click(&mut self, hit: Hit) {
        match hit {
            Hit::Cell(r, _) | Hit::Index(r) => {
                let v = if let Hit::Cell(_, v) = hit { Some(v) } else { None };
                if !self.sel.contains(r) {
                    self.sel.select_only(r);
                }
                let row = self.mask.get(r);
                self.right_clicked = Some((row, v.map(|v| self.layout.src(v))));
                self.events.push(match v {
                    Some(v) if self.opts.cell_selectable => TableEvent::RightClickedCell(row, self.layout.src(v)),
                    _ => TableEvent::RightClickedRow(Some(row)),
                });
            }
            _ => {
                self.right_clicked = None;
                self.events.push(TableEvent::RightClickedRow(None));
            }
        }
    }

    fn click_row(&mut self, r: usize, col: Option<usize>, mods: Mods, clicks: usize) {
        let row = self.mask.get(r);
        self.right_clicked = None;
        if let Some(v) = col {
            let (render, src) = (self.layout.column(v).render, self.layout.src(v));
            match render {
                CellRender::Checkbox => return self.events.push(TableEvent::CheckRow(row, src)),
                CellRender::Actions => return self.events.push(TableEvent::CellAction(row, src)),
                _ => {}
            }
        }
        let cell_mode = self.opts.cell_selectable && col.is_some_and(|v| self.layout.column(v).selectable);
        if cell_mode {
            let c = self.layout.src(col.unwrap_or(0));
            self.sel.select_only(r);
            self.sel_col = None;
            self.sel_cell = Some((row, c));
            self.events.push(TableEvent::SelectCell(row, c));
            if clicks >= 2 {
                self.events.push(TableEvent::DoubleClickedCell(row, c));
            }
        } else if self.opts.row_selectable {
            let multi = self.opts.select_on_modifier_click;
            self.sel.click(r, if multi { mods } else { Mods::default() });
            self.sel_col = None;
            self.sel_cell = None;
            self.events.push(TableEvent::SelectRow(row));
            if clicks >= 2 {
                self.events.push(TableEvent::DoubleClickedRow(row));
            }
        }
    }

    fn drag_thumb(&mut self, p: Point) {
        let g = self.geo();
        match self.drag {
            Drag::VThumb { grab } => {
                if let Some((_, th)) = self.vthumb(&g) {
                    let range = max_scroll(self.rows(), f64::from(g.row_h), f64::from(g.body_h));
                    let t = f64::from(p.y - grab - g.top) / f64::from((g.body_h - th).max(1.0));
                    self.scroll.1 = t.clamp(0.0, 1.0) * range;
                    self.clamp_scroll();
                }
            }
            Drag::HThumb { grab } => {
                if let Some((_, tw)) = self.hthumb(&g) {
                    let range = f64::from(self.layout.scroll_width() - g.view_w);
                    let t = f64::from(p.x - grab - g.x0) / f64::from((g.view_w - tw).max(1.0));
                    self.scroll.0 = t.clamp(0.0, 1.0) * range;
                    self.clamp_scroll();
                }
            }
            _ => {}
        }
    }

    /// Pointer moved to `p`. Returns whether to redraw.
    pub fn pointer_move(&mut self, p: Point) -> bool {
        let g = self.geo();
        match self.drag {
            Drag::Resize { v, x, w } => {
                if self.layout.set_width(v, w + p.x - x) {
                    self.widths_changed();
                    return true;
                }
            }
            Drag::Header { v, x, moved, .. } => {
                let col = self.layout.column(v);
                if (moved || (p.x - x).abs() > MOVE_SLOP) && self.opts.col_movable && col.movable {
                    // Blender's column move: the target is the column under the pointer
                    let to = self.col_at(&g, p.x.max(g.index_w)).unwrap_or(v);
                    let to = if (to < self.layout.pinned()) == (v < self.layout.pinned()) { to } else { v };
                    self.drag = Drag::Header { v, x, moved: true, to };
                    return true;
                }
            }
            Drag::VThumb { .. } | Drag::HThumb { .. } => {
                self.drag_thumb(p);
                return true;
            }
            Drag::None => {}
        }
        let h = match self.hit(p) {
            Hit::Cell(r, _) | Hit::Index(r) => Some(r),
            _ => None,
        };
        if h == self.hover {
            return false;
        }
        self.hover = h;
        true
    }

    /// Pointer released: finishes drags, and a header press that didn't move
    /// is a click (sort, or select the column). `by_mask` says whether a sort
    /// reorders the row mask (false: the data sorts itself).
    pub fn pointer_up<S: CellSource + ?Sized>(&mut self, src: &S, by_mask: bool) {
        let drag = std::mem::replace(&mut self.drag, Drag::None);
        let Drag::Header { v, moved, to, .. } = drag else { return };
        if moved {
            if self.layout.move_to(v, to) {
                self.events.push(TableEvent::MoveColumn(v, to));
            }
            return;
        }
        let col = self.layout.column(v);
        let source = self.layout.src(v);
        if col.render == CellRender::Checkbox {
            self.events.push(TableEvent::CheckAll(source));
        } else if col.header_menu {
            self.events.push(TableEvent::HeaderMenu(source));
        } else if self.opts.sortable && col.sort.is_some() {
            let next = col.sort.map_or(ColumnSort::Ascending, ColumnSort::next);
            self.sort_by(src, source, next, by_mask);
        } else if self.opts.col_selectable && col.selectable {
            self.set_selected_col(source);
        }
    }

    /// Pixels a wheel of `lines` > 0 (up) scrolls: three rows per line,
    /// negative when scrolling up.
    #[must_use]
    pub fn wheel_pixels(&self, lines: f32) -> f64 {
        f64::from(-lines) * f64::from(self.geo().row_h) * 3.0
    }

    /// The display row a vertical key moves to from `active`.
    fn nav_target(&self, key: Key, shift: bool, active: Option<usize>, page: usize) -> Option<usize> {
        let rows = self.rows();
        let last = rows.saturating_sub(1);
        let wrap = self.opts.loop_selection && !shift;
        match key {
            Key::Down => Some(active.map_or(0, |a| if a + 1 < rows { a + 1 } else if wrap { 0 } else { a })),
            Key::Up => Some(active.map_or(last, |a| if a > 0 { a - 1 } else if wrap { last } else { 0 })),
            Key::PageDown => Some(active.map_or(0, |a| (a + page).min(last))),
            Key::PageUp => Some(active.map_or(0, |a| a.saturating_sub(page))),
            Key::Home => Some(0),
            Key::End => Some(last),
            _ => None,
        }
    }

    /// A key while the table has focus: arrows, PageUp/PageDown, Home/End,
    /// Ctrl+A (select all), Ctrl+C (copy as TSV), Escape. Returns whether it
    /// was handled.
    pub fn key_down<S: CellSource + ?Sized>(&mut self, key: Key, mods: Mods, src: &S) -> bool {
        let rows = self.rows();
        let g = self.geo();
        let page = ((g.body_h / g.row_h).floor() as usize).max(1);
        if let (Some(t), true) = (self.nav_target(key, mods.shift, self.sel.active(), page), rows > 0) {
            self.sel.move_to(t, mods.shift && self.opts.select_on_modifier_click);
            self.sel_col = None;
            if let Some((_, c)) = self.sel_cell {
                self.sel_cell = Some((self.mask.get(t), c));
            }
            self.show_view_row(t);
            self.events.push(TableEvent::SelectRow(self.mask.get(t)));
            return true;
        }
        match key {
            Key::Left | Key::Right if self.opts.cell_selectable || self.opts.col_selectable => {
                self.move_column_selection(key == Key::Right);
                true
            }
            Key::Escape => {
                self.clear_selection();
                true
            }
            Key::A if mods.control && self.opts.row_selectable => {
                self.sel.select_all(rows);
                self.sel_col = None;
                self.sel_cell = None;
                self.events.push(TableEvent::SelectRow(self.sel.active().map_or(0, |a| self.mask.get(a))));
                true
            }
            Key::C if mods.control => {
                let text = self.selection_text(src);
                if !text.is_empty() {
                    self.events.push(TableEvent::Copied(text));
                }
                true
            }
            _ => false,
        }
    }

    /// Left / Right with a column or cell selection mode: move it one column.
    fn move_column_selection(&mut self, right: bool) {
        let n = self.layout.len();
        if n == 0 {
            return;
        }
        let cur = self.sel_cell.map(|(_, c)| c).or(self.sel_col).and_then(|s| self.layout.view_of(s));
        let wrap = self.opts.loop_selection;
        let next = match (cur, right) {
            (None, _) => 0,
            (Some(c), true) => if c + 1 < n { c + 1 } else if wrap { 0 } else { c },
            (Some(c), false) => if c > 0 { c - 1 } else if wrap { n - 1 } else { 0 },
        };
        let src = self.layout.src(next);
        if self.opts.cell_selectable {
            let r = self.sel.active().map_or(0, |a| self.mask.get(a));
            self.sel_cell = Some((r, src));
            self.events.push(TableEvent::SelectCell(r, src));
        } else {
            self.sel_col = Some(src);
            self.events.push(TableEvent::SelectColumn(src));
        }
        if next >= self.layout.pinned() {
            self.show_view_col(next);
        }
    }
}
