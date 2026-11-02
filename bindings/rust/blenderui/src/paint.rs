//! Painting: a frame of laid-out boxes becomes one display list on the kit
//! canvas. [`Painter`] is also what `canvas(|bounds, painter| ...)` draws with.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::CString;
use std::rc::Rc;

use taffy::prelude::*;

use crate::color::Rgba;
use crate::geometry::{rect, Bounds, Units};
use crate::layout::{BoxNode, Content, Frame, Replaced};
use crate::style::{Computed, DisplayKind, TextAlign, WhiteSpace};
use crate::text;
use blenderui_sys as sys;

/// RGBA pixels for `<img>` / `img().image_rgba(...)`.
#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    /// `width * height * 4` bytes, row-major, straight alpha.
    pub rgba: Vec<u8>,
}

impl Image {
    /// Decode a PNG file.
    pub fn from_png(path: &str) -> Option<Image> {
        let decoder = png::Decoder::new(std::fs::File::open(path).ok()?);
        let mut decoder = decoder;
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info().ok()?;
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).ok()?;
        let px = (info.width * info.height) as usize;
        let rgba = match info.color_type {
            png::ColorType::Rgba => buf[..px * 4].to_vec(),
            png::ColorType::Rgb => buf[..px * 3].chunks(3).flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
            png::ColorType::GrayscaleAlpha => {
                buf[..px * 2].chunks(2).flat_map(|c| [c[0], c[0], c[0], c[1]]).collect()
            }
            png::ColorType::Grayscale => buf[..px].iter().flat_map(|&g| [g, g, g, 255]).collect(),
            png::ColorType::Indexed => return None,
        };
        Some(Image { width: info.width, height: info.height, rgba })
    }

    /// Nearest-neighbour resample to `w` x `h`.
    fn resized(&self, w: u32, h: u32) -> Vec<u8> {
        if w == self.width && h == self.height {
            return self.rgba.clone();
        }
        let mut out = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            let sy = (y as u64 * self.height as u64 / h.max(1) as u64) as u32;
            for x in 0..w {
                let sx = (x as u64 * self.width as u64 / w.max(1) as u64) as u32;
                let s = ((sy * self.width + sx) * 4) as usize;
                let d = ((y * w + x) * 4) as usize;
                out[d..d + 4].copy_from_slice(&self.rgba[s..s + 4]);
            }
        }
        out
    }
}

thread_local! {
    static IMAGES: RefCell<HashMap<String, Option<Rc<Image>>>> = RefCell::new(HashMap::new());
}

/// `src` of an `<img>`: PNG files, decoded once.
pub(crate) fn load_image(src: &str) -> Option<Rc<Image>> {
    IMAGES.with(|m| {
        m.borrow_mut()
            .entry(src.to_owned())
            .or_insert_with(|| Image::from_png(src).map(Rc::new))
            .clone()
    })
}

#[derive(Clone)]
enum Kind {
    Rect { radius: f32, outline: Rgba, width: f32 },
    Oval { outline: Rgba, width: f32 },
    Line { width: f32 },
    Polygon,
    Arc { start: f32, extent: f32 },
    Text { text: CString, size: f32, flags: i32 },
    Image(Rc<Image>),
}

#[derive(Clone)]
struct Item {
    kind: Kind,
    coords: Vec<f32>,
    fill: Rgba,
    clip: Option<Bounds<f32>>,
}

/// Draws into a canvas display list, in logical pixels.
///
/// ```ignore
/// canvas(|bounds, p| {
///     p.fill(bounds, rgb(0x4772b3));
///     p.text(bounds.origin.x + 4.0, bounds.origin.y + 4.0, "hi", 13.0, white());
/// })
/// ```
#[derive(Default)]
pub struct Painter {
    items: Vec<Item>,
    clip: Option<Bounds<f32>>,
    opacity: f32,
}

impl Painter {
    pub(crate) fn new() -> Painter {
        Painter { items: Vec::new(), clip: None, opacity: 1.0 }
    }

    fn push(&mut self, kind: Kind, coords: Vec<f32>, fill: Rgba) {
        let fill = fill.opacity(self.opacity);
        let kind = match kind {
            Kind::Rect { radius, outline, width } => {
                Kind::Rect { radius, outline: outline.opacity(self.opacity), width }
            }
            Kind::Oval { outline, width } => Kind::Oval { outline: outline.opacity(self.opacity), width },
            k => k,
        };
        self.items.push(Item { kind, coords, fill, clip: self.clip });
    }

