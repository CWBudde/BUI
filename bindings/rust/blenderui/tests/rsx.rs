//! `rsx!` end to end: markup lowered to builder calls, rendered headless.

use blenderui::prelude::*;

/// A component: a struct with props and children.
#[derive(Props)]
struct Card {
    title: SharedString,
    children: Vec<AnyElement>,
}

impl IntoElement for Card {
    fn into_any_element(self) -> AnyElement {
        rsx! {
            <section class="p-2" id={self.title.to_string()}>
                <h2>{self.title.clone()}</h2>
                {...self.children}
            </section>
        }
        .into_any_element()
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    A,
    B,
}

struct App2 {
    count: usize,
    mode: Mode,
}

impl Render for App2 {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let items = ["x", "y", "z"];
        rsx! {
            <body style="margin: 0">
                <div id="btn" class="w-20 h-6" onClick={cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.count += 1;
                    cx.notify();
                })}></div>
                <ul id="list">{for it in items { <li class="h-3">{it}</li> }}</ul>
                {if self.count > 0 { <div id="clicked" class="h-4"></div> }}
                {match self.mode {
                    Mode::A => <div id="mode-a" class="h-5"></div>,
                    Mode::B => <div id="mode-b"></div>,
                }}
                <Card title="card">
                    <p id="inside" class="h-7">inside the card</p>
                </Card>
                <>
                    <div id="frag1" class="h-2"></div>
                    <div id="frag2" class="h-2"></div>
                </>
                <my-widget id="custom" data-x="1" class="block h-1"></my-widget>
                <input id="box" type="checkbox" checked={true} />
                <div id="noop" onClick={|| ()} w={px(33.0)} class="h-1"></div>
            </body>
        }
    }
}

#[test]
fn rsx_renders_and_reacts() {
    Application::headless().run(|cx| {
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: point(px(0.0), px(0.0)),
                size: size(px(300.0), px(400.0)),
            })),
            ..Default::default()
        };
        let win = cx.open_window(opts, |_, cx| cx.new(|_| App2 { count: 0, mode: Mode::A })).unwrap();
        let id = win.id;
        cx.draw(id);
        let r = |cx: &mut App, n: &str| cx.element_bounds(id, n).unwrap_or_else(|| panic!("{n}"));
        assert_eq!(r(cx, "btn").size.width, 80.0);
        assert_eq!(r(cx, "list").size.height, 36.0, "three li from the for loop");
        assert!(cx.element_bounds(id, "clicked").is_none(), "if without else: nothing");
        assert_eq!(r(cx, "mode-a").size.height, 20.0, "match arm");
        assert_eq!(r(cx, "inside").size.height, 28.0, "component children");
        assert!(r(cx, "card").size.height > 28.0);
        assert_eq!(r(cx, "frag2").origin.y - r(cx, "frag1").origin.y, 8.0, "fragment children");
        assert_eq!(r(cx, "custom").size.height, 4.0, "custom element");
        assert!(r(cx, "box").size.width > 0.0, "a real checkbox widget");
        assert_eq!(r(cx, "noop").size.width, 33.0, "{{expr}} attributes call builder methods");
        let b = r(cx, "btn");
        cx.simulate_click(id, b.origin.x + 5.0, b.origin.y + 3.0);
        assert_eq!(win.root().read(cx).count, 1);
        cx.draw(id);
        assert_eq!(r(cx, "clicked").size.height, 16.0, "re-rendered after the click");
    });
}

struct Variants;

