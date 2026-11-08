//! The picker's icons, drawn with strokes (an icon font would be one more thing to ship).

use blenderui::prelude::*;

pub fn calendar(color: u32) -> Div {
    let c = rgb(color);
    canvas(move |b, p| {
        let (x, y, w, h) = (b.origin.x, b.origin.y, b.size.width, b.size.height);
        p.quad(b, 4.0, rgba(0x00000000), c, 1.7);
        p.line(&[(x + 1.0, y + h * 0.34), (x + w - 1.0, y + h * 0.34)], c, 1.7);
        for at in [0.32, 0.68] {
            p.line(&[(x + w * at, y - 0.5), (x + w * at, y + h * 0.22)], c, 1.9);
        }
    })
    .w(px(18.0))
    .h(px(18.0))
}

pub fn close(color: u32) -> Div {
    let c = rgb(color);
    canvas(move |b, p| {
        let (x, y, w, h, pad) = (b.origin.x, b.origin.y, b.size.width, b.size.height, 3.0);
        p.line(&[(x + pad, y + pad), (x + w - pad, y + h - pad)], c, 1.8);
        p.line(&[(x + w - pad, y + pad), (x + pad, y + h - pad)], c, 1.8);
    })
    .w(px(13.0))
    .h(px(13.0))
}

/// The small "v" beside a month's name.
pub fn chevron_down(color: u32) -> Div {
    let c = rgb(color);
    canvas(move |b, p| {
        let (x, y, w, h) = (b.origin.x, b.origin.y, b.size.width, b.size.height);
        p.line(&[(x, y), (x + w / 2.0, y + h)], c, 1.7);
        p.line(&[(x + w, y), (x + w / 2.0, y + h)], c, 1.7);
    })
    .w(px(10.0))
    .h(px(6.0))
}

/// The "<" and ">" of the month buttons.
pub fn chevron_side(color: u32, right: bool) -> Div {
    let c = rgb(color);
    canvas(move |b, p| {
        let (x, y, w, h) = (b.origin.x, b.origin.y, b.size.width, b.size.height);
        let pad = 2.5; // room for the stroke, which a canvas clips at its edge
        let (tail, tip) = if right { (x + pad, x + w - pad) } else { (x + w - pad, x + pad) };
        let (y, h) = (y + pad, h - 2.0 * pad);
        p.line(&[(tail, y), (tip, y + h / 2.0)], c, 1.9);
        p.line(&[(tip, y + h / 2.0), (tail, y + h)], c, 1.9);
    })
    .w(px(12.0))
    .h(px(18.0))
}
