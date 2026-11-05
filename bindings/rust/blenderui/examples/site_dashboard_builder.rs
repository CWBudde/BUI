//! The render-farm dashboard of `site_dashboard`, written in gpui's builder
//! syntax instead of `rsx!` (same elements, same stylesheet, same pixels).
//!
//! ```text
//! cargo run --release --example site_dashboard_builder
//! ```

#[path = "site/common.rs"]
mod common;
#[path = "site/dashboard_data.rs"]
mod data;

use blenderui::prelude::*;
use data::{CSS, JOBS, NODES, STATS};

struct Dashboard;

impl Render for Dashboard {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let stat = |(label, value, delta, fill): (&str, &str, &str, u32)| {
            div().class("card")
                .child(span().class("muted").child(label.to_string()))
                .child(strong().child(value.to_string()))
                .child(div().class("meter").child(div().class("fill").style(format!("width: {fill}%"))))
                .child(span().class("delta").child(delta.to_string()))
        };
        let job = |(name, engine, frames, pct): (&str, &str, &str, u32)| {
            div().class("job")
                .child(span().class("name").child(name.to_string()))
                .child(span().class(format!("badge {}", engine.to_lowercase())).child(engine.to_string()))
                .child(span().class("muted frames").child(frames.to_string()))
                .child(ProgressBar().class("bar").attr("max", "100").value(pct.to_string()))
                .child(span().class("pct").child(format!("{pct}%")))
        };
        let node = |(name, state, load): (&str, &str, u32)| {
            div().class("node")
                .child(span().class(format!("dot {state}")))
                .child(span().class("name").child(name.to_string()))
                .child(div().class("meter small").child(div().class("fill").style(format!("width: {load}%"))))
        };
        html().styled()
            .child(head().child(style().child(CSS)))
            .child(body()
                .child(header()
                    .child(div().class("title")
                        .child(h1().child("Render Farm"))
                        .child(span().class("muted").child("Studio cluster · 8 nodes")))
                    .child(Dropdown().child(option().child("Last 24 hours")).child(option().child("Last 7 days")))
                    .child(Button().child("New Job")))
                .child(section().class("stats").children(STATS.map(stat)))
                .child(section().class("columns")
                    .child(div().class("panel jobs").child(h2().child("Jobs")).children(JOBS.map(job)))
                    .child(div().class("panel nodes")
                        .child(h2().child("Nodes"))
                        .children(NODES.map(node))
                        .child(label().class("row").child(Checkbox().checked(true)).child("Wake idle nodes")))))
    }
}

fn main() {
    common::run(700.0, 370.0, "Render Farm", || Dashboard);
}
