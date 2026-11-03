//! Modern CSS end to end: custom properties, math functions, grid areas,
//! floats, sizing keywords, positioning. Each test lays out a page headless
//! and checks the boxes (and pixels where paint matters).

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

/// `(x, y, w, h)` of the element with `id`.
fn rect(cx: &mut App, win: WindowId, id: &str) -> (f32, f32, f32, f32) {
    let b = cx.element_bounds(win, id).unwrap_or_else(|| panic!("no element {id}"));
    (b.origin.x, b.origin.y, b.size.width, b.size.height)
}

/// A page from a stylesheet and a body built by `f`.
struct Page {
    css: &'static str,
    body: fn() -> Div,
}

impl Render for Page {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        (self.body)().child(style().child(self.css))
    }
}

fn page(cx: &mut App, w: f32, h: f32, css: &'static str, body: fn() -> Div) -> WindowId {
    let win = open(cx, w, h, Page { css, body });
    cx.draw(win.id);
    win.id
}

#[test]
fn custom_properties_and_math() {
    Application::headless().run(|cx| {
        let w = page(cx, 400.0, 300.0, "
            :root { --gap: 10px; --w: calc(100% - 2 * var(--gap)); --unit: 8px }
            body { margin: 0 }
            #a { width: var(--w); height: var(--missing, 20px); margin-left: var(--gap) }
            #b { width: min(50%, 150px); height: clamp(10px, 5vh, 40px) }
            #c { width: calc(var(--unit) * 3 + 1rem); height: round(up, 13px, 5px) }
            .scope { --unit: 2px }
            #d { width: calc(var(--unit) * 10); height: 4px }
            #e { width: max(10px, calc(25% - var(--gap))); height: abs(-6px) }
        ", || {
            body()
                .child(div().id("a"))
                .child(div().id("b"))
                .child(div().id("c"))
                .child(div().class("scope").child(div().id("d")))
                .child(div().id("e"))
        });
        assert_eq!(rect(cx, w, "a"), (10.0, 0.0, 380.0, 20.0));
        assert_eq!(rect(cx, w, "b").2, 150.0);
        assert_eq!(rect(cx, w, "b").3, 15.0); // 5vh of 300
        assert_eq!(rect(cx, w, "c").2, 40.0);
        assert_eq!(rect(cx, w, "c").3, 15.0);
        assert_eq!(rect(cx, w, "d").2, 20.0, "inherited override");
        assert_eq!(rect(cx, w, "e").2, 90.0);
        assert_eq!(rect(cx, w, "e").3, 6.0);
    });
}

#[test]
fn grid_template_areas_and_named_lines() {
    Application::headless().run(|cx| {
        let w = page(cx, 300.0, 200.0, "
            body { margin: 0 }
            .g { display: grid; width: 300px; height: 200px;
                 grid-template-columns: [side] 100px [main-start] 1fr [main-end];
                 grid-template-rows: 40px 1fr 30px;
                 grid-template-areas: 'head head' 'side main' 'foot foot' }
            #h { grid-area: head } #s { grid-area: side } #m { grid-area: main } #f { grid-area: foot }
            #x { grid-column: main-start / main-end; grid-row: 3 }
        ", || {
            body().child(div().class("g")
                .child(div().id("h")).child(div().id("s")).child(div().id("m")).child(div().id("f")))
        });
        assert_eq!(rect(cx, w, "h"), (0.0, 0.0, 300.0, 40.0));
        assert_eq!(rect(cx, w, "s"), (0.0, 40.0, 100.0, 130.0));
        assert_eq!(rect(cx, w, "m"), (100.0, 40.0, 200.0, 130.0));
        assert_eq!(rect(cx, w, "f"), (0.0, 170.0, 300.0, 30.0));
    });
}

#[test]
fn sizing_keywords_and_mixed_calc() {
    Application::headless().run(|cx| {
        let w = page(cx, 400.0, 200.0, "
            body { margin: 0 }
            .wrap { width: 300px }
            #fit { width: fit-content; height: 10px; background: red }
            #fit > div { width: 70px; height: 10px }
            #mix { width: calc(50% + 20px); height: 10px }
            #st { display: flex; width: 200px }
            #st > div { width: stretch; height: 10px }
        ", || {
            body().child(div().class("wrap")
                .child(div().id("fit").child(div()))
                .child(div().id("mix"))
                .child(div().id("st").child(div().id("st1"))))
        });
        assert_eq!(rect(cx, w, "fit").2, 70.0);
        assert_eq!(rect(cx, w, "mix").2, 170.0);
        assert_eq!(rect(cx, w, "st1").2, 200.0);
    });
}

