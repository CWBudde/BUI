//! Plain HTML: form controls and tables with a browser's default look
//! (the same page as `site_styled`, without `styled`).
//!
//! ```text
//! cargo run --release --example site_plain_html
//! ```

#[path = "site/common.rs"]
mod common;
#[path = "site/form_page.rs"]
mod form_page;

use blenderui::prelude::*;

struct Page;

impl Render for Page {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        form_page::page(false)
    }
}

fn main() {
    common::run(560.0, 520.0, "Export Animation", || Page);
}