    fn corners(b: Bounds<f32>) -> Vec<f32> {
        vec![b.origin.x, b.origin.y, b.origin.x + b.size.width, b.origin.y + b.size.height]
    }

    /// A filled rectangle (gpui's `paint_quad(fill(bounds, color))`).
    pub fn fill(&mut self, bounds: Bounds<f32>, color: impl Into<Rgba>) {
        let c = color.into();
        if !c.is_transparent() && bounds.size.width > 0.0 && bounds.size.height > 0.0 {
            self.push(Kind::Rect { radius: 0.0, outline: Rgba::TRANSPARENT, width: 0.0 },
                      Self::corners(bounds), c);
        }
    }

    /// A rounded rectangle with an optional border (gpui's `quad`).
    pub fn quad(&mut self, bounds: Bounds<f32>, radius: f32, fill: impl Into<Rgba>,
                border: impl Into<Rgba>, border_width: f32) {
        let (fill, border) = (fill.into(), border.into());
        if fill.is_transparent() && (border.is_transparent() || border_width <= 0.0) {
            return;
        }
        let max_r = bounds.size.width.min(bounds.size.height) / 2.0;
        self.push(Kind::Rect { radius: radius.clamp(0.0, max_r.max(0.0)), outline: border,
                               width: border_width },
                  Self::corners(bounds), fill);
    }

    /// An ellipse inscribed in `bounds`.
    pub fn oval(&mut self, bounds: Bounds<f32>, fill: impl Into<Rgba>, outline: impl Into<Rgba>,
                width: f32) {
        self.push(Kind::Oval { outline: outline.into(), width }, Self::corners(bounds), fill.into());
    }

    /// A polyline through `points`.
    pub fn line(&mut self, points: &[(f32, f32)], color: impl Into<Rgba>, width: f32) {
        self.push(Kind::Line { width }, points.iter().flat_map(|&(x, y)| [x, y]).collect(), color.into());
    }

    /// A filled polygon (gpui's `paint_path`).
    pub fn polygon(&mut self, points: &[(f32, f32)], color: impl Into<Rgba>) {
        self.push(Kind::Polygon, points.iter().flat_map(|&(x, y)| [x, y]).collect(), color.into());
    }

    /// A pie slice: degrees counter-clockwise from 3 o'clock.
    pub fn arc(&mut self, bounds: Bounds<f32>, start: f32, extent: f32, color: impl Into<Rgba>) {
        self.push(Kind::Arc { start, extent }, Self::corners(bounds), color.into());
    }

    /// Text with its top-left at (x, y).
    pub fn text(&mut self, x: f32, y: f32, text: &str, size: f32, color: impl Into<Rgba>) {
        self.styled_text(x, y, text, size, 0, color.into());
    }

    pub(crate) fn styled_text(&mut self, x: f32, y: f32, text: &str, size: f32, flags: i32, color: Rgba) {
        if !text.trim().is_empty() {
            let text = CString::new(text).unwrap_or_else(|_| CString::new(text.replace('\0', "")).unwrap_or_default());
            self.push(Kind::Text { text, size, flags }, vec![x, y], color);
        }
    }

    /// An image scaled to `bounds`.
    pub fn image(&mut self, bounds: Bounds<f32>, image: &Rc<Image>) {
        self.push(Kind::Image(image.clone()), Self::corners(bounds), Rgba::TRANSPARENT);
    }

    /// Restrict the following drawing to `bounds` (intersected with any current clip).
    pub fn with_clip(&mut self, bounds: Bounds<f32>, f: impl FnOnce(&mut Painter)) {
        let saved = self.clip;
        self.clip = Some(match saved {
            Some(c) => c.intersect(&bounds),
            None => bounds,
        });
        f(self);
        self.clip = saved;
    }

