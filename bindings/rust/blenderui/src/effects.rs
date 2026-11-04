//! CSS effects that work on an element's pixels: `filter`,
//! `backdrop-filter`, `mix-blend-mode`, `clip-path`, `mask`, and transforms
//! beyond `translate` (`rotate`, `skew`, `scale`, `matrix`).
//!
//! The element's subtree is painted into its own display list, rasterized
//! offscreen with the canvas' own item code (`BK_canvas_rasterize`), then
//! filtered, clipped, masked, transformed and composited here, and drawn back
//! as one image item. That works the same under the CPU and GPU renderers;
//! results are cached by content, so a page that doesn't change costs
//! nothing.
//!
//! The formulas are the W3C ones (Filter Effects 1, Compositing and Blending
//! 1), which is what browsers implement. Blur is a three-pass box blur, the
//! Gaussian approximation browsers use. Resampling is bilinear over
//! premultiplied pixels, like Blender's `IMB_transform`.

use crate::color::Rgba;
use crate::geometry::{Length, Units};
use crate::style::{BoxEdge, Gradient, Pos};

// ==========================================================================
// Style values

/// One `filter` / `backdrop-filter` function. Amounts are fractions
/// (`50%` = 0.5); `hue-rotate` in degrees; lengths in px.
#[derive(Clone, Debug, PartialEq)]
pub enum Filter {
    Blur(f32),
    Brightness(f32),
    Contrast(f32),
    Grayscale(f32),
    HueRotate(f32),
    Invert(f32),
    Opacity(f32),
    Saturate(f32),
    Sepia(f32),
    DropShadow { x: f32, y: f32, blur: f32, color: Rgba },
}

/// `mix-blend-mode` (and `background-blend-mode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Blend {
    #[default]
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
    PlusLighter,
}

impl Blend {
    pub fn parse(s: &str) -> Option<Blend> {
        Some(match s {
            "normal" => Blend::Normal,
            "multiply" => Blend::Multiply,
            "screen" => Blend::Screen,
            "overlay" => Blend::Overlay,
            "darken" => Blend::Darken,
            "lighten" => Blend::Lighten,
            "color-dodge" => Blend::ColorDodge,
            "color-burn" => Blend::ColorBurn,
            "hard-light" => Blend::HardLight,
            "soft-light" => Blend::SoftLight,
            "difference" => Blend::Difference,
            "exclusion" => Blend::Exclusion,
            "hue" => Blend::Hue,
            "saturation" => Blend::Saturation,
            "color" => Blend::Color,
            "luminosity" => Blend::Luminosity,
            "plus-lighter" => Blend::PlusLighter,
            _ => return None,
        })
    }
}

/// One `transform` function.
#[derive(Clone, Debug, PartialEq)]
pub enum TransformFn {
    Translate(Length, Length),
    Scale(f32, f32),
    /// radians
    Rotate(f32),
    /// radians
    Skew(f32, f32),
    Matrix([f32; 6]),
}

/// A `clip-path` shape radius: a length, or `closest-side` / `farthest-side`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShapeRadius {
    Len(Length),
    Closest,
    Farthest,
}

/// A `clip-path` basic shape, in its reference box.
#[derive(Clone, Debug, PartialEq)]
pub enum ClipShape {
    /// top, right, bottom, left insets and a corner radius
    Inset([Length; 4], Length),
    Circle(ShapeRadius, [Pos; 2]),
    Ellipse(ShapeRadius, ShapeRadius, [Pos; 2]),
    Polygon(Vec<(Length, Length)>),
    /// just the reference box (`clip-path: padding-box`)
    Box,
}

/// A mask or border-image source: a gradient or a PNG file.
#[derive(Clone, Debug, PartialEq)]
pub enum ImageSource {
    Gradient(Gradient),
    Url(std::sync::Arc<str>),
}

/// `mask` / `mask-image` and its placement.
#[derive(Clone, Debug, PartialEq)]
pub struct Mask {
    pub image: Option<ImageSource>,
    /// `mask-mode: luminance` (else alpha)
    pub luminance: bool,
    pub size: crate::style::BgSize,
    pub position: [Pos; 2],
    pub repeat: (bool, bool),
}

impl Default for Mask {
    fn default() -> Mask {
        Mask { image: None, luminance: false, size: crate::style::BgSize::Auto, position: [Pos::START; 2],
               repeat: (true, true) }
    }
}

