// The startup benchmark's settings window in gpui syntax, compiled unchanged
// against blenderui, gpui and gpui-fast (bench/run.py). gpui has no form
// widgets, so the controls are styled divs. The including file imports the
// framework (`div`, `px`, `rgb`, `Render`, `Window` ...), defines `run_app`
// and calls `settings_main()`.
//
// "ready" is printed on the second render: frameworks render again after the
// first frame was presented when asked with `request_animation_frame`.

struct Settings {
    renders: u32,
}

impl Render for Settings {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.renders += 1;
        if self.renders == 1 {
            window.request_animation_frame();
        } else if self.renders == 2 {
            println!("ready");
        }
        let field = |label: &'static str| {
            div().flex().items_center().gap(px(8.0))
                .child(div().w(px(70.0)).text_color(rgb(0xb8b8b8)).child(label))
                .child(div().flex_1().h(px(20.0)).px(px(6.0)).rounded(px(4.0)).bg(rgb(0x1d1d1d))
                    .border_1().border_color(rgb(0x3d3d3d)).text_color(rgb(0xe6e6e6)).child("Cube"))
        };
        div()
            .size_full().flex().flex_col().gap(px(8.0)).p(px(12.0))
            .bg(rgb(0x303030)).text_color(rgb(0xe6e6e6)).text_size(px(13.0))
            .child(div().text_size(px(16.0)).child("Settings"))
            .child(field("Name"))
            .child(div().flex().items_center().gap(px(8.0))
                .child(div().size(px(14.0)).rounded(px(3.0)).bg(rgb(0x4772b3)))
                .child("Shadows"))
            .child(div().h(px(20.0)).rounded(px(4.0)).bg(rgb(0x545454))
                .child(div().w(px(120.0)).h(px(20.0)).rounded(px(4.0)).bg(rgb(0x4772b3))))
            .child(div().w(px(80.0)).h(px(22.0)).rounded(px(4.0)).bg(rgb(0x545454))
                .flex().items_center().justify_center().child("Apply"))
    }
}