#[test]
fn floats_and_clear() {
    Application::headless().run(|cx| {
        let w = page(cx, 300.0, 200.0, "
            body { margin: 0 }
            .c { width: 300px }
            #l { float: left; width: 50px; height: 40px }
            #r { float: right; width: 60px; height: 20px }
            #cl { clear: both; height: 10px }
        ", || {
            body().child(div().class("c")
                .child(div().id("l")).child(div().id("r")).child(div().id("cl")))
        });
        assert_eq!(rect(cx, w, "l"), (0.0, 0.0, 50.0, 40.0));
        assert_eq!(rect(cx, w, "r"), (240.0, 0.0, 60.0, 20.0));
        assert_eq!(rect(cx, w, "cl").1, 40.0, "cleared below both floats");
    });
}

#[test]
fn absolute_and_fixed_positioning() {
    Application::headless().run(|cx| {
        let w = page(cx, 300.0, 200.0, "
            body { margin: 0 }
            .rel { position: relative; margin: 20px; width: 100px; height: 100px }
            #abs { position: absolute; right: 10px; bottom: 5px; width: 20px; height: 20px }
            #fix { position: fixed; left: 0; top: 0; width: 15px; height: 15px }
            #st { position: static; left: 50px; width: 10px; height: 10px }
        ", || {
            body().child(div().class("rel")
                .child(div().child(div().id("abs")))
                .child(div().id("fix"))
                .child(div().id("st")))
        });
        assert_eq!(rect(cx, w, "abs"), (90.0, 95.0, 20.0, 20.0), "relative to the positioned ancestor");
        assert_eq!(rect(cx, w, "fix"), (0.0, 0.0, 15.0, 15.0), "relative to the viewport");
        assert_eq!(rect(cx, w, "st").0, 20.0, "static ignores insets");
    });
}

#[test]
fn selectors_level_4() {
    Application::headless().run(|cx| {
        let w = page(cx, 400.0, 400.0, "
            body { margin: 0 }
            div { height: 10px }
            .card:has(> img) { height: 31px }
            .card:has(.deep) { height: 32px }
            .list > :nth-child(2 of .on) { height: 33px }
            .list > p:last-of-type { height: 34px }
            .list > p:nth-of-type(1) { height: 35px }
            :is(.x, .y):not(.z) { height: 36px }
            :where(.w) { height: 37px }
            .w { height: 38px }
            [data-k=\"VaLuE\" i] { height: 39px }
            .nest { height: 1px; & > .kid { height: 40px } .outer & { height: 41px } }
        ", || {
            body()
                .child(div().id("c1").class("card").child(img()))
                .child(div().id("c2").class("card").child(div().child(span().class("deep"))))
                .child(div().id("c3").class("card").child(span()))
                .child(div().class("list")
                    .child(p().id("p1").class("on")).child(div().id("d1"))
                    .child(p().id("p2").class("on")).child(p().id("p3")))
                .child(div().id("x").class("x")).child(div().id("xz").class("x z"))
                .child(div().id("w").class("w"))
                .child(div().id("k").attr("data-k", "value"))
                .child(div().class("nest").child(div().id("kid").class("kid")))
                .child(div().class("outer").child(div().id("n2").class("nest")))
        });
        let h = |cx: &mut App, id: &str| rect(cx, w, id).3;
        assert_eq!(h(cx, "c1"), 31.0, ":has(> img)");
        assert_eq!(h(cx, "c2"), 32.0, ":has(.deep) through a descendant");
        assert_eq!(h(cx, "c3"), 10.0, "no :has match");
        assert_eq!(h(cx, "p2"), 33.0, ":nth-child(2 of .on)");
        assert_eq!(h(cx, "p3"), 34.0, ":last-of-type");
        assert_eq!(h(cx, "p1"), 35.0, ":nth-of-type(1)");
        assert_eq!(h(cx, "x"), 36.0);
        assert_eq!(h(cx, "xz"), 10.0, ":not()");
        assert_eq!(h(cx, "w"), 38.0, ":where() has no specificity");
        assert_eq!(h(cx, "k"), 39.0, "case-insensitive attribute");
        assert_eq!(h(cx, "kid"), 40.0, "nested `& > .kid`");
        assert_eq!(h(cx, "n2"), 41.0, "nested `.outer &`");
    });
}