/// A `border-image-slice` / `-width` / `-outset` component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Side {
    /// a plain number: image pixels (slice) or multiples of the border width
    Num(f32),
    Len(Length),
    Auto,
}

/// `border-image-repeat` along one axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Tile {
    #[default]
    Stretch,
    Repeat,
    Round,
    Space,
}

/// `border-image` and its longhands.
#[derive(Clone, Debug, PartialEq)]
pub struct BorderImage {
    pub source: Option<ImageSource>,
    pub slice: [Side; 4],
    pub fill: bool,
    pub width: [Side; 4],
    pub outset: [Side; 4],
    pub repeat: (Tile, Tile),
}

impl Default for BorderImage {
    fn default() -> BorderImage {
        BorderImage { source: None, slice: [Side::Len(Length::Percent(100.0)); 4], fill: false,
                      width: [Side::Num(1.0); 4], outset: [Side::Num(0.0); 4], repeat: (Tile::Stretch, Tile::Stretch) }
    }
}

/// The effects of one element (boxed in `Computed`: most elements have none).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Effects {
    pub filter: Vec<Filter>,
    pub backdrop: Vec<Filter>,
    pub blend: Blend,
    pub isolate: bool,
    pub clip_path: Option<(ClipShape, BoxEdge)>,
    pub mask: Option<Mask>,
    /// `transform` functions that need pixels (anything but translate)
    pub transform: Vec<TransformFn>,
    /// the individual `rotate` property, radians
    pub rotate: f32,
    /// the individual `scale` property when it isn't uniform
    pub scale2: Option<(f32, f32)>,
    /// `transform-origin` (default: the centre)
    pub origin: Option<[Pos; 2]>,
    pub border_image: Option<BorderImage>,
}

impl Effects {
    /// Whether the element must be painted into its own layer.
    pub fn needs_layer(&self) -> bool {
        !self.filter.is_empty() || self.blend != Blend::Normal || self.clip_path.is_some()
            || self.mask.as_ref().is_some_and(|m| m.image.is_some()) || !self.transform.is_empty()
            || self.rotate != 0.0 || self.scale2.is_some()
    }
}

// ==========================================================================
// Affine transforms: [a, b, c, d, e, f] maps (x, y) to (a x + c y + e, b x + d y + f)

pub type Affine = [f32; 6];

pub const IDENTITY: Affine = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// `m * n`: n applied first.
pub fn mul(m: &Affine, n: &Affine) -> Affine {
    [
        m[0] * n[0] + m[2] * n[1],
        m[1] * n[0] + m[3] * n[1],
        m[0] * n[2] + m[2] * n[3],
        m[1] * n[2] + m[3] * n[3],
        m[0] * n[4] + m[2] * n[5] + m[4],
        m[1] * n[4] + m[3] * n[5] + m[5],
    ]
}

