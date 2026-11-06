//! Columns: their description ([`Column`], named as in gpui-component) and
//! their live layout (order, widths, edges) in [`ColumnLayout`].
//!
//! Blender keeps one `SpreadsheetColumn` per column with a width in
//! `SPREADSHEET_WIDTH_UNIT`s and recomputes the left edges while drawing
//! (`spreadsheet_draw.cc`); here the edges are cached in two strips (pinned
//! columns, then the scrolling ones) and only rebuilt when a width or the
//! order changes, so drawing culls them by bisection ([`visible_cols`]).
//!
//! [`visible_cols`]: crate::visible::visible_cols

use std::borrow::Cow;

/// Column and header text: a static string or an owned one.
pub type Text = Cow<'static, str>;

/// Horizontal alignment of a column's cells.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// Whether a column sorts and in which direction (gpui-component's name).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum ColumnSort {
    /// Sortable, not sorted now (the original order).
    #[default]
    Default,
    /// Sorted ascending.
    Ascending,
    /// Sorted descending.
    Descending,
}

impl ColumnSort {
    /// What a header click does next: default, ascending, descending, default.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Default => Self::Ascending,
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Default,
        }
    }
}

/// How a column's cells are drawn. shadcn's `cell: ({ row }) => <JSX/>`
/// becomes one of these typed kinds: the table paints them straight from
/// the cell value, with no element per cell.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum CellRender {
    /// The cell's text.
    #[default]
    Text,
    /// A checkbox, checked when the cell is a non-zero number. A click
    /// raises [`TableEvent::CheckRow`](crate::TableEvent::CheckRow); the
    /// header shows [`CellSource::header_check`](crate::CellSource::header_check)
    /// and raises [`TableEvent::CheckAll`](crate::TableEvent::CheckAll).
    Checkbox,
    /// The cell's text in a pill, colored by
    /// [`CellSource::badge`](crate::CellSource::badge).
    Badge,
    /// A progress bar: the cell is a number from 0 to 100.
    Progress,
    /// A row-actions button (three dots); a click raises
    /// [`TableEvent::CellAction`](crate::TableEvent::CellAction).
    Actions,
    /// An image: the cell's text is a file path, shown as a square thumbnail
    /// (`object-fit: cover`). Only visible rows paint, so only they load.
    Image,
}

/// shadcn's Badge variants.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum BadgeVariant {
    /// `default`: the accent color.
    Default,
    /// `secondary`: a muted fill.
    #[default]
    Secondary,
    /// `destructive`: red.
    Destructive,
    /// `outline`: a border only.
    Outline,
}

/// How numbers are shown, like `Intl.NumberFormat`: a prefix (a currency
/// sign), a suffix and thousands grouping. Applied to number cells only.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NumberFormat {
    pub prefix: Text,
    pub suffix: Text,
    /// Group the integer digits in threes (`1,234,567`).
    pub group: bool,
}

impl NumberFormat {
    /// US dollars: `$1,234.50` (with the cell's own decimals).
    #[must_use]
    pub const fn usd() -> Self {
        Self { prefix: Cow::Borrowed("$"), suffix: Cow::Borrowed(""), group: true }
    }

    /// Whether this format changes nothing.
    #[must_use]
    pub fn is_plain(&self) -> bool {
        self.prefix.is_empty() && self.suffix.is_empty() && !self.group
    }
}

/// A pinned column.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ColumnFixed {
    /// Stays at the left edge while the table scrolls horizontally.
    Left,
}

/// A header cell spanning several columns (gpui-component's `ColumnGroup`).
#[derive(Debug, Clone)]
pub struct ColumnGroup {
    pub label: Text,
    pub span: usize,
}

impl ColumnGroup {
    pub fn new(label: impl Into<Text>, span: usize) -> Self {
        Self { label: label.into(), span }
    }
}

/// A column, as the delegate describes it (gpui-component's fields and
/// builders). Widths are logical pixels.
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct Column {
    /// Identifies the column in your data (usually the field name).
    pub key: Text,
    /// The header text.
    pub name: Text,
    pub align: TextAlign,
    /// `None`: not sortable.
    pub sort: Option<ColumnSort>,
    pub width: f32,
    pub fixed: Option<ColumnFixed>,
    pub resizable: bool,
    pub movable: bool,
    pub selectable: bool,
    pub min_width: f32,
    pub max_width: f32,
    /// Hidden columns take no space and are never drawn.
    pub visible: bool,
    /// How the cells are drawn.
    pub render: CellRender,
    /// How numbers are shown.
    pub format: NumberFormat,
    /// A header click raises [`TableEvent::HeaderMenu`](crate::TableEvent::HeaderMenu)
    /// (shadcn's sortable header with a menu) instead of sorting at once.
    pub header_menu: bool,
}

