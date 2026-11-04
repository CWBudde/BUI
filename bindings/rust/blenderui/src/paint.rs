//! Painting: a frame of laid-out boxes becomes one display list on the kit
//! canvas. [`Painter`] is also what `canvas(|bounds, painter| ...)` draws with.

use std::collections::HashMap;
use std::sync::Arc;

use taffy::prelude::*;

use crate::color::Rgba;
use crate::geometry::{rect, Bounds, Units};
use crate::layout::{BoxNode, Content, Frame, Replaced, StyleStore};
use crate::style::{BorderStyle, BoxEdge, Computed, DecoStyle, DisplayKind, TextAlign, WhiteSpace};
use crate::text::{self, TextCache};
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

/// Decoded `<img src>` files, kept by the window (each file is decoded once).
#[derive(Default)]
pub(crate) struct ImageCache {
    files: HashMap<String, Option<Arc<Image>>>,
}

impl ImageCache {
    /// `src` of an `<img>`: PNG files.
    pub(crate) fn load(&mut self, src: &str) -> Option<Arc<Image>> {
        if let Some(i) = self.files.get(src) {
            return i.clone();
        }
        let img = Image::from_png(src).map(Arc::new);
        self.files.insert(src.to_owned(), img.clone());
        img
    }
}

#[derive(Clone)]
enum Kind {
    Rect { radius: f32, outline: Rgba, width: f32 },
    Oval { outline: Rgba, width: f32 },
    Line { width: f32 },
    Polygon,
    Arc { start: f32, extent: f32 },
    /// `start..end` of the painter's text arena (NUL-terminated there).
    Text { start: u32, size: f32, flags: i32 },
    Image(Arc<Image>),
}

/// Coordinates: up to four inline (rects, ovals, text, images), more on the heap.
#[derive(Clone)]
enum Coords {
    Inline([f32; 4], u8),
    Heap(Vec<f32>),
}

impl Coords {
    fn of(v: &[f32]) -> Coords {
        if v.len() <= 4 {
            let mut a = [0.0; 4];
            a[..v.len()].copy_from_slice(v);
            Coords::Inline(a, v.len() as u8)
        } else {
            Coords::Heap(v.to_vec())
        }
    }

    fn as_slice(&self) -> &[f32] {
        match self {
            Coords::Inline(a, n) => &a[..*n as usize],
            Coords::Heap(v) => v,
        }
    }
}

#[derive(Clone)]
struct Item {
    kind: Kind,
    coords: Coords,
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
    /// Every text item's bytes, NUL-terminated, back to back.
    text: Vec<u8>,
    clip: Option<Bounds<f32>>,
    opacity: f32,
}

impl Painter {
    pub(crate) fn new() -> Painter {
        Painter { items: Vec::new(), text: Vec::new(), clip: None, opacity: 1.0 }
    }

    /// Empty for the next frame, keeping the buffers.
    pub(crate) fn reset(&mut self) {
        self.items.clear();
        self.text.clear();
        self.clip = None;
        self.opacity = 1.0;
    }

    fn push(&mut self, kind: Kind, coords: Coords, fill: Rgba) {
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

    fn corners(b: Bounds<f32>) -> Coords {
        Coords::Inline([b.origin.x, b.origin.y, b.origin.x + b.size.width, b.origin.y + b.size.height], 4)
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
        let v: Vec<f32> = points.iter().flat_map(|&(x, y)| [x, y]).collect();
        self.push(Kind::Line { width }, Coords::of(&v), color.into());
    }

    /// A filled polygon (gpui's `paint_path`).
    pub fn polygon(&mut self, points: &[(f32, f32)], color: impl Into<Rgba>) {
        let v: Vec<f32> = points.iter().flat_map(|&(x, y)| [x, y]).collect();
        self.push(Kind::Polygon, Coords::of(&v), color.into());
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
            let start = self.text.len() as u32;
            self.text.extend(text.bytes().filter(|&b| b != 0));
            self.text.push(0);
            self.push(Kind::Text { start, size, flags }, Coords::Inline([x, y, 0.0, 0.0], 2), color);
        }
    }

