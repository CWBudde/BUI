//! A chat list: [`ChatStyle`] is the [`Measurer`] and [`ItemPainter`] for
//! messages (avatar, name and time on the first message of a run, a date
//! divider on the first of a day, wrapped text, an image placeholder of the
//! height the record carries), and [`ChatList`] owns the messages and the
//! [`ListView`] together so the two cannot disagree about what changed.
//!
//! Neighbour-dependent layout is decided when a message is inserted
//! ([`classify`]) and stored in its flags, so a message's height depends on
//! its own record alone and any message can be measured without its
//! neighbours. Whole-message selection is here; selecting text across
//! messages is not done.

use std::fmt::Write as _;

use blenderui_paint::{PaintSink, Palette, Rect, Rgba};

use crate::estimate::Estimator;
use crate::list::List;
use crate::measure::Measurer;
use crate::scrollbar::WIDTH as SCROLL_W;
use crate::source::{ItemSource, Message, Record, VecSource, FLAG_DIVIDER, FLAG_HEADER};
use crate::view::Options;
use crate::wrap::Lines;
use crate::paint::ItemPainter;

/// Header and divider flags for a message after `prev` (its sender and
/// time; `None` for the first message): a new run starts when the sender
/// changes or after five quiet minutes, a divider when the day changes.
#[must_use]
pub const fn classify(prev: Option<(u16, u32)>, sender: u16, time: u32) -> u8 {
    match prev {
        None => FLAG_HEADER | FLAG_DIVIDER,
        Some((ps, pt)) => {
            let mut f = 0;
            if ps != sender || time.saturating_sub(pt) > 300 {
                f |= FLAG_HEADER;
            }
            if time / 86_400 != pt / 86_400 {
                f |= FLAG_DIVIDER | FLAG_HEADER;
            }
            f
        }
    }
}

/// Year, month, day of a day count since 1970-01-01 (Hinnant's `civil_from_days`).
const fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + (m <= 2) as i64, m, d)
}

/// Sizes and colors of a chat. Metrics in px.
#[derive(Clone, Debug)]
pub struct ChatStyle {
    pub pal: Palette,
    pub font_size: f32,
    pub line_height: f32,
    pub avatar: f32,
    pub pad_x: f32,
    pub header_h: f32,
    pub divider_h: f32,
    /// Id of the selected message.
    pub selected: Option<u64>,
}

impl ChatStyle {
    #[must_use]
    pub const fn new(pal: Palette) -> Self {
        Self { pal, font_size: 14.0, line_height: 19.0, avatar: 32.0, pad_x: 10.0, header_h: 20.0, divider_h: 30.0, selected: None }
    }

    fn text_left(&self) -> f32 {
        self.pad_x + self.avatar + 10.0
    }

    const fn top(&self, rec: &Record<'_>) -> f32 {
        if rec.flags & FLAG_HEADER != 0 { 10.0 } else { 1.0 }
    }

    fn avatar_color(&self, sender: u16) -> Rgba {
        const HUES: [(f32, f32, f32); 8] = [
            (0.30, 0.55, 0.85),
            (0.85, 0.45, 0.30),
            (0.35, 0.70, 0.45),
            (0.75, 0.40, 0.70),
            (0.85, 0.70, 0.30),
            (0.30, 0.70, 0.75),
            (0.70, 0.35, 0.40),
            (0.55, 0.55, 0.85),
        ];
        let (r, g, b) = HUES[usize::from(sender) % HUES.len()];
        Rgba::new(r, g, b, 1.0)
    }
}

impl Measurer for ChatStyle {
    fn font_size(&self) -> f32 {
        self.font_size
    }

    fn line_height(&self) -> f32 {
        self.line_height
    }

    fn wrap_width(&self, _rec: &Record<'_>, list_width: f32) -> f32 {
        (list_width - SCROLL_W - self.text_left() - self.pad_x).max(40.0)
    }

    fn chrome(&self, rec: &Record<'_>) -> f32 {
        let mut h = self.top(rec) + 3.0;
        if rec.flags & FLAG_HEADER != 0 {
            h += self.header_h;
        }
        if rec.flags & FLAG_DIVIDER != 0 {
            h += self.divider_h;
        }
        if rec.media_h > 0 {
            h += f32::from(rec.media_h) + 6.0;
        }
        h
    }

    fn estimator(&self, list_width: f32) -> Estimator {
        // a third of the messages start a run; images are rare
        let pad = f64::from(self.top(&Record { id: 0, time: 0, sender: 0, flags: 0, media_h: 0, text: "", media: "" }) + 3.0) + f64::from(self.header_h) * 0.33;
        let w = f64::from(self.wrap_width(&Record { id: 0, time: 0, sender: 0, flags: 0, media_h: 0, text: "", media: "" }, list_width));
        Estimator::new(pad, f64::from(self.line_height), f64::from(self.font_size) * 0.5, w)
    }
}

impl ItemPainter for ChatStyle {
    fn palette(&self) -> &Palette {
        &self.pal
    }

