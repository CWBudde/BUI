//! Painting: a frame of laid-out boxes becomes one display list on the kit
//! canvas. [`Painter`] is also what `canvas(|bounds, painter| ...)` draws with.

use std::collections::HashMap;
use std::sync::Arc;

use taffy::prelude::*;

use crate::color::Rgba;
use crate::geometry::{rect, Bounds, Length, Units};
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

    /// The kit item for `it`, scaled to device pixels; its coordinates go
    /// into `coords` (point `ci.coords` at them once filled) and image
    /// pixels into `keep`. None: nothing to draw.
    fn build_item(&self, it: &Item, scale: f32, coords: &mut Vec<f32>, keep: &mut Option<Vec<u8>>)
                  -> Option<sys::BKCanvasItem> {
        coords.clear();
        coords.extend(it.coords.as_slice().iter().map(|v| v * scale));
        let mut ci = sys::BKCanvasItem { fill: it.fill.to_array(), ..Default::default() };
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
                    return None;
                }
                let px = img.resized(w, h);
                ci.type_ = sys::BK_ITEM_IMAGE;
                ci.anchor = sys::BK_ANCHOR_N | sys::BK_ANCHOR_W;
                ci.rgba = px.as_ptr();
                ci.iw = w as i32;
                ci.ih = h as i32;
                coords.truncate(2);
                *keep = Some(px);
            }
        }
        if let Some(c) = it.clip {
            ci.clip_on = 1;
            ci.clip = [c.origin.x * scale, c.origin.y * scale, c.size.width * scale, c.size.height * scale];
        }
        ci.ncoords = coords.len() as i32;
        Some(ci)
    }

    /// Replace the canvas display list with these items, scaled to device pixels.
    pub(crate) fn flush(&self, canvas: *mut sys::BKNode, scale: f32) {
        let mut coords: Vec<f32> = Vec::new();
        let mut count: i32 = 0;
        for it in &self.items {
            let mut keep = None;
            let Some(mut ci) = self.build_item(it, scale, &mut coords, &mut keep) else { continue };
            ci.coords = coords.as_ptr();
            // SAFETY: every pointer in `ci` outlives the call; the kit copies
            // them into the slot's existing buffers.
            unsafe { sys::BK_canvas_put_item(canvas, count, &ci) };
            count += 1;
        }
        // SAFETY: `canvas` is a live kit canvas node owned by the window.
        unsafe { sys::BK_canvas_truncate(canvas, count) };
    }

    /// Draw the items offscreen into a `w` x `h` device-pixel region whose
    /// top left is (`x0`, `y0`) (CSS effects).
    pub(crate) fn rasterize(&self, scale: f32, x0: f32, y0: f32, w: usize, h: usize)
                            -> Option<crate::effects::Pixels> {
        if w == 0 || h == 0 || w > 8192 || h > 8192 {
            return None;
        }
        let mut coords: Vec<Vec<f32>> = Vec::with_capacity(self.items.len());
        let mut keep: Vec<Option<Vec<u8>>> = Vec::with_capacity(self.items.len());
        let mut items: Vec<sys::BKCanvasItem> = Vec::with_capacity(self.items.len());
        for it in &self.items {
            let (mut c, mut k) = (Vec::new(), None);
            if let Some(ci) = self.build_item(it, scale, &mut c, &mut k) {
                items.push(ci);
                coords.push(c);
                keep.push(k);
            }
        }
        for (ci, c) in items.iter_mut().zip(&coords) {
            ci.coords = c.as_ptr();
        }
        let mut rgba = vec![0u8; w * h * 4];
        // SAFETY: `items` point into `coords`, `keep` and `self.text`, all alive here.
        let ok = unsafe {
            sys::BK_canvas_rasterize(items.as_ptr(), items.len() as i32, x0, y0, w as i32, h as i32,
                                     rgba.as_mut_ptr())
        };
        drop(keep);
        (ok != 0).then(|| crate::effects::Pixels::from_rgba8(w, h, &rgba))
    }

    /// A hash of everything drawn (layer caching).
    fn content_hash(&self, h: &mut impl std::hash::Hasher) {
        use std::hash::Hash;
        for it in &self.items {
            for v in it.coords.as_slice() {
                v.to_bits().hash(h);
            }
            it.fill.to_array().map(f32::to_bits).hash(h);
            match &it.kind {
                Kind::Rect { radius, outline, width } => {
                    (0u8, radius.to_bits(), outline.to_array().map(f32::to_bits), width.to_bits()).hash(h)
                }
                Kind::Oval { outline, width } => (1u8, outline.to_array().map(f32::to_bits), width.to_bits()).hash(h),
                Kind::Line { width } => (2u8, width.to_bits()).hash(h),
                Kind::Polygon => 3u8.hash(h),
                Kind::Arc { start, extent } => (4u8, start.to_bits(), extent.to_bits()).hash(h),
                Kind::Text { start, size, flags } => {
                    let t = &self.text[*start as usize..];
                    let end = t.iter().position(|&b| b == 0).unwrap_or(t.len());
                    (5u8, &t[..end], size.to_bits(), flags).hash(h)
                }
                Kind::Image(img) => (6u8, Arc::as_ptr(img) as usize).hash(h),
            }
            if let Some(c) = it.clip {
                (c.origin.x.to_bits(), c.origin.y.to_bits(), c.size.width.to_bits(), c.size.height.to_bits()).hash(h);
            }
        }
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

/// Paint every box of `frame`, in stacking order. `scale`: device pixels
/// per logical pixel (for elements whose effects work on pixels).
pub(crate) fn paint_frame(frame: &Frame, styles: &StyleStore, tc: &mut TextCache, p: &mut Painter,
                          vp: (f32, f32), scale: f32) {
    let mut order: Vec<usize> = (0..frame.nodes.len()).collect();
    if frame.nodes.iter().any(|n| styles[n.style].z_index.is_some()) {
        order.sort_by_key(|&i| frame.z_of(i, styles)); // stable: tree order within a level
    }
    let has_effects = frame.nodes.iter().any(|n| styles[n.style].effects.is_some());
    if !has_effects {
        for i in order {
            let n = &frame.nodes[i];
            if !n.visible || n.opacity <= 0.0 {
                continue;
            }
            p.opacity = n.opacity;
            p.clip = n.clip;
            paint_node(frame, styles, tc, i, p, vp);
        }
    } else {
        let mut cx = LayerCx { frame, styles, tc, vp, scale, done: vec![false; frame.nodes.len()] };
        paint_list(&mut cx, &order, p, None, 1.0);
        LAYERS.with(|l| l.borrow_mut().end_frame());
    }
    p.opacity = 1.0;
    p.clip = None;
}

struct LayerCx<'a> {
    frame: &'a Frame,
    styles: &'a StyleStore,
    tc: &'a mut TextCache,
    vp: (f32, f32),
    scale: f32,
    done: Vec<bool>,
}

