//! A chat with a million messages in Blender's theme: variable heights from
//! wrapped text and images, run headers, date dividers, opens at the newest
//! message, follows new ones while at the bottom, whole-message selection.
//!
//! ```text
//! cargo run --release --example site_chat
//! ```

#[path = "site/common.rs"]
mod common;

use blenderui::prelude::*;
use blenderui::vlist::{chat_list, synth, theme_palette, ChatList, ChatStyle, ItemSource, ListState, Options, VecSource};

const MESSAGES: u64 = 1_000_000;

struct Page {
    chat: Entity<ListState<VecSource, ChatStyle>>,
}

impl Render for Page {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (n, shown) = {
            let s = self.chat.read(cx);
            let p = s.view.placed();
            (s.source.len(), p.first().zip(p.last()).map(|(a, b)| (a.item, b.item)))
        };
        let shown = shown.map_or_else(String::new, |(a, b)| format!("showing {a}..{b}"));
        html().styled().size_full().child(
            body().m_0().size_full().child(
                // fills the window: the list takes what the status bar leaves
                div().flex().flex_col().size_full()
                    .child(div().flex_1().min_h(px(0.0)).child(chat_list(&self.chat)))
                    .child(
                        div().flex().items_center().h(px(24.0)).px_2().gap_4().text_size(px(12.0))
                            .child(format!("{n} messages"))
                            .child(shown),
                    ),
            ),
        )
    }
}

fn main() {
    let shot = common::screenshot_arg();
    let app = if shot.is_some() { Application::headless() } else { Application::new() };
    app.run(move |cx| {
        let win = cx
            .open_window(common::options(760.0, 460.0, "Chat"), |_, cx| {
                // a screenshot only needs the newest page; the live window holds the lot
                let n = if shot.is_some() { 20_000 } else { MESSAGES };
                let opts = Options { start_at_end: true, ..Options::default() };
                let chat = cx.new(|cx| ListState::new(ChatList::from_messages(synth::messages(0, n), theme_palette(), 760.0, 436.0, opts), cx));
                cx.new(|_| Page { chat })
            })
            .unwrap();
        if let Some(path) = shot {
            for _ in 0..3 {
                cx.draw(win.id);
            }
            // select a message the way a click does
            cx.simulate_click(win.id, 300.0, 380.0);
            cx.draw(win.id);
            let (w, h, px) = cx.draw(win.id);
            common::save(&path, w, h, &px);
        }
    });
}
