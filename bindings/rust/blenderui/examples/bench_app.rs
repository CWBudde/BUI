//! Benchmark app (bench/run.py): the shared small UI. Prints "ready" once the
//! first frame is on screen, then idles.

use blenderui::prelude::*;

struct Bench {
    ready: bool,
}

impl Render for Bench {
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        if !self.ready {
            if window.presented_frames() > 0 {
                self.ready = true;
                println!("ready");
            } else {
                window.request_animation_frame(); // render once more after the first present
            }
        }
        div().styled().flex().flex_col().gap_2().p_3()
            .child(span().child("Settings"))
            .child(input().value("Cube"))
            .child(label().child(input().input_type("checkbox").checked(true)).child(" Shadows"))
            .child(input().input_type("range").min(0.0).max(10.0))
            .child(button().child("Apply"))
    }
}

fn main() {
    let opts = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(100.0), px(100.0)),
            size: size(px(360.0), px(260.0)),
        })),
        ..Default::default()
    };
    Application::new().run(move |cx: &mut App| {
        cx.open_window(opts, |_, cx| cx.new(|_| Bench { ready: false })).unwrap();
    });
}