pub fn apply(m: &Affine, x: f32, y: f32) -> (f32, f32) {
    (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
}

pub fn invert(m: &Affine) -> Option<Affine> {
    let det = m[0] * m[3] - m[1] * m[2];
    if det.abs() < 1e-9 {
        return None;
    }
    let (a, b, c, d) = (m[3] / det, -m[1] / det, -m[2] / det, m[0] / det);
    Some([a, b, c, d, -(a * m[4] + c * m[5]), -(b * m[4] + d * m[5])])
}

/// The matrix of a function list for a box of size `w` x `h` (percentages
/// of translate resolve against it), before the origin is applied.
pub fn matrix_of(fns: &[TransformFn], w: f32, h: f32, u: &Units) -> Affine {
    let mut m = IDENTITY;
    for f in fns {
        let n = match *f {
            TransformFn::Translate(x, y) => [1.0, 0.0, 0.0, 1.0, x.resolve(u, w), y.resolve(u, h)],
            TransformFn::Scale(x, y) => [x, 0.0, 0.0, y, 0.0, 0.0],
            TransformFn::Rotate(a) => [a.cos(), a.sin(), -a.sin(), a.cos(), 0.0, 0.0],
            TransformFn::Skew(ax, ay) => [1.0, ay.tan(), ax.tan(), 1.0, 0.0, 0.0],
            TransformFn::Matrix(v) => v,
        };
        m = mul(&m, &n);
    }
    m
}

// ==========================================================================
// Pixels: premultiplied linear-in-sRGB f32 RGBA, row-major

pub struct Pixels {
    pub w: usize,
    pub h: usize,
    pub px: Vec<[f32; 4]>,
}

impl Pixels {
    pub fn new(w: usize, h: usize) -> Pixels {
        Pixels { w, h, px: vec![[0.0; 4]; w * h] }
    }

    /// From straight-alpha RGBA8.
    pub fn from_rgba8(w: usize, h: usize, rgba: &[u8]) -> Pixels {
        let px = rgba.chunks_exact(4).map(|c| {
            let a = c[3] as f32 / 255.0;
            [c[0] as f32 / 255.0 * a, c[1] as f32 / 255.0 * a, c[2] as f32 / 255.0 * a, a]
        }).collect();
        Pixels { w, h, px }
    }

    /// To straight-alpha RGBA8.
    pub fn to_rgba8(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.px.len() * 4);
        for p in &self.px {
            let a = p[3].clamp(0.0, 1.0);
            let un = |v: f32| if a > 0.0 { (v / a).clamp(0.0, 1.0) } else { 0.0 };
            out.extend([(un(p[0]) * 255.0).round() as u8, (un(p[1]) * 255.0).round() as u8,
                        (un(p[2]) * 255.0).round() as u8, (a * 255.0).round() as u8]);
        }
        out
    }

    fn get(&self, x: isize, y: isize) -> [f32; 4] {
        if x < 0 || y < 0 || x >= self.w as isize || y >= self.h as isize {
            [0.0; 4]
        } else {
            self.px[y as usize * self.w + x as usize]
        }
    }

    /// Bilinear sample at a pixel-space position (pixel centres at +0.5).
    pub fn sample(&self, x: f32, y: f32) -> [f32; 4] {
        let (fx, fy) = (x - 0.5, y - 0.5);
        let (x0, y0) = (fx.floor(), fy.floor());
        let (tx, ty) = (fx - x0, fy - y0);
        let (x0, y0) = (x0 as isize, y0 as isize);
        let (a, b, c, d) = (self.get(x0, y0), self.get(x0 + 1, y0), self.get(x0, y0 + 1), self.get(x0 + 1, y0 + 1));
        let mut o = [0.0; 4];
        for k in 0..4 {
            let top = a[k] + (b[k] - a[k]) * tx;
            let bot = c[k] + (d[k] - c[k]) * tx;
            o[k] = top + (bot - top) * ty;
        }
        o
    }
}

// -- filters ---------------------------------------------------------------------

/// Box sizes for three box blurs approximating a Gaussian of `sigma`.
fn boxes(sigma: f32) -> [usize; 3] {
    let n = 3.0;
    let w_ideal = (12.0 * sigma * sigma / n + 1.0).sqrt();
    let mut wl = w_ideal.floor() as i32;
    if wl % 2 == 0 {
        wl -= 1;
    }
    let wu = wl + 2;
    let m_ideal = (12.0 * sigma * sigma - n * (wl * wl) as f32 - 4.0 * n * wl as f32 - 3.0 * n)
        / (-4.0 * wl as f32 - 4.0);
    let m = m_ideal.round() as i32;
    let mut out = [0usize; 3];
    for (i, o) in out.iter_mut().enumerate() {
        *o = (if (i as i32) < m { wl } else { wu }).max(1) as usize;
    }
    out
}

fn box_pass(src: &[[f32; 4]], dst: &mut [[f32; 4]], w: usize, h: usize, r: usize, horizontal: bool) {
    let (len, lines) = if horizontal { (w, h) } else { (h, w) };
    let idx = |line: usize, i: usize| if horizontal { line * w + i } else { i * w + line };
    let norm = 1.0 / (2 * r + 1) as f32;
    for line in 0..lines {
        let mut acc = [0.0f32; 4];
        // window over [i - r, i + r], transparent outside
        for i in 0..=r.min(len.saturating_sub(1)) {
            let p = src[idx(line, i)];
            for k in 0..4 {
                acc[k] += p[k];
            }
        }
        for i in 0..len {
            let o = &mut dst[idx(line, i)];
            for k in 0..4 {
                o[k] = acc[k] * norm;
            }
            if i + r + 1 < len {
                let p = src[idx(line, i + r + 1)];
                for k in 0..4 {
                    acc[k] += p[k];
                }
            }
            if i >= r {
                let p = src[idx(line, i - r)];
                for k in 0..4 {
                    acc[k] -= p[k];
                }
            }
        }
    }
}

