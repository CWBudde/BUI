//! The variable list and the chat, end to end in off-screen windows.

use blenderui::prelude::*;
use blenderui::vlist::{chat_list, synth, theme_palette, ChatList, ChatStyle, ItemSource, ListState, Options, VecSource};

type Chat = Entity<ListState<VecSource, ChatStyle>>;

struct View {
    chat: Chat,
}

impl Render for View {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        body().m_0().child(div().w(px(700.0)).h(px(400.0)).child(chat_list(&self.chat)))
    }
}

fn open(cx: &mut App, n: u64) -> (WindowId, Chat) {
    let opts = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds { origin: point(px(0.0), px(0.0)), size: size(px(700.0), px(400.0)) })),
        ..Default::default()
    };
    let mut handle = None;
    let win = cx
        .open_window(opts, |_, cx| {
            let o = Options { start_at_end: true, ..Options::default() };
            let chat = cx.new(|cx| ListState::new(ChatList::from_messages(synth::messages(0, n), theme_palette(), 700.0, 400.0, o), cx));
            handle = Some(chat);
            cx.new(|_| View { chat })
        })
        .unwrap();
    (win.id, handle.unwrap())
}

fn frame(cx: &mut App, id: WindowId) -> (usize, usize, Vec<u32>) {
    cx.draw(id); // the first frame learns the size, the second lays out with it
    cx.draw(id);
    cx.draw(id)
}

fn run(theme: &str, f: impl FnOnce(&mut App)) {
    Application::headless().theme(theme).run(f);
}

fn distinct(img: &(usize, usize, Vec<u32>)) -> usize {
    let mut v = img.2.clone();
    v.sort_unstable();
    v.dedup();
    v.len()
}

#[test]
fn opens_at_the_newest_message_and_draws_text() {
    run("dark", |cx| {
        let (id, chat) = open(cx, 100_000);
        let img = frame(cx, id);
        let s = chat.read(cx);
        assert_eq!(s.view.placed().last().unwrap().item, 99_999);
        assert!(s.view.at_bottom());
        assert!(s.view.index().measured_items() < 3000, "only the tail was measured");
        // text, avatars and a scrollbar: far more than a flat fill
        assert!(distinct(&img) > 12, "{}", distinct(&img));
    });
}

#[test]
fn scrolling_up_moves_the_content_and_stops_following() {
    run("dark", |cx| {
        let (id, chat) = open(cx, 50_000);
        let a = frame(cx, id);
        cx.simulate_scroll(id, 300.0, 200.0, 10);
        let mut b = frame(cx, id);
        for _ in 0..60 {
            b = cx.draw(id);
        }
        assert_ne!(a.2, b.2, "the wheel scrolled");
        assert!(!chat.read(cx).view.at_bottom());
        // a new message does not drag the reader down
        let id_before = {
            let s = chat.read(cx);
            s.view.anchor_id(&s.source)
        };
        chat.update(cx, |s, _| s.push(synth::message(50_000)));
        cx.notify_all();
        frame(cx, id);
        let s = chat.read(cx);
        assert_eq!(s.view.anchor_id(&s.source), id_before);
        assert_eq!(s.source.len(), 50_001);
    });
}

#[test]
fn a_new_message_follows_at_the_bottom() {
    run("dark", |cx| {
        let (id, chat) = open(cx, 10_000);
        frame(cx, id);
        chat.update(cx, |s, _| s.push(synth::message(10_000)));
        cx.notify_all();
        frame(cx, id);
        assert_eq!(chat.read(cx).view.placed().last().unwrap().item, 10_000);
    });
}

#[test]
fn clicking_selects_a_whole_message() {
    run("dark", |cx| {
        let (id, chat) = open(cx, 10_000);
        let a = frame(cx, id);
        cx.simulate_click(id, 300.0, 380.0);
        let b = frame(cx, id);
        assert!(chat.read(cx).style.selected.is_some());
        assert_ne!(a.2, b.2, "the selection is painted");
        cx.simulate_click(id, 300.0, 380.0);
        frame(cx, id);
        assert!(chat.read(cx).style.selected.is_none());
    });
}

#[test]
fn the_background_follows_the_theme() {
    let shot = |theme: &str| {
        let mut px = 0;
        run(theme, |cx| {
            let (id, chat) = open(cx, 10_000);
            px = frame(cx, id).2[0];
            assert!(chat.read(cx).view.at_bottom());
        });
        px
    };
    assert_ne!(shot("light"), shot("dark"));
}
