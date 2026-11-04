//! CSS animations and transitions. Headless windows run on a virtual clock
//! that only `App::advance_clock` moves, so every sample is deterministic.

use std::time::Duration;

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
}

impl Render for Page {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        body()
            .child(div().id("a"))
            .child(div().id("b"))
            .child(div().id("c"))
            .child(style().child(self.css))
    }
}

fn page(cx: &mut App, css: &'static str) -> WindowId {
    let win = open(cx, 400.0, 300.0, Page { css });
    cx.draw(win.id);
    win.id
}

fn width(cx: &mut App, win: WindowId, id: &str) -> f32 {
    cx.element_bounds(win, id).unwrap_or_else(|| panic!("no element {id}")).size.width
}

fn ms(cx: &mut App, n: u64) {
    cx.advance_clock(Duration::from_millis(n));
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() <= 1.0
}

/// The window is idle: not animating and nothing scheduled.
fn idle(cx: &mut App, win: WindowId) -> bool {
    cx.draw(win);
    let w = cx.window(win).unwrap();
    !w.is_animating() && !w.needs_render()
}


#[test]
fn keyframes_width_from_50_to() {
    Application::headless().run(|cx| {
        let w = page(cx, "body { margin: 0 } div { height: 10px; width: 30px }
            @keyframes grow { 0% { width: 0 } 50% { width: 100px } 100% { width: 50px } }
            #a { animation: grow 2s linear }");
        assert_eq!(width(cx, w, "a"), 0.0, "0%");
        assert!(cx.window(w).unwrap().is_animating());
        ms(cx, 500);
        assert_eq!(width(cx, w, "a"), 50.0, "25%: halfway to the 50% keyframe");
        ms(cx, 500);
        assert_eq!(width(cx, w, "a"), 100.0, "50%");
        ms(cx, 500);
        assert_eq!(width(cx, w, "a"), 75.0, "75%");
        ms(cx, 600);
        assert_eq!(width(cx, w, "a"), 30.0, "finished, no fill: the cascaded value");
        assert!(idle(cx, w), "no frames once finished");
        assert_eq!(width(cx, w, "b"), 30.0, "other elements untouched");
    });
}

#[test]
fn from_to_selector_lists_and_later_blocks() {
    Application::headless().run(|cx| {
        let w = page(cx, "body { margin: 0 } div { height: 10px; width: 30px }
            @keyframes k { from, to { width: 100px } to { width: 200px } }
            @keyframes k2 { from { width: 1px } }
            @keyframes k2 { to { width: 300px } }
            @media (max-width: 10px) { @keyframes k2 { to { width: 5px } } }
            #a { animation: k 1s linear }
            #b { animation: k2 1s linear }");
        ms(cx, 500);
        assert_eq!(width(cx, w, "a"), 150.0, "`to` given twice: the later block wins");
        assert_eq!(width(cx, w, "b"), 165.0, "the last matching @keyframes wins, from the underlying 30px");
    });
}

#[test]
fn iterations_and_alternate() {
    Application::headless().run(|cx| {
        let w = page(cx, "body { margin: 0 } div { height: 10px; width: 30px }
            @keyframes k { from { width: 0 } to { width: 100px } }
            #a { animation: k 1s linear 2 alternate forwards }
            #b { animation: k 1s linear infinite reverse }");
        ms(cx, 250);
        assert_eq!(width(cx, w, "a"), 25.0);
        assert_eq!(width(cx, w, "b"), 75.0, "reverse");
        ms(cx, 1000);
        assert_eq!(width(cx, w, "a"), 75.0, "second iteration runs backwards");
        assert_eq!(width(cx, w, "b"), 75.0);
        ms(cx, 2000);
        assert_eq!(width(cx, w, "a"), 0.0, "forwards fill holds the end of the last (reversed) iteration");
        assert!(cx.window(w).unwrap().is_animating(), "infinite keeps going");
    });
}

#[test]
fn fill_modes_and_delays() {
    Application::headless().run(|cx| {
        let w = page(cx, "body { margin: 0 } div { height: 10px; width: 30px }
            @keyframes k { from { width: 10px } to { width: 110px } }
            #a { animation: k 1s linear 1s backwards }
            #b { animation: k 1s linear 1s }
            #c { animation: k 1s linear -500ms forwards }");
        assert_eq!(width(cx, w, "a"), 10.0, "backwards: the first keyframe during the delay");
        assert_eq!(width(cx, w, "b"), 30.0, "no fill: the cascaded value during the delay");
        assert_eq!(width(cx, w, "c"), 60.0, "negative delay starts midway");
        ms(cx, 1500);
        assert_eq!(width(cx, w, "a"), 60.0);
        assert_eq!(width(cx, w, "b"), 60.0);
        assert_eq!(width(cx, w, "c"), 110.0, "forwards: the last keyframe after the end");
        ms(cx, 1000);
        assert_eq!(width(cx, w, "a"), 30.0, "backwards only: no fill after");
        assert_eq!(width(cx, w, "c"), 110.0);
        assert!(idle(cx, w), "filling forwards doesn't need frames");
    });
}

#[test]
fn steps_and_cubic_bezier() {
    Application::headless().run(|cx| {
        let w = page(cx, "body { margin: 0 } div { height: 10px; width: 30px }
            @keyframes k { from { width: 0 } to { width: 100px } }
            #a { animation: k 1s steps(4) }
            #b { animation: k 1s steps(4, jump-start) }
            #c { animation: k 1s cubic-bezier(0.25, 0.1, 0.25, 1) }");
        ms(cx, 300);
        assert_eq!(width(cx, w, "a"), 25.0);
        assert_eq!(width(cx, w, "b"), 50.0);
        ms(cx, 200);
        assert!(near(width(cx, w, "c"), 80.24), "ease at 50%: {}", width(cx, w, "c"));
        ms(cx, 499);
        assert_eq!(width(cx, w, "a"), 75.0);
    });
}

#[test]
fn per_keyframe_easing_and_shorthands() {
    Application::headless().run(|cx| {
        let w = page(cx, "body { margin: 0 } div { height: 10px; width: 30px }
            @keyframes k { from { width: 0; animation-timing-function: step-end } to { width: 100px } }
            @keyframes m { to { margin-left: 100px; opacity: 0 } }
            #a { animation: k 1s linear }
            #b { animation-name: m; animation-duration: 1s; animation-timing-function: linear }");
        ms(cx, 900);
        assert_eq!(width(cx, w, "a"), 0.0, "the keyframe's own easing holds the value");
        let b = cx.element_bounds(w, "b").unwrap();
        assert_eq!(b.origin.x, 90.0, "margin-left animates from the underlying 0");
    });
}

#[test]
fn paused_and_important() {
    Application::headless().run(|cx| {
        let w = page(cx, "body { margin: 0 } div { height: 10px; width: 30px }
            @keyframes k { from { width: 0 } to { width: 100px } }
            #a { animation: k 1s linear paused }
            #b { animation: k 1s linear; width: 40px !important }");
        ms(cx, 500);
        assert_eq!(width(cx, w, "a"), 0.0, "paused at its start");
        assert_eq!(width(cx, w, "b"), 40.0, "!important beats animations");
        ms(cx, 600);
        assert!(idle(cx, w), "paused animations request no frames");
    });
}

#[test]
fn unknown_keyframes_and_none_cost_nothing() {
    Application::headless().run(|cx| {
        let w = page(cx, "body { margin: 0 } div { height: 10px; width: 30px }
            #a { animation: nope 1s } #b { transition: width 1s } #c { animation: none }");
        assert_eq!(width(cx, w, "a"), 30.0);
        assert!(idle(cx, w), "nothing to animate: idle");
    });
}

struct Hover;

impl Render for Hover {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        body()
            .child(div().id("t"))
            .child(style().child("body { margin: 0; background: black }
                #t { width: 100px; height: 50px; background: rgb(255, 0, 0);
                     transition: width 1s linear, background-color 1s linear }
                #t:hover { width: 200px; background: rgb(0, 0, 255) }"))
    }
}

#[test]
fn transition_on_hover_and_reverse() {
    Application::headless().run(|cx| {
        let win = open(cx, 400.0, 100.0, Hover).id;
        cx.draw(win);
        assert!(idle(cx, win), "transitions alone don't animate");
        cx.simulate_mouse_move(win, 10.0, 10.0);
        assert_eq!(width(cx, win, "t"), 100.0, "starts at the old value");
        assert!(cx.window(win).unwrap().is_animating());
        ms(cx, 500);
        assert_eq!(width(cx, win, "t"), 150.0);
        let img = cx.draw(win);
        let s = img.0 as f32 / 400.0;
        let p = img.2[(10.0 * s) as usize * img.0 + (10.0 * s) as usize];
        let (r, b) = ((p >> 16) & 0xff, p & 0xff);
        assert!((126..=129).contains(&r) && (126..=129).contains(&b), "halfway color {p:06x}");
        // leave midway: back to 100px, taking only as long as it ran
        cx.simulate_mouse_move(win, 300.0, 80.0);
        assert_eq!(width(cx, win, "t"), 150.0, "reverses from the current value");
        ms(cx, 250);
        assert_eq!(width(cx, win, "t"), 125.0);
        ms(cx, 250);
        assert_eq!(width(cx, win, "t"), 100.0);
        assert!(idle(cx, win), "no frames after the transition ends");
    });
}

struct Toggle {
    wide: bool,
}

impl Render for Toggle {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        body()
            .child(div().id("t").class(if self.wide { "wide" } else { "narrow" }))
            .child(style().child("body { margin: 0 }
                #t { height: 10px; transition: all 1s cubic-bezier(0, 0, 1, 1); opacity: 1 }
                .narrow { width: 100px } .wide { width: 300px; opacity: 0.5 }"))
    }
}

#[test]
fn transition_all_and_interrupt() {
    Application::headless().run(|cx| {
        let h = open(cx, 400.0, 100.0, Toggle { wide: false });
        let win = h.id;
        cx.draw(win);
        h.update(cx, |v, _, cx| {
            v.wide = true;
            cx.notify();
        });
        cx.draw(win);
        ms(cx, 500);
        assert_eq!(width(cx, win, "t"), 200.0);
        // back midway: from the current value, shortened to the time it ran
        h.update(cx, |v, _, cx| {
            v.wide = false;
            cx.notify();
        });
        assert_eq!(width(cx, win, "t"), 200.0);
        ms(cx, 250);
        assert_eq!(width(cx, win, "t"), 150.0, "reversed: half the duration (shortened)");
        ms(cx, 300);
        assert_eq!(width(cx, win, "t"), 100.0);
        assert!(idle(cx, win));
    });
}

struct Late {
    second: bool,
}

impl Render for Late {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut b = body().child(div().id("p").class("x").child(div().id("kid")));
        if self.second {
            b = b.child(div().id("q").class("x"));
        }
        b.child(style().child("body { margin: 0 } div { height: 10px }
            @keyframes k { from { width: 0; color: rgb(0, 0, 0) } to { width: 100px; color: rgb(200, 0, 0) } }
            .x { animation: k 1s linear } #kid { width: 5px; transition: color 1s }"))
    }
}

#[test]
fn each_element_has_its_own_clock_and_shared_styles_stay_clean() {
    Application::headless().run(|cx| {
        let h = open(cx, 400.0, 100.0, Late { second: false });
        let win = h.id;
        cx.draw(win);
        ms(cx, 500);
        assert_eq!(width(cx, win, "p"), 50.0);
        h.update(cx, |v, _, cx| {
            v.second = true;
            cx.notify();
        });
        assert_eq!(width(cx, win, "q"), 0.0, "same rules, started later: its own start time");
        assert_eq!(width(cx, win, "p"), 50.0, "the shared style isn't the animated one");
        ms(cx, 250);
        assert_eq!(width(cx, win, "p"), 75.0);
        assert_eq!(width(cx, win, "q"), 25.0);
        ms(cx, 1000);
        assert_eq!(width(cx, win, "p"), 400.0, "done: the cascaded `auto` width");
        assert_eq!(width(cx, win, "q"), 400.0);
        // #kid transitions the color it inherits from the animation's end
        assert!(cx.window(win).unwrap().is_animating());
        ms(cx, 1000);
        assert!(idle(cx, win));
    });
}
