//! The gpui side of painting, shared by every core-driven gpui view (the data
//! table, the variable list): a [`PaintSink`] that paints straight into the
//! window scene (`Window::paint_quad`, shaped lines), with a text-run cache so
//! a scrolling view never reshapes a string it showed a frame ago.

use blenderui_paint::{PaintSink, Rect, Rgba as CoreRgba, TextCache};
use gpui::{
    App, Bounds, BorderStyle, ContentMask, Font, Hsla, PathBuilder, Pixels, Point, ShapedLine, SharedString, TextAlign,
    TextRun, Window, point, px, quad, size, transparent_black,
};

/// Line height as a multiple of the font size.
const LINE_HEIGHT: f32 = 1.3;

/// The cache key's "style" for a text color: the table's main text color is 0.
fn style_key(c: CoreRgba, text: CoreRgba) -> u32 {
    if c == text {
        return 0;
    }
    let b = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
    // 0 is taken; fold the channels into the rest of the range
    1 + ((b(c.r) << 24 | b(c.g) << 16 | b(c.b) << 8 | b(c.a)) >> 1)
}

fn hsla(c: CoreRgba) -> Hsla {
    gpui::Rgba { r: c.r, g: c.g, b: c.b, a: c.a }.into()
}

/// Draws into a gpui window at `origin`.
pub struct GpuiSink<'a> {
    window: &'a mut Window,
    cx: &'a mut App,
    origin: Point<Pixels>,
    text: &'a mut TextCache<ShapedLine>,
    font: &'a Font,
    /// The table's main text color (cache style 0).
    main: CoreRgba,
}

impl<'a> GpuiSink<'a> {
    pub const fn new(
        window: &'a mut Window,
        cx: &'a mut App,
        origin: Point<Pixels>,
        text: &'a mut TextCache<ShapedLine>,
        font: &'a Font,
        main: CoreRgba,
    ) -> Self {
        Self { window, cx, origin, text, font, main }
    }

    fn bounds(&self, r: Rect) -> Bounds<Pixels> {
        Bounds { origin: point(self.origin.x + px(r.x), self.origin.y + px(r.y)), size: size(px(r.w), px(r.h)) }
    }
}

/// The shaped line for `text`, from the cache or shaped now.
fn shaped<'c>(
    cache: &'c mut TextCache<ShapedLine>,
    window: &Window,
    font: &Font,
    text: &str,
    key: u32,
    size_px: f32,
    color: Hsla,
) -> &'c ShapedLine {
    cache.get_or_insert_with(text, key, size_px, || {
        let run = TextRun { len: text.len(), font: font.clone(), color, background_color: None, underline: None, strikethrough: None };
        window.text_system().shape_line(SharedString::from(text.to_owned()), px(size_px), &[run], None)
    })
}

impl PaintSink for GpuiSink<'_> {
    fn fill(&mut self, r: Rect, c: CoreRgba) {
        if c.a > 0.0 && r.w > 0.0 && r.h > 0.0 {
            let b = self.bounds(r);
            self.window.paint_quad(gpui::fill(b, hsla(c)));
        }
    }

    fn stroke(&mut self, r: Rect, c: CoreRgba, width: f32) {
        let b = self.bounds(r);
        self.window.paint_quad(quad(b, px(0.0), transparent_black(), px(width), hsla(c), BorderStyle::Solid));
    }

    fn round(&mut self, r: Rect, radius: f32, c: CoreRgba) {
        let b = self.bounds(r);
        self.window.paint_quad(quad(b, px(radius), hsla(c), px(0.0), transparent_black(), BorderStyle::Solid));
    }

    fn line(&mut self, a: (f32, f32), b: (f32, f32), c: CoreRgba, width: f32) {
        let o = self.origin;
        let mut path = PathBuilder::stroke(px(width));
        path.move_to(point(o.x + px(a.0), o.y + px(a.1)));
        path.line_to(point(o.x + px(b.0), o.y + px(b.1)));
        if let Ok(path) = path.build() {
            self.window.paint_path(path, hsla(c));
        }
    }

    fn text(&mut self, x: f32, cy: f32, text: &str, size_px: f32, c: CoreRgba) {
        if text.trim().is_empty() {
            return;
        }
        let key = style_key(c, self.main);
        let line_h = px(size_px * LINE_HEIGHT);
        let line = shaped(self.text, self.window, self.font, text, key, size_px, hsla(c));
        let origin = point(self.origin.x + px(x), self.origin.y + px(cy) - line_h / 2.0);
        let _ = line.paint(origin, line_h, TextAlign::Left, None, self.window, self.cx);
    }

    fn text_width(&mut self, text: &str, size_px: f32) -> f32 {
        let main = hsla(self.main);
        f32::from(shaped(self.text, self.window, self.font, text, 0, size_px, main).width())
    }

    fn clipped(&mut self, r: Rect, f: &mut dyn FnMut(&mut dyn PaintSink)) {
        let mask = ContentMask { bounds: self.bounds(r) };
        let Self { window, cx, origin, text, font, main } = self;
        let (origin, font, main) = (*origin, *font, *main);
        window.with_content_mask(Some(mask), |window| {
            f(&mut GpuiSink { window, cx, origin, text, font, main });
        });
    }
}
