//! Plain rows: TanStack Virtual's model. [`CountSource`] is "`count` items,
//! item `i` is whatever `text(i)` writes" (nothing is stored per item), and
//! [`RowStyle`] draws and measures a row of wrapped text. Together they are a
//! [`List`](crate::List) of any length with the same variable-height engine
//! as the chat; the chat's own look is [`ChatStyle`](crate::ChatStyle).

use std::ops::Range;

use blenderui_paint::{PaintSink, Palette, Rect};

use crate::estimate::Estimator;
use crate::list::List;
use crate::measure::Measurer;
use crate::paint::ItemPainter;
use crate::scrollbar::WIDTH as SCROLL_W;
use crate::source::{ItemSource, Page, Record};
use crate::wrap::Lines;

/// Writes the text of item `i` into the buffer.
pub type ItemText = Box<dyn Fn(usize, &mut String)>;

/// Maps an item index to its stable key; keys must ascend with the index.
pub type ItemKey = Box<dyn Fn(usize) -> u64>;

/// `count` items whose text is computed when they are fetched (the visible
/// ones only). The key of item `i` is `i` unless [`with_key`](Self::with_key)
/// says otherwise (TanStack Virtual's `getItemKey`).
pub struct CountSource {
    count: usize,
    text: ItemText,
    key: Option<ItemKey>,
    /// Bytes of text an item is assumed to have, for height estimates.
    avg_bytes: u64,
    version: u64,
}

impl CountSource {
    #[must_use]
    pub fn new(count: usize, text: impl Fn(usize, &mut String) + 'static) -> Self {
        Self { count, text: Box::new(text), key: None, avg_bytes: 80, version: 0 }
    }

    /// `getItemKey`: stable keys that survive reordering of the underlying
    /// data. They must be strictly ascending with the index.
    #[must_use]
    pub fn with_key(mut self, key: impl Fn(usize) -> u64 + 'static) -> Self {
        self.key = Some(Box::new(key));
        self
    }

    /// Replace how item text is computed (a new render's closure).
    pub fn set_text(&mut self, text: impl Fn(usize, &mut String) + 'static) {
        self.text = Box::new(text);
    }

    /// Replace the key function (`None`: the index is the key).
    pub fn set_key(&mut self, key: Option<ItemKey>) {
        self.key = key;
    }

    /// The bytes of text an item is assumed to have before it is measured.
    #[must_use]
    pub const fn with_avg_bytes(mut self, bytes: u64) -> Self {
        self.avg_bytes = bytes;
        self
    }

    /// Change the count; returns the old one. The owner tells the view
    /// ([`crate::ListView::appended`] / [`removed`](crate::ListView::removed)).
    pub const fn set_count(&mut self, count: usize) -> usize {
        self.version += 1;
        std::mem::replace(&mut self.count, count)
    }

    /// Bytes an item is assumed to have (for the view's calls).
    #[must_use]
    pub const fn avg_bytes(&self) -> u64 {
        self.avg_bytes
    }

    fn key_of(&self, i: usize) -> u64 {
        self.key.as_ref().map_or(i as u64, |k| k(i))
    }
}

impl ItemSource for CountSource {
    fn len(&self) -> usize {
        self.count
    }

    fn version(&self) -> u64 {
        self.version
    }

    fn bytes(&self, r: Range<usize>) -> u64 {
        let (a, b) = (r.start.min(self.count), r.end.min(self.count));
        (b - a.min(b)) as u64 * self.avg_bytes
    }

    fn fetch(&self, range: Range<usize>, page: &mut Page) {
        let end = range.end.min(self.count);
        page.reset(range.start);
        for i in range.start..end.max(range.start) {
            // the text is written straight into the page's buffer
            let a = page.text_mut().len();
            (self.text)(i, page.text_mut());
            let b = page.text_mut().len();
            page.push_span(&Record { id: self.key_of(i), time: 0, sender: 0, flags: 0, media_h: 0, text: "", media: "" }, a..b);
        }
    }

    fn id_at(&self, i: usize) -> Option<u64> {
        (i < self.count).then(|| self.key_of(i))
    }

    fn index_of(&self, id: u64) -> Option<usize> {
        // keys ascend with the index: a binary search
        let (mut lo, mut hi) = (0usize, self.count);
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            match self.key_of(mid).cmp(&id) {
                std::cmp::Ordering::Less => lo = mid + 1,
                std::cmp::Ordering::Equal => return Some(mid),
                std::cmp::Ordering::Greater => hi = mid,
            }
        }
        None
    }
}

