//! Renders a fixed scene off-screen to a PNG, for pixel comparisons between
//! builds: `cargo run --release --example golden -- out.png`.

use blenderui::prelude::*;

struct Scene;

impl Render for Scene {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        body()
            .m_0()
            .child(style().child(".hdr { color: #e6a23c; } li:nth-child(2) { color: #78aef0 } .box:hover { opacity: 0.5 }"))
            .child(
                div().flex().flex_wrap().gap(px(2.0)).p(px(2.0)).children((0..120).map(|i| {
                    let d = div().w(px(70.0)).h(px(28.0)).bg(rgb(0x4772b3 + (i as u32 * 0x010203) % 0x303030))
                        .rounded(px(3.0)).text_size(px(11.0)).text_color(white());
                    let d = match i % 5 {
                        0 => d.flex().items_center().justify_center(),
                        1 => d.flex().flex_col().justify_end(),
                        2 => d.text_center(),
                        3 => d.p(px(3.0)),
                        _ => d.flex().justify_end().items_end().px_1(),
                    };
                    d.child(format!("{i} cell"))
                })),
            )
            .child(div().class("hdr").p_2().child(h2().child("Heading")).child(
                p().child("Plain text with ").child(b().child("bold")).child(", ").child(i().child("italic"))
                    .child(" and a ").child(a().href("#").child("link")).child(" that wraps across lines in a narrow box.")
                    .w(px(260.0)),
            ))
            .child(ul().child(li().child("one")).child(li().child("two")).child(li().child("three")))
            .child(table().child(tbody().children((0..3).map(|r| {
                tr().child(td().child(format!("r{r}"))).child(td().child("value")).child(td().child(format!("{}", r * 7)))
            }))))
            .child(div().class("box").size(px(40.0)).bg(rgb(0xe6a23c)).shadow_md().rounded_md())
    }
}

fn main() {
    let path = std::env::args().nth(1).expect("output path");
    Application::headless().run(move |cx| {
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: point(px(0.0), px(0.0)),
                size: size(px(1000.0), px(800.0)),
            })),
            ..Default::default()
        };
        let w = cx.open_window(opts, |_, cx| cx.new(|_| Scene)).unwrap();
        let (width, height, pixels) = cx.draw(w.id);
        let rgba: Vec<u8> = pixels.iter().flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8, 255]).collect();
        let file = std::io::BufWriter::new(std::fs::File::create(&path).unwrap());
        let mut enc = png::Encoder::new(file, width as u32, height as u32);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header().unwrap().write_image_data(&rgba).unwrap();
    });
}
