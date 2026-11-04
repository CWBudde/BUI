//! The CSS 2.1 leftovers and everyday visual properties: border and outline
//! styles, decoration styles, background clip / origin, `clip`, counters,
//! `empty-cells`, `word-break` / `overflow-wrap` and `tab-size`. Each test
//! lays out a page headless and checks boxes or pixels.

use blenderui::prelude::*;

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

struct Page {
    css: &'static str,
    body: fn() -> Div,
}

impl Render for Page {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        (self.body)().child(style().child(self.css))
    }
}

/// Lay out a page; returns the window and its pixels (0x00RRGGBB) with a
/// lookup in logical coordinates.
fn page(cx: &mut App, w: f32, h: f32, css: &'static str, body: fn() -> Div)
        -> (WindowId, impl Fn(f32, f32) -> u32 + use<>) {
    let win = open(cx, w, h, Page { css, body });
    let img = cx.draw(win.id);
    let s = img.0 as f32 / w;
    (win.id, move |x: f32, y: f32| img.2[(y * s) as usize * img.0 + (x * s) as usize])
}

fn width(cx: &mut App, win: WindowId, id: &str) -> f32 {
    cx.element_bounds(win, id).unwrap_or_else(|| panic!("no element {id}")).size.width
}

fn height(cx: &mut App, win: WindowId, id: &str) -> f32 {
    cx.element_bounds(win, id).unwrap_or_else(|| panic!("no element {id}")).size.height
}

const BG: u32 = 0x000000;

#[test]
fn border_and_outline_styles() {
    Application::headless().run(|cx| {
        let (_, at) = page(cx, 300.0, 200.0, "
            html, body { margin: 0; background: #000 }
            div { width: 100px; height: 20px; margin-bottom: 10px }
            #solid  { border: 2px solid #f00 }
            #dashed { border: 2px dashed #f00 }
            #double { border: 6px double #f00 }
            #inset  { border: 4px inset #f00 }
            #outl   { outline: 2px dashed #0f0; outline-offset: 0; margin-left: 10px }
        ", || {
            body().child(div().id("solid")).child(div().id("dashed")).child(div().id("double"))
                .child(div().id("inset")).child(div().id("outl"))
        });
        // solid: the whole top edge
        assert_eq!(at(9.0, 1.5), 0xff0000);
        // dashed (y = 30): a dash at the start, a gap after it (dash 6px, gap > 6px)
        assert_eq!(at(2.0, 30.5), 0xff0000);
        assert_eq!(at(9.0, 30.5), BG, "gap between dashes");
        // double (y = 60): outer and inner thirds drawn, the middle third empty
        assert_eq!(at(50.0, 60.5), 0xff0000);
        assert_eq!(at(50.0, 62.5), BG, "middle of a double border");
        assert_eq!(at(50.0, 64.5), 0xff0000);
        // inset (y = 90): the top side darker than the bottom side
        let (top, bottom) = (at(50.0, 91.0), at(50.0, 90.0 + 20.0 - 2.0));
        assert_eq!(bottom, 0xff0000);
        assert!(top >> 16 < 0xc0 && top >> 16 > 0x40, "inset top is shaded: {top:06x}");
        // dashed outline (y = 120, outside the box): dash then gap
        assert_eq!(at(10.0 - 1.0, 120.0 - 1.0), 0x00ff00);
        assert_eq!(at(10.0 - 2.0 + 9.0, 120.0 - 1.0), BG);
    });
}

#[test]
fn background_clip_and_origin() {
    Application::headless().run(|cx| {
        let (_, at) = page(cx, 200.0, 100.0, "
            html, body { margin: 0; background: #000 }
            #a { width: 60px; height: 60px; padding: 10px; border: 10px solid transparent;
                 background: #00f; background-clip: content-box }
            #b { position: absolute; left: 100px; top: 0; width: 60px; height: 60px;
                 border: 10px solid transparent; background-color: #00f; background-clip: padding-box }
        ", || body().child(div().id("a")).child(div().id("b")));
        assert_eq!(at(5.0, 5.0), BG, "border area unpainted");
        assert_eq!(at(15.0, 15.0), BG, "padding area unpainted");
        assert_eq!(at(25.0, 25.0), 0x0000ff, "content painted");
        assert_eq!(at(105.0, 5.0), BG, "padding-box: border unpainted");
        assert_eq!(at(115.0, 15.0), 0x0000ff);
    });
}

#[test]
fn clip_rect_on_absolute_boxes() {
    Application::headless().run(|cx| {
        let (_, at) = page(cx, 200.0, 100.0, "
            html, body { margin: 0; background: #000 }
            #a { position: absolute; left: 0; top: 0; width: 80px; height: 80px; background: #f00;
                 clip: rect(10px, 50px, auto, 20px) }
        ", || body().child(div().id("a")));
        assert_eq!(at(5.0, 5.0), BG, "above / left of the clip");
        assert_eq!(at(30.0, 20.0), 0xff0000);
        assert_eq!(at(60.0, 20.0), BG, "right of the clip");
        assert_eq!(at(30.0, 79.0), 0xff0000, "auto bottom = the box's bottom");
    });
}

#[test]
fn counters_in_generated_content() {
    Application::headless().run(|cx| {
        let (w, _) = page(cx, 400.0, 200.0, r#"
            body { margin: 0; counter-reset: n }
            span { display: inline-block }
            .i { counter-increment: n }
            .i::before { content: counter(n, upper-roman) }
            .nest { counter-reset: n 4 }
            #deep::before { content: counters(n, "-") }
            #r1::before { content: "I" }
            #r3::before { content: "III" }
            #rdeep::before { content: "3-5" }
            #after-nest::before { content: counter(n) }
            #r4::before { content: "4" }
        "#, || {
            body()
                .child(span().id("i1").class("i"))
                .child(span().id("i2").class("i"))
                .child(span().id("i3").class("i"))
                .child(div().class("nest").child(span().id("deep").class("i")))
                // the nested reset is out of scope again: the outer counter goes on
                .child(span().id("after-nest").class("i"))
                .child(span().id("r1")).child(span().id("r3")).child(span().id("rdeep")).child(span().id("r4"))
        });
        assert_eq!(width(cx, w, "i1"), width(cx, w, "r1"));
        assert_eq!(width(cx, w, "i3"), width(cx, w, "r3"));
        assert!(width(cx, w, "i3") > width(cx, w, "i1"));
        assert_eq!(width(cx, w, "deep"), width(cx, w, "rdeep"), "counters() joins nested instances");
        assert_eq!(width(cx, w, "after-nest"), width(cx, w, "r4"));
    });
}

#[test]
fn empty_cells_hide() {
    Application::headless().run(|cx| {
        let (_, at) = page(cx, 200.0, 100.0, "
            html, body { margin: 0; background: #000 }
            table { width: 100px; empty-cells: hide; border-spacing: 0 }
            td { width: 50px; height: 30px; background: #0f0; padding: 0 }
        ", || {
            body().child(table().child(tr().child(td().child("x")).child(td().child(" "))))
        });
        assert_eq!(at(10.0, 25.0), 0x00ff00, "a cell with content paints");
        assert_eq!(at(90.0, 25.0), BG, "an empty cell doesn't");
    });
}

#[test]
fn word_break_overflow_wrap_and_tabs() {
    Application::headless().run(|cx| {
        let (w, _) = page(cx, 400.0, 400.0, "
            body { margin: 0 }
            p { margin: 0; width: 60px; line-height: 20px }
            #normal { }
            #anywhere { overflow-wrap: anywhere }
            #all { word-break: break-all }
            #mixed { word-break: break-all }
            pre { margin: 0; display: inline-block }
            #t8 { tab-size: 8 }
            #t2 { tab-size: 2 }
        ", || {
            body()
                .child(p().id("normal").child("aaaaaaaaaaaaaaaaaaaaaaaa"))
                .child(p().id("anywhere").child("aaaaaaaaaaaaaaaaaaaaaaaa"))
                .child(p().id("all").child("aaaaaaaaaaaaaaaaaaaaaaaa"))
                .child(p().id("mixed").child("ab aaaaaaaaaaaaaaaaaaaaaa"))
                .child(pre().id("t8").child("\tx"))
                .child(pre().id("t2").child("\tx"))
        });
        assert_eq!(height(cx, w, "normal"), 20.0, "a long word overflows on one line");
        assert!(height(cx, w, "anywhere") >= 40.0, "overflow-wrap breaks it");
        assert!(height(cx, w, "all") >= 40.0, "word-break: break-all breaks it");
        assert!(height(cx, w, "mixed") >= 40.0, "break-all inside line boxes");
        assert!(width(cx, w, "t8") > width(cx, w, "t2"), "tab-size widens the tab");
    });
}

#[test]
fn decoration_styles_parse_and_paint() {
    Application::headless().run(|cx| {
        let (_, at) = page(cx, 200.0, 100.0, "
            html, body { margin: 0; background: #000 }
            p { margin: 0; font-size: 20px; line-height: 30px; color: #000;
                text-decoration: underline wavy #f00 3px; text-underline-offset: 2px }
            #b { position: absolute; top: 50px; left: 0; width: 100px; height: 20px;
                 font-size: 20px; line-height: 20px; color: #000;
                 text-decoration: underline dotted #0f0; text-decoration-thickness: 4px }
        ", || {
            body().child(p().child("mmmmmmmm")).child(div().id("b").child("mmmmmmmm"))
        });
        // wavy: red appears somewhere on the decoration band
        let red = (0..60).flat_map(|x| (20..32).map(move |y| (x as f32, y as f32)))
            .any(|(x, y)| at(x, y) == 0xff0000);
        assert!(red, "wavy underline painted");
        // dotted 4px: green dots with gaps along one row
        let row: Vec<bool> = (0..40).map(|x| {
            (50..75).any(|y| at(x as f32 + 0.5, y as f32 + 0.5) == 0x00ff00)
        }).collect();
        assert!(row.iter().any(|g| *g) && row.iter().any(|g| !*g), "dots with gaps: {row:?}");
    });
}

#[test]
fn styled_scopes_the_blender_theme() {
    Application::headless().run(|cx| {
        let (w, at) = page(cx, 300.0, 200.0, "
            body { margin: 0 }
            .box { height: 40px }
        ", || {
            body()
                .child(div().id("plain").class("box").child(pre().id("plain-pre").child("x")))
                .child(div().id("themed").class("box").styled().child(pre().id("themed-pre").child("x")))
                .child(div().styled().child(div().id("opt-out").class("box").attr("styled", "false")))
        });
        // the page itself: browser defaults
        assert_eq!(at(290.0, 5.0), 0xffffff, "unstyled page is white");
        let t = cx.element_bounds(w, "themed").unwrap();
        assert_eq!(at(290.0, t.origin.y + t.size.height - 2.0), 0x303030,
                   "a styled element gets Blender's background");
        // UA rules differ inside: Blender's <pre> has padding and a dark background
        let plain = cx.element_bounds(w, "plain-pre").unwrap();
        let themed = cx.element_bounds(w, "themed-pre").unwrap();
        assert!(themed.size.height > plain.size.height, "Blender pre is padded");
        assert_eq!(at(themed.origin.x + 2.0, themed.origin.y + 2.0), 0x232323);
        // styled="false" turns it back off for a subtree
        let o = cx.element_bounds(w, "opt-out").unwrap();
        assert_eq!(at(o.origin.x + 2.0, o.origin.y + 2.0), 0x303030,
                   "the styled parent's background shows through");
        assert!(height(cx, w, "opt-out") == 40.0);
    });
}

#[test]
fn styled_in_rsx() {
    struct V;
    impl Render for V {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            rsx! { <body m-0><div id="t" styled h-10>"hi"</div></body> }
        }
    }
    Application::headless().run(|cx| {
        let win = open(cx, 100.0, 100.0, V);
        let img = cx.draw(win.id);
        let s = img.0 as f32 / 100.0;
        assert_eq!(img.2[(30.0 * s) as usize * img.0 + (90.0 * s) as usize], 0x303030);
        assert_eq!(img.2[(80.0 * s) as usize * img.0 + (90.0 * s) as usize], 0xffffff);
    });
}
