//! The few geometry and color types the core needs. Plain `f32` logical
//! pixels; adapters convert to their framework's types at the paint sink.

/// A point.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

/// A rectangle: left, top, width, height.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    #[must_use]
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
}

/// A color with straight alpha, channels in `0..=1`.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgba {
    pub const TRANSPARENT: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };

    #[must_use]
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Alpha multiplied by `factor`.
    #[must_use]
    pub fn opacity(self, factor: f32) -> Self {
        Self { a: self.a * factor, ..self }
    }

    /// Blender's `shade`: add `offset / 255` to each color channel.
    #[must_use]
    pub fn shade(self, offset: f32) -> Self {
        let o = offset / 255.0;
        Self { r: (self.r + o).clamp(0.0, 1.0), g: (self.g + o).clamp(0.0, 1.0), b: (self.b + o).clamp(0.0, 1.0), a: self.a }
    }
}
