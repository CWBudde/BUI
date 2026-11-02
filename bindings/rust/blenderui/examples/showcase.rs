//! A tour of elements, CSS and Blender widgets.
//!
//! ```text
//! cargo run --example showcase                       # native window
//! cargo run --example showcase -- --screenshot a.png  # off-screen render
//! ```

use blenderui::prelude::*;

struct Showcase {
    clicks: u32,
    name: String,
    volume: f64,
    dark: bool,
}

const CSS: &str = r#"
body { margin: 0; }
.app { display: flex; height: 100vh; }
nav { width: 150px; background: #282828; padding: 10px 0; border-right: 1px solid #1d1d1d; }
nav a { display: block; padding: 4px 14px; color: #c8c8c8; text-decoration: none; }
nav a:hover { background: #4772b3; color: white; }
main { flex: 1; padding: 12px 20px; overflow-y: scroll; }
.cards { display: grid; grid-template-columns: repeat(3, 1fr); gap: 10px; margin: 10px 0; }
.card { padding: 10px; border-radius: 6px; background: #3a3a3a; box-shadow: 0 2px 6px rgba(0,0,0,0.4); }
.card.accent { background: linear-gradient(135deg, #4772b3, #7a4fb3); color: white; }
.card h4 { margin: 0 0 4px 0; }
table { width: 100%; border-collapse: collapse; }
td, th { border-bottom: 1px solid #484848; text-align: left; }
tbody tr:nth-child(odd) { background: #353535; }
"#;

impl Render for Showcase {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let card = |title: &'static str, body: &'static str| {
            div().class("card").child(h4().child(title)).child(small().child(body))
        };
        html()
            .child(head().child(title().child("BUI showcase")).child(style().child(CSS)))
            .child(body().child(
                div().class("app")
                    .child(nav().children(["Overview", "Forms", "Tables", "Lists", "About"]
                        .map(|s| a().href("#").child(s))))
                    .child(blenderui::html::main()
                        .child(h1().child("Blender UI, from Rust"))
                        .child(p()
                            .child("Every ").child(b().child("HTML element"))
                            .child(", ").child(i().child("CSS")).child(" and ")
                            .child(code().child(".gpui_style()"))
                            .child(" method, drawn by Blender's widget code. Text wraps in line boxes, \
                                    links are ")
                            .child(a().href("#").child("underlined"))
                            .child(" and ").child(mark().child("marked text")).child(" works."))
                        .child(div().class("cards")
                            .child(card("Layout", "block, inline, flex, grid, tables").class("accent"))
                            .child(card("Selectors", "classes, ids, :hover, :nth-child, @media"))
                            .child(card("Widgets", "native Blender buttons, fields, sliders")))
                        .child(fieldset()
                            .child(legend().child("Form controls"))
                            .child(div().flex().flex_wrap().gap_2().items_center()
                                .child(label().child("Name"))
                                .child(input().id("name").placeholder("Your name").value(self.name.clone())
                                    .on_change(cx.listener(|this, e: &ChangeEvent, _, cx| {
                                        this.name = e.value.clone();
                                        cx.notify();
                                    })))
                                .child(input().id("count").input_type("number").value("3").min(0.0).max(10.0))
                                .child(input().id("vol").input_type("range").min(0.0).max(100.0)
                                    .value(format!("{}", self.volume))
                                    .on_input(cx.listener(|this, e: &ChangeEvent, _, cx| {
                                        this.volume = e.number;
                                        cx.notify();
                                    })))
                                .child(select().id("mode")
                                    .child(option().child("Object Mode"))
                                    .child(option().child("Edit Mode"))
                                    .child(option().child("Sculpt Mode")))
                                .child(input().input_type("color").value("#e6a23c")))
                            .child(div().flex().gap_4().items_center().mt_2()
                                .child(input().id("dark").input_type("checkbox").checked(self.dark)
                                    .on_change(cx.listener(|this, e: &ChangeEvent, _, cx| {
                                        this.dark = e.checked;
                                        cx.notify();
                                    })))
                                .child(label().child("Dark"))
                                .child(input().input_type("radio").name("unit").checked(true))
                                .child(label().child("Metric"))
                                .child(input().input_type("radio").name("unit"))
                                .child(label().child("Imperial"))
                                .child(button().id("go").child(format!("Clicked {}", self.clicks))
                                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                        this.clicks += 1;
                                        cx.notify();
                                    }))))
                            .child(div().flex().gap_4().items_center().mt_2()
                                .child(span().child(format!("Volume {:.0}%", self.volume)))
                                .child(progress().value(format!("{}", self.volume / 100.0)).w(px(180.0)))))
                        .child(h3().child("Table"))
                        .child(table()
                            .child(thead().child(tr().child(th().child("Object")).child(th().child("Type"))
                                .child(th().child("Vertices"))))
                            .child(tbody().children([("Cube", "Mesh", "8"), ("Camera", "Camera", "-"),
                                                     ("Light", "Point", "-"), ("Suzanne", "Mesh", "507")]
                                .map(|(o, t, v)| tr().child(td().child(o)).child(td().child(t))
                                    .child(td().child(v))))))
                        .child(div().flex().gap_6()
                            .child(div().child(h3().child("Lists"))
                                .child(ul().child(li().child("Modeling"))
                                    .child(li().child("Sculpting").child(ul().child(li().child("Brushes"))))
                                    .child(li().child("Shading")))
                                .child(ol().child(li().child("First")).child(li().child("Second"))))
                            .child(div().flex_1().child(h3().child("Details"))
                                .child(details().open(true).child(summary().child("Render settings"))
                                    .child(p().child("Samples, denoising, light paths.")))
                                .child(details().child(summary().child("Output properties"))
                                    .child(p().child("Hidden until opened.")))
                                .child(blockquote().child("Everything is a Div; tags only change defaults."))
                                .child(pre().child("fn main() {\n    println!(\"hi\");\n}")))))))
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let shot = args.iter().position(|a| a == "--screenshot").and_then(|i| args.get(i + 1)).cloned();
    let app = if shot.is_some() { Application::headless() } else { Application::new() };
    app.run(move |cx| {
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: point(px(0.0), px(0.0)),
                size: size(px(900.0), px(760.0)),
            })),
            ..Default::default()
        };
        let win = cx
            .open_window(opts, |_, cx| {
                cx.new(|_| Showcase { clicks: 0, name: String::new(), volume: 40.0, dark: true })
            })
            .unwrap();
        if let Some(path) = shot {
            let (w, h, px) = cx.draw(win.id);
            let (w, h, px) = if w == 0 { cx.draw(win.id) } else { (w, h, px) };
            let rgba: Vec<u8> = px.iter().flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8, 255]).collect();
            let file = std::fs::File::create(&path).unwrap();
            let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w as u32, h as u32);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            enc.write_header().unwrap().write_image_data(&rgba).unwrap();
            println!("wrote {path} ({w}x{h})");
        }
    });
}