/// Gaussian blur with standard deviation `sigma` (device pixels).
pub fn blur(p: &mut Pixels, sigma: f32) {
    if sigma < 0.3 || p.w == 0 || p.h == 0 {
        return;
    }
    let mut tmp = vec![[0.0f32; 4]; p.px.len()];
    for b in boxes(sigma) {
        let r = (b - 1) / 2;
        box_pass(&p.px, &mut tmp, p.w, p.h, r, true);
        box_pass(&tmp, &mut p.px, p.w, p.h, r, false);
    }
}

/// A 4x5 colour matrix (rows r, g, b, a; columns r g b a offset) over
/// unpremultiplied colour, as Filter Effects' feColorMatrix.
fn color_matrix(p: &mut Pixels, m: &[[f32; 5]; 4]) {
    for px in &mut p.px {
        let a = px[3];
        if a <= 0.0 {
            continue;
        }
        let c = [px[0] / a, px[1] / a, px[2] / a, a];
        let mut o = [0.0f32; 4];
        for (r, row) in m.iter().enumerate() {
            o[r] = (row[0] * c[0] + row[1] * c[1] + row[2] * c[2] + row[3] * c[3] + row[4]).clamp(0.0, 1.0);
        }
        *px = [o[0] * o[3], o[1] * o[3], o[2] * o[3], o[3]];
    }
}

fn diag(r: f32, g: f32, b: f32, a: f32, off: f32) -> [[f32; 5]; 4] {
    [[r, 0.0, 0.0, 0.0, off], [0.0, g, 0.0, 0.0, off], [0.0, 0.0, b, 0.0, off], [0.0, 0.0, 0.0, a, 0.0]]
}

fn rgb3(m: [[f32; 3]; 3]) -> [[f32; 5]; 4] {
    [[m[0][0], m[0][1], m[0][2], 0.0, 0.0], [m[1][0], m[1][1], m[1][2], 0.0, 0.0],
     [m[2][0], m[2][1], m[2][2], 0.0, 0.0], [0.0, 0.0, 0.0, 1.0, 0.0]]
}

/// Apply one filter function; `scale` = device pixels per CSS px.
pub fn apply_filter(p: &mut Pixels, f: &Filter, scale: f32) {
    match *f {
        Filter::Blur(r) => blur(p, r * scale),
        Filter::Brightness(b) => color_matrix(p, &diag(b, b, b, 1.0, 0.0)),
        Filter::Contrast(c) => color_matrix(p, &diag(c, c, c, 1.0, 0.5 - 0.5 * c)),
        Filter::Opacity(o) => p.px.iter_mut().for_each(|px| px.iter_mut().for_each(|v| *v *= o.clamp(0.0, 1.0))),
        Filter::Invert(a) => {
            let a = a.clamp(0.0, 1.0);
            color_matrix(p, &diag(1.0 - 2.0 * a, 1.0 - 2.0 * a, 1.0 - 2.0 * a, 1.0, a))
        }
        Filter::Grayscale(a) => {
            let s = 1.0 - a.clamp(0.0, 1.0);
            color_matrix(p, &rgb3([
                [0.2126 + 0.7874 * s, 0.7152 - 0.7152 * s, 0.0722 - 0.0722 * s],
                [0.2126 - 0.2126 * s, 0.7152 + 0.2848 * s, 0.0722 - 0.0722 * s],
                [0.2126 - 0.2126 * s, 0.7152 - 0.7152 * s, 0.0722 + 0.9278 * s],
            ]))
        }
        Filter::Sepia(a) => {
            let s = 1.0 - a.clamp(0.0, 1.0);
            color_matrix(p, &rgb3([
                [0.393 + 0.607 * s, 0.769 - 0.769 * s, 0.189 - 0.189 * s],
                [0.349 - 0.349 * s, 0.686 + 0.314 * s, 0.168 - 0.168 * s],
                [0.272 - 0.272 * s, 0.534 - 0.534 * s, 0.131 + 0.869 * s],
            ]))
        }
        Filter::Saturate(s) => color_matrix(p, &rgb3([
            [0.213 + 0.787 * s, 0.715 - 0.715 * s, 0.072 - 0.072 * s],
            [0.213 - 0.213 * s, 0.715 + 0.285 * s, 0.072 - 0.072 * s],
            [0.213 - 0.213 * s, 0.715 - 0.715 * s, 0.072 + 0.928 * s],
        ])),
        Filter::HueRotate(deg) => {
            let (c, s) = (deg.to_radians().cos(), deg.to_radians().sin());
            color_matrix(p, &rgb3([
                [0.213 + c * 0.787 - s * 0.213, 0.715 - c * 0.715 - s * 0.715, 0.072 - c * 0.072 + s * 0.928],
                [0.213 - c * 0.213 + s * 0.143, 0.715 + c * 0.285 + s * 0.140, 0.072 - c * 0.072 - s * 0.283],
                [0.213 - c * 0.213 - s * 0.787, 0.715 - c * 0.715 + s * 0.715, 0.072 + c * 0.928 + s * 0.072],
            ]))
        }
        Filter::DropShadow { x, y, blur: b, color } => {
            // the source's alpha, coloured, blurred and offset, under the source
            let (dx, dy) = ((x * scale).round() as isize, (y * scale).round() as isize);
            let mut sh = Pixels::new(p.w, p.h);
            for yy in 0..p.h as isize {
                for xx in 0..p.w as isize {
                    let a = p.get(xx - dx, yy - dy)[3] * color.a;
                    sh.px[yy as usize * p.w + xx as usize] = [color.r * a, color.g * a, color.b * a, a];
                }
            }
            blur(&mut sh, b * scale / 2.0);
            for (s, d) in p.px.iter_mut().zip(&sh.px) {
                let ia = 1.0 - s[3];
                for k in 0..4 {
                    s[k] += d[k] * ia;
                }
            }
        }
    }
}