impl Default for Column {
    fn default() -> Self {
        Self {
            key: Text::default(),
            name: Text::default(),
            align: TextAlign::Left,
            sort: None,
            width: 100.0,
            fixed: None,
            resizable: true,
            movable: true,
            selectable: true,
            min_width: 20.0,
            max_width: 1200.0,
            visible: true,
            render: CellRender::Text,
            format: NumberFormat { prefix: Cow::Borrowed(""), suffix: Cow::Borrowed(""), group: false },
            header_menu: false,
        }
    }
}

impl Column {
    /// `w` limited to this column's min and max; a `NaN` (or a min above the
    /// max) cannot make it through: the default width is used for a `NaN`.
    #[must_use]
    pub fn clamp_width(&self, w: f32) -> f32 {
        let w = if w.is_nan() { Self::default().width } else { w };
        let (lo, hi) = if self.min_width <= self.max_width { (self.min_width, self.max_width) } else { (self.max_width, self.max_width) };
        w.clamp(if lo.is_nan() { 0.0 } else { lo }, if hi.is_nan() { f32::MAX } else { hi })
    }

    /// This column's own width, limited as [`clamp_width`](Self::clamp_width) does.
    #[must_use]
    pub fn clamped_width(&self) -> f32 {
        self.clamp_width(self.width)
    }

    pub fn new(key: impl Into<Text>, name: impl Into<Text>) -> Self {
        Self { key: key.into(), name: name.into(), ..Self::default() }
    }
    /// Sortable (header click cycles ascending, descending, original order).
    #[must_use]
    pub const fn sortable(mut self) -> Self {
        self.sort = Some(ColumnSort::Default);
        self
    }
    #[must_use]
    pub const fn sort(mut self, sort: ColumnSort) -> Self {
        self.sort = Some(sort);
        self
    }
    #[must_use]
    pub const fn ascending(self) -> Self {
        self.sort(ColumnSort::Ascending)
    }
    #[must_use]
    pub const fn descending(self) -> Self {
        self.sort(ColumnSort::Descending)
    }
    #[must_use]
    pub const fn text_center(mut self) -> Self {
        self.align = TextAlign::Center;
        self
    }
    #[must_use]
    pub const fn text_right(mut self) -> Self {
        self.align = TextAlign::Right;
        self
    }
    #[must_use]
    pub fn width(mut self, width: impl Into<f32>) -> Self {
        self.width = width.into();
        self
    }
    #[must_use]
    pub const fn fixed(mut self, fixed: ColumnFixed) -> Self {
        self.fixed = Some(fixed);
        self
    }
    #[must_use]
    pub const fn fixed_left(self) -> Self {
        self.fixed(ColumnFixed::Left)
    }
    #[must_use]
    pub const fn resizable(mut self, on: bool) -> Self {
        self.resizable = on;
        self
    }
    #[must_use]
    pub const fn movable(mut self, on: bool) -> Self {
        self.movable = on;
        self
    }
    #[must_use]
    pub const fn selectable(mut self, on: bool) -> Self {
        self.selectable = on;
        self
    }
    #[must_use]
    pub fn min_width(mut self, w: impl Into<f32>) -> Self {
        self.min_width = w.into();
        self
    }
    #[must_use]
    pub fn max_width(mut self, w: impl Into<f32>) -> Self {
        self.max_width = w.into();
        self
    }
    #[must_use]
    pub const fn visible(mut self, on: bool) -> Self {
        self.visible = on;
        self
    }
    #[must_use]
    pub const fn render(mut self, render: CellRender) -> Self {
        self.render = render;
        self
    }
    #[must_use]
    pub fn format(mut self, format: NumberFormat) -> Self {
        self.format = format;
        self
    }
    #[must_use]
    pub const fn header_menu(mut self, on: bool) -> Self {
        self.header_menu = on;
        self
    }
}

/// Order, widths and edges of the columns on screen.
///
/// "View" indices count visible columns left to right: pinned ones first,
/// then the scrolling ones. "Source" indices are the delegate's.
#[derive(Debug, Clone, Default)]
pub struct ColumnLayout {
    cols: Vec<Column>,
    /// View index -> source index.
    order: Vec<u32>,
    /// Width by source index.
    widths: Vec<f32>,
    pinned: usize,
    /// Left edges within the pinned strip (`pinned + 1` entries).
    xf: Vec<f32>,
    /// Left edges within the scrolling strip.
    xs: Vec<f32>,
}