    fn item(&self, sink: &mut dyn PaintSink, r: Rect, rec: &Record<'_>, lines: &Lines) {
        let pal = self.pal;
        let mut y = r.y + self.top(rec);
        if self.selected == Some(rec.id) {
            sink.fill(r, pal.sel.opacity(0.35));
        }
        if rec.flags & FLAG_DIVIDER != 0 {
            let cy = y + self.divider_h * 0.5 - 4.0;
            sink.fill(Rect::new(self.pad_x, cy, r.w - SCROLL_W - 2.0 * self.pad_x, 1.0), pal.line);
            let (yy, mm, dd) = civil(i64::from(rec.time / 86_400));
            let mut label = StackStr::default();
            let _ = write!(label, "{yy}-{mm:02}-{dd:02}");
            let tw = sink.text_width(label.as_str(), self.font_size - 1.0);
            let x = (r.w - SCROLL_W - tw) * 0.5;
            sink.fill(Rect::new(x - 8.0, cy - 8.0, tw + 16.0, 17.0), pal.back);
            sink.text(x, cy, label.as_str(), self.font_size - 1.0, pal.dim);
            y += self.divider_h;
        }
        if rec.flags & FLAG_HEADER != 0 {
            let a = self.avatar;
            sink.round(Rect::new(self.pad_x, y, a, a), a * 0.5, self.avatar_color(rec.sender));
            let mut buf = StackStr::default();
            let _ = write!(buf, "{}", char::from(b'A' + (rec.sender % 26) as u8));
            let lw = sink.text_width(buf.as_str(), self.font_size + 1.0);
            sink.text(self.pad_x + (a - lw) * 0.5, y + a * 0.5, buf.as_str(), self.font_size + 1.0, Rgba::new(1.0, 1.0, 1.0, 1.0));
            let mut name = StackStr::default();
            let _ = write!(name, "User {}", rec.sender);
            let nx = self.text_left();
            sink.text(nx, y + self.header_h * 0.5, name.as_str(), self.font_size, self.avatar_color(rec.sender));
            let nw = sink.text_width(name.as_str(), self.font_size);
            let mut at = StackStr::default();
            let s = rec.time % 86_400;
            let _ = write!(at, "{:02}:{:02}", s / 3600, (s / 60) % 60);
            sink.text(nx + nw + 8.0, y + self.header_h * 0.5, at.as_str(), self.font_size - 2.0, pal.dim);
            y += self.header_h;
        }
        let x = self.text_left();
        for i in 0..lines.count() {
            sink.text(x, y + self.line_height * 0.5, lines.line(rec.text, i), self.font_size, pal.text);
            y += self.line_height;
        }
        if rec.media_h > 0 {
            let w = (r.w - SCROLL_W - x - self.pad_x).min(f32::from(rec.media_h) * 1.5);
            let at = Rect::new(x, y + 3.0, w, f32::from(rec.media_h));
            if rec.media.is_empty() {
                sink.round(at, 4.0, pal.dim.opacity(0.25));
                sink.text(x + 8.0, y + 3.0 + f32::from(rec.media_h) * 0.5, "image", self.font_size - 1.0, pal.dim);
            } else {
                sink.image(at, rec.media); // the height was known before it loads: nothing shifts
            }
        }
    }

    fn skeleton(&self, sink: &mut dyn PaintSink, r: Rect, rec: Option<&Record<'_>>) {
        let pal = self.pal;
        let y = r.y + rec.map_or(10.0, |rec| self.top(rec));
        let dim = pal.dim.opacity(0.18);
        sink.round(Rect::new(self.pad_x, y, self.avatar, self.avatar), self.avatar * 0.5, dim);
        let x = self.text_left();
        sink.round(Rect::new(x, y + 3.0, 90.0, 10.0), 5.0, dim);
        sink.round(Rect::new(x, y + 20.0, (r.w - x - 40.0).max(20.0) * 0.7, 10.0), 5.0, dim);
    }
}

/// Messages plus a view over them. All changes go through here, so the view
/// always knows what happened.
pub type ChatList = List<VecSource, ChatStyle>;

impl ChatList {
    /// A chat of `messages` (already classified, see [`crate::synth::messages`]).
    #[must_use]
    pub fn from_messages(messages: Vec<Message>, pal: Palette, width: f32, height: f32, opts: Options) -> Self {
        // 2^40 items is far past what a Vec holds
        Self::new(VecSource::from_messages(messages), ChatStyle::new(pal), width, height, opts).expect("a Vec cannot exceed the index capacity")
    }

    /// A new message at the end: its run/day flags are decided now.
    pub fn push(&mut self, mut m: Message) {
        let prev = self.source.len().checked_sub(1).and_then(|i| self.source.get(i)).map(|p| (p.sender, p.time));
        m.flags = classify(prev, m.sender, m.time);
        let bytes = m.text.len() as u64;
        self.source.push(m);
        let _ = self.view.appended(1, bytes);
    }

    /// A page of older messages at the start (classified, oldest first). The
    /// first message that was shown first keeps its flags.
    pub fn prepend(&mut self, older: Vec<Message>) {
        let (n, bytes) = (older.len(), older.iter().map(|m| m.text.len() as u64).sum());
        self.source.prepend(older);
        let _ = self.view.prepended(n, bytes);
    }

    /// Replace message `i`'s text (an edit, a streaming token).
    pub fn edit(&mut self, i: usize, text: &str) {
        let Some(old) = self.source.get(i).map(|m| m.text.len() as u64) else { return };
        self.source.set_text(i, text);
        self.view.edited(i, old, text.len() as u64);
    }

    /// Delete message `i`.
    pub fn remove(&mut self, i: usize) {
        if i < self.source.len() {
            let m = self.source.remove(i);
            self.view.removed(i, 1, m.text.len() as u64);
        }
    }

    /// Select message `item` (deselect it when it is selected already).
    pub fn toggle_selected(&mut self, item: usize) {
        let id = self.source.id_at(item);
        self.style.selected = if self.style.selected == id { None } else { id };
    }

}

/// A small fixed buffer for formatting a label without allocating.
#[derive(Default)]
struct StackStr {
    buf: [u8; 32],
    len: usize,
}

impl std::fmt::Write for StackStr {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        let end = self.len + s.len();
        self.buf.get_mut(self.len..end).ok_or(std::fmt::Error)?.copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}

impl StackStr {
    fn as_str(&self) -> &str {
        std::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }
}
