//! zopra / React / Leptos-style components and hooks, rendered headless.

use blenderui::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};

static EFFECTS: AtomicUsize = AtomicUsize::new(0);
static LEPTOS_EFFECTS: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone)]
struct Theme(&'static str);

// zopra style: lowercase component, `&str` prop, bare state closures
#[component]
fn profile(username: &str, id: String) {
    let (likes, set_likes) = use_state(0usize);
    use_effect!(move || { EFFECTS.fetch_add(1, Ordering::SeqCst); }, [likes()]);
    let theme: Theme = use_context();
    rsx! {
        <div id={id.clone()} class="h-4" onClick={move || set_likes(likes() + 1)}
             data-theme={theme.0}>
            {username.to_string()} " has " {likes().to_string()}
            {if likes() > 0 { <div id={format!("{id}-liked")} class="h-2" /> }}
        </div>
    }
}

// Leptos style: signal halves, auto-tracked memo and effect
#[component]
fn Counter(start: i32) {
    let (count, set_count) = signal(start);
    let doubled = Memo::new(move || count.get() * 2);
    Effect::new(move || {
        let _ = count.get();
        LEPTOS_EFFECTS.fetch_add(1, Ordering::SeqCst);
    });
    rsx! {
        <div id="counter" class="h-1" w={px(doubled.get() as f32)}
             onClick={move || set_count.update(|c| *c += 1)} />
    }
}

struct Page {
    show_bob: bool,
}

impl Render for Page {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        provide_context(Theme("dark"));
        let show_bob = self.show_bob;
        rsx! {
            <body styled style="margin: 0">
                <profile username="Alice" id={"alice".to_string()} />
                {if show_bob { <profile username="Bob" id={"bob".to_string()} /> }}
                <Counter start={5} />
            </body>
        }
    }
}

#[test]
fn hooks_state_effects_context() {
    Application::headless().run(|cx| {
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: point(px(0.0), px(0.0)),
                size: size(px(400.0), px(300.0)),
            })),
            ..Default::default()
        };
        let win = cx.open_window(opts, |_, cx| cx.new(|_| Page { show_bob: true })).unwrap();
        let id = win.id;
        cx.draw(id);
        let has = |cx: &mut App, n: &str| cx.element_bounds(id, n).is_some();
        let click = |cx: &mut App, n: &str| {
            let b = cx.element_bounds(id, n).unwrap();
            cx.simulate_click(id, b.origin.x + 2.0, b.origin.y + 2.0);
            cx.draw(id);
        };
        // effects ran once per profile after the first render
        assert_eq!(EFFECTS.load(Ordering::SeqCst), 2);
        assert_eq!(LEPTOS_EFFECTS.load(Ordering::SeqCst), 1);
        assert_eq!(cx.element_bounds(id, "counter").unwrap().size.width, 10.0, "memo: 5 * 2");
        // per-instance state: clicking Alice doesn't touch Bob
        click(cx, "alice");
        assert!(has(cx, "alice-liked") && !has(cx, "bob-liked"));
        assert_eq!(EFFECTS.load(Ordering::SeqCst), 3, "Alice's deps changed, Bob's didn't");
        // re-render without changes: no effect re-runs
        cx.notify_all();
        cx.draw(id);
        assert_eq!(EFFECTS.load(Ordering::SeqCst), 3);
        assert_eq!(LEPTOS_EFFECTS.load(Ordering::SeqCst), 1, "untracked render: effect stays");
        // Leptos: the memo and the effect follow the signal
        click(cx, "counter");
        assert_eq!(cx.element_bounds(id, "counter").unwrap().size.width, 12.0, "memo: 6 * 2");
        assert_eq!(LEPTOS_EFFECTS.load(Ordering::SeqCst), 2);
        // unmount Bob, mount him again: fresh state
        click(cx, "bob");
        assert!(has(cx, "bob-liked"));
        win.root().update(cx, |p, cx| {
            p.show_bob = false;
            cx.notify();
        });
        cx.draw(id);
        assert!(!has(cx, "bob"));
        win.root().update(cx, |p, cx| {
            p.show_bob = true;
            cx.notify();
        });
        cx.draw(id);
        assert!(has(cx, "bob") && !has(cx, "bob-liked"), "state was freed with the component");
        assert!(has(cx, "alice-liked"), "Alice kept hers");
    });
}

// optional props: `note` may be left out in markup
#[component]
fn Row(name: &str, note: Option<String>) {
    let (n, set_n) = use_state(0usize);
    let note = note.unwrap_or_default();
    rsx! {
        <div id={name.to_string()} class="h-2" onClick={move || set_n(n() + 1)} data-note={note}>
            {if n() > 0 { <div id={format!("{name}-clicked-{}", n())} class="h-1" /> }}
        </div>
    }
}

struct List {
    items: Vec<&'static str>,
    keyed: bool,
}

impl Render for List {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let keyed = self.keyed;
        rsx! {
            <body styled style="margin: 0">
                {for name in self.items.clone() {
                    if keyed {
                        rsx! { <Row key={name} name={name} /> }.into_any_element()
                    } else {
                        rsx! { <Row name={name} note={"plain".to_string()} /> }.into_any_element()
                    }
                }}
            </body>
        }
    }
}

#[test]
fn keys_and_optional_props() {
    for keyed in [true, false] {
        Application::headless().run(|cx| {
            let win = cx.open_window(WindowOptions::default(), |_, cx| {
                cx.new(|_| List { items: vec!["a", "b", "c"], keyed })
            }).unwrap();
            let id = win.id;
            cx.draw(id);
            let b = cx.element_bounds(id, "b").unwrap();
            cx.simulate_click(id, b.origin.x + 2.0, b.origin.y + 1.0);
            cx.draw(id);
            assert!(cx.element_bounds(id, "b-clicked-1").is_some());
            // remove "a": with keys, b keeps its state; without, state follows position
            win.root().update(cx, |l, cx| {
                l.items.remove(0);
                cx.notify();
            });
            cx.draw(id);
            let b_kept = cx.element_bounds(id, "b-clicked-1").is_some();
            let c_got = cx.element_bounds(id, "c-clicked-1").is_some();
            if keyed {
                assert!(b_kept && !c_got, "keyed: state follows the key");
            } else {
                assert!(!b_kept && c_got, "unkeyed: state follows the position (React semantics)");
            }
        });
    }
}