    /// An image scaled to `bounds`.
    pub fn image(&mut self, bounds: Bounds<f32>, image: &Arc<Image>) {
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
        let mut heap: Vec<f32> = Vec::new();
        let mut count: i32 = 0;
        for it in &self.items {
            let src = it.coords.as_slice();
            let mut inline = [0.0f32; 4];
            let coords: &mut [f32] = if src.len() <= 4 {
                for (d, s) in inline.iter_mut().zip(src) {
                    *d = s * scale;
                }
                &mut inline[..src.len()]
            } else {
                heap.clear();
                heap.extend(src.iter().map(|v| v * scale));
                &mut heap[..]
            };
            let mut ncoords = coords.len();
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
                Kind::Text { start, size, flags } => {
                    ci.type_ = sys::BK_ITEM_TEXT;
                    ci.size = size * scale;
                    (ci.family, ci.text_flags) = crate::text::family(*flags);
                    ci.anchor = sys::BK_ANCHOR_N | sys::BK_ANCHOR_W;
                    ci.text = self.text[*start as usize..].as_ptr().cast();
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
                    ncoords = 2;
                    _keep_px = Some(px);
                }
            }
            if let Some(c) = it.clip {
                ci.clip_on = 1;
                ci.clip = [c.origin.x * scale, c.origin.y * scale, c.size.width * scale,
                           c.size.height * scale];
            }
            ci.coords = coords.as_ptr();
            ci.ncoords = ncoords as i32;
            // SAFETY: every pointer in `ci` outlives the call; the kit copies
            // them into the slot's existing buffers.
            unsafe { sys::BK_canvas_put_item(canvas, count, &ci) };
            count += 1;
        }
        // SAFETY: `canvas` is a live kit canvas node owned by the window.
        unsafe { sys::BK_canvas_truncate(canvas, count) };
    }
}

// ==========================================================================
// Painting a frame

