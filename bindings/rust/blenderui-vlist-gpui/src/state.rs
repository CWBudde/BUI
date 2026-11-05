//! [`ListState`]: the entity around a [`List`], like the data table's
//! `TableState`: the core's API reads as the state's own through `Deref`; what
//! is here needs gpui (the focus handle, the shaped-line cache, the frame clock).

use std::ops::{Deref, DerefMut};
use std::time::{Duration, Instant};

use blenderui_paint_gpui::GpuiSink;
use blenderui_vlist::{Budget, ItemPainter, ItemSource, List, Measurer, TextCache};
use gpui::{App, Bounds, Context, FocusHandle, Focusable, Pixels, ShapedLine, Window};

/// Shaped runs kept across frames (one per distinct line shown).
const TEXT_CACHE_CAPACITY: usize = 8192;

/// The list.
pub struct ListState<S: ItemSource + 'static, P: Measurer + ItemPainter + 'static> {
    list: List<S, P>,
    focus: FocusHandle,
    /// Shaped text kept across frames: scrolling moves the same lines.
    pub(crate) text: TextCache<ShapedLine>,
    pub(crate) text_scale: f32,
    last_frame: Option<Instant>,
}

impl<S: ItemSource, P: Measurer + ItemPainter> Deref for ListState<S, P> {
    type Target = List<S, P>;
    fn deref(&self) -> &Self::Target {
        &self.list
    }
}

impl<S: ItemSource, P: Measurer + ItemPainter> DerefMut for ListState<S, P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.list
    }
}

impl<S: ItemSource, P: Measurer + ItemPainter> Focusable for ListState<S, P> {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl<S: ItemSource, P: Measurer + ItemPainter> ListState<S, P> {
    /// An entity state for `list` (call inside `cx.new`).
    pub fn new(list: List<S, P>, cx: &mut Context<Self>) -> Self {
        Self { list, focus: cx.focus_handle(), text: TextCache::new(TEXT_CACHE_CAPACITY), text_scale: 1.0, last_frame: None }
    }

    /// Called while preparing to paint, with the list's bounds: resize,
    /// advance the smooth scroll, lay the frame out (text widths from the
    /// shaper, cached for the paint that follows) and do idle measuring.
    pub(crate) fn sync(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let (w, h) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        if w < 1.0 || h < 1.0 {
            return;
        }
        let scale = window.scale_factor();
        if (self.text_scale - scale).abs() > f32::EPSILON {
            self.text.clear();
            self.text_scale = scale;
        }
        let (vw, vh) = self.list.view.viewport();
        if (vw - w).abs() > f32::EPSILON || (vh - h).abs() > f32::EPSILON {
            self.list.resize(w, h);
        }
        let now = Instant::now();
        let dt = self.last_frame.replace(now).map_or(1.0 / 60.0, |t| now.duration_since(t).as_secs_f64().min(0.1));
        let mut more = self.list.view.tick(dt);
        self.text.next_generation();
        self.text.ensure_capacity(self.list.view.placed().len() * 6 + 256);
        let font = window.text_style().font();
        let Self { list, text, .. } = self;
        let mut sink = GpuiSink::new(window, cx, bounds.origin, text, &font, list.style.palette().text);
        {
            use blenderui_vlist::PaintSink as _;
            let mut tw = |s: &str, z: f32| sink.text_width(s, z);
            list.layout(&mut tw);
            if !list.view.flinging() {
                more |= list.prefetch(&mut tw, Budget::Time(Duration::from_micros(800)));
            }
        }
        if more {
            window.request_animation_frame();
        } else {
            self.last_frame = None;
        }
    }
}

impl<S: ItemSource, P: Measurer + ItemPainter> ListState<S, P> {
    /// Paint the laid-out frame into the window's scene at `origin`.
    pub(crate) fn paint_into(&mut self, window: &mut Window, cx: &mut App, origin: gpui::Point<Pixels>) {
        let font = window.text_style().font();
        let Self { list, text, .. } = self;
        let mut sink = GpuiSink::new(window, cx, origin, text, &font, list.style.palette().text);
        list.paint(&mut sink);
    }
}
