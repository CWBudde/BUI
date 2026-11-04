//! Pixel effects (filter, backdrop-filter, mix-blend-mode, clip-path, mask,
//! rotate / skew / scale transforms), rounded dashed borders, border-image,
//! table column sizing, scroll snapping, overflow-wrap in inline text and
//! the capitalized Blender components, end to end on headless pages.

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

fn page(cx: &mut App, w: f32, h: f32, css: &'static str, body: fn() -> Div)
        -> (WindowId, impl Fn(f32, f32) -> u32 + use<>) {
    let win = open(cx, w, h, Page { css, body });
    let img = cx.draw(win.id);
    let s = img.0 as f32 / w;
    (win.id, move |x: f32, y: f32| img.2[(y * s) as usize * img.0 + (x * s) as usize])
}

fn rgb_of(p: u32) -> (i32, i32, i32) {
    ((p >> 16 & 255) as i32, (p >> 8 & 255) as i32, (p & 255) as i32)
}

fn near(p: u32, q: u32, tol: i32) -> bool {
    let (a, b) = (rgb_of(p), rgb_of(q));
    (a.0 - b.0).abs() <= tol && (a.1 - b.1).abs() <= tol && (a.2 - b.2).abs() <= tol
}

const BASE: &str = "html, body { margin: 0; background: #000 } .b { position: absolute }";

#[test]
fn filters() {
    Application::headless().run(|cx| {
        let (_, at) = page(cx, 300.0, 100.0, "
            html, body { margin: 0; background: #000 } .b { position: absolute; top: 10px; width: 40px; height: 40px }
            #g { left: 10px; background: #f00; filter: grayscale(1) }
            #i { left: 60px; background: #ff0000; filter: invert(1) }
            #o { left: 110px; background: #fff; filter: opacity(0.5) }
            #bl { left: 180px; background: #fff; filter: blur(4px) }
        ", || {
            body().child(div().id("g").class("b")).child(div().id("i").class("b"))
                .child(div().id("o").class("b")).child(div().id("bl").class("b"))
        });
        let (r, g, b) = rgb_of(at(30.0, 30.0));
        assert!((r - g).abs() <= 2 && (g - b).abs() <= 2 && r > 40 && r < 80, "grayscale red: {r} {g} {b}");
        assert!(near(at(80.0, 30.0), 0x00ffff, 3), "invert: {:06x}", at(80.0, 30.0));
        assert!(near(at(130.0, 30.0), 0x808080, 3), "opacity: {:06x}", at(130.0, 30.0));
        // blur spreads past the box and softens its edge
        assert!(rgb_of(at(176.0, 30.0)).0 > 10, "blurred outside: {:06x}", at(176.0, 30.0));
        assert!(rgb_of(at(181.0, 30.0)).0 < 250, "soft edge: {:06x}", at(181.0, 30.0));
        assert!(near(at(200.0, 30.0), 0xffffff, 8), "centre stays: {:06x}", at(200.0, 30.0));
    });
}

#[test]
fn transforms_rotate_scale_skew() {
    Application::headless().run(|cx| {
        let (_, at) = page(cx, 300.0, 200.0, "
            html, body { margin: 0; background: #000 } .b { position: absolute; background: #0f0 }
            #r { left: 50px; top: 50px; width: 80px; height: 20px; transform: rotate(90deg) }
            #s { left: 200px; top: 20px; width: 20px; height: 20px; scale: 2 }
            #k { left: 150px; top: 150px; width: 40px; height: 20px; transform: skewX(45deg) }
        ", || body().child(div().id("r").class("b")).child(div().id("s").class("b")).child(div().id("k").class("b")));
        // a 80x20 bar rotated about its centre (90, 60) stands 20 wide, 80 tall
        assert_eq!(at(90.0, 30.0), 0x00ff00, "rotated bar reaches up");
        assert_eq!(at(60.0, 60.0), 0x000000, "and no longer reaches left");
        // scale(2) about the centre (210, 30): 20px grows to 40px
        assert_eq!(at(193.0, 30.0), 0x00ff00, "scaled box grows");
        // skewX(45deg) about the centre leans the top right, the bottom left
        // (y points down, so the top edge moves left and the bottom right)
        assert_eq!(at(143.0, 152.0), 0x00ff00, "top edge pushed left");
        assert_eq!(at(193.0, 168.0), 0x00ff00, "bottom edge pushed right");
    });
}

#[test]
fn clip_path_and_mask() {
    Application::headless().run(|cx| {
        let (_, at) = page(cx, 300.0, 120.0, "
            html, body { margin: 0; background: #000 } .b { position: absolute; top: 10px; width: 80px; height: 80px; background: #f00 }
            #c { left: 10px; clip-path: circle(40%) }
            #p { left: 110px; clip-path: polygon(0 0, 100% 0, 0 100%) }
            #m { left: 210px; mask-image: linear-gradient(to right, #000, transparent) }
        ", || body().child(div().id("c").class("b")).child(div().id("p").class("b")).child(div().id("m").class("b")));
        assert_eq!(at(50.0, 50.0), 0xff0000, "circle centre");
        assert_eq!(at(13.0, 13.0), 0x000000, "circle corner clipped");
        assert_eq!(at(120.0, 20.0), 0xff0000, "triangle kept part");
        assert_eq!(at(180.0, 80.0), 0x000000, "triangle clipped part");
        let (l, r) = (rgb_of(at(215.0, 50.0)).0, rgb_of(at(285.0, 50.0)).0);
        assert!(l > 200 && r < 50, "mask fades left to right: {l} {r}");
    });
}

#[test]
fn blend_modes_and_backdrop_filter() {
    Application::headless().run(|cx| {
        let (_, at) = page(cx, 300.0, 120.0, "
            html, body { margin: 0; background: #000 }
            .under { position: absolute; top: 10px; width: 60px; height: 60px; background: #ffff00 }
            #m { position: absolute; top: 10px; left: 10px; width: 60px; height: 60px; background: #00ffff; mix-blend-mode: multiply }
            #d { position: absolute; top: 10px; left: 110px; width: 60px; height: 60px; background: #808080; mix-blend-mode: difference }
            #bd { position: absolute; top: 10px; left: 210px; width: 60px; height: 60px; backdrop-filter: invert(1) }
        ", || {
            body().child(div().class("under").style("left: 10px")).child(div().class("under").style("left: 110px"))
                .child(div().class("under").style("left: 210px"))
                .child(div().id("m")).child(div().id("d")).child(div().id("bd"))
        });
        assert!(near(at(40.0, 40.0), 0x00ff00, 3), "yellow x cyan = green: {:06x}", at(40.0, 40.0));
        assert!(near(at(140.0, 40.0), 0x7f7f80, 3), "difference: {:06x}", at(140.0, 40.0));
        assert!(near(at(240.0, 40.0), 0x0000ff, 3), "inverted backdrop: {:06x}", at(240.0, 40.0));
    });
}

#[test]
fn rounded_dashed_borders_and_border_image() {
    Application::headless().run(|cx| {
        let (_, at) = page(cx, 300.0, 120.0, "
            html, body { margin: 0; background: #000 }
            #d { position: absolute; left: 10px; top: 10px; width: 100px; height: 60px;
                 border: 4px dashed #f00; border-radius: 20px }
            #i { position: absolute; left: 150px; top: 10px; width: 100px; height: 60px; border: 10px solid;
                 border-image: linear-gradient(#0f0, #00f) 10 }
        ", || body().child(div().id("d")).child(div().id("i")));
        // a dashed rounded border: the corner region inside the radius stays empty
        assert_eq!(at(11.0, 11.0), 0x000000, "corner outside the curve");
        let top: Vec<bool> = (30..90).map(|x| at(x as f32 + 0.5, 12.0) == 0xff0000).collect();
        assert!(top.iter().any(|v| *v) && top.iter().any(|v| !*v), "dashes along the top");
        // the border image paints the border area, not the inside
        let edge = at(152.0, 40.0);
        assert!(edge != 0x000000, "border image drawn: {edge:06x}");
        assert_eq!(at(200.0, 40.0), 0x000000, "middle not filled without `fill`");
    });
}

#[test]
fn tables_size_columns_to_content() {
    Application::headless().run(|cx| {
        let (w, _) = page(cx, 600.0, 200.0, "
            body { margin: 0 } table { border-spacing: 0 } td { padding: 0 }
            #fixed { table-layout: fixed; width: 300px }
        ", || {
            body()
                .child(table().id("auto").child(tbody()
                    .child(tr().child(td().id("a1").child("x")).child(td().id("b1").child("a much longer cell")))
                    .child(tr().child(td().id("a2").child("y")).child(td().id("b2").child("z")))))
                .child(table().id("fixed").child(tbody()
                    .child(tr().child(td().id("f1").style("width: 100px").child("x")).child(td().id("f2").child("y")))))
        });
        let r = |id: &str, cx: &mut App| cx.element_bounds(w, id).unwrap();
        let (a1, b1, a2, b2) = (r("a1", cx), r("b1", cx), r("a2", cx), r("b2", cx));
        assert!(b1.size.width > 2.0 * a1.size.width, "columns follow content");
        assert_eq!(a1.size.width, a2.size.width, "a column lines up across rows");
        assert_eq!(b1.origin.x, b2.origin.x);
        let t = r("auto", cx);
        assert!(t.size.width < 400.0, "an auto table shrinks to its columns: {}", t.size.width);
        assert_eq!(r("f1", cx).size.width, 100.0, "fixed layout uses the first row's widths");
        assert_eq!(r("f2", cx).size.width, 200.0);
    });
}

#[test]
fn scroll_snap_mandatory() {
    Application::headless().run(|cx| {
        let (w, _) = page(cx, 200.0, 200.0, "
            body { margin: 0 }
            #s { height: 100px; overflow-y: scroll; scroll-snap-type: y mandatory }
            .p { height: 100px; scroll-snap-align: start }
        ", || {
            body().child(div().id("s").child(div().class("p")).child(div().id("p2").class("p")).child(div().class("p")))
        });
        cx.simulate_scroll(w, 50.0, 50.0, -1);
        cx.draw(w);
        // one wheel notch lands exactly on the next page: the second child at the top
        let p2 = cx.element_bounds(w, "p2").unwrap();
        assert_eq!(p2.origin.y, 0.0, "snapped to the second child: {p2:?}");
    });
}

#[test]
fn overflow_wrap_in_inline_text() {
    Application::headless().run(|cx| {
        let (w, _) = page(cx, 400.0, 400.0, "
            body { margin: 0 } p { width: 60px; margin: 0; line-height: 20px; overflow-wrap: anywhere }
        ", || {
            body().child(p().id("p").child("short ").child(b().child("aaaaaaaaaaaaaaaaaaaaaaaaaaaa")))
        });
        cx.draw(w); // widths settle after one more layout
        let h = cx.element_bounds(w, "p").unwrap().size.height;
        assert!(h >= 60.0, "the long bold word breaks across lines: {h}");
    });
}

struct Blender;

impl Render for Blender {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        rsx! {
            <body m-0>
                <Button id="bb" onClick={|| {}}>"Apply"</Button>
                <NumberSlider id="ns" min={0.0} max={10.0} />
                <Checkbox id="cb" checked={true} />
                <input id="plain" type="range" />
            </body>
        }
    }
}

#[test]
fn blender_components_are_elements() {
    Application::headless().run(|cx| {
        let win = open(cx, 300.0, 200.0, Blender);
        cx.draw(win.id);
        for id in ["bb", "ns", "cb", "plain"] {
            assert!(cx.element_bounds(win.id, id).is_some(), "{id} laid out");
        }
        // gpui syntax: the same components are functions
        let d = Checkbox().checked(true);
        let _ = d;
    });
}

#[allow(dead_code)]
const _BASE: &str = BASE;