impl ColumnLayout {
    pub fn new(cols: Vec<Column>) -> Self {
        let mut l = Self { widths: cols.iter().map(Column::clamped_width).collect(), cols, ..Self::default() };
        l.reorder_default();
        l
    }

    /// Replace the column list (the delegate changed): widths of columns
    /// with the same key survive, and so does their order.
    pub fn reset(&mut self, cols: Vec<Column>) {
        let old: Vec<(Text, f32)> =
            self.order.iter().map(|&s| (self.cols[s as usize].key.clone(), self.widths[s as usize])).collect();
        *self = Self::new(cols);
        // keep the user's widths and order for the columns that remain
        let mut placed = 0;
        for (key, w) in old {
            if let Some(src) = self.cols.iter().position(|c| c.key == key && !key.is_empty()) {
                self.widths[src] = self.cols[src].clamp_width(w);
                if let Some(at) = self.order.iter().position(|&s| s as usize == src)
                    && at >= placed && self.group(src) == self.group(self.order[placed] as usize) {
                        self.order.swap(at, placed);
                        placed += 1;
                    }
            }
        }
        self.rebuild_edges();
    }

    fn group(&self, src: usize) -> bool {
        self.cols[src].fixed.is_some()
    }

    fn reorder_default(&mut self) {
        let vis = |c: &Column| c.visible;
        self.order = (0..self.cols.len() as u32)
            .filter(|&i| vis(&self.cols[i as usize]) && self.cols[i as usize].fixed.is_some())
            .chain((0..self.cols.len() as u32).filter(|&i| vis(&self.cols[i as usize]) && self.cols[i as usize].fixed.is_none()))
            .collect();
        self.rebuild_edges();
    }

    fn rebuild_edges(&mut self) {
        self.pinned = self.order.iter().take_while(|&&s| self.cols[s as usize].fixed.is_some()).count();
        let (xf, xs) = (&mut self.xf, &mut self.xs);
        xf.clear();
        xs.clear();
        xf.push(0.0);
        xs.push(0.0);
        for (v, &s) in self.order.iter().enumerate() {
            let strip = if v < self.pinned { &mut *xf } else { &mut *xs };
            let last = strip.last().copied().unwrap_or(0.0);
            strip.push(last + self.widths[s as usize]);
        }
    }

    /// Show the sort indicator on source column `src` only.
    pub fn mark_sorted(&mut self, src: usize, sort: ColumnSort) {
        for (i, c) in self.cols.iter_mut().enumerate() {
            if c.sort.is_some() {
                c.sort = Some(if i == src { sort } else { ColumnSort::Default });
            }
        }
    }

