//! The scrollbar: a thumb sized and placed from the *estimated* content
//! height, with a minimum size so it stays grabbable at 10M items, and the
//! drag mapping back to a fraction of the content. Dragging far picks an
//! anchor from the estimate ([`crate::ListView::scroll_to_fraction`]); as
//! the items around it are measured the estimate improves but the anchor
//! stays, so the content under the pointer does not fight the user (only
//! the thumb adjusts). The look is Blender's slider: a thin rounded thumb
//! on a darkened track.

use blenderui_paint::{PaintSink, Palette, Rect};

/// Scrollbar width in px.
pub const WIDTH: f32 = 10.0;
/// The smallest thumb.
pub const MIN_THUMB: f32 = 24.0;

/// The thumb inside a track.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Thumb {
    pub y: f32,
    pub h: f32,
}

impl Thumb {
    /// The thumb for scroll position `top` of `total` px of content with a
    /// `view` px viewport in a `track` px track; `None` when everything fits.
    #[must_use]
    pub fn new(top: f64, total: f64, view: f64, track: f32) -> Option<Self> {
        if !(total > view && view > 0.0 && track > 0.0) {
            return None;
        }
        let h = ((f64::from(track) * view / total) as f32).clamp(MIN_THUMB.min(track), track);
        let free = f64::from(track - h);
        let y = (free * (top / (total - view)).clamp(0.0, 1.0)) as f32;
        Some(Self { y, h })
    }

    /// The content fraction for a thumb whose top is at `y` in the track.
    #[must_use]
    pub fn fraction(&self, y: f32, track: f32) -> f64 {
        let free = f64::from(track - self.h);
        if free <= 0.0 { 0.0 } else { (f64::from(y) / free).clamp(0.0, 1.0) }
    }

    /// Paint the track and thumb at the right edge of a `width` x `height` viewport.
    pub fn paint(&self, sink: &mut dyn PaintSink, pal: &Palette, width: f32, height: f32) {
        sink.fill(Rect::new(width - WIDTH, 0.0, WIDTH, height), pal.back.shade(-6.0));
        sink.round(Rect::new(width - WIDTH + 2.0, self.y + 1.0, WIDTH - 4.0, (self.h - 2.0).max(1.0)), (WIDTH - 4.0) * 0.5, pal.text.opacity(0.35));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumb_is_proportional_and_bounded() {
        let t = Thumb::new(0.0, 1000.0, 100.0, 200.0).unwrap();
        assert_eq!((t.y, t.h), (0.0, 24.0));
        let end = Thumb::new(900.0, 1000.0, 100.0, 200.0).unwrap();
        assert!((end.y + end.h - 200.0).abs() < 0.01);
        // 10M items: the thumb stays grabbable
        let tiny = Thumb::new(0.0, 6e8, 600.0, 600.0).unwrap();
        assert_eq!(tiny.h, MIN_THUMB);
        assert!(Thumb::new(0.0, 50.0, 100.0, 200.0).is_none());
    }

    #[test]
    fn fraction_inverts_position() {
        let t = Thumb::new(450.0, 1000.0, 100.0, 200.0).unwrap();
        assert!((t.fraction(t.y, 200.0) - 0.5).abs() < 0.01);
        assert_eq!(t.fraction(-5.0, 200.0), 0.0);
        assert_eq!(t.fraction(1e9, 200.0), 1.0);
    }
}
