//! The table's rectangles for a given size and where a point falls: one
//! source of truth for painting and hit testing. Logical pixels, relative to
//! the table's top-left.
//!
//! Layout metrics are Blender's spreadsheet: row = `UI_UNIT_Y`, header row =
//! `1.1 * UI_UNIT_Y`, index column = `2 * UI_UNIT_X` (`SpreadsheetDrawer`
//! constructor in `spreadsheet_draw.cc`), all scaled by the UI scale.

use crate::geom::Point;
use crate::model::TableModel;
use crate::visible::{max_scroll, visible_cols, visible_rows};

use crate::event::TableVisibleRange;

/// The table's rectangles.
#[derive(Copy, Clone, Debug)]
pub struct Geometry {
    pub w: f32,
    pub h: f32,
    pub unit: f32,
    pub row_h: f32,
    pub header_h: f32,
    pub groups_h: f32,
    /// Top of the rows.
    pub top: f32,
    pub index_w: f32,
    pub pinned_w: f32,
    /// Left edge of the scrolling columns.
    pub x0: f32,
    pub body_h: f32,
    pub view_w: f32,
    pub bar: f32,
}

/// Where a point in the table is.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Hit {
    None,
    Header(usize),
    /// The border after view column `v` (a resize handle).
    Border(usize),
    Group,
    Index(usize),
    /// Display row and view column.
    Cell(usize, usize),
    /// A vertical scrollbar; true over its thumb.
    VBar(bool),
    HBar(bool),
}

/// What the user can drag a pointer over, 4 px of slack to grab a border.
const GRAB: f32 = 4.0;

impl TableModel {
    #[must_use]
    pub fn geometry(&self, w: f32, h: f32) -> Geometry {
        let unit = self.unit;
        let row_h = (unit * self.opts.size.factor()).round().max(1.0);
        let header_h = self.opts.header_row_height.unwrap_or(unit * 1.1);
        let groups_h = self.groups.as_ref().map_or(0.0, |g| g.len() as f32 * header_h);
        let top = groups_h + header_h;
        // Blender: 2 * UI_UNIT_X; wider when the row numbers need it
        let digits = self.total.max(1).ilog10() as f32 + 1.0;
        let index_w = if self.opts.row_header { (unit * 2.0).max(digits * unit * 0.38 + unit * 0.7) } else { 0.0 };
        let pinned_w = self.layout.pinned_width();
        let x0 = index_w + pinned_w;
        Geometry {
            w,
            h,
            unit,
            row_h,
            header_h,
            groups_h,
            top,
            index_w,
            pinned_w,
            x0,
            body_h: (h - top).max(0.0),
            view_w: (w - x0).max(0.0),
            bar: (unit * 0.45).max(6.0),
        }
    }

    /// The geometry for the table's current size.
    #[must_use]
    pub fn geo(&self) -> Geometry {
        self.geometry(self.size.0, self.size.1)
    }

    /// Rows and columns in view for `g` (the same functions the painter uses).
    #[must_use]
    pub fn compute_visible(&self, g: &Geometry) -> TableVisibleRange {
        let rows = visible_rows(self.scroll.1, f64::from(g.row_h), f64::from(g.body_h), self.rows());
        let sx = self.scroll.0 as f32;
        let cols = visible_cols(self.layout.scroll_edges(), sx, sx + g.view_w);
        let p = self.layout.pinned();
        TableVisibleRange { rows, cols: cols.start + p..cols.end + p }
    }

    /// Table-x of the left edge of view column `v`.
    #[must_use]
    pub fn col_x(&self, g: &Geometry, v: usize) -> f32 {
        if v < self.layout.pinned() {
            g.index_w + self.layout.left(v)
        } else {
            g.x0 + self.layout.left(v) - self.scroll.0 as f32
        }
    }

    /// The view column under table-x `x` (pinned columns cover scrolled ones).
    #[must_use]
    pub fn col_at(&self, g: &Geometry, x: f32) -> Option<usize> {
        if x < g.index_w {
            None
        } else if x < g.x0 {
            self.layout.at_x(true, x - g.index_w)
        } else {
            self.layout.at_x(false, x - g.x0 + self.scroll.0 as f32)
        }
    }