/// How far a filter list can spread pixels past the element (CSS px).
pub fn filter_outset(filters: &[Filter]) -> f32 {
    let mut o = 0.0f32;
    for f in filters {
        o += match *f {
            Filter::Blur(r) => 3.0 * r,
            Filter::DropShadow { x, y, blur, .. } => x.abs().max(y.abs()) + 1.5 * blur,
            _ => 0.0,
        };
    }
    o.ceil()
}

// -- compositing (W3C Compositing and Blending 1) ------------------------------------

fn lum(c: [f32; 3]) -> f32 {
    0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2]
}

fn clip_color(c: [f32; 3]) -> [f32; 3] {
    let l = lum(c);
    let n = c[0].min(c[1]).min(c[2]);
    let x = c[0].max(c[1]).max(c[2]);
    let mut o = c;
    if n < 0.0 {
        o = o.map(|v| l + (v - l) * l / (l - n).max(1e-6));
    }
    if x > 1.0 {
        o = o.map(|v| l + (v - l) * (1.0 - l) / (x - l).max(1e-6));
    }
    o
}

fn set_lum(c: [f32; 3], l: f32) -> [f32; 3] {
    let d = l - lum(c);
    clip_color([c[0] + d, c[1] + d, c[2] + d])
}

fn sat(c: [f32; 3]) -> f32 {
    c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2])
}

fn set_sat(c: [f32; 3], s: f32) -> [f32; 3] {
    let (mx, mn) = (c[0].max(c[1]).max(c[2]), c[0].min(c[1]).min(c[2]));
    if mx - mn <= 1e-6 {
        return [0.0; 3];
    }
    c.map(|v| (v - mn) * s / (mx - mn))
}

fn separable(mode: Blend, cb: f32, cs: f32) -> f32 {
    match mode {
        Blend::Multiply => cb * cs,
        Blend::Screen => cb + cs - cb * cs,
        Blend::Overlay => separable(Blend::HardLight, cs, cb),
        Blend::Darken => cb.min(cs),
        Blend::Lighten => cb.max(cs),
        Blend::ColorDodge => {
            if cb <= 0.0 { 0.0 } else if cs >= 1.0 { 1.0 } else { (cb / (1.0 - cs)).min(1.0) }
        }
        Blend::ColorBurn => {
            if cb >= 1.0 { 1.0 } else if cs <= 0.0 { 0.0 } else { 1.0 - ((1.0 - cb) / cs).min(1.0) }
        }
        Blend::HardLight => {
            if cs <= 0.5 { cb * 2.0 * cs } else { separable(Blend::Screen, cb, 2.0 * cs - 1.0) }
        }
        Blend::SoftLight => {
            if cs <= 0.5 {
                cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb)
            } else {
                let d = if cb <= 0.25 { ((16.0 * cb - 12.0) * cb + 4.0) * cb } else { cb.sqrt() };
                cb + (2.0 * cs - 1.0) * (d - cb)
            }
        }
        Blend::Difference => (cb - cs).abs(),
        Blend::Exclusion => cb + cs - 2.0 * cb * cs,
        _ => cs,
    }
}

