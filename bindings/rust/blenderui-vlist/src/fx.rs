//! Heights as fixed-point integers: 1/64 px in a `u32`.
//!
//! Absolute offsets are sums of heights, so they are integers too (`u64`,
//! never `f32`: 10M rows of 60 px are 600M px, past where `f32` keeps a
//! pixel). Integers also make the index exact: a height set and read back
//! is the same number, and the sums never drift. An item is clamped to
//! [`Fx::MAX`] px so a block of [`crate::index::MAX_BLOCK`] items cannot
//! overflow its `u32` prefix sums; NaN and negative inputs become 0. This is
//! the validation at the boundary: past `Fx::from_px` every value is legal.

/// Fixed-point units per pixel.
pub const UNITS_PER_PX: f64 = 64.0;

/// A legal item height.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fx(u32);

impl Fx {
    pub const ZERO: Self = Self(0);
    /// The tallest item: 16384 px.
    pub const MAX: Self = Self(16384 * 64);

    /// From pixels: NaN and negative become 0, too tall becomes [`Fx::MAX`].
    #[must_use]
    pub fn from_px(px: f64) -> Self {
        if px.is_nan() || px <= 0.0 {
            return Self::ZERO;
        }
        Self(((px * UNITS_PER_PX).round().min(f64::from(Self::MAX.0))) as u32)
    }

    /// From raw units (clamped).
    #[must_use]
    pub fn from_units(units: u64) -> Self {
        Self(units.min(u64::from(Self::MAX.0)) as u32)
    }

    #[must_use]
    pub const fn units(self) -> u32 {
        self.0
    }

    #[must_use]
    pub fn px(self) -> f64 {
        f64::from(self.0) / UNITS_PER_PX
    }
}

/// Fixed-point units to pixels (offsets and totals).
#[must_use]
pub fn units_to_px(units: u64) -> f64 {
    units as f64 / UNITS_PER_PX
}

/// Pixels to fixed-point units, saturating; NaN and negative are 0.
#[must_use]
pub fn px_to_units(px: f64) -> u64 {
    if px.is_nan() || px <= 0.0 {
        return 0;
    }
    (px * UNITS_PER_PX).round().min(u64::MAX as f64 / 2.0) as u64
}