#[test]
fn at_rules_layers_media_supports_scope() {
    Application::headless().run(|cx| {
        let w = page(cx, 500.0, 300.0, "
            body { margin: 0 }
            @layer reset, base, theme;
            @layer reset { div { height: 10px } }
            @layer theme { #l { height: 20px } }
            @layer base { #l { height: 99px } #l2 { height: 98px } }
            #l2 { height: 21px }
            @media (400px <= width <= 600px) { #m { height: 22px } }
            @media (width > 600px) { #m { height: 99px } }
            @media (prefers-color-scheme: dark) and (hover: hover) { #d { height: 23px } }
            @supports (display: grid) and (not (display: banana)) { #s { height: 24px } }
            @supports (display: banana) { #s { height: 99px } }
            @scope (.panel) to (.stop) { .t { height: 25px } }
        ", || {
            body()
                .child(div().id("l")).child(div().id("l2")).child(div().id("m")).child(div().id("d"))
                .child(div().id("s"))
                .child(div().class("panel").child(div().id("t1").class("t"))
                    .child(div().class("stop").child(div().id("t2").class("t"))))
                .child(div().id("t3").class("t"))
        });
        let h = |cx: &mut App, id: &str| rect(cx, w, id).3;
        assert_eq!(h(cx, "l"), 20.0, "later layer wins");
        assert_eq!(h(cx, "l2"), 21.0, "unlayered beats layered");
        assert_eq!(h(cx, "m"), 22.0, "media range");
        assert_eq!(h(cx, "d"), 23.0);
        assert_eq!(h(cx, "s"), 24.0, "@supports");
        assert_eq!(h(cx, "t1"), 25.0, "in scope");
        assert_eq!(h(cx, "t2"), 10.0, "below the scope limit");
        assert_eq!(h(cx, "t3"), 10.0, "outside the scope");
    });
}

#[test]
fn generated_content() {
    Application::headless().run(|cx| {
        let w = page(cx, 400.0, 200.0, "
            body { margin: 0 }
            .a::before { content: ''; display: block; height: 12px }
            .a::after { content: attr(data-x); display: block; height: 7px }
            .a > div { height: 5px }
            .b:empty::before { content: none; display: block; height: 50px }
        ", || {
            body()
                .child(div().id("a").class("a").attr("data-x", "hi").child(div().id("inner")))
                .child(div().id("b").class("b"))
        });
        assert_eq!(rect(cx, w, "inner").1, 12.0, "::before comes first");
        assert_eq!(rect(cx, w, "a").3, 24.0, "::before + child + ::after");
        assert_eq!(rect(cx, w, "b").3, 0.0, "content: none generates nothing");
    });
}

#[test]
fn tailwind_classes() {
    Application::headless().run(|cx| {
        let w = page(cx, 800.0, 400.0, "body { margin: 0 }", || {
            body()
                .child(div().id("a").class(blenderui::tw!("w-1/2 h-8 p-4 px-2 bg-sky-500/50 rounded-lg")))
                .child(div().id("b").class("flex gap-4 w-[calc(100%-2rem)] mx-auto")
                    .child(div().id("b1").class("size-10 shrink-0"))
                    .child(div().id("b2").class("grow h-6 md:h-12 max-sm:h-1")))
                .child(div().id("c").class("grid grid-cols-3 gap-x-2 w-60")
                    .child(div().id("c1").class("col-span-2 h-4")).child(div().id("c2").class("h-4")))
                .child(div().class("group").child(div().id("g").class("h-2 group-hover:h-9")))
                .child(div().id("d").class("h-5 data-[state=open]:h-7").attr("data-state", "open"))
                .child(div().id("e").class("hidden md:block h-3"))
                .child(div().id("f").class("h-[37px] [&>*]:h-1").child(div().id("f1")))
        });
        assert_eq!(rect(cx, w, "a").2, 400.0, "w-1/2");
        assert_eq!(rect(cx, w, "a").3, 32.0, "h-8 (border-box, as with Tailwind preflight)");
        let b = rect(cx, w, "b");
        assert_eq!((b.0, b.2), (16.0, 768.0), "arbitrary calc width + mx-auto");
        assert_eq!(rect(cx, w, "b1").2, 40.0);
        assert_eq!(rect(cx, w, "b2").0, 16.0 + 40.0 + 16.0, "gap-4");
        assert_eq!(rect(cx, w, "b2").3, 48.0, "md: applies at 800px");
        assert_eq!(rect(cx, w, "c1").2, 157.0, "col-span-2 of 3 (gap-x-2 = 8px), rounded");
        assert_eq!(rect(cx, w, "g").3, 8.0);
        assert_eq!(rect(cx, w, "d").3, 28.0, "data-[state=open]:");
        assert_eq!(rect(cx, w, "e").3, 12.0, "hidden md:block");
        assert_eq!(rect(cx, w, "f").3, 37.0);
        assert_eq!(rect(cx, w, "f1").3, 4.0, "[&>*]: arbitrary variant");
    });
}