    /// Replace the canvas display list with these items, scaled to device pixels.
    pub(crate) fn flush(&self, canvas: *mut sys::BKNode, scale: f32) {
        // SAFETY: `canvas` is a live kit canvas node owned by the window.
        unsafe { sys::BK_canvas_delete(canvas, -1) };
        for it in &self.items {
            let mut coords: Vec<f32> = it.coords.iter().map(|v| v * scale).collect();
            let mut ci = sys::BKCanvasItem { fill: it.fill.to_array(), ..Default::default() };
            let mut _keep_px = None;
            match &it.kind {
                Kind::Rect { radius, outline, width } => {
                    ci.type_ = if *radius > 0.0 { sys::BK_ITEM_ROUNDRECT } else { sys::BK_ITEM_RECT };
                    ci.radius = radius * scale;
                    if *width > 0.0 {
                        ci.outline = outline.to_array();
                        ci.width = (width * scale).max(1.0);
                    }
                }
                Kind::Oval { outline, width } => {
                    ci.type_ = sys::BK_ITEM_OVAL;
                    if *width > 0.0 {
                        ci.outline = outline.to_array();
                        ci.width = (width * scale).max(1.0);
                    }
                }
                Kind::Line { width } => {
                    ci.type_ = sys::BK_ITEM_LINE;
                    ci.width = (width * scale).max(1.0);
                }
                Kind::Polygon => ci.type_ = sys::BK_ITEM_POLYGON,
                Kind::Arc { start, extent } => {
                    ci.type_ = sys::BK_ITEM_ARC;
                    ci.start = *start;
                    ci.extent = *extent;
                }
                Kind::Text { text, size, flags } => {
                    ci.type_ = sys::BK_ITEM_TEXT;
                    ci.size = size * scale;
                    ci.text_flags = *flags;
                    ci.anchor = sys::BK_ANCHOR_N | sys::BK_ANCHOR_W;
                    ci.text = text.as_ptr();
                }
                Kind::Image(img) => {
                    let w = (coords[2] - coords[0]).round().max(1.0) as u32;
                    let h = (coords[3] - coords[1]).round().max(1.0) as u32;
                    if img.width == 0 || img.height == 0 {
                        continue;
                    }
                    let px = img.resized(w, h);
                    ci.type_ = sys::BK_ITEM_IMAGE;
                    ci.anchor = sys::BK_ANCHOR_N | sys::BK_ANCHOR_W;
                    ci.rgba = px.as_ptr();
                    ci.iw = w as i32;
                    ci.ih = h as i32;
                    coords.truncate(2);
                    _keep_px = Some(px);
                }
            }
            if let Some(c) = it.clip {
                ci.clip_on = 1;
                ci.clip = [c.origin.x * scale, c.origin.y * scale, c.size.width * scale,
                           c.size.height * scale];
            }
            ci.coords = coords.as_ptr();
            ci.ncoords = coords.len() as i32;
            // SAFETY: every pointer in `ci` outlives the call; the kit copies them.
            unsafe { sys::BK_canvas_add_item(canvas, &ci) };
        }
    }
}

// ==========================================================================
// Painting a frame

fn lp(v: LengthPercentage, basis: f32) -> f32 {
    match v {
        LengthPercentage::Length(p) => p,
        LengthPercentage::Percent(f) => f * basis,
    }
}

/// Border widths (top, right, bottom, left).
fn border_widths(c: &Computed, w: f32) -> [f32; 4] {
    let b = c.layout.border;
    [lp(b.top, w), lp(b.right, w), lp(b.bottom, w), lp(b.left, w)]
}

fn radius_px(c: &Computed, b: Bounds<f32>, vp: (f32, f32)) -> f32 {
    let u = Units { font_size: c.font_size, viewport: vp };
    let m = b.size.width.min(b.size.height);
    c.radius.iter().map(|r| r.resolve(&u, m)).fold(0.0, f32::max).min(m / 2.0)
}

/// Paint every box of `frame`, in stacking order.
pub(crate) fn paint_frame(frame: &Frame, p: &mut Painter, vp: (f32, f32)) {
    let mut order: Vec<usize> = (0..frame.nodes.len()).collect();
    order.sort_by_key(|&i| frame.z_of(i)); // stable: tree order within a level
    for i in order {
        let n = &frame.nodes[i];
        if !n.visible || n.opacity <= 0.0 {
            continue;
        }
        p.opacity = n.opacity;
        p.clip = n.clip;
        paint_node(frame, n, p, vp);
    }
    p.opacity = 1.0;
    p.clip = None;
}

/// A background that repaints exactly the pixels already under it: an opaque
/// plain fill with the colour and bounds of the nearest painted ancestor, with
/// only undecorated same-sized boxes in between (so nothing else can show).
fn redundant_background(frame: &Frame, n: &BoxNode) -> bool {
    let c = &*n.style;
    if c.background.a < 1.0 || c.gradient.is_some() || !c.shadows.is_empty() {
        return false;
    }
    let mut cur = n.parent;
    while let Some(i) = cur {
        let a = &frame.nodes[i];
        let ac = &*a.style;
        if a.bounds != n.bounds || a.taffy.is_none() || !ac.shadows.is_empty() || ac.gradient.is_some()
            || a.opacity < 1.0 || a.clip.is_some() != n.clip.is_some()
        {
            return false;
        }
        if !ac.background.is_transparent() {
            return ac.background == c.background && n.opacity >= 1.0;
        }
        if border_widths(ac, a.bounds.size.width).iter().any(|w| *w > 0.0) {
            return false;
        }
        cur = a.parent;
    }
    false
}

