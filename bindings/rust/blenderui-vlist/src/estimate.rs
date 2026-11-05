//! Height estimates for items nobody has measured yet (an *Estimated* block
//! keeps only a count and the text bytes it holds). Blender's text view has
//! no such thing: it wraps every line on the spot and is O(n) for totals
//! (`textview_draw` in `editors/space_info/textview.cc`). Here the estimate
//! is `pad + lines * line_height`, where `lines` comes from the text size
//! and the wrap width, and a calibration factor learned from every real
//! measurement pulls it toward the truth ([`Estimator::observe`]), so the
//! scrollbar converges as the user scrolls.

use crate::fx::{px_to_units, Fx};

/// Below this many samples the calibration is not trusted.
const MIN_SAMPLES: u32 = 24;
/// The calibration changes the index only when it moved this much (relative).
const RECALIBRATE_AT: f64 = 0.04;

/// See the module docs. All sizes in logical px.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Estimator {
    /// Fixed height per item (padding, header ...).
    pub pad: f64,
    pub line_height: f64,
    /// Average advance of one text byte.
    pub char_width: f64,
    /// Width text wraps at.
    pub text_width: f64,
    calib: f64,
    applied: f64,
    est_sum: f64,
    real_sum: f64,
    samples: u32,
}

impl Estimator {
    /// A fresh estimator (calibration 1).
    #[must_use]
    pub const fn new(pad: f64, line_height: f64, char_width: f64, text_width: f64) -> Self {
        Self { pad, line_height, char_width, text_width, calib: 1.0, applied: 1.0, est_sum: 0.0, real_sum: 0.0, samples: 0 }
    }

    /// An estimator that assumes `one_line_px` for an item of one line of
    /// text (TanStack Virtual's `estimateSize`): what is measured later
    /// still calibrates it.
    #[must_use]
    pub const fn with_item_size(mut self, one_line_px: f64) -> Self {
        self.pad = (one_line_px - self.line_height).max(0.0);
        self
    }

    /// The same estimator at another wrap width, keeping what it learned.
    #[must_use]
    pub const fn with_text_width(mut self, w: f64) -> Self {
        self.text_width = w;
        self
    }

    fn lines_exact(&self, bytes: u64) -> f64 {
        (bytes as f64 * self.char_width / self.text_width.max(1.0)).ceil().max(1.0)
    }

    fn lines_mean(&self, avg_bytes: f64) -> f64 {
        // the mean of a ceiling is about x + 1/2 once x is past one line
        (avg_bytes * self.char_width / self.text_width.max(1.0) + 0.5).max(1.0)
    }

    /// Estimated height of one item with `bytes` of text, before calibration.
    #[must_use]
    pub fn item_raw(&self, bytes: u64) -> f64 {
        self.pad + self.lines_exact(bytes) * self.line_height
    }

    /// Estimated height of one item with `bytes` of text.
    #[must_use]
    pub fn item(&self, bytes: u64) -> Fx {
        Fx::from_px(self.item_raw(bytes) * self.applied)
    }

    /// Estimated height of `n` items holding `bytes` of text together, in
    /// fixed-point units (a block is far taller than one item).
    #[must_use]
    pub fn block(&self, n: u64, bytes: u64) -> u64 {
        if n == 0 {
            return 0;
        }
        let per = self.pad + self.lines_mean(bytes as f64 / n as f64) * self.line_height;
        px_to_units(per * self.applied * n as f64)
    }

    /// Learn from one measured item. Returns whether the calibration changed
    /// enough that every estimated block must be re-summed.
    pub fn observe(&mut self, bytes: u64, real_px: f64) -> bool {
        self.est_sum += self.item_raw(bytes);
        self.real_sum += real_px;
        self.samples = self.samples.saturating_add(1);
        if self.est_sum > 1e9 {
            // forget the oldest half: the estimator follows the text being read now
            self.est_sum *= 0.5;
            self.real_sum *= 0.5;
        }
        if self.samples < MIN_SAMPLES || self.est_sum <= 0.0 {
            return false;
        }
        self.calib = (self.real_sum / self.est_sum).clamp(0.25, 4.0);
        if (self.calib - self.applied).abs() > self.applied * RECALIBRATE_AT {
            self.applied = self.calib;
            return true;
        }
        false
    }

    /// The calibration factor in force.
    #[must_use]
    pub const fn calibration(&self) -> f64 {
        self.applied
    }
}
