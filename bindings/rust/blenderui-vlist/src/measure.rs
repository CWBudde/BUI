//! Measuring an item: its height from its own record and the list width.
//!
//! A [`Measurer`] describes the item's layout (how wide its text may be,
//! how tall everything that is not text is); [`TextLayout`] does the
//! wrapping and remembers it. The wrapped lines are cached per
//! `(text, wrap width, size)` in the shared bounded
//! [`TextCache`](blenderui_paint::TextCache) (the design of Blender's BLF
//! glyph cache, `blf_glyph.cc`): painting a frame is mostly cache hits, a
//! resize only refills what is on screen, and the cache never holds more
//! than its capacity. A style change (font, size, theme metrics) is
//! [`TextLayout::clear`].

use blenderui_paint::TextCache;

use crate::estimate::Estimator;
use crate::fx::Fx;
use crate::source::Record;
use crate::wrap::{wrap, Advances, Lines};

/// The text measurement the host provides: width of `text` at `size`.
pub type TextWidth<'a> = &'a mut dyn FnMut(&str, f32) -> f32;

/// How an item lays out. Heights depend on the record and the width only.
pub trait Measurer {
    fn font_size(&self) -> f32;
    fn line_height(&self) -> f32;

    /// The width `rec`'s text wraps at in a list `list_width` wide.
    fn wrap_width(&self, rec: &Record<'_>, list_width: f32) -> f32;

    /// Everything of `rec`'s height that is not wrapped text (padding,
    /// header, divider, image).
    fn chrome(&self, rec: &Record<'_>) -> f32;

    /// The estimator for blocks nobody measured, for this list width.
    fn estimator(&self, list_width: f32) -> Estimator;
}

/// Wrapping with a cache.
pub struct TextLayout {
    cache: TextCache<Lines>,
    adv: Advances,
}

impl TextLayout {
    /// A layout cache holding up to `capacity` wrapped texts.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self { cache: TextCache::new(capacity), adv: Advances::default() }
    }

    /// Start a frame (ages the cache).
    pub const fn next_generation(&mut self) {
        self.cache.next_generation();
    }

    /// Make room for a screenful and the next (call with the visible count).
    pub fn ensure_capacity(&mut self, n: usize) {
        self.cache.ensure_capacity(n);
    }

    /// Forget everything (style change).
    pub fn clear(&mut self) {
        self.cache.clear();
        self.adv.reset(0.0);
    }

    /// `(hits, misses)`.
    #[must_use]
    pub const fn stats(&self) -> (u64, u64) {
        self.cache.stats()
    }

    /// The wrapped lines of `rec`'s text at `list_width`.
    pub fn lines(&mut self, rec: &Record<'_>, m: &impl Measurer, list_width: f32, tw: TextWidth<'_>) -> &Lines {
        let (size, w) = (m.font_size(), m.wrap_width(rec, list_width));
        if (self.adv.size() - size).abs() > f32::EPSILON {
            self.adv.reset(size);
        }
        let adv = &mut self.adv;
        self.cache.get_or_insert_with(rec.text, w.to_bits(), size, || wrap(rec.text, w, adv, tw))
    }

    /// The wrapped lines if they are cached (painting reads through a shared
    /// reference; [`ListView::layout`](crate::ListView::layout) has cached what is on screen).
    #[must_use]
    pub fn peek(&self, rec: &Record<'_>, m: &impl Measurer, list_width: f32) -> Option<&Lines> {
        self.cache.peek(rec.text, m.wrap_width(rec, list_width).to_bits(), m.font_size())
    }

    /// `rec`'s full height at `list_width`.
    pub fn height(&mut self, rec: &Record<'_>, m: &impl Measurer, list_width: f32, tw: TextWidth<'_>) -> Fx {
        let lines = self.lines(rec, m, list_width, tw).count();
        Fx::from_px(f64::from(m.chrome(rec)) + lines as f64 * f64::from(m.line_height()))
    }
}
