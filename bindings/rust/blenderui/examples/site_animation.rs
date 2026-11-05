//! CSS animations and transitions: `@keyframes` with easing on a spinner,
//! a progress sweep, a bouncing ball and a fading toast. With
//! `--screenshot` it renders three moments (0.25 s, 0.6 s, 1.0 s) of the
//! deterministic headless clock side by side.
//!
//! ```text
//! cargo run --release --example site_animation
//! ```

#[path = "site/common.rs"]
mod common;

use std::time::Duration;

use blenderui::prelude::*;

const CSS: &str = r#"
body { margin: 0; padding: 14px; }
.card { border-radius: 10px; background: rgba(128, 128, 128, 0.16); padding: 12px; display: flex;
        flex-direction: column; gap: 12px; height: 222px; }
h3 { margin: 0; font-size: 14px; }
.row { display: flex; align-items: center; gap: 12px; }
.spinner { width: 28px; height: 28px; border-radius: 14px; border: 4px solid rgba(128, 128, 128, 0.35);
           position: relative; animation: spin 1.2s linear infinite; }
.spinner .arc { position: absolute; left: 9px; top: -4px; width: 10px; height: 10px; border-radius: 5px;
                background: #4772b3; }
@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
.track { flex: 1; height: 8px; border-radius: 4px; background: rgba(128, 128, 128, 0.3); }
.sweep { height: 8px; border-radius: 4px; background: linear-gradient(90deg, #4772b3, #9b6fe0);
         animation: grow 1.2s ease-in-out forwards; }
@keyframes grow { from { width: 5%; } to { width: 100%; } }
.lane { height: 54px; position: relative; border-bottom: 2px solid rgba(128, 128, 128, 0.4); }
.ball { position: absolute; width: 22px; height: 22px; border-radius: 11px; left: 0; top: 30px;
        background: #e8903c; border: 3px solid #ffd27a;
        animation: bounce 1.2s ease-out forwards; }
@keyframes bounce { 0% { left: 0px; top: 30px; } 25% { top: 0px; } 50% { top: 30px; }
                    75% { top: 12px; } 100% { left: 170px; top: 30px; } }
.toast { padding: 6px 10px; border-radius: 6px; background: #3fa66b; color: white; font-size: 12px;
         animation: fade 1.2s ease forwards; }
@keyframes fade { 0% { opacity: 0; transform: translateY(10px); } 60% { opacity: 1; transform: translateY(0px); }
                  100% { opacity: 1; transform: translateY(0px); } }
"#;

struct Anim;

impl Render for Anim {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        html().styled()
            .child(head().child(style().child(CSS)))
            .child(body().child(div().class("card")
                .child(h3().child("Uploading render…"))
                .child(div().class("row").child(div().class("spinner").child(div().class("arc")))
                    .child(div().class("track").child(div().class("sweep"))))
                .child(div().class("lane").child(div().class("ball")))
                .child(div().class("row").child(div().class("toast").child("✓ robot_arm.mp4 uploaded")))))
    }
}

fn main() {
    let Some(path) = common::screenshot_arg() else {
        common::run(240.0, 250.0, "CSS animation", || Anim);
        return;
    };
    Application::headless().run(move |cx| {
        let win = cx.open_window(common::options(240.0, 250.0, "CSS animation"), |_, cx| cx.new(|_| Anim))
            .unwrap();
        cx.draw(win.id);
        let mut frames = Vec::new();
        let mut t = 0u64;
        for ms in [250u64, 600, 1000] {
            cx.advance_clock(Duration::from_millis(ms - t));
            t = ms;
            frames.push(cx.draw(win.id));
        }
        // a filmstrip: the frames side by side, a gap of the page background
        let (fw, fh, ref first) = frames[0];
        let gap = fw / 40;
        let bg = first[fh / 2 * fw + 1];
        let w = fw * 3 + gap * 2;
        let mut out = vec![bg; w * fh];
        for (i, (_, _, px)) in frames.iter().enumerate() {
            for y in 0..fh {
                let x0 = i * (fw + gap);
                out[y * w + x0..y * w + x0 + fw].copy_from_slice(&px[y * fw..(y + 1) * fw]);
            }
        }
        common::save(&path, w, fh, &out);
    });
}
