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
            <body style="margin: 0">
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