/// Rendered layers by content hash, kept while they're used (frames that
/// don't change reuse them: no rasterizing, no filtering).
#[derive(Default)]
struct LayerCache {
    map: HashMap<u64, (Arc<Image>, Bounds<f32>, bool)>,
}

impl LayerCache {
    fn get(&mut self, k: u64) -> Option<(Arc<Image>, Bounds<f32>)> {
        self.map.get_mut(&k).map(|e| {
            e.2 = true;
            (e.0.clone(), e.1)
        })
    }
    fn put(&mut self, k: u64, img: Arc<Image>, b: Bounds<f32>) {
        self.map.insert(k, (img, b, true));
    }
    fn end_frame(&mut self) {
        self.map.retain(|_, e| std::mem::replace(&mut e.2, false));
    }
}

thread_local! {
    static LAYERS: std::cell::RefCell<LayerCache> = std::cell::RefCell::new(LayerCache::default());
}

fn is_inside(frame: &Frame, mut k: usize, root: usize) -> bool {
    loop {
        if k == root {
            return true;
        }
        match frame.nodes[k].parent {
            Some(p) => k = p,
            None => return false,
        }
    }
}

/// Paint `list` (in stacking order) into `p`; elements whose effects need
/// pixels are painted with their subtree into a layer. `root`: the layer
/// being painted (its own effects were applied by the caller); `base`: its
/// opacity (descendants paint relative to it: the layer takes it as a group).
fn paint_list(cx: &mut LayerCx, list: &[usize], p: &mut Painter, root: Option<usize>, base: f32) {
    for (idx, &i) in list.iter().enumerate() {
        if cx.done[i] {
            continue;
        }
        let n = &cx.frame.nodes[i];
        if !n.visible || n.opacity <= 0.0 {
            continue;
        }
        let fx = cx.styles[n.style].effects.as_deref();
        if fx.is_some_and(|e| !e.backdrop.is_empty()) && Some(i) != root {
            paint_backdrop(cx, i, p);
        }
        if fx.is_some_and(|e| e.needs_layer()) && Some(i) != root {
            let sub: Vec<usize> = list[idx..].iter().copied()
                .filter(|&k| !cx.done[k] && is_inside(cx.frame, k, i)).collect();
            // the layer's own pass paints these; mark them after
            paint_layer(cx, i, &sub, p, base);
            for &k in &sub {
                cx.done[k] = true;
            }
            continue;
        }
        cx.done[i] = true;
        p.opacity = (n.opacity / base.max(1e-6)).min(1.0);
        p.clip = n.clip;
        let (frame, styles, vp) = (cx.frame, cx.styles, cx.vp);
        paint_node(frame, styles, cx.tc, i, p, vp);
    }
}

