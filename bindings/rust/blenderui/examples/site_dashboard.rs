//! A render-farm dashboard in `rsx!` (the same app as `site_dashboard_builder`
//! in gpui's builder syntax): HTML + CSS in Blender's theme (`styled`) with
//! Blender widgets mixed in.
//!
//! ```text
//! cargo run --release --example site_dashboard
//! BUI_THEME=light BUI_UI_SCALE=2 cargo run --release --example site_dashboard -- --screenshot out.png
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
        rsx! {
            <html styled>
                <head><style>{CSS}</style></head>
                <body>
                    <header>
                        <div class="title">
                            <h1>"Render Farm"</h1>
                            <span class="muted">"Studio cluster · 8 nodes"</span>
                        </div>
                        <Dropdown>
                            <option>"Last 24 hours"</option>
                            <option>"Last 7 days"</option>
                        </Dropdown>
                        <Button>"New Job"</Button>
                    </header>
                    <section class="stats">
                        {for (label, value, delta, fill) in STATS {
                            <div class="card">
                                <span class="muted">{label}</span>
                                <strong>{value}</strong>
                                <div class="meter"><div class="fill" style={format!("width: {fill}%")} /></div>
                                <span class="delta">{delta}</span>
                            </div>
                        }}
                    </section>
                    <section class="columns">
                        <div class="panel jobs">
                            <h2>"Jobs"</h2>
                            {for (name, engine, frames, pct) in JOBS {
                                <div class="job">
                                    <span class="name">{name}</span>
                                    <span class={format!("badge {}", engine.to_lowercase())}>{engine}</span>
                                    <span class="muted frames">{frames}</span>
                                    <ProgressBar class="bar" max="100" value={pct.to_string()} />
                                    <span class="pct">{format!("{pct}%")}</span>
                                </div>
                            }}
                        </div>
                        <div class="panel nodes">
                            <h2>"Nodes"</h2>
                            {for (name, state, load) in NODES {
                                <div class="node">
                                    <span class={format!("dot {state}")} />
                                    <span class="name">{name}</span>
                                    <div class="meter small"><div class="fill" style={format!("width: {load}%")} /></div>
                                </div>
                            }}
                            <label class="row"><Checkbox checked={true} />"Wake idle nodes"</label>
                        </div>
                    </section>
                </body>
            </html>
        }
    }
}

fn main() {
    common::run(700.0, 370.0, "Render Farm", || Dashboard);
}
