//! Text placement against a browser: the baseline in the line box and the
//! extent of text decorations, drawn at a resolution scale of 2 (the size
//! the site screenshots use, where measuring and drawing sizes differ).

use blenderui::prelude::*;

/// Inter's content area (OS/2 typo ascender / descender) and cap height, in em.
const ASCENT: f32 = 1984.0 / 2048.0;
const DESCENT: f32 = 494.0 / 2048.0;
const CAP: f32 = 1490.0 / 2048.0;

/// Every test here draws at scale 2 (the kit reads this once per run).
fn scale_2() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    // SAFETY: set once, before any window opens; every test calls this first.
    ONCE.call_once(|| unsafe { std::env::set_var("BUI_UI_SCALE", "2") });
}

fn open<V: Render>(cx: &mut App, w: f32, h: f32, v: V) -> WindowHandle<V> {
    let opts = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.0), px(0.0)),
            size: size(px(w), px(h)),
        })),
        ..Default::default()
    };
    cx.open_window(opts, |_, cx| cx.new(|_| v)).unwrap()
}

/// Device-pixel rows / columns of `img` inside `[x0, x1) x [y0, y1)` where
/// `ink` holds, as (first row, last row, first column, last column).
fn ink_box(img: &(usize, usize, Vec<u32>), (x0, y0, x1, y1): (usize, usize, usize, usize),
           ink: impl Fn(u32) -> bool) -> Option<(usize, usize, usize, usize)> {
    let mut b: Option<(usize, usize, usize, usize)> = None;
    for y in y0..y1.min(img.1) {
        for x in x0..x1.min(img.0) {
            if ink(img.2[y * img.0 + x]) {
                b = Some(b.map_or((y, y, x, x), |(t, bo, l, r)| (t.min(y), bo.max(y), l.min(x), r.max(x))));
            }
        }
    }
    b
}

fn red(px: u32) -> u32 {
    (px >> 16) & 0xff
}

struct Pill;

impl Render for Pill {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        body().m_0().child(
            div().style("display: flex; align-items: center; padding: 10px").child(
                div().id("pill")
                    .style("font-size: 10px; padding: 1px 8px; border-radius: 8px; background: #000; color: #fff")
                    .child("HEH"),
            ),
        )
    }
}

/// `.tag { font-size: 10px; padding: 1px 8px }`: the capitals sit where a
/// browser puts them, centred in the pill (they used to ride ~1.4px high).
#[test]
fn text_is_centred_like_a_browser_line_box() {
    scale_2();
    Application::headless().run(|cx| {
        let id = open(cx, 200.0, 60.0, Pill).id;
        let img = cx.draw(id);
        let s = img.0 as f32 / 200.0;
        assert!((s - 2.0).abs() < 1e-3, "scale {s}");
        let b = cx.element_bounds(id, "pill").unwrap();
        // white glyphs on the black pill, inside the padding (off its round corners)
        let area = (((b.origin.x + 8.0) * s) as usize, (b.origin.y * s) as usize,
                    ((b.origin.x + b.size.width - 8.0) * s) as usize, ((b.origin.y + b.size.height) * s) as usize);
        let (top, bottom, _, _) = ink_box(&img, area, |p| red(p) > 128).expect("no text drawn");
        // the browser: the content area centred in the 13px line box
        let (y, lh, size) = (b.origin.y + 1.0, 13.0, 10.0);
        let base = y + (lh - (ASCENT + DESCENT) * size) / 2.0 + ASCENT * size;
        let want = (base - CAP * size / 2.0) * s;
        let got = (top + bottom + 1) as f32 / 2.0;
        assert!((got - want).abs() <= 1.0, "cap centre {got} device px, a browser's {want}");
        // and so centred in the pill box itself
        let mid = (b.origin.y + b.size.height / 2.0) * s;
        assert!((got - mid).abs() <= 1.0, "cap centre {got}, pill centre {mid}");
    });
}

struct Struck;

impl Render for Struck {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        body().m_0().child(
            div().style("display: flex; align-items: center; width: 400px; height: 32px; padding: 0 10px").child(
                div().id("text").style("flex: 1; font-size: 13px; line-height: 17px; text-decoration: line-through; color: #000")
                    .child("Retopologize the robot arm"),
            ),
        )
    }
}

/// `.item .text { flex: 1 }` struck through: the line ends with the text
/// (it overshot ~9px when widths were measured at the logical size).
#[test]
fn line_through_ends_with_the_text() {
    scale_2();
    Application::headless().run(|cx| {
        let id = open(cx, 440.0, 40.0, Struck).id;
        let img = cx.draw(id);
        let s = img.0 as f32 / 440.0;
        let b = cx.element_bounds(id, "text").unwrap();
        let (x0, x1) = ((b.origin.x * s) as usize, ((b.origin.x + b.size.width) * s) as usize);
        let dark = |p: u32| red(p) < 96;
        let (top, bottom, left, right) = ink_box(&img, (x0, 0, x1, img.1), dark).expect("no text drawn");
        // the stroke: the rows dark furthest right of the text's own ink
        let rows: Vec<usize> = (top..=bottom)
            .filter(|&y| (left..=right).filter(|&x| dark(img.2[y * img.0 + x])).count() > (right - left) * 3 / 4)
            .collect();
        assert!(!rows.is_empty(), "no line-through");
        // the glyphs' own right edge: rows off the stroke
        let (glyph_right, _) = (top..=bottom).filter(|y| !rows.contains(y))
            .filter_map(|y| (left..=right).rev().find(|&x| dark(img.2[y * img.0 + x])).map(|x| (x, y)))
            .max().unwrap();
        let over = (right as f32 - glyph_right as f32) / s;
        assert!(over.abs() <= 1.0, "line-through ends {over} px past the last glyph");
        // a browser strikes through the x-height, below the cap centre
        let (y, lh, size) = (b.origin.y, 17.0, 13.0);
        let base = y + (lh - (ASCENT + DESCENT) * size) / 2.0 + ASCENT * size;
        let mid = (rows[0] + rows[rows.len() - 1] + 1) as f32 / 2.0 / s;
        assert!(mid < base && mid > base - CAP * size / 2.0, "stroke at {mid}, baseline {base}");
    });
}
