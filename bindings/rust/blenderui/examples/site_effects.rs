//! CSS effects on element pixels: `filter`, `clip-path`, transforms,
//! `mix-blend-mode` and `backdrop-filter`, on a small landscape made of
//! gradients (no images).
//!
//! ```text
//! cargo run --release --example site_effects
//! ```

#[path = "site/common.rs"]
mod common;

use blenderui::prelude::*;

const CSS: &str = r#"
body { margin: 0; padding: 14px; }
.grid { display: grid; grid-template-columns: repeat(5, 1fr); gap: 12px; }
figure { margin: 0; display: flex; flex-direction: column; align-items: center; gap: 5px; }
figcaption { font-size: 11px; font-family: monospace; opacity: 0.75; }
.scene { width: 120px; height: 84px; border-radius: 8px; overflow: hidden; position: relative;
         background: linear-gradient(180deg, #2d5fa8 0%, #e89a5a 70%, #f2c46d 100%); }
.sun { position: absolute; left: 70px; top: 22px; width: 30px; height: 30px; border-radius: 15px;
       background: #ffd25a; box-shadow: 0 0 14px 4px rgba(255, 214, 120, 0.8); }
.hill1 { position: absolute; left: -20px; top: 46px; width: 110px; height: 60px; border-radius: 55px 55px 0 0;
         background: #2f6b4a; }
.hill2 { position: absolute; left: 50px; top: 54px; width: 110px; height: 60px; border-radius: 55px 55px 0 0;
         background: #24543a; }
.f-gray { filter: grayscale(1); }
.f-sepia { filter: sepia(0.9); }
.f-hue { filter: hue-rotate(150deg) saturate(1.4); }
.f-blur { filter: blur(3px); }
.f-shadow { filter: drop-shadow(0 6px 6px rgba(0, 0, 0, 0.55)); border-radius: 0; overflow: visible;
            clip-path: polygon(50% 0%, 61% 35%, 98% 35%, 68% 57%, 79% 91%, 50% 70%, 21% 91%, 32% 57%, 2% 35%, 39% 35%); }
.c-circle { clip-path: circle(40% at 50% 50%); }
.c-poly { clip-path: polygon(0 0, 100% 15%, 85% 100%, 10% 85%); }
.t-rotate { transform: rotate(-12deg); }
.t-skew { transform: skewX(-14deg) scale(0.9); }
.blend { width: 120px; height: 84px; position: relative; border-radius: 8px; background: #e6e6e6; }
.blend div { position: absolute; width: 50px; height: 50px; border-radius: 25px; }
.blend .r { left: 18px; top: 8px; background: #e04a4a; }
.blend .g { left: 52px; top: 8px; background: #3fb36b; }
.blend .b { left: 35px; top: 32px; background: #4a7ae0; }
.m-multiply div { mix-blend-mode: multiply; }
.m-screen { background: #1d1d1d; }
.m-screen div { mix-blend-mode: screen; }
.m-diff div { mix-blend-mode: difference; }
.frost { position: relative; }
.glass { position: absolute; left: 14px; top: 20px; width: 92px; height: 44px; border-radius: 8px;
         backdrop-filter: blur(6px); background: rgba(255, 255, 255, 0.25);
         border: 1px solid rgba(255, 255, 255, 0.5); }
"#;

fn scene(class: &str) -> Div {
    div().class(format!("scene {class}"))
        .child(div().class("sun")).child(div().class("hill1")).child(div().class("hill2"))
}

fn fig(content: Div, caption: &'static str) -> Div {
    figure().child(content).child(figcaption().child(caption))
}

fn blend(class: &str) -> Div {
    div().class(format!("blend {class}"))
        .child(div().class("r")).child(div().class("g")).child(div().class("b"))
}

struct Effects;

impl Render for Effects {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        html().styled()
            .child(head().child(style().child(CSS)))
            .child(body().child(div().class("grid")
                .child(fig(scene(""), "original"))
                .child(fig(scene("f-gray"), "grayscale(1)"))
                .child(fig(scene("f-sepia"), "sepia(0.9)"))
                .child(fig(scene("f-hue"), "hue-rotate(150deg)"))
                .child(fig(scene("f-blur"), "blur(3px)"))
                .child(fig(scene("c-circle"), "clip-path: circle()"))
                .child(fig(scene("c-poly"), "clip-path: polygon()"))
                .child(fig(scene("f-shadow"), "drop-shadow + star"))
                .child(fig(scene("t-rotate"), "rotate(-12deg)"))
                .child(fig(scene("t-skew"), "skewX() scale()"))
                .child(fig(blend("m-multiply"), "blend: multiply"))
                .child(fig(blend("m-screen"), "blend: screen"))
                .child(fig(blend("m-diff"), "blend: difference"))
                .child(fig(scene("frost").child(div().class("glass")), "backdrop-filter"))
                .child(fig(scene("f-hue t-rotate c-circle"), "all of them"))))
    }
}

fn main() {
    common::run(720.0, 352.0, "CSS effects", || Effects);
}