fn paint_node(frame: &Frame, n: &BoxNode, p: &mut Painter, vp: (f32, f32)) {
    let c = &*n.style;
    let b = n.bounds;
    match &n.content {
        Content::Word(w) => {
            // inline element backgrounds paint per line fragment (here: per word)
            let owner = &frame.nodes[n.owner];
            if owner.taffy.is_none() {
                p.fill(b, owner.style.background);
            }
            paint_text_line(p, c, w, b.origin.x, b.origin.y, b.size.height, None);
        }
        // a text run as a flex/grid item (in line boxes its words paint instead)
        Content::Text(t) if n.taffy.is_some() => {
            let flags = text::flags(c);
            let nowrap = matches!(c.white_space, WhiteSpace::NoWrap | WhiteSpace::Pre);
            if !c.ellipsis && !t.contains([' ', '\n']) {
                // one word: one line (the common label case)
                let lw = text::width(t, c.font_size, flags);
                let x = match c.text_align {
                    TextAlign::Center => b.origin.x + (b.size.width - lw) / 2.0,
                    TextAlign::Right => b.origin.x + b.size.width - lw,
                    _ => b.origin.x,
                };
                paint_text_line(p, c, t, x, b.origin.y, c.line_px(), Some(lw));
                return;
            }
            let lines = if nowrap {
                vec![t.clone()]
            } else {
                text::wrap(t, c.font_size, flags, b.size.width + 0.5)
            };
            let lh = c.line_px();
            for (k, line) in lines.iter().enumerate() {
                let line = if c.ellipsis { text::ellipsize(line, c.font_size, flags, b.size.width) }
                           else { line.clone() };
                let lw = text::width(&line, c.font_size, flags);
                let x = match c.text_align {
                    TextAlign::Center => b.origin.x + (b.size.width - lw) / 2.0,
                    TextAlign::Right => b.origin.x + b.size.width - lw,
                    _ => b.origin.x,
                };
                paint_text_line(p, c, &line, x, b.origin.y + lh * k as f32, lh, Some(lw));
            }
        }
        Content::Element | Content::Replaced(_) if n.taffy.is_some() => {
            paint_box(n, p, vp, redundant_background(frame, n));
            if let Some(m) = &n.marker {
                let bw = border_widths(c, b.size.width);
                let pad = lp(c.layout.padding.top, b.size.width);
                let flags = text::flags(c);
                let mw = text::width(m, c.font_size, flags);
                paint_text_line(p, c, m, b.origin.x - mw - 6.0, b.origin.y + bw[0] + pad, c.line_px(),
                                Some(mw));
            }
            if let Content::Replaced(r) = &n.content {
                let inner = content_box(c, b);
                match r {
                    Replaced::Image(img) => p.image(inner, img),
                    Replaced::Painted(f) => {
                        let f = f.clone();
                        p.with_clip(inner, |p| f(inner, p));
                    }
                    Replaced::Placeholder(tag) => {
                        p.fill(inner, crate::color::rgb(0x282828));
                        let label = n.attr("alt").unwrap_or(tag);
                        let w = text::width(label, c.font_size, 0);
                        p.styled_text(inner.origin.x + (inner.size.width - w) / 2.0,
                                      inner.origin.y + (inner.size.height - c.font_size) / 2.0,
                                      label, c.font_size, 0, crate::color::rgb(0x8a8a8a));
                    }
                }
            }
            paint_scrollbars(n, p);
        }
        _ => {}
    }
}

fn content_box(c: &Computed, b: Bounds<f32>) -> Bounds<f32> {
    let w = b.size.width;
    let bw = border_widths(c, w);
    let pd = c.layout.padding;
    let (pt, pr, pb, pl) = (lp(pd.top, w), lp(pd.right, w), lp(pd.bottom, w), lp(pd.left, w));
    rect(b.origin.x + bw[3] + pl, b.origin.y + bw[0] + pt,
         (w - bw[1] - bw[3] - pl - pr).max(0.0), (b.size.height - bw[0] - bw[2] - pt - pb).max(0.0))
}

