//! CSS grid: an asset browser laid out with `grid-template-columns`, spans
//! and `repeat(auto-fill, minmax())`, in Blender's theme.
//!
//! ```text
//! cargo run --release --example site_grid
//! ```

#[path = "site/common.rs"]
mod common;

use blenderui::prelude::*;

const CSS: &str = r#"
body { margin: 0; padding: 12px; }
.layout { display: grid; grid-template-columns: 150px 1fr; grid-template-rows: auto 1fr; gap: 10px;
          height: 446px; }
.top { grid-column: 1 / span 2; display: flex; align-items: center; gap: 10px; }
.top h1 { font-size: 17px; margin: 0; flex: 1; }
aside { border-radius: 8px; background: rgba(128, 128, 128, 0.14); padding: 8px; display: flex;
        flex-direction: column; gap: 2px; }
aside .cat { padding: 4px 8px; border-radius: 4px; font-size: 12px; display: flex; }
aside .cat span { flex: 1; }
aside .cat.on { background: #4772b3; color: white; }
main { display: grid; grid-template-columns: repeat(auto-fill, minmax(96px, 1fr)); grid-auto-rows: 92px; grid-auto-flow: dense;
       gap: 8px; }
.tile { border-radius: 8px; display: flex; flex-direction: column; justify-content: flex-end;
        padding: 6px 8px; color: white; font-size: 11px; box-shadow: 0 2px 6px rgba(0, 0, 0, 0.25); }
.tile b { font-size: 12px; }
.wide { grid-column: span 2; }
.tall { grid-row: span 2; }
"#;

const TILES: [(&str, &str, &str, &str); 13] = [
    ("Brushed Steel", "PBR · 4K", "linear-gradient(135deg, #8a9099, #4b525c)", "wide"),
    ("Car Paint", "Clear coat", "radial-gradient(circle at 30% 30%, #ff7a6b, #a3122a)", "tall"),
    ("Oak", "Wood · 2K", "linear-gradient(160deg, #c08a52, #7a4b25)", ""),
    ("Marble", "Stone", "linear-gradient(120deg, #f2f0ea, #b9b4a8)", ""),
    ("Neon", "Emission", "radial-gradient(circle, #ff4fd8, #3a0b6e)", ""),
    ("Ocean", "Volume", "linear-gradient(180deg, #2bc0d6, #0b3d73)", "wide"),
    ("Moss", "Organic", "linear-gradient(140deg, #8fbf4a, #2f5b22)", ""),
    ("Rubber", "Matte", "linear-gradient(160deg, #3a3a3a, #121212)", ""),
    ("Gold", "Metal", "linear-gradient(135deg, #ffe08a, #b07a1c)", ""),
    ("Lava", "Emission", "radial-gradient(circle at 70% 70%, #ffb347, #b3240b)", ""),
    ("Glass", "Transmission", "linear-gradient(135deg, #cfeffd, #6fb3d9)", ""),
    ("Concrete", "Stone", "linear-gradient(150deg, #a3a3a0, #5f5f5c)", ""),
    ("Velvet", "Fabric", "radial-gradient(circle at 40% 30%, #8a3cc2, #2e0f4a)", ""),
];

const CATS: [(&str, u32, bool); 6] = [("All", 128, false), ("Metals", 18, false), ("Paints", 24, true),
                                     ("Woods", 15, false), ("Stone", 21, false), ("Emissive", 9, false)];

struct Grid;

impl Render for Grid {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        html().styled()
            .child(head().child(style().child(CSS)))
            .child(body().child(div().class("layout")
                .child(div().class("top")
                    .child(h1().child("Material Library"))
                    .child(TextField().placeholder("Search materials").w(px(180.0)))
                    .child(Dropdown().child(option().child("Sort by name")).child(option().child("Sort by date"))))
                .child(aside().children(CATS.map(|(name, n, on)| {
                    div().class(if on { "cat on" } else { "cat" })
                        .child(span().child(name)).child(small().child(n.to_string()))
                })))
                .child(blenderui::html::main().children(TILES.map(|(name, kind, bg, span)| {
                    div().class(format!("tile {span}")).style(format!("background: {bg}"))
                        .child(b().child(name)).child(span_el(kind))
                })))))
    }
}

fn span_el(text: &'static str) -> Div {
    span().child(text)
}

fn main() {
    common::run(640.0, 470.0, "CSS grid", || Grid);
}
