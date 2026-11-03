// The benchmark scene on gpui, shared by gpui-bench and gpui-fast-bench
// (included by each; `APP_NAME` labels the results).

#[path = "../../common/measure.rs"]
mod measure;

use gpui::{
    App, AppContext, Bounds, Context, IntoElement, ParentElement, Render, Styled, Window,
    WindowBounds, WindowOptions, div, px, rgb, size,
};

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
        div()
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
                    .bg(rgb(measure::cell_color(i, cfg.t_for(i, t))))
                    .rounded(px(3.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(font))
                    .text_color(rgb(0xffffff))
                    .child(measure::cell_label(i, cfg.t_for(i, t)))
            }))
    }
}

fn main() {
    let cfg = measure::Config::from_args();
    measure::start(APP_NAME, &cfg);
    new_app().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(cfg.width), px(cfg.height)), cx);
        cx.open_window(
            WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), ..Default::default() },
            |_, cx| cx.new(|_| Scene { cells: cfg.cells, animate: cfg.animate, t: 0 }),
        )
        .unwrap();
        cx.activate(true);
    });
}
