//! Headless frame-cost probe: `BUI_PROFILE=1 cargo run --release --example raster_probe -- [text|notext|square]`.
use blenderui::prelude::*;

struct Scene { t: u64, text: bool, round: bool }

impl Render for Scene {
    fn render(&mut self, w: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.t += 1;
        w.refresh();
        let t = self.t;
        let grid = div().size_full().flex().flex_wrap().gap(px(2.0)).p(px(2.0)).children((0..3000).map(|i| {
            let d = div().w(px(15.0)).h(px(12.0)).bg(rgb(((i as u32 * 7 + t as u32 * 3) % 360) * 0x010203))
                .flex().items_center().justify_center().text_size(px(8.0)).text_color(white());
            let d = if self.round { d.rounded(px(3.0)) } else { d };
            if self.text { d.child(format!("{}", (i as u64 * 13 + t) % 1000)) } else { d }
        }));
        if std::env::var_os("FLEXROOT").is_some() {
            html().flex().child(body().m_0().flex().flex_1().child(grid))
        } else {
            body().m_0().child(grid)
        }
    }
}

fn main() {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "text".into());
    Application::headless().run(|cx| {
        let opts = WindowOptions { window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.0), px(0.0)), size: size(px(1000.0), px(800.0)) })), ..Default::default() };
        let win = cx.open_window(opts, |_, cx| cx.new(|_| Scene { t: 0, text: mode != "notext", round: mode != "square" })).unwrap();
        let t0 = std::time::Instant::now();
        for _ in 0..120 { cx.draw(win.id); }
        eprintln!("{mode}: {:.2} ms/frame total", t0.elapsed().as_secs_f64() * 1000.0 / 120.0);
    });
}