/// One line of text in a line box of height `lh`, with decorations.
fn paint_text_line(p: &mut Painter, c: &Computed, s: &str, x: f32, y: f32, lh: f32, width: Option<f32>) {
    let flags = text::flags(c);
    let size = c.font_size;
    let top = y + ((lh - size) / 2.0).max(0.0);
    let s = if c.small_caps { s.to_uppercase() } else { s.to_owned() };
    let size_draw = if c.small_caps { size * 0.8 } else { size };
    if let Some(sh) = c.text_shadow {
        p.styled_text(x + sh.x, top + sh.y, &s, size_draw, flags, sh.color);
    }
    if c.letter_spacing != 0.0 {
        let mut cx = x;
        for ch in s.chars() {
            let t = ch.to_string();
            p.styled_text(cx, top, &t, size_draw, flags, c.color);
            cx += text::width(&t, size_draw, flags) + c.letter_spacing;
        }
    } else {
        p.styled_text(x, top, &s, size_draw, flags, c.color);
    }
    if c.underline || c.line_through || c.overline {
        let w = width.unwrap_or_else(|| text::width(s.trim_end(), size_draw, flags));
        let thick = (size / 14.0).max(1.0);
        let dc = c.decoration_color.unwrap_or(c.color);
        let mut deco = |yy: f32| p.fill(rect(x, yy, w, thick), dc);
        if c.underline {
            deco(top + size * 0.92);
        }
        if c.line_through {
            deco(top + size * 0.48);
        }
        if c.overline {
            deco(top);
        }
    }
}

/// Shadows, background, borders and outline of an element box.
fn paint_box(n: &BoxNode, p: &mut Painter, vp: (f32, f32), skip_background: bool) {
    let c = &*n.style;
    let b = n.bounds;
    if matches!(c.display, DisplayKind::Contents) || b.size.width <= 0.0 && b.size.height <= 0.0 {
        return;
    }
    let r = radius_px(c, b, vp);
    for s in c.shadows.iter().filter(|s| !s.inset) {
        let steps = (s.blur / 2.0).ceil().clamp(1.0, 8.0) as usize;
        for k in 0..steps {
            let grow = s.spread + s.blur * (k as f32 + 1.0) / (steps as f32 + 1.0);
            let a = s.color.a / steps as f32 * if s.blur > 0.0 { 1.0 } else { steps as f32 };
            let sb = rect(b.origin.x + s.x - grow, b.origin.y + s.y - grow,
                          b.size.width + 2.0 * grow, b.size.height + 2.0 * grow);
            p.quad(sb, r + grow, s.color.alpha(a), Rgba::TRANSPARENT, 0.0);
        }
    }
    match &c.gradient {
        Some(g) => p.with_clip(b, |p| paint_gradient(p, g, b)),
        None if skip_background => {}
        None => p.quad(b, r, c.background, Rgba::TRANSPARENT, 0.0),
    }
    for s in c.shadows.iter().filter(|s| s.inset) {
        let w = s.blur.max(1.0) + s.spread;
        p.with_clip(b, |p| {
            p.quad(rect(b.origin.x + s.x, b.origin.y + s.y, b.size.width, b.size.height), r,
                   Rgba::TRANSPARENT, s.color, w);
        });
    }
    let bw = border_widths(c, b.size.width);
    let visible: Vec<usize> = (0..4).filter(|&k| bw[k] > 0.0 && c.border_drawn[k]
                                            && !c.border_color[k].is_transparent()).collect();
    let uniform = visible.len() == 4 && bw.iter().all(|w| *w == bw[0])
        && c.border_color.iter().all(|x| *x == c.border_color[0]);
    if uniform {
        let h = bw[0] / 2.0;
        p.quad(rect(b.origin.x + h, b.origin.y + h, b.size.width - bw[0], b.size.height - bw[0]),
               (r - h).max(0.0), Rgba::TRANSPARENT, c.border_color[0], bw[0]);
    } else {
        let (x, y, w, h) = (b.origin.x, b.origin.y, b.size.width, b.size.height);
        for k in visible {
            let side = match k {
                0 => rect(x, y, w, bw[0]),
                1 => rect(x + w - bw[1], y, bw[1], h),
                2 => rect(x, y + h - bw[2], w, bw[2]),
                _ => rect(x, y, bw[3], h),
            };
            p.fill(side, c.border_color[k]);
        }
    }
    let (ow, oc, off) = c.outline;
    if ow > 0.0 && !oc.is_transparent() {
        let g = off + ow / 2.0;
        p.quad(rect(b.origin.x - g, b.origin.y - g, b.size.width + 2.0 * g, b.size.height + 2.0 * g),
               r + g, Rgba::TRANSPARENT, oc, ow);
    }
}

