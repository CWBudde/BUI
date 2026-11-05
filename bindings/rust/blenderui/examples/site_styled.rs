//! The `site_plain_html` page inside `styled`: the same markup, drawn in
//! Blender's theme with Blender's widgets (dark or light: BUI_THEME).
//!
//! ```text
//! cargo run --release --example site_styled
//! ```

#[path = "site/common.rs"]
mod common;
#[path = "site/form_page.rs"]
mod form_page;

use blenderui::prelude::*;

struct Page;

impl Render for Page {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        form_page::page(true)
    }
}

fn main() {
    common::run(560.0, 520.0, "Export Animation", || Page);
}
