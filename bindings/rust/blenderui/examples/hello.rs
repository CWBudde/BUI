//! The README's hello world: a component with state, Tailwind utilities as
//! attributes, and a hover style. `cargo run --release --example hello`
//! (`--screenshot out.png` renders it headless instead).

use blenderui::prelude::*;

#[component]
fn counter(label: &str) {
    let (count, set_count) = use_state(0);
    rsx! {
        <div flex items-center gap-3 p-4 rounded-xl bg-zinc-800>
            <span text-zinc-300>{label.to_string()} ": " {count().to_string()}</span>
            <div px-3 py-1 rounded-md bg-sky-700 hover:bg-sky-500 cursor-pointer
                 onClick={move || set_count(count() + 1)}>
                "Add one"
            </div>
        </div>
    }
}

struct Hello;

impl Render for Hello {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        rsx! {
            <body m-0 p-6 min-h-screen flex flex-col gap-4 bg-zinc-900 text-white>
                <h1 text-2xl font-bold>"Hello, BUI"</h1>
                <counter label="Clicks" />
                <input type="range" min={0.0} max={10.0} />
            </body>
        }
    }
}

fn main() {
    let shot = std::env::args().skip_while(|a| a != "--screenshot").nth(1);
    let app = if shot.is_some() { Application::headless() } else { Application::new() };
    app.run(move |cx| {
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: point(px(0.0), px(0.0)),
                size: size(px(420.0), px(260.0)),
            })),
            ..Default::default()
        };
        let win = cx.open_window(opts, |_, cx| cx.new(|_| Hello)).unwrap();
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