/// How a plain row measures and paints: wrapped text between two paddings,
/// a hairline under it, a tint when selected.
#[derive(Clone, Debug)]
pub struct RowStyle {
    pub pal: Palette,
    pub font_size: f32,
    pub line_height: f32,
    pub pad_x: f32,
    pub pad_y: f32,
    /// Key of the selected item.
    pub selected: Option<u64>,
}

impl RowStyle {
    #[must_use]
    pub const fn new(pal: Palette) -> Self {
        Self { pal, font_size: 14.0, line_height: 19.0, pad_x: 12.0, pad_y: 8.0, selected: None }
    }
}

impl Measurer for RowStyle {
    fn font_size(&self) -> f32 {
        self.font_size
    }

    fn line_height(&self) -> f32 {
        self.line_height
    }

    fn wrap_width(&self, _: &Record<'_>, list_width: f32) -> f32 {
        (list_width - SCROLL_W - 2.0 * self.pad_x).max(40.0)
    }

    fn chrome(&self, _: &Record<'_>) -> f32 {
        2.0 * self.pad_y + 1.0
    }

    fn estimator(&self, list_width: f32) -> Estimator {
        let w = f64::from(self.wrap_width(&Record { id: 0, time: 0, sender: 0, flags: 0, media_h: 0, text: "", media: "" }, list_width));
        Estimator::new(f64::from(2.0 * self.pad_y + 1.0), f64::from(self.line_height), f64::from(self.font_size) * 0.5, w)
    }
}

impl ItemPainter for RowStyle {
    fn palette(&self) -> &Palette {
        &self.pal
    }

    fn item(&self, sink: &mut dyn PaintSink, r: Rect, rec: &Record<'_>, lines: &Lines) {
        let pal = self.pal;
        if self.selected == Some(rec.id) {
            sink.fill(r, pal.sel.opacity(0.35));
        }
        let mut y = r.y + self.pad_y;
        for i in 0..lines.count() {
            sink.text(self.pad_x, y + self.line_height * 0.5, lines.line(rec.text, i), self.font_size, pal.text);
            y += self.line_height;
        }
        sink.fill(Rect::new(0.0, r.y + r.h - 1.0, r.w - SCROLL_W, 1.0), pal.line);
    }

    fn skeleton(&self, sink: &mut dyn PaintSink, r: Rect, _: Option<&Record<'_>>) {
        let dim = self.pal.dim.opacity(0.18);
        sink.round(Rect::new(self.pad_x, r.y + self.pad_y + 3.0, (r.w - 2.0 * self.pad_x - SCROLL_W).max(20.0) * 0.6, 10.0), 5.0, dim);
    }
}

/// A list of plain rows ([`CountSource`] drawn by [`RowStyle`]).
pub type RowList = List<CountSource, RowStyle>;

#[cfg(test)]
mod tests {
    use super::*;

    fn tw(s: &str, size: f32) -> f32 {
        s.chars().count() as f32 * size * 0.5
    }

    fn rows(n: usize) -> RowList {
        let src = CountSource::new(n, |i, out| {
            use std::fmt::Write;
            let _ = write!(out, "row {i} {}", "x".repeat(i % 7 * 30));
        });
        List::new(src, RowStyle::new(Palette::blender_dark()), 500.0, 300.0, crate::Options::default()).unwrap()
    }

    #[test]
    fn a_count_source_fetches_only_what_is_shown() {
        let mut l = rows(10_000_000);
        let placed = l.layout(&mut tw).to_vec();
        assert_eq!(placed[0].item, 0);
        assert!(placed.len() < 100, "only the viewport is placed: {}", placed.len());
        let mut p = Page::new();
        l.source.fetch(5..8, &mut p);
        assert_eq!(p.record(6).unwrap().text, format!("row 6 {}", "x".repeat(6 * 30)));
        assert_eq!(p.record(7).unwrap().id, 7);
    }

    #[test]
    fn keys_index_and_count_changes() {
        let mut l = rows(1000);
        assert_eq!(l.source.id_at(10), Some(10));
        assert_eq!(l.source.index_of(999), Some(999));
        assert_eq!(l.source.index_of(1000), None);
        l.source = CountSource::new(100, |i, out| out.push_str(&i.to_string())).with_key(|i| i as u64 * 10 + 5);
        assert_eq!(l.source.id_at(3), Some(35));
        assert_eq!(l.source.index_of(35), Some(3));
        assert_eq!(l.source.index_of(36), None);
        let old = l.source.set_count(150);
        assert_eq!((old, l.source.len()), (100, 150));
        assert_eq!(l.source.bytes(0..10), 800);
        l.view.appended(50, 50 * l.source.avg_bytes()).unwrap();
        l.view.jump_to(149, crate::Align::End);
        l.layout(&mut tw);
        assert_eq!(l.view.placed().last().unwrap().item, 149);
    }
}