/// The logical-pixel area a layer's content can cover: its boxes, their
/// outlines and shadows, and how far its filters spread.
fn layer_area(cx: &LayerCx, list: &[usize], fx: &crate::effects::Effects) -> Bounds<f32> {
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for &k in list {
        let n = &cx.frame.nodes[k];
        let c = &cx.styles[n.style];
        let mut grow = c.outline.0 + c.outline.2.max(0.0);
        for sh in c.shadows.iter().filter(|s| !s.inset) {
            grow = grow.max(sh.x.abs().max(sh.y.abs()) + sh.blur + sh.spread.max(0.0));
        }
        if let Some(bi) = c.effects.as_ref().and_then(|e| e.border_image.as_ref()) {
            for o in bi.outset {
                if let crate::effects::Side::Len(Length::Px(v)) = o {
                    grow = grow.max(v);
                }
            }
        }
        let b = n.bounds;
        x0 = x0.min(b.origin.x - grow);
        y0 = y0.min(b.origin.y - grow);
        x1 = x1.max(b.origin.x + b.size.width + grow);
        y1 = y1.max(b.origin.y + b.size.height + grow);
    }
    let g = crate::effects::filter_outset(&fx.filter);
    rect(x0 - g, y0 - g, (x1 - x0 + 2.0 * g).max(0.0), (y1 - y0 + 2.0 * g).max(0.0))
}

/// Device-pixel rectangle covering `b`: (x0, y0, w, h).
fn device_rect(b: Bounds<f32>, scale: f32) -> (f32, f32, usize, usize) {
    let x0 = (b.origin.x * scale).floor();
    let y0 = (b.origin.y * scale).floor();
    let x1 = ((b.origin.x + b.size.width) * scale).ceil();
    let y1 = ((b.origin.y + b.size.height) * scale).ceil();
    (x0, y0, (x1 - x0).max(0.0) as usize, (y1 - y0).max(0.0) as usize)
}

