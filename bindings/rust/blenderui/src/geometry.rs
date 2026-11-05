//! Units and geometry, named as in gpui: `px`, `rems`, `relative`, `size`,
//! `point`, `Bounds`.

use std::ops::{Add, Mul, Sub};

/// Logical pixels (scaled by the monitor's DPI when drawn).
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Default)]
pub struct Pixels(pub f32);

/// Multiples of the root font size (16 px, as in gpui and browsers).
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Rems(pub f32);

pub fn px(v: f32) -> Pixels {
    Pixels(v)
}
pub fn rems(v: f32) -> Rems {
    Rems(v)
}

/// A fraction of the parent (`relative(0.5)` = 50%).
pub fn relative(fraction: f32) -> Length {
    Length::Percent(fraction * 100.0)
}

/// A CSS length before resolution.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum Length {
    Px(f32),
    Rem(f32),
    Em(f32),
    Percent(f32),
    Vw(f32),
    Vh(f32),
    Vmin(f32),
    Vmax(f32),
    /// Any CSS math (`calc()`, `min()`, `clamp()` ...), interned.
    Calc(&'static crate::calc::Calc),
    #[default]
    Auto,
}

impl From<Pixels> for Length {
    fn from(p: Pixels) -> Self {
        Length::Px(p.0)
    }
}
impl From<Rems> for Length {
    fn from(r: Rems) -> Self {
        Length::Rem(r.0)
    }
}
impl From<f32> for Length {
    fn from(v: f32) -> Self {
        Length::Px(v)
    }
}

pub(crate) const ROOT_FONT_SIZE: f32 = 16.0;

/// What lengths resolve against.
#[derive(Clone, Copy, Debug)]
pub struct Units {
    pub font_size: f32,
    pub viewport: (f32, f32),
}

impl Length {
    /// Absolute pixels, or None for percentages and `auto` (left to layout).
    pub(crate) fn to_px(self, u: &Units) -> Option<f32> {
        match self {
            Length::Px(v) => Some(v),
            Length::Rem(v) => Some(v * ROOT_FONT_SIZE),
            Length::Em(v) => Some(v * u.font_size),
            Length::Vw(v) => Some(v * u.viewport.0 / 100.0),
            Length::Vh(v) => Some(v * u.viewport.1 / 100.0),
            Length::Vmin(v) => Some(v * u.viewport.0.min(u.viewport.1) / 100.0),
            Length::Vmax(v) => Some(v * u.viewport.0.max(u.viewport.1) / 100.0),
            Length::Calc(c) => c.to_px(u),
            Length::Percent(_) | Length::Auto => None,
        }
    }

    /// Pixels with percentages of `basis` (auto = 0).
    pub(crate) fn resolve(self, u: &Units, basis: f32) -> f32 {
        match self {
            Length::Percent(p) => p * basis / 100.0,
            Length::Calc(c) => c.eval_units(u, basis),
            Length::Auto => 0.0,
            other => other.to_px(u).unwrap_or(0.0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Point<T> {
    pub x: T,
    pub y: T,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Size<T> {
    pub width: T,
    pub height: T,
}

pub fn point<T>(x: T, y: T) -> Point<T> {
    Point { x, y }
}
pub fn size<T>(width: T, height: T) -> Size<T> {
    Size { width, height }
}

/// A rectangle: origin + size.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Bounds<T> {
    pub origin: Point<T>,
    pub size: Size<T>,
}

impl Bounds<Pixels> {
    /// A window of `size` centered on the primary screen (gpui's helper).
    pub fn centered(_display: Option<()>, size: Size<Pixels>, _cx: &crate::App) -> Self {
        Bounds { origin: point(px(0.0), px(0.0)), size }
    }
}

impl Bounds<f32> {
    pub(crate) fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.origin.x
            && y >= self.origin.y
            && x < self.origin.x + self.size.width
            && y < self.origin.y + self.size.height
    }

    pub(crate) fn intersect(&self, o: &Bounds<f32>) -> Bounds<f32> {
        let x0 = self.origin.x.max(o.origin.x);
        let y0 = self.origin.y.max(o.origin.y);
        let x1 = (self.origin.x + self.size.width).min(o.origin.x + o.size.width);
        let y1 = (self.origin.y + self.size.height).min(o.origin.y + o.size.height);
        Bounds { origin: point(x0, y0), size: size((x1 - x0).max(0.0), (y1 - y0).max(0.0)) }
    }
}

pub(crate) fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds<f32> {
    Bounds { origin: point(x, y), size: size(w, h) }
}

impl Add for Pixels {
    type Output = Pixels;
    fn add(self, o: Pixels) -> Pixels {
        Pixels(self.0 + o.0)
    }
}
impl Sub for Pixels {
    type Output = Pixels;
    fn sub(self, o: Pixels) -> Pixels {
        Pixels(self.0 - o.0)
    }
}
impl Mul<f32> for Pixels {
    type Output = Pixels;
    fn mul(self, k: f32) -> Pixels {
        Pixels(self.0 * k)
    }
}

impl From<Pixels> for f32 {
    fn from(p: Pixels) -> Self {
        p.0
    }
}