    /// Visible columns.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.order.len()
    }
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.order.is_empty()
    }
    /// Pinned columns (the first `pinned()` view columns).
    #[must_use]
    pub const fn pinned(&self) -> usize {
        self.pinned
    }
    /// The delegate column at view index `v`.
    #[must_use]
    pub fn column(&self, v: usize) -> &Column {
        &self.cols[self.order[v] as usize]
    }
    /// Source index of view column `v`.
    #[must_use]
    pub fn src(&self, v: usize) -> usize {
        self.order[v] as usize
    }
    /// View index of source column `s`, if visible.
    #[must_use]
    pub fn view_of(&self, s: usize) -> Option<usize> {
        self.order.iter().position(|&o| o as usize == s)
    }
    #[must_use]
    pub fn width(&self, v: usize) -> f32 {
        self.widths[self.order[v] as usize]
    }
    /// All widths in view order.
    #[must_use]
    pub fn widths(&self) -> Vec<f32> {
        (0..self.len()).map(|v| self.width(v)).collect()
    }
    /// Width of the pinned strip.
    #[must_use]
    pub fn pinned_width(&self) -> f32 {
        self.xf.last().copied().unwrap_or(0.0)
    }
    /// Width of the scrolling strip (the horizontal scroll extent).
    #[must_use]
    pub fn scroll_width(&self) -> f32 {
        self.xs.last().copied().unwrap_or(0.0)
    }
    /// Left edges within the pinned strip, `pinned + 1` entries.
    #[must_use]
    pub fn pinned_edges(&self) -> &[f32] {
        &self.xf
    }
    /// Left edges within the scrolling strip (`len - pinned + 1` entries).
    #[must_use]
    pub fn scroll_edges(&self) -> &[f32] {
        &self.xs
    }
    /// Left edge of view column `v` in its strip.
    #[must_use]
    pub fn left(&self, v: usize) -> f32 {
        if v < self.pinned { self.xf[v] } else { self.xs[v - self.pinned] }
    }

    /// Set the width of view column `v`, limited to its min and max.
    /// Returns whether it changed.
    pub fn set_width(&mut self, v: usize, w: f32) -> bool {
        let s = self.order[v] as usize;
        let c = &self.cols[s];
        let w = c.clamp_width(w);
        if (w - self.widths[s]).abs() < f32::EPSILON {
            return false;
        }
        self.widths[s] = w;
        self.rebuild_edges();
        true
    }

    /// Move view column `from` so it ends at `to` (inside its own strip).
    /// Returns whether the order changed.
    pub fn move_to(&mut self, from: usize, to: usize) -> bool {
        let n = self.len();
        if from >= n || to >= n || from == to || (from < self.pinned) != (to < self.pinned) {
            return false;
        }
        let s = self.order.remove(from);
        self.order.insert(to, s);
        self.rebuild_edges();
        true
    }

    /// The view column at strip x (`x` measured from the strip's left
    /// edge), by bisection.
    #[must_use]
    pub fn at_x(&self, pinned: bool, x: f32) -> Option<usize> {
        let (edges, base) = if pinned { (&self.xf, 0) } else { (&self.xs, self.pinned) };
        let n = edges.len().saturating_sub(1);
        if n == 0 || x < 0.0 || x >= edges[n] {
            return None;
        }
        Some(base + edges[1..].partition_point(|&r| r <= x))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cols() -> Vec<Column> {
        vec![
            Column::new("a", "A").width(50.0_f32),
            Column::new("b", "B").width(30.0_f32).fixed_left(),
            Column::new("c", "C").width(20.0_f32),
            Column::new("d", "D").width(40.0_f32).visible(false),
        ]
    }

    #[test]
    fn pinned_first_hidden_skipped() {
        let l = ColumnLayout::new(cols());
        assert_eq!(l.len(), 3);
        assert_eq!(l.pinned(), 1);
        assert_eq!(l.column(0).key, "b");
        assert_eq!(l.pinned_width(), 30.0);
        assert_eq!(l.scroll_width(), 70.0);
        assert_eq!(l.left(2), 50.0);
    }

    #[test]
    fn resize_clamps_and_moves_edges() {
        let mut l = ColumnLayout::new(cols());
        assert!(l.set_width(1, 5.0));
        assert_eq!(l.width(1), 20.0);
        assert_eq!(l.scroll_width(), 40.0);
        assert!(l.set_width(1, 1e6));
        assert_eq!(l.width(1), 1200.0);
        assert!(!l.set_width(1, 1e6));
    }

    #[test]
    fn move_stays_in_strip() {
        let mut l = ColumnLayout::new(cols());
        assert!(!l.move_to(0, 1)); // pinned -> scrolling
        assert!(l.move_to(2, 1));
        assert_eq!(l.column(1).key, "c");
        assert_eq!(l.left(1), 0.0);
        assert_eq!(l.at_x(false, 5.0), Some(1));
        assert_eq!(l.at_x(false, 25.0), Some(2));
        assert_eq!(l.at_x(false, 70.0), None);
        assert_eq!(l.at_x(true, 29.9), Some(0));
    }

    #[test]
    fn reset_keeps_user_widths() {
        let mut l = ColumnLayout::new(cols());
        l.set_width(2, 77.0);
        l.reset(cols());
        assert_eq!(l.width(2), 77.0);
    }

    #[test]
    fn hostile_widths_cannot_poison_the_edges() {
        let cols = vec![
            Column::new("a", "A").width(f32::NAN),
            Column::new("b", "B").width(50.0_f32).min_width(500.0_f32).max_width(10.0_f32),
            Column::new("c", "C").width(f32::INFINITY),
        ];
        let mut l = ColumnLayout::new(cols);
        assert!(l.scroll_width().is_finite());
        assert!(l.set_width(0, f32::NAN) || l.width(0).is_finite());
        l.reset(vec![Column::new("b", "B").min_width(500.0_f32).max_width(10.0_f32)]);
        assert!(l.width(0).is_finite());
        assert!((0..l.len()).all(|v| l.left(v).is_finite()));
    }

    #[test]
    fn sort_cycle() {
        assert_eq!(ColumnSort::Default.next().next().next(), ColumnSort::Default);
    }
}