/// Paint element `root` and its subtree `list` through its effects.
fn paint_layer(cx: &mut LayerCx, root: usize, list: &[usize], p: &mut Painter, base: f32) {
    use crate::effects as fx;
    use std::hash::{Hash, Hasher};
    let n = &cx.frame.nodes[root];
    let c = &cx.styles[n.style];
    let Some(e) = c.effects.as_deref().cloned() else { return };
    let rel_opacity = (n.opacity / base.max(1e-6)).min(1.0);
    // the content, painted at full opacity relative to the root
    let mut sub = Painter::new();
    paint_list(cx, list, &mut sub, Some(root), n.opacity);
    let n = &cx.frame.nodes[root];
    let c = &cx.styles[n.style];
    let screen = rect(-cx.vp.0, -cx.vp.1, cx.vp.0 * 3.0, cx.vp.1 * 3.0);
    let area = layer_area(cx, list, &e).intersect(&screen);
    let (x0, y0, w, h) = device_rect(area, cx.scale);
    if w == 0 || h == 0 {
        return;
    }
    let b = n.bounds;
    let u = Units { font_size: c.font_size, viewport: cx.vp };
    // the transform, in device pixels about transform-origin
    let mut m = fx::IDENTITY;
    if e.rotate != 0.0 {
        m = fx::mul(&m, &[e.rotate.cos(), e.rotate.sin(), -e.rotate.sin(), e.rotate.cos(), 0.0, 0.0]);
    }
    if let Some((sx, sy)) = e.scale2 {
        m = fx::mul(&m, &[sx, 0.0, 0.0, sy, 0.0, 0.0]);
    }
    if !e.transform.is_empty() {
        m = fx::mul(&m, &fx::matrix_of(&e.transform, b.size.width, b.size.height, &u));
    }
    let transformed = m != fx::IDENTITY;
    let origin = e.origin.unwrap_or([crate::style::Pos::CENTER; 2]);
    let (ox, oy) = ((b.origin.x + origin[0].at(b.size.width)) * cx.scale,
                    (b.origin.y + origin[1].at(b.size.height)) * cx.scale);
    let m_dev = {
        let lin = [m[0], m[1], m[2], m[3], m[4] * cx.scale, m[5] * cx.scale];
        fx::mul(&fx::mul(&[1.0, 0.0, 0.0, 1.0, ox, oy], &lin), &[1.0, 0.0, 0.0, 1.0, -ox, -oy])
    };
    let key = {
        let mut hh = Fnv64::default();
        sub.content_hash(&mut hh);
        format!("{e:?}").hash(&mut hh);
        (x0.to_bits(), y0.to_bits(), w, h, cx.scale.to_bits(), rel_opacity.to_bits(),
         b.origin.x.to_bits(), b.origin.y.to_bits(), b.size.width.to_bits(), b.size.height.to_bits()).hash(&mut hh);
        hh.finish()
    };
    let blend = e.blend;
    let emit = |p: &mut Painter, img: Arc<Image>, at: Bounds<f32>| {
        p.opacity = 1.0;
        p.clip = n.clip;
        p.image(at, &img);
    };
    if blend == fx::Blend::Normal {
        if let Some((img, at)) = LAYERS.with(|l| l.borrow_mut().get(key)) {
            emit(p, img, at);
            return;
        }
    }
    let Some(mut px) = sub.rasterize(cx.scale, x0, y0, w, h) else { return };
    for f in &e.filter {
        fx::apply_filter(&mut px, f, cx.scale);
    }
    if let Some((shape, edge)) = &e.clip_path {
        let mut mp = Painter::new();
        clip_shape(&mut mp, shape, *edge, c, b, &u);
        if let Some(mask) = mp.rasterize(cx.scale, x0, y0, w, h) {
            fx::apply_mask(&mut px, &mask, false);
        }
    }
    if let Some(mask) = e.mask.as_ref().filter(|m| m.image.is_some()) {
        let mut mp = Painter::new();
        paint_source(&mut mp, mask.image.as_ref().unwrap(), b, Some(mask), cx.vp);
        if let Some(mk) = mp.rasterize(cx.scale, x0, y0, w, h) {
            fx::apply_mask(&mut px, &mk, mask.luminance);
        }
    }
    if rel_opacity < 1.0 {
        px.px.iter_mut().for_each(|q| q.iter_mut().for_each(|v| *v *= rel_opacity));
    }
    let (mut px, (mut tx, mut ty)) = (px, (x0, y0));
    if transformed {
        match fx::transform(&px, (x0, y0), &m_dev) {
            Some((out, at)) => {
                px = out;
                (tx, ty) = at;
            }
            None => return, // a degenerate matrix draws nothing
        }
    }
    if blend != fx::Blend::Normal {
        if let Some(back) = p.rasterize(cx.scale, tx, ty, px.w, px.h) {
            px = fx::composite(&back, &px, blend);
        }
    }
    let img = Arc::new(Image { width: px.w as u32, height: px.h as u32, rgba: px.to_rgba8() });
    let at = rect(tx / cx.scale, ty / cx.scale, px.w as f32 / cx.scale, px.h as f32 / cx.scale);
    if blend == fx::Blend::Normal {
        LAYERS.with(|l| l.borrow_mut().put(key, img.clone(), at));
    }
    emit(p, img, at);
}

/// `backdrop-filter`: what's already painted under the element's border
/// box, filtered and clipped to its rounded box.
fn paint_backdrop(cx: &mut LayerCx, i: usize, p: &mut Painter) {
    use crate::effects as fx;
    let n = &cx.frame.nodes[i];
    let c = &cx.styles[n.style];
    let Some(e) = c.effects.as_deref() else { return };
    let b = n.bounds;
    let g = fx::filter_outset(&e.backdrop);
    let grown = rect(b.origin.x - g, b.origin.y - g, b.size.width + 2.0 * g, b.size.height + 2.0 * g);
    let (x0, y0, w, h) = device_rect(grown, cx.scale);
    let Some(mut px) = p.rasterize(cx.scale, x0, y0, w, h) else { return };
    for f in &e.backdrop {
        fx::apply_filter(&mut px, f, cx.scale);
    }
    let mut mp = Painter::new();
    mp.quad(b, radius_px(c, b, cx.vp), Rgba { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }, Rgba::TRANSPARENT, 0.0);
    if let Some(mask) = mp.rasterize(cx.scale, x0, y0, w, h) {
        fx::apply_mask(&mut px, &mask, false);
    }
    let img = Arc::new(Image { width: w as u32, height: h as u32, rgba: px.to_rgba8() });
    p.opacity = 1.0;
    p.clip = n.clip;
    p.image(rect(x0 / cx.scale, y0 / cx.scale, w as f32 / cx.scale, h as f32 / cx.scale), &img);
}