impl Render for Variants {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let dynamic = "h-9";
        rsx! {
            <body style="margin: 0">
                <div id="a" class="h-4" hover:class="h-8 w-12" />
                <div id="b" class="h-4" hover:style="height: 20px; min-width: 10px" />
                <div id="c" class="h-4" hover:height="22px" />
                <div id="d" class="h-4" hover:h={px(24.0)} />
                <div id="e" class="h-4" md:class="h-6" max-sm:class="h-1" />
                <div id="f" class="h-4" md:hover:class="h-10" />
                <div id="g" class="group h-4"><div id="g1" class="h-1" group-hover:class="h-3" /></div>
                <div id="h" class="h-4" hover:class={dynamic} />
                <div id="i" class="h-2" data-on="1" data-[on]:class="h-5" 2xl:class="h-11" />
            </body>
        }
    }
}

#[test]
fn rsx_variant_attributes() {
    Application::headless().run(|cx| {
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: point(px(0.0), px(0.0)),
                size: size(px(800.0), px(600.0)),
            })),
            ..Default::default()
        };
        let win = cx.open_window(opts, |_, cx| cx.new(|_| Variants)).unwrap();
        let id = win.id;
        cx.draw(id);
        let h = |cx: &mut App, n: &str| cx.element_bounds(id, n).unwrap_or_else(|| panic!("{n}")).size.height;
        let hover = |cx: &mut App, n: &str| {
            let b = cx.element_bounds(id, n).unwrap();
            cx.simulate_mouse_move(id, b.origin.x + 1.0, b.origin.y + 1.0);
            cx.draw(id);
        };
        assert_eq!(h(cx, "e"), 24.0, "md: at 800px, max-sm: not");
        assert_eq!(h(cx, "i"), 20.0, "data-[on]: (2xl: not at 800px)");
        for (n, want) in [("a", 32.0), ("b", 20.0), ("c", 22.0), ("d", 24.0), ("f", 40.0), ("h", 36.0)] {
            assert_eq!(h(cx, n), 16.0, "{n} before hover");
            hover(cx, n);
            assert_eq!(h(cx, n), want, "{n} hovered");
        }
        hover(cx, "g1");
        assert_eq!(h(cx, "g1"), 12.0, "group-hover:");
    });
}

struct Attributify;

impl Render for Attributify {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        rsx! {
            <body style="margin: 0">
                <div id="a" w-1/2 h-[37px] bg-[#4772b3] p-2 />
                <div id="b" h-4 hover:h-[50px] hover:bg-sky-500/50 />
                <div id="c" h-4 md:h-6 max-sm:h-1 -mt-2 />
                <div id="d" class="h-2" w-[calc(100%-2rem)] />
                <div id="e" h-3 hidden md:block />
                <input id="f" type="checkbox" checked disabled />
            </body>
        }
    }
}

#[test]
fn rsx_attributify_utilities() {
    Application::headless().run(|cx| {
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: point(px(0.0), px(0.0)),
                size: size(px(800.0), px(600.0)),
            })),
            ..Default::default()
        };
        let win = cx.open_window(opts, |_, cx| cx.new(|_| Attributify)).unwrap();
        let id = win.id;
        let img = cx.draw(id);
        let r = |cx: &mut App, n: &str| cx.element_bounds(id, n).unwrap_or_else(|| panic!("{n}"));
        assert_eq!((r(cx, "a").size.width, r(cx, "a").size.height), (400.0, 37.0));
        let s = img.0 as f32 / 800.0;
        assert_eq!(img.2[(10.0 * s) as usize * img.0 + (10.0 * s) as usize], 0x4772b3, "bg-[#4772b3]");
        assert_eq!(r(cx, "c").size.height, 24.0);
        assert_eq!(r(cx, "d").size.width, 768.0);
        assert!(r(cx, "f").size.width > 0.0, "checked / disabled stay attributes");
        // `hidden` is an HTML boolean attribute: it stays an attribute (not the utility)
        assert_eq!(r(cx, "b").size.height, 16.0);
        let b = r(cx, "b");
        cx.simulate_mouse_move(id, b.origin.x + 1.0, b.origin.y + 1.0);
        cx.draw(id);
        assert_eq!(r(cx, "b").size.height, 50.0, "hover:h-[50px]");
    });
}