/// B(Cb, Cs) for unpremultiplied colours.
fn blend_fn(mode: Blend, cb: [f32; 3], cs: [f32; 3]) -> [f32; 3] {
    match mode {
        Blend::Hue => set_lum(set_sat(cs, sat(cb)), lum(cb)),
        Blend::Saturation => set_lum(set_sat(cb, sat(cs)), lum(cb)),
        Blend::Color => set_lum(cs, lum(cb)),
        Blend::Luminosity => set_lum(cb, lum(cs)),
        _ => [separable(mode, cb[0], cs[0]), separable(mode, cb[1], cs[1]), separable(mode, cb[2], cs[2])],
    }
}

/// Composite `src` over `backdrop` (same size, premultiplied) with `mode`.
pub fn composite(backdrop: &Pixels, src: &Pixels, mode: Blend) -> Pixels {
    let mut out = Pixels::new(src.w, src.h);
    for ((o, b), s) in out.px.iter_mut().zip(&backdrop.px).zip(&src.px) {
        let (ab, as_) = (b[3], s[3]);
        if mode == Blend::PlusLighter {
            *o = [(b[0] + s[0]).min(1.0), (b[1] + s[1]).min(1.0), (b[2] + s[2]).min(1.0), (ab + as_).min(1.0)];
            continue;
        }
        let un = |p: &[f32; 4], a: f32| if a > 0.0 { [p[0] / a, p[1] / a, p[2] / a] } else { [0.0; 3] };
        let (cb, cs) = (un(b, ab), un(s, as_));
        let mixed = blend_fn(mode, cb, cs);
        for k in 0..3 {
            // co = cs (1 - ab) + cb (1 - as) + as ab B(Cb, Cs), premultiplied
            o[k] = s[k] * (1.0 - ab) + b[k] * (1.0 - as_) + as_ * ab * mixed[k];
        }
        o[3] = as_ + ab * (1.0 - as_);
    }
    out
}

/// Multiply every pixel by the coverage in `mask` (alpha, or luminance).
pub fn apply_mask(p: &mut Pixels, mask: &Pixels, luminance: bool) {
    for (d, m) in p.px.iter_mut().zip(&mask.px) {
        let f = if luminance {
            // premultiplied luminance = luminance x alpha
            0.2126 * m[0] + 0.7152 * m[1] + 0.0722 * m[2]
        } else {
            m[3]
        };
        for v in d.iter_mut() {
            *v *= f;
        }
    }
}

/// Resample `src` (whose top-left is at `origin` in device pixels) through
/// the affine `m` (device pixels). Returns the result and its top-left.
pub fn transform(src: &Pixels, origin: (f32, f32), m: &Affine) -> Option<(Pixels, (f32, f32))> {
    let inv = invert(m)?;
    let (x0, y0) = origin;
    let (x1, y1) = (x0 + src.w as f32, y0 + src.h as f32);
    let pts = [apply(m, x0, y0), apply(m, x1, y0), apply(m, x0, y1), apply(m, x1, y1)];
    let minx = pts.iter().map(|p| p.0).fold(f32::INFINITY, f32::min).floor();
    let miny = pts.iter().map(|p| p.1).fold(f32::INFINITY, f32::min).floor();
    let maxx = pts.iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max).ceil();
    let maxy = pts.iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max).ceil();
    let (w, h) = ((maxx - minx) as usize, (maxy - miny) as usize);
    if w == 0 || h == 0 || w > 8192 || h > 8192 {
        return None;
    }
    let mut out = Pixels::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let (sx, sy) = apply(&inv, minx + x as f32 + 0.5, miny + y as f32 + 0.5);
            out.px[y * w + x] = src.sample(sx - x0, sy - y0);
        }
    }
    Some((out, (minx, miny)))
}