/// The coverage of a `clip-path` shape, painted white.
fn clip_shape(p: &mut Painter, shape: &crate::effects::ClipShape, edge: crate::style::BoxEdge, c: &Computed,
              b: Bounds<f32>, u: &Units) {
    use crate::effects::{ClipShape as S, ShapeRadius as R};
    let white = Rgba { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    let bw = border_widths(c, b.size.width);
    let rb = match edge {
        crate::style::BoxEdge::Border => b,
        crate::style::BoxEdge::Padding => rect(b.origin.x + bw[3], b.origin.y + bw[0],
                                                (b.size.width - bw[1] - bw[3]).max(0.0),
                                                (b.size.height - bw[0] - bw[2]).max(0.0)),
        crate::style::BoxEdge::Content => content_box(c, b),
    };
    let (x, y, w, h) = (rb.origin.x, rb.origin.y, rb.size.width, rb.size.height);
    let radius = |r: &R, cx: f32, cy: f32, horizontal: Option<bool>| -> f32 {
        let (dx, dy) = (cx.min(w - cx), cy.min(h - cy));
        let (fx_, fy) = (cx.max(w - cx), cy.max(h - cy));
        match (r, horizontal) {
            (R::Len(l), Some(true)) => l.resolve(u, w),
            (R::Len(l), Some(false)) => l.resolve(u, h),
            // circle: percentages of the box's normalised diagonal
            (R::Len(l), None) => l.resolve(u, (w * w + h * h).sqrt() / std::f32::consts::SQRT_2),
            (R::Closest, Some(true)) => dx,
            (R::Closest, Some(false)) => dy,
            (R::Closest, None) => dx.min(dy),
            (R::Farthest, Some(true)) => fx_,
            (R::Farthest, Some(false)) => fy,
            (R::Farthest, None) => fx_.max(fy),
        }
    };
    match shape {
        S::Box => p.fill(rb, white),
        S::Inset(s, r) => {
            let (t, rr, bb, l) = (s[0].resolve(u, h), s[1].resolve(u, w), s[2].resolve(u, h), s[3].resolve(u, w));
            let ib = rect(x + l, y + t, (w - l - rr).max(0.0), (h - t - bb).max(0.0));
            p.quad(ib, r.resolve(u, ib.size.width.min(ib.size.height)), white, Rgba::TRANSPARENT, 0.0);
        }
        S::Circle(r, at) => {
            let (cx_, cy) = (at[0].at(w), at[1].at(h));
            let rad = radius(r, cx_, cy, None);
            p.oval(rect(x + cx_ - rad, y + cy - rad, 2.0 * rad, 2.0 * rad), white, Rgba::TRANSPARENT, 0.0);
        }
        S::Ellipse(rx, ry, at) => {
            let (cx_, cy) = (at[0].at(w), at[1].at(h));
            let (a, bb) = (radius(rx, cx_, cy, Some(true)), radius(ry, cx_, cy, Some(false)));
            p.oval(rect(x + cx_ - a, y + cy - bb, 2.0 * a, 2.0 * bb), white, Rgba::TRANSPARENT, 0.0);
        }
        S::Polygon(pts) => {
            let pts: Vec<(f32, f32)> = pts.iter().map(|(px_, py)| (x + px_.resolve(u, w), y + py.resolve(u, h))).collect();
            p.polygon(&pts, white);
        }
    }
}

/// A mask or border-image source painted over `area` (sized, positioned and
/// tiled like a background when `mask` gives the placement).
fn paint_source(p: &mut Painter, src: &crate::effects::ImageSource, area: Bounds<f32>,
                mask: Option<&crate::effects::Mask>, _vp: (f32, f32)) {
    match src {
        crate::effects::ImageSource::Gradient(g) => p.with_clip(area, |p| paint_gradient(p, g, area)),
        crate::effects::ImageSource::Url(u) => {
            let mut st = crate::style::ImageStyle { bg_url: Some(u.clone()), ..Default::default() };
            if let Some(m) = mask {
                st.bg_size = m.size;
                st.bg_position = m.position;
                st.bg_repeat = m.repeat;
            }
            paint_background_image(p, &st, area, area);
        }
    }
}

/// FNV-1a for layer keys.
#[derive(Default)]
struct Fnv64(u64);

impl std::hash::Hasher for Fnv64 {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        let mut h = if self.0 == 0 { 0xcbf29ce484222325 } else { self.0 };
        for b in bytes {
            h = (h ^ *b as u64).wrapping_mul(0x100000001b3);
        }
        self.0 = h;
    }
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
    let border_image = c.effects.as_ref().and_then(|e| e.border_image.as_ref()).filter(|bi| bi.source.is_some());
    if let Some(bi) = border_image {
        paint_border_image(p, bi, b, &bw, &Units { font_size: c.font_size, viewport: vp });
    } else if styled && uniform && r > 0.0 && c.border_style.iter().all(|s| *s == c.border_style[0]) {
        paint_rounded_styled(p, b, r, bw[0], c.border_color[0], c.border_style[0]);
    } else if styled {
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

/// Blender's rounded-box corner tessellation (`cornervec`,
/// interface_widgets.cc): 9 points from the start of a quarter circle to its
/// end, as fractions of the radius.
const CORNERVEC: [[f32; 2]; 9] = [
    [0.0, 0.0], [0.195, 0.02], [0.383, 0.067], [0.55, 0.169], [0.707, 0.293], [0.831, 0.45], [0.924, 0.617],
    [0.98, 0.805], [1.0, 1.0],
];

/// A closed rounded rectangle outline as points (clockwise from the top
/// edge's start), with each point's outward normal direction: 0 top,
/// 1 right, 2 bottom, 3 left (corners split at their middle).
fn rounded_path(b: Bounds<f32>, r: f32) -> Vec<(f32, f32, usize)> {
    let (x0, y0, x1, y1) = (b.origin.x, b.origin.y, b.origin.x + b.size.width, b.origin.y + b.size.height);
    let r = r.min(b.size.width / 2.0).min(b.size.height / 2.0).max(0.0);
    let mut pts = Vec::with_capacity(40);
    // top-right, bottom-right, bottom-left, top-left corners, like Blender's round_box edges
    for (k, v) in CORNERVEC.iter().enumerate() {
        pts.push((x1 - r + v[0] * r, y0 + r - (1.0 - v[1]) * r, if k < 5 { 0 } else { 1 }));
    }
    for (k, v) in CORNERVEC.iter().enumerate() {
        pts.push((x1 - r + (1.0 - v[1]) * r, y1 - r + v[0] * r, if k < 5 { 1 } else { 2 }));
    }
    for (k, v) in CORNERVEC.iter().enumerate() {
        pts.push((x0 + r - v[0] * r, y1 - r + (1.0 - v[1]) * r, if k < 5 { 2 } else { 3 }));
    }
    for (k, v) in CORNERVEC.iter().enumerate() {
        pts.push((x0 + r - (1.0 - v[1]) * r, y0 + r - v[0] * r, if k < 5 { 3 } else { 0 }));
    }
    pts
}

/// Non-solid uniform borders on a rounded box: dashes and dots follow the
/// curve, `double` is two rounded lines, and groove / ridge / inset / outset
/// shade each side's part of the path.
fn paint_rounded_styled(p: &mut Painter, b: Bounds<f32>, r: f32, w: f32, col: Rgba, style: BorderStyle) {
    let inset = |d: f32| rect(b.origin.x + d, b.origin.y + d, (b.size.width - 2.0 * d).max(0.0),
                              (b.size.height - 2.0 * d).max(0.0));
    match style {
        BorderStyle::Double if w >= 3.0 => {
            let t = w / 3.0;
            p.quad(inset(t / 2.0), (r - t / 2.0).max(0.0), Rgba::TRANSPARENT, col, t);
            p.quad(inset(w - t / 2.0), (r - w + t / 2.0).max(0.0), Rgba::TRANSPARENT, col, t);
        }
        BorderStyle::Dashed | BorderStyle::Dotted => {
            let mut path = rounded_path(inset(w / 2.0), (r - w / 2.0).max(0.0));
            path.push(path[0]);
            let total: f32 = path.windows(2).map(|q| ((q[1].0 - q[0].0).powi(2) + (q[1].1 - q[0].1).powi(2)).sqrt()).sum();
            let dotted = style == BorderStyle::Dotted;
            let (on, gap) = if dotted { (w, w) } else { (w * 3.0, w * 3.0) };
            let n = ((total / (on + gap)).floor()).max(1.0);
            let period = total / n; // an even number of segments around the box
            let on = on.min(period);
            // walk the path, emitting [start, start + on) of every period
            let point_at = |d: f32| -> (f32, f32) {
                let mut acc = 0.0;
                for q in path.windows(2) {
                    let len = ((q[1].0 - q[0].0).powi(2) + (q[1].1 - q[0].1).powi(2)).sqrt();
                    if acc + len >= d && len > 0.0 {
                        let t = (d - acc) / len;
                        return (q[0].0 + (q[1].0 - q[0].0) * t, q[0].1 + (q[1].1 - q[0].1) * t);
                    }
                    acc += len;
                }
                (path[0].0, path[0].1)
            };
            for k in 0..n as usize {
                let start = k as f32 * period;
                if dotted {
                    let (cx, cy) = point_at(start + on / 2.0);
                    p.oval(rect(cx - w / 2.0, cy - w / 2.0, w, w), col, Rgba::TRANSPARENT, 0.0);
                } else {
                    let steps = 6;
                    let pts: Vec<(f32, f32)> = (0..=steps).map(|j| point_at(start + on * j as f32 / steps as f32)).collect();
                    p.line(&pts, col, w);
                }
            }
        }
        _ => {
            // shaded styles: the path in two halves of the width, coloured per side
            let dark = shade(col, 0.5);
            let side_col = |side: usize, outer: bool| -> Rgba {
                let lit = side == 0 || side == 3;
                match style {
                    BorderStyle::Inset => if lit { dark } else { col },
                    BorderStyle::Outset => if lit { col } else { dark },
                    BorderStyle::Groove => if lit == outer { dark } else { col },
                    BorderStyle::Ridge => if lit == outer { col } else { dark },
                    _ => col,
                }
            };
            let halves: &[(f32, bool)] = if matches!(style, BorderStyle::Groove | BorderStyle::Ridge) {
                &[(w / 4.0, true), (w * 3.0 / 4.0, false)]
            } else {
                &[(w / 2.0, true)]
            };
            let lw = if halves.len() == 2 { w / 2.0 } else { w };
            for &(d, outer) in halves {
                let mut path = rounded_path(inset(d), (r - d).max(0.0));
                path.push(path[0]);
                for q in path.windows(2) {
                    p.line(&[(q[0].0, q[0].1), (q[1].0, q[1].1)], side_col(q[0].2, outer), lw);
                }
            }
        }
    }
}

/// Border-image source pixels, cached per source and size.
fn border_image_source(src: &crate::effects::ImageSource, w: f32, h: f32) -> Option<Arc<Image>> {
    match src {
        crate::effects::ImageSource::Url(u) => background_file(u),
        crate::effects::ImageSource::Gradient(g) => {
            thread_local! {
                static GRADIENTS: std::cell::RefCell<HashMap<String, Arc<Image>>> =
                    std::cell::RefCell::new(HashMap::new());
            }
            let (iw, ih) = (w.ceil().max(1.0) as usize, h.ceil().max(1.0) as usize);
            let key = format!("{g:?}{iw}x{ih}");
            if let Some(i) = GRADIENTS.with(|m| m.borrow().get(&key).cloned()) {
                return Some(i);
            }
            let mut gp = Painter::new();
            let area = rect(0.0, 0.0, iw as f32, ih as f32);
            gp.with_clip(area, |gp| paint_gradient(gp, g, area));
            let px = gp.rasterize(1.0, 0.0, 0.0, iw, ih)?;
            let img = Arc::new(Image { width: iw as u32, height: ih as u32, rgba: px.to_rgba8() });
            GRADIENTS.with(|m| {
                let mut m = m.borrow_mut();
                if m.len() > 64 {
                    m.clear();
                }
                m.insert(key, img.clone());
            });
            Some(img)
        }
    }
}

/// A sub-rectangle of an image (whole pixels), cached.
fn crop(img: &Arc<Image>, x: u32, y: u32, w: u32, h: u32) -> Option<Arc<Image>> {
    if w == 0 || h == 0 || x + w > img.width || y + h > img.height {
        return None;
    }
    thread_local! {
        static CROPS: std::cell::RefCell<HashMap<(usize, [u32; 4]), Arc<Image>>> =
            std::cell::RefCell::new(HashMap::new());
    }
    let key = (Arc::as_ptr(img) as usize, [x, y, w, h]);
    if let Some(c) = CROPS.with(|m| m.borrow().get(&key).cloned()) {
        return Some(c);
    }
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for row in y..y + h {
        let s = ((row * img.width + x) * 4) as usize;
        rgba.extend_from_slice(&img.rgba[s..s + (w * 4) as usize]);
    }
    let c = Arc::new(Image { width: w, height: h, rgba });
    CROPS.with(|m| {
        let mut m = m.borrow_mut();
        if m.len() > 512 {
            m.clear();
        }
        m.insert(key, c.clone());
    });
    Some(c)
}

/// `border-image`: the source cut into nine parts by `slice`; corners go in
/// the corners of the border-image area, edges are stretched or tiled along
/// the sides, and the middle is drawn with `fill`.
fn paint_border_image(p: &mut Painter, bi: &crate::effects::BorderImage, b: Bounds<f32>, bw: &[f32; 4], u: &Units) {
    use crate::effects::{Side, Tile};
    let resolve = |v: Side, k: usize, basis: f32| -> f32 {
        match v {
            Side::Num(n) => n * bw[k],
            Side::Len(l) => l.resolve(u, basis),
            Side::Auto => bw[k],
        }
    };
    let out: [f32; 4] = std::array::from_fn(|k| resolve(bi.outset[k], k, 0.0));
    let area = rect(b.origin.x - out[3], b.origin.y - out[0], b.size.width + out[1] + out[3],
                    b.size.height + out[0] + out[2]);
    let Some(src) = bi.source.as_ref().and_then(|s| border_image_source(s, area.size.width, area.size.height)) else {
        return;
    };
    let (iw, ih) = (src.width as f32, src.height as f32);
    // slices in image pixels (numbers are pixels; percentages of the image)
    let sl: [f32; 4] = std::array::from_fn(|k| {
        let basis = if k % 2 == 0 { ih } else { iw };
        match bi.slice[k] {
            Side::Num(n) => n,
            Side::Len(Length::Percent(pc)) => basis * pc / 100.0,
            Side::Len(l) => l.resolve(u, basis),
            Side::Auto => 0.0,
        }.clamp(0.0, basis)
    });
    // widths in CSS px (numbers: multiples of the border width; auto: the slice)
    let mut wd: [f32; 4] = std::array::from_fn(|k| match bi.width[k] {
        Side::Auto => sl[k],
        v => resolve(v, k, if k % 2 == 0 { area.size.height } else { area.size.width }),
    });
    // scale down when opposite widths overlap
    let f = (area.size.width / (wd[1] + wd[3]).max(1e-6)).min(area.size.height / (wd[0] + wd[2]).max(1e-6)).min(1.0);
    wd.iter_mut().for_each(|v| *v *= f);
    let (ax, ay, aw, ah) = (area.origin.x, area.origin.y, area.size.width, area.size.height);
    let (sx, sy) = ([0.0, sl[3], iw - sl[1], iw], [0.0, sl[0], ih - sl[2], ih]);
    let (dx, dy) = ([ax, ax + wd[3], ax + aw - wd[1], ax + aw], [ay, ay + wd[0], ay + ah - wd[2], ay + ah]);
    for j in 0..3 {
        for i in 0..3 {
            if i == 1 && j == 1 && !bi.fill {
                continue;
            }
            let (cx, cy) = (sx[i].round() as u32, sy[j].round() as u32);
            let (cw, ch) = ((sx[i + 1] - sx[i]).round() as u32, (sy[j + 1] - sy[j]).round() as u32);
            let dest = rect(dx[i], dy[j], dx[i + 1] - dx[i], dy[j + 1] - dy[j]);
            if dest.size.width <= 0.0 || dest.size.height <= 0.0 {
                continue;
            }
            let Some(part) = crop(&src, cx, cy, cw, ch) else { continue };
            // edges tile along their length; corners always stretch
            let tile_x = if j != 1 || i == 1 { if i == 1 { bi.repeat.0 } else { Tile::Stretch } } else { Tile::Stretch };
            let tile_y = if i != 1 || j == 1 { if j == 1 { bi.repeat.1 } else { Tile::Stretch } } else { Tile::Stretch };
            // the tile's size: scaled to the edge's thickness
            let (mut tw, mut th) = (dest.size.width, dest.size.height);
            if tile_x != Tile::Stretch && i == 1 {
                tw = cw as f32 * (dest.size.height / ch.max(1) as f32);
            }
            if tile_y != Tile::Stretch && j == 1 {
                th = ch as f32 * (dest.size.width / cw.max(1) as f32);
            }
            let fit = |len: f32, t: f32, mode: Tile| -> (f32, usize, f32) {
                if mode == Tile::Stretch || t <= 0.0 {
                    return (len, 1, 0.0);
                }
                match mode {
                    Tile::Round => {
                        let n = (len / t).round().max(1.0);
                        (len / n, n as usize, 0.0)
                    }
                    Tile::Space => {
                        let n = (len / t).floor();
                        if n < 1.0 { (t, 0, 0.0) } else { (t, n as usize, (len - n * t) / (n + 1.0)) }
                    }
                    _ => {
                        // repeat: centred, partial tiles at both ends
                        let n = (len / t).ceil() as usize + 1;
                        (t, n, 0.0)
                    }
                }
            };
            let (tw2, nx, gx) = fit(dest.size.width, tw, if i == 1 { tile_x } else { Tile::Stretch });
            let (th2, ny, gy) = fit(dest.size.height, th, if j == 1 { tile_y } else { Tile::Stretch });
            let start = |len: f32, t: f32, n: usize, g: f32, mode: Tile| -> f32 {
                if mode == Tile::Repeat { (len - n as f32 * t) / 2.0 } else { g }
            };
            let x0 = dest.origin.x + start(dest.size.width, tw2, nx, gx, if i == 1 { tile_x } else { Tile::Stretch });
            let y0 = dest.origin.y + start(dest.size.height, th2, ny, gy, if j == 1 { tile_y } else { Tile::Stretch });
            p.with_clip(dest, |p| {
                for yy in 0..ny {
                    for xx in 0..nx {
                        let at = rect(x0 + xx as f32 * (tw2 + gx), y0 + yy as f32 * (th2 + gy), tw2, th2);
                        p.image(at, &part);
                    }
                }
            });
        }
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
