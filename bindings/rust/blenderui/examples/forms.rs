//! Form controls three ways: plain HTML (browser look), inside `styled`
//! (Blender look), and the capitalized Blender components.
//! `cargo run --example forms` (`--screenshot out.png` renders it headless).

use blenderui::prelude::*;

struct Forms;

fn controls() -> Div {
    div().flex().flex_col().gap_2().p_3()
        .child(div().flex().gap_2().items_center()
            .child(button().child("Button"))
            .child(input().value("Text field"))
            .child(input().input_type("number").value("42")))
        .child(div().flex().gap_2().items_center()
            .child(label().child(input().input_type("checkbox").checked(true)).child(" Checkbox"))
            .child(label().child(input().input_type("radio").checked(true)).child(" Radio"))
            .child(input().input_type("range").min(0.0).max(10.0).value("6")))
        .child(div().flex().gap_2().items_center()
            .child(select().child(optgroup().attr("label", "Mesh").child(option().child("Cube")).child(option().child("Sphere"))))
            .child(progress().attr("value", "0.6"))
            .child(meter().attr("value", "0.3"))
            .child(input().input_type("color").value("#4772b3")))
        .child(textarea().child("A textarea"))
}

impl Render for Forms {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        body().m_0().flex().flex_col()
            .child(h3().child("Plain HTML (browser look)").p_3().m_0())
            .child(controls())
            .child(div().styled().child(h3().child("Inside styled (Blender look)").p_3().m_0()).child(controls()))
            .child(div().child(h3().child("Blender components").p_3().m_0())
                .child(div().flex().gap_2().p_3().items_center()
                    .child(Button().child("Button"))
                    .child(TextField().value("Text"))
                    .child(NumberSlider().min(0.0).max(10.0).value("6"))
                    .child(Checkbox().checked(true))))
    }
}

fn main() {
    let shot = std::env::args().skip_while(|a| a != "--screenshot").nth(1);
    let app = if shot.is_some() { Application::headless() } else { Application::new() };
    app.run(move |cx| {
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: point(px(0.0), px(0.0)),
                size: size(px(620.0), px(620.0)),
            })),
            ..Default::default()
        };
        let win = cx.open_window(opts, |_, cx| cx.new(|_| Forms)).unwrap();
        if let Some(path) = &shot {
            let (w, h, px) = cx.draw(win.id);
            let rgba: Vec<u8> = px.iter().flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8, 255]).collect();
            let file = std::fs::File::create(path).expect("create png");
            let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w as u32, h as u32);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            enc.write_header().expect("png").write_image_data(&rgba).expect("png");
        }
    });
}
