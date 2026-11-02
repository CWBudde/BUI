//! The benchmark scene on blenderui (same element tree as gpui-bench).

#[path = "../../common/measure.rs"]
mod measure;

use blenderui::prelude::*;

struct Scene {
    cells: usize,
    animate: bool,
    t: u64,
}

impl Render for Scene {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        measure::frame();
        let cfg = measure::Config::from_args();
        let (_, w, h, font) = cfg.grid();
        let t = self.t;
        if self.animate {
            self.t += 1;
            window.request_animation_frame();
        }
        // gpui's root fills the window: no <body> margin
        body().m_0().size_full().child(div()
            .size_full()
            .bg(rgb(0x303030))
            .flex()
            .flex_wrap()
            .content_start()
            .gap(px(2.0))
            .p(px(2.0))
            .children((0..self.cells).map(|i| {
                div()
                    .w(px(w))
                    .h(px(h))
                    .bg(rgb(measure::cell_color(i, t)))
                    .rounded(px(3.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(font))
                    .text_color(rgb(0xffffff))
                    .child(measure::cell_label(i, t))
            })))
    }
}

fn main() {
    let cfg = measure::Config::from_args();
    measure::start("blenderui", &cfg);
    let uncapped = std::env::args().any(|a| a == "--uncapped");
    Application::new().run(move |cx: &mut App| {
        let bounds = Bounds { origin: point(px(0.0), px(0.0)), size: size(px(cfg.width), px(cfg.height)) };
        cx.open_window(
            WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), ..Default::default() },
            |window, cx| {
                window.set_vsync(!uncapped);
                cx.new(|_| Scene { cells: cfg.cells, animate: cfg.animate, t: 0 })
            },
        )
        .unwrap();
    });
}