fn lp(v: LengthPercentage, basis: f32) -> f32 {
    use taffy::style::ExpandedLengthPercentage as E;
    match v.expand() {
        E::Length(p) => p,
        E::Percent(f) => f * basis,
        // SAFETY: calc pointers in styles come from crate::calc's interner
        E::Calc(ptr) => unsafe { crate::calc::Resolved::from_ptr(ptr) }.eval(basis),
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
pub(crate) fn paint_frame(frame: &Frame, styles: &StyleStore, tc: &mut TextCache, p: &mut Painter,
                          vp: (f32, f32)) {
    let mut order: Vec<usize> = (0..frame.nodes.len()).collect();
    if frame.nodes.iter().any(|n| styles[n.style].z_index.is_some()) {
        order.sort_by_key(|&i| frame.z_of(i, styles)); // stable: tree order within a level
    }
    for i in order {
        let n = &frame.nodes[i];
        if !n.visible || n.opacity <= 0.0 {
            continue;
        }
        p.opacity = n.opacity;
        p.clip = n.clip;
        paint_node(frame, styles, tc, i, p, vp);
    }
    p.opacity = 1.0;
    p.clip = None;
}

/// A background that repaints exactly the pixels already under it: an opaque
/// plain fill with the colour and bounds of the nearest painted ancestor, with
/// only undecorated same-sized boxes in between (so nothing else can show).
fn redundant_background(frame: &Frame, styles: &StyleStore, n: &BoxNode) -> bool {
    let c = &styles[n.style];
    if c.background.a < 1.0 || c.gradient.is_some() || !c.shadows.is_empty()
        || c.images.as_ref().is_some_and(|i| i.bg_url.is_some()) {
        return false;
    }
    let mut cur = n.parent;
    while let Some(i) = cur {
        let a = &frame.nodes[i];
        let ac = &styles[a.style];
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

fn paint_node(frame: &Frame, styles: &StyleStore, tc: &mut TextCache, i: usize, p: &mut Painter,
              vp: (f32, f32)) {
    let n = &frame.nodes[i];
    let c = &styles[n.style];
    let b = n.bounds;
    match &n.content {
        Content::Word { .. } => {
            // inline element backgrounds paint per line fragment (here: per word)
            let owner = &frame.nodes[n.owner];
            if owner.taffy.is_none() {
                p.fill(b, styles[owner.style].background);
            }
            paint_text_line(p, tc, c, frame.text(i), b.origin.x, b.origin.y, b.size.height, None);
        }
        // a text run as a flex/grid item (in line boxes its words paint instead)
        Content::Text(t) if n.taffy.is_some() || n.folded => {
            let flags = text::flags(c);
            let nowrap = matches!(c.white_space, WhiteSpace::NoWrap | WhiteSpace::Pre);
            if !c.ellipsis && !t.contains([' ', '\n']) && text::break_mode(c) == 0 {
                // one word: one line (the common label case)
                let lw = tc.width(t, c.font_size, flags);
                let x = match c.text_align {
                    TextAlign::Center => b.origin.x + (b.size.width - lw) / 2.0,
                    TextAlign::Right => b.origin.x + b.size.width - lw,
                    _ => b.origin.x,
                };
                paint_text_line(p, tc, c, t, x, b.origin.y, c.line_px(), Some(lw));
                return;
            }
            let lines = if nowrap {
                vec![t.clone()]
            } else {
                tc.wrap(t, c.font_size, flags, b.size.width + 0.5, text::break_mode(c))
            };
            let lh = c.line_px();
            for (k, line) in lines.iter().enumerate() {
                let line = if c.ellipsis { tc.ellipsize(line, c.font_size, flags, b.size.width) }
                           else { line.clone() };
                let lw = tc.width(&line, c.font_size, flags);
                let x = match c.text_align {
                    TextAlign::Center => b.origin.x + (b.size.width - lw) / 2.0,
                    TextAlign::Right => b.origin.x + b.size.width - lw,
                    _ => b.origin.x,
                };
                paint_text_line(p, tc, c, &line, x, b.origin.y + lh * k as f32, lh, Some(lw));
            }
        }
        Content::Element | Content::Replaced(_) if n.taffy.is_some() => {
            let hidden_cell = c.empty_cells_hide && c.display == DisplayKind::TableCell
                && !c.border_collapse && frame.is_empty(i);
            if !hidden_cell {
                paint_box(n, c, p, vp, redundant_background(frame, styles, n));
            }
            let marker_img = n.marker.as_ref().and(c.list_image.as_deref()).and_then(background_file);
            if let Some(img) = marker_img {
                // `list-style-image`: the image, at most a line tall, left of the box
                let bw = border_widths(c, b.size.width);
                let pad = lp(c.layout.padding.top, b.size.width);
                let lh = c.line_px();
                let (iw, ih) = (img.width as f32, img.height as f32);
                let s = (lh / ih.max(1.0)).min(1.0);
                let (w, h) = (iw * s, ih * s);
                p.image(rect(b.origin.x - w - 6.0, b.origin.y + bw[0] + pad + (lh - h) / 2.0, w, h), &img);
            } else if let Some(m) = n.marker.as_ref().filter(|m| !m.is_empty()) {
                let bw = border_widths(c, b.size.width);
                let pad = lp(c.layout.padding.top, b.size.width);
                let flags = text::flags(c);
                let mw = tc.width(m, c.font_size, flags);
                paint_text_line(p, tc, c, m, b.origin.x - mw - 6.0, b.origin.y + bw[0] + pad, c.line_px(),
                                Some(mw));
            }
            if let Content::Replaced(r) = &n.content {
                let inner = content_box(c, b);
                match r {
                    Replaced::Image(img) => match &c.images {
                        Some(st) if st.object_fit != crate::style::ObjectFit::Fill
                            || st.object_position != [crate::style::Pos::CENTER; 2] => {
                            let (x, y, w, h) = st.object_rect(
                                (inner.origin.x, inner.origin.y, inner.size.width, inner.size.height),
                                (img.width as f32, img.height as f32));
                            p.with_clip(inner, |p| p.image(rect(x, y, w, h), img));
                        }
                        _ => p.image(inner, img),
                    },
                    Replaced::Painted(f) => p.with_clip(inner, |p| f(inner, p)),
                    Replaced::Placeholder(tag) => {
                        p.fill(inner, crate::color::rgb(0x282828));
                        let label = frame.attr(i, "alt").unwrap_or(tag);
                        let w = tc.width(label, c.font_size, 0);
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

pub(crate) fn content_box(c: &Computed, b: Bounds<f32>) -> Bounds<f32> {
    let w = b.size.width;
    let bw = border_widths(c, w);
    let pd = c.layout.padding;
    let (pt, pr, pb, pl) = (lp(pd.top, w), lp(pd.right, w), lp(pd.bottom, w), lp(pd.left, w));
    rect(b.origin.x + bw[3] + pl, b.origin.y + bw[0] + pt,
         (w - bw[1] - bw[3] - pl - pr).max(0.0), (b.size.height - bw[0] - bw[2] - pt - pb).max(0.0))
}

/// One line of text in a line box of height `lh`, with decorations.
#[allow(clippy::too_many_arguments)]
fn paint_text_line(p: &mut Painter, tc: &mut TextCache, c: &Computed, s: &str, x: f32, y: f32, lh: f32,
                   width: Option<f32>) {
    let flags = text::flags(c);
    let size = c.font_size;
    let top = y + ((lh - size) / 2.0).max(0.0);
    let s: std::borrow::Cow<str> = if c.small_caps { s.to_uppercase().into() } else { s.into() };
    let size_draw = if c.small_caps { size * 0.8 } else { size };
    if let Some(sh) = c.text_shadow {
        p.styled_text(x + sh.x, top + sh.y, &s, size_draw, flags, sh.color);
    }
    if c.letter_spacing != 0.0 {
        let mut cx = x;
        for ch in s.chars() {
            let t = ch.to_string();
            p.styled_text(cx, top, &t, size_draw, flags, c.color);
            cx += tc.width(&t, size_draw, flags) + c.letter_spacing;
        }
    } else {
        p.styled_text(x, top, &s, size_draw, flags, c.color);
    }
    if c.underline || c.line_through || c.overline {
        let w = width.unwrap_or_else(|| tc.width(s.trim_end(), size_draw, flags));
        let thick = c.decoration_thickness.unwrap_or(size / 14.0).max(1.0);
        let dc = c.decoration_color.unwrap_or(c.color);
        if c.underline {
            // the offset is measured from the baseline (about 0.8em down)
            let y = match c.underline_offset {
                Some(o) => top + size * 0.8 + o,
                None => top + size * 0.92,
            };
            decoration(p, c.decoration_style, x, y, w, thick, dc);
        }
        if c.line_through {
            decoration(p, c.decoration_style, x, top + size * 0.48, w, thick, dc);
        }
        if c.overline {
            decoration(p, c.decoration_style, x, top, w, thick, dc);
        }
    }
}

/// One text decoration line of `text-decoration-style` from `x` to `x + w`.
fn decoration(p: &mut Painter, style: DecoStyle, x: f32, y: f32, w: f32, t: f32, col: Rgba) {
    match style {
        DecoStyle::Solid => p.fill(rect(x, y, w, t), col),
        DecoStyle::Double => {
            p.fill(rect(x, y, w, t), col);
            p.fill(rect(x, y + t * 2.0, w, t), col);
        }
        DecoStyle::Dotted | DecoStyle::Dashed => {
            let (on, gap) = if style == DecoStyle::Dotted { (t, t) } else { (t * 3.0, t * 2.0) };
            let mut cx = x;
            while cx < x + w {
                p.fill(rect(cx, y, on.min(x + w - cx), t), col);
                cx += on + gap;
            }
        }
        DecoStyle::Wavy => {
            let (amp, period) = (t * 1.5, t * 6.0);
            let steps = ((w / (period / 8.0)).ceil() as usize).clamp(2, 4096);
            let pts: Vec<(f32, f32)> = (0..=steps).map(|k| {
                let px = x + w * k as f32 / steps as f32;
                (px, y + amp * (std::f32::consts::TAU * (px - x) / period).sin())
            }).collect();
            p.line(&pts, col, t);
        }
    }
}

/// Shadows, background, borders and outline of an element box.
fn paint_box(n: &BoxNode, c: &Computed, p: &mut Painter, vp: (f32, f32), skip_background: bool) {
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
    // `background-clip`: the painting area; its radius shrinks with it
    let bw = border_widths(c, b.size.width);
    let clip_box = edge_box(c, b, c.bg_clip, &bw);
    let clip_r = (r - (clip_box.origin.x - b.origin.x)).max(0.0);
    match &c.gradient {
        Some(g) => p.with_clip(clip_box, |p| paint_gradient(p, g, clip_box)),
        None if skip_background && c.bg_clip == BoxEdge::Border => {}
        None => p.quad(clip_box, clip_r, c.background, Rgba::TRANSPARENT, 0.0),
    }
    if let Some(st) = c.images.as_deref().filter(|st| st.bg_url.is_some()) {
        // `background-origin` positions the image, `-attachment: fixed` against the viewport
        let area = if st.bg_fixed { rect(0.0, 0.0, vp.0, vp.1) } else { edge_box(c, b, c.bg_origin, &bw) };
        let clip_to = if st.bg_fixed { clip_box } else { clip_box.intersect(&b) };
        paint_background_image(p, st, area, clip_to);
    }
    for s in c.shadows.iter().filter(|s| s.inset) {
        let w = s.blur.max(1.0) + s.spread;
        p.with_clip(b, |p| {
            p.quad(rect(b.origin.x + s.x, b.origin.y + s.y, b.size.width, b.size.height), r,
                   Rgba::TRANSPARENT, s.color, w);
        });
    }
    let visible: Vec<usize> = (0..4).filter(|&k| bw[k] > 0.0 && c.border_drawn[k]
                                            && !c.border_color[k].is_transparent()).collect();
    let uniform = visible.len() == 4 && bw.iter().all(|w| *w == bw[0])
        && c.border_color.iter().all(|x| *x == c.border_color[0]);
    let styled = visible.iter().any(|&k| c.border_style[k] != BorderStyle::Solid);
    if styled {
        let styles: [BorderStyle; 4] = std::array::from_fn(|k| c.border_style[k]);
        let widths: [f32; 4] = std::array::from_fn(|k| if visible.contains(&k) { bw[k] } else { 0.0 });
        paint_edges(p, b, widths, c.border_color, styles);
    } else if uniform {
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
    if ow > 0.0 && !oc.is_transparent() && c.outline_style != BorderStyle::Solid {
        let g = off + ow;
        let ob = rect(b.origin.x - g, b.origin.y - g, b.size.width + 2.0 * g, b.size.height + 2.0 * g);
        paint_edges(p, ob, [ow; 4], [oc; 4], [c.outline_style; 4]);
    } else if ow > 0.0 && !oc.is_transparent() {
        let g = off + ow / 2.0;
        p.quad(rect(b.origin.x - g, b.origin.y - g, b.size.width + 2.0 * g, b.size.height + 2.0 * g),
               r + g, Rgba::TRANSPARENT, oc, ow);
    }
}

/// The border, padding or content box of `b`.
fn edge_box(c: &Computed, b: Bounds<f32>, edge: BoxEdge, bw: &[f32; 4]) -> Bounds<f32> {
    match edge {
        BoxEdge::Border => b,
        BoxEdge::Padding => rect(b.origin.x + bw[3], b.origin.y + bw[0], (b.size.width - bw[1] - bw[3]).max(0.0),
                                 (b.size.height - bw[0] - bw[2]).max(0.0)),
        BoxEdge::Content => content_box(c, b),
    }
}

/// A colour darkened (`f` < 1) or lightened toward white (`f` > 1), for 3D borders.
fn shade(c: Rgba, f: f32) -> Rgba {
    if f <= 1.0 {
        Rgba { r: c.r * f, g: c.g * f, b: c.b * f, a: c.a }
    } else {
        mix(c, Rgba { r: 1.0, g: 1.0, b: 1.0, a: c.a }, f - 1.0)
    }
}

/// Borders (or an outline) with non-solid styles, side by side inside
/// `b`: dashes and dots run along each side, `double` is two lines, and
/// groove / ridge / inset / outset shade the sides. Corners are square.
fn paint_edges(p: &mut Painter, b: Bounds<f32>, w: [f32; 4], col: [Rgba; 4], style: [BorderStyle; 4]) {
    let (x, y, bw, bh) = (b.origin.x, b.origin.y, b.size.width, b.size.height);
    // the band of side k, inset by `from` and `to` (fractions of its width)
    let band = |k: usize, from: f32, to: f32| -> Bounds<f32> {
        let (a, t) = (w[k] * from, w[k] * (to - from));
        match k {
            0 => rect(x, y + a, bw, t),
            1 => rect(x + bw - w[1] + a, y, t, bh),
            2 => rect(x, y + bh - w[2] + a, bw, t),
            _ => rect(x + a, y, t, bh),
        }
    };
    for k in 0..4 {
        if w[k] <= 0.0 || col[k].is_transparent() {
            continue;
        }
        let c = col[k];
        // top and left are the "light source" sides for inset / outset
        let lit = k == 0 || k == 3;
        match style[k] {
            BorderStyle::Solid => p.fill(band(k, 0.0, 1.0), c),
            BorderStyle::Double if w[k] >= 3.0 => {
                p.fill(band(k, 0.0, 1.0 / 3.0), c);
                p.fill(band(k, 2.0 / 3.0, 1.0), c);
            }
            BorderStyle::Double => p.fill(band(k, 0.0, 1.0), c),
            BorderStyle::Inset => p.fill(band(k, 0.0, 1.0), if lit { shade(c, 0.5) } else { c }),
            BorderStyle::Outset => p.fill(band(k, 0.0, 1.0), if lit { c } else { shade(c, 0.5) }),
            BorderStyle::Groove | BorderStyle::Ridge => {
                let (dark, light) = (shade(c, 0.5), c);
                let outer_dark = (style[k] == BorderStyle::Groove) == lit;
                p.fill(band(k, 0.0, 0.5), if outer_dark { dark } else { light });
                p.fill(band(k, 0.5, 1.0), if outer_dark { light } else { dark });
            }
            BorderStyle::Dashed | BorderStyle::Dotted => {
                let horizontal = k == 0 || k == 2;
                let full = band(k, 0.0, 1.0);
                let len = if horizontal { full.size.width } else { full.size.height };
                let t = w[k];
                let (on, gap) = if style[k] == BorderStyle::Dotted { (t, t) } else { (t * 3.0, t * 3.0) };
                // spread the segments so both ends start and end with one
                let n = ((len + gap) / (on + gap)).floor().max(1.0);
                let gap = if n > 1.0 { (len - n * on) / (n - 1.0) } else { 0.0 };
                for j in 0..n as usize {
                    let o = j as f32 * (on + gap);
                    let seg = if horizontal {
                        rect(full.origin.x + o, full.origin.y, on.min(len - o), t)
                    } else {
                        rect(full.origin.x, full.origin.y + o, t, on.min(len - o))
                    };
                    if style[k] == BorderStyle::Dotted && t >= 3.0 {
                        p.oval(seg, c, Rgba::TRANSPARENT, 0.0);
                    } else {
                        p.fill(seg, c);
                    }
                }
            }
        }
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

/// `background-image: url()` files (PNG), decoded once per process.
fn background_file(url: &str) -> Option<Arc<Image>> {
    static FILES: std::sync::Mutex<Option<HashMap<String, Option<Arc<Image>>>>> = std::sync::Mutex::new(None);
    let mut files = FILES.lock().unwrap_or_else(|e| e.into_inner());
    let files = files.get_or_insert_with(HashMap::new);
    if let Some(i) = files.get(url) {
        return i.clone();
    }
    let img = Image::from_png(url).map(Arc::new);
    files.insert(url.to_owned(), img.clone());
    img
}

/// A background image in its positioning `area` (`background-origin`, or the
/// viewport when fixed): sized, positioned and tiled, clipped to `clip_to`.
fn paint_background_image(p: &mut Painter, st: &crate::style::ImageStyle, area: Bounds<f32>,
                          clip_to: Bounds<f32>) {
    let Some(img) = st.bg_url.as_deref().and_then(background_file) else { return };
    let (ax, ay, aw, ah) = (area.origin.x, area.origin.y, area.size.width, area.size.height);
    let (tw, th) = st.tile((aw, ah), (img.width as f32, img.height as f32));
    if tw < 0.5 || th < 0.5 || aw <= 0.0 || ah <= 0.0 {
        return;
    }
    let x0 = ax + st.bg_position[0].at(aw - tw);
    let y0 = ay + st.bg_position[1].at(ah - th);
    // first tile at or before the area's start, when repeating
    let sx = if st.bg_repeat.0 { x0 - ((x0 - ax) / tw).ceil() * tw } else { x0 };
    let sy = if st.bg_repeat.1 { y0 - ((y0 - ay) / th).ceil() * th } else { y0 };
    // repeated tiles cover the painting area, which may reach past the positioning area
    let (ex, ey) = (clip_to.origin.x + clip_to.size.width, clip_to.origin.y + clip_to.size.height);
    let sx = if st.bg_repeat.0 && clip_to.origin.x < sx { sx - ((sx - clip_to.origin.x) / tw).ceil() * tw } else { sx };
    let sy = if st.bg_repeat.1 && clip_to.origin.y < sy { sy - ((sy - clip_to.origin.y) / th).ceil() * th } else { sy };
    let nx = if st.bg_repeat.0 { ((ex.max(ax + aw) - sx) / tw).ceil() as usize } else { 1 };
    let ny = if st.bg_repeat.1 { ((ey.max(ay + ah) - sy) / th).ceil() as usize } else { 1 };
    if nx.saturating_mul(ny) > 4096 {
        return; // tiny tiles over a huge box: not worth thousands of quads
    }
    p.with_clip(clip_to, |p| {
        for j in 0..ny {
            for i in 0..nx {
                p.image(rect(sx + i as f32 * tw, sy + j as f32 * th, tw, th), &img);
            }
        }
    });
}