fn mix(a: Rgba, b: Rgba, t: f32) -> Rgba {
    Rgba { r: a.r + (b.r - a.r) * t, g: a.g + (b.g - a.g) * t, b: a.b + (b.b - a.b) * t,
           a: a.a + (b.a - a.a) * t }
}

/// Colour at position `t` (0..1) along a gradient.
fn gradient_at(g: &crate::style::Gradient, t: f32) -> Rgba {
    let n = g.stops.len();
    if n == 0 {
        return Rgba::TRANSPARENT;
    }
    let pos: Vec<f32> = g.stops.iter().enumerate()
        .map(|(k, (_, p))| p.unwrap_or(if n == 1 { 0.0 } else { k as f32 / (n - 1) as f32 }))
        .collect();
    if t <= pos[0] {
        return g.stops[0].0;
    }
    for k in 1..n {
        if t <= pos[k] {
            let span = (pos[k] - pos[k - 1]).max(1e-6);
            return mix(g.stops[k - 1].0, g.stops[k].0, (t - pos[k - 1]) / span);
        }
    }
    g.stops[n - 1].0
}

/// Gradients as bands: horizontal/vertical strips, or rings for radial ones.
fn paint_gradient(p: &mut Painter, g: &crate::style::Gradient, b: Bounds<f32>) {
    const BANDS: usize = 48;
    let (x, y, w, h) = (b.origin.x, b.origin.y, b.size.width, b.size.height);
    if g.radial {
        for k in (0..BANDS).rev() {
            let t = (k as f32 + 0.5) / BANDS as f32;
            let f = (k + 1) as f32 / BANDS as f32;
            let (rw, rh) = (w * f * std::f32::consts::SQRT_2, h * f * std::f32::consts::SQRT_2);
            p.oval(rect(x + (w - rw) / 2.0, y + (h - rh) / 2.0, rw, rh), gradient_at(g, t),
                   Rgba::TRANSPARENT, 0.0);
        }
        return;
    }
    // angle: 180deg = to bottom, 90deg = to right
    let a = g.angle_deg.to_radians();
    let (dx, dy) = (a.sin(), -a.cos());
    let len = (w * dx.abs() + h * dy.abs()).max(1.0);
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    for k in 0..BANDS {
        let t0 = k as f32 / BANDS as f32;
        let t1 = (k + 1) as f32 / BANDS as f32;
        let col = gradient_at(g, (t0 + t1) / 2.0);
        // a band perpendicular to the gradient line, long enough to cover the box
        let (s0, s1) = ((t0 - 0.5) * len, (t1 - 0.5) * len + 0.6);
        let (px, py) = (-dy, dx);
        let span = w + h;
        let pts = [
            (cx + dx * s0 + px * span, cy + dy * s0 + py * span),
            (cx + dx * s1 + px * span, cy + dy * s1 + py * span),
            (cx + dx * s1 - px * span, cy + dy * s1 - py * span),
            (cx + dx * s0 - px * span, cy + dy * s0 - py * span),
        ];
        p.polygon(&pts, col);
    }
}

/// Overlay scrollbars on scroll containers that overflow.
fn paint_scrollbars(n: &BoxNode, p: &mut Painter) {
    let (rx, ry) = n.scroll_range;
    let b = n.bounds;
    let (sx, sy) = n.scroll_pos;
    let thumb = crate::color::rgba(0x8a8a8aa0);
    if ry > 0.0 {
        let h = b.size.height;
        let th = (h * h / (h + ry)).max(16.0);
        let ty = b.origin.y + (h - th) * (sy / ry);
        p.quad(rect(b.origin.x + b.size.width - 6.0, ty + 1.0, 4.0, th - 2.0), 2.0, thumb,
               Rgba::TRANSPARENT, 0.0);
    }
    if rx > 0.0 {
        let w = b.size.width;
        let tw = (w * w / (w + rx)).max(16.0);
        let tx = b.origin.x + (w - tw) * (sx / rx);
        p.quad(rect(tx + 1.0, b.origin.y + b.size.height - 6.0, tw - 2.0, 4.0), 2.0, thumb,
               Rgba::TRANSPARENT, 0.0);
    }
}