    /// Table-y of the top of display row `r` for the current scroll.
    #[must_use]
    pub fn row_top(&self, g: &Geometry, r: usize) -> f32 {
        g.top + (r as f64 * f64::from(g.row_h) - self.scroll.1) as f32
    }

    /// The vertical thumb: top and height.
    #[must_use]
    pub fn vthumb(&self, g: &Geometry) -> Option<(f32, f32)> {
        let content = self.rows() as f64 * f64::from(g.row_h);
        let view = f64::from(g.body_h);
        if !self.opts.scrollbars.0 || content <= view || view <= 0.0 {
            return None;
        }
        let th = ((view * view / content) as f32).max(g.unit);
        let range = max_scroll(self.rows(), f64::from(g.row_h), view);
        let ty = g.top + (f64::from(g.body_h - th) * (self.scroll.1 / range)) as f32;
        Some((ty, th))
    }

    /// The horizontal thumb: left and width.
    #[must_use]
    pub fn hthumb(&self, g: &Geometry) -> Option<(f32, f32)> {
        let content = self.layout.scroll_width();
        if !self.opts.scrollbars.1 || content <= g.view_w || g.view_w <= 0.0 {
            return None;
        }
        let tw = (g.view_w * g.view_w / content).max(g.unit);
        let range = f64::from(content - g.view_w);
        let tx = g.x0 + (f64::from(g.view_w - tw) * (self.scroll.0 / range)) as f32;
        Some((tx, tw))
    }

    /// What is at table-relative point `p`.
    #[must_use]
    pub fn hit(&self, p: Point) -> Hit {
        let g = self.geo();
        let (x, y) = (p.x, p.y);
        if x < 0.0 || y < 0.0 || x >= g.w || y >= g.h {
            return Hit::None;
        }
        if y >= g.top {
            if let (Some((ty, th)), true) = (self.vthumb(&g), x >= g.w - g.bar) {
                return Hit::VBar(y >= ty && y < ty + th);
            }
            if let (Some((tx, tw)), true) = (self.hthumb(&g), y >= g.h - g.bar && x >= g.x0) {
                return Hit::HBar(x >= tx && x < tx + tw);
            }
        }
        if y < g.groups_h {
            return Hit::Group;
        }
        if y < g.top {
            let Some(v) = self.col_at(&g, x) else { return Hit::None };
            let (l, w) = (self.col_x(&g, v), self.layout.width(v));
            if l + w - x <= GRAB {
                return Hit::Border(v);
            }
            if x - l <= GRAB && v > 0 && (v != self.layout.pinned() || g.pinned_w > 0.0) {
                return Hit::Border(v - 1);
            }
            return Hit::Header(v);
        }
        let row = ((f64::from(y - g.top) + self.scroll.1) / f64::from(g.row_h)).floor();
        if row < 0.0 || row >= self.rows() as f64 {
            return Hit::None;
        }
        let row = row as usize;
        if x < g.index_w {
            return Hit::Index(row);
        }
        match self.col_at(&g, x) {
            Some(v) => Hit::Cell(row, v),
            None => Hit::None,
        }
    }

    /// Header borders to put a resize cursor over: (x, y, height) in table
    /// coordinates, for the resizable columns in view.
    #[must_use]
    pub fn header_handles(&self) -> Vec<(f32, f32, f32)> {
        let g = self.geo();
        if !self.opts.col_resizable {
            return Vec::new();
        }
        let vis = self.compute_visible(&g);
        let pinned = self.layout.pinned();
        (0..pinned)
            .chain(vis.cols)
            .filter(|&v| self.layout.column(v).resizable)
            .map(|v| (self.col_x(&g, v) + self.layout.width(v), g.groups_h, g.header_h))
            .filter(|&(x, _, _)| x >= g.index_w && x <= g.w - 4.0)
            .collect()
    }
}
