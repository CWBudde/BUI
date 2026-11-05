//! The shadcn-style virtual list: ten million rows (`VirtualList`) next to a
//! chat (`ChatList`), written in `rsx!` / `#[component]` syntax, with a
//! status line fed by `on_range_change`.
//!
//! ```text
//! cargo run --release --example site_shadcn_list
//! ```

#[path = "site/common.rs"]
mod common;

use std::cell::Cell as StdCell;
use std::fmt::Write;
use std::sync::Arc;

use blenderui::prelude::*;
use blenderui::shadcn::*;
use blenderui::vlist::synth;

const ROWS: usize = 10_000_000;

fn row_text(i: usize, out: &mut String) {
    let words = ["render", "mesh", "shader", "node", "bake", "light"];
    let _ = write!(out, "Row {i}: ");
    for k in 0..(i * 7 % 11) + 1 {
        let _ = write!(out, "{} ", words[(i + k) % words.len()]);
    }
}

struct Page {
    rows: StdCell<Option<ListHandle>>,
    chat: Arc<[Message]>,
}

impl Render for Page {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let rows = use_virtual_list();
        self.rows.set(Some(rows));
        let chat = self.chat.clone();
        let status = format!("{ROWS} rows, showing {}..{}", rows.range().start, rows.range().end);
        html().styled().child(body().m_0().p_4().child(rsx! {
            <div flex flex-col gap-2>
                <div flex gap-4>
                    <VirtualList list={rows} count={ROWS} item={row_text} estimate_size={36.0} overscan={12} w={px(380.0)} h={px(300.0)} />
                    <ChatList messages={chat} w={px(420.0)} h={px(300.0)} />
                </div>
                <div text-sm opacity-70>{status}</div>
            </div>
        }))
    }
}

fn main() {
    let shot = common::screenshot_arg();
    let app = if shot.is_some() { Application::headless() } else { Application::new() };
    app.run(move |cx| {
        let mut page = None;
        let win = cx
            .open_window(common::options(852.0, 372.0, "Virtual list"), |_, cx| {
                let p = cx.new(|_| Page { rows: StdCell::new(None), chat: synth::messages(0, 500).into() });
                page = Some(p);
                p
            })
            .unwrap();
        if let Some(path) = shot {
            for _ in 0..4 {
                cx.draw(win.id);
            }
            let rows = page.unwrap().read(cx).rows.get().expect("rendered");
            rows.scroll_to_index(6_283_185, Align::Start, cx);
            for _ in 0..6 {
                cx.draw(win.id);
            }
            let (w, h, px) = cx.draw(win.id);
            common::save(&path, w, h, &px);
        }
    });
}
