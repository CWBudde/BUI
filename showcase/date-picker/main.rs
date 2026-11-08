//! A date picker you can type into: "last 3 fridays", "weekends since october", "between sep 1 and
//! sep 5", "next weekend", "q1 2026" and the calendar lights up.
//!
//!   cargo run --release --bin date-picker
//!   cargo run --release --bin date-picker -- --query "weekends this month" --screenshot out.png
//!
//! The window is blenderui (Blender's UI as a library); the dates are `chrono` and `chrono-english`.

mod icons;
mod model;
mod parse;
mod selection;
mod ui;

use blenderui::prelude::*;
use chrono::prelude::*;
use model::Picker;

fn main() {
    let mut args = std::env::args().skip(1);
    let (mut shot, mut query) = (None, None);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--screenshot" => shot = args.next(),
            "--query" => query = args.next(),
            _ => {}
        }
    }

    let app = if shot.is_some() { Application::headless() } else { Application::new() };
    app.run(move |cx: &mut App| {
        let win = open(cx);
        if let Some(query) = &query {
            win.root().update(cx, |picker, cx| picker.set_query(query, cx));
        }
        if let Some(path) = &shot {
            let _ = cx.draw(win.id); // the first frame settles the fonts
            write_png(path, cx.draw(win.id));
        }
    });
}

fn open(cx: &mut App) -> WindowHandle<Picker> {
    let bounds = Bounds { origin: point(px(0.0), px(0.0)), size: size(px(920.0), px(660.0)) };
    let options = WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), ..Default::default() };
    let today = Local::now().date_naive();
    cx.open_window(options, |_, cx| cx.new(|_| Picker::new(today))).expect("a window")
}

fn write_png(path: &str, (w, h, pixels): (usize, usize, Vec<u32>)) {
    let rgba: Vec<u8> = pixels.iter().flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8, 255]).collect();
    let file = std::fs::File::create(path).expect("create the png");
    let mut png = png::Encoder::new(std::io::BufWriter::new(file), w as u32, h as u32);
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header().and_then(|mut w| w.write_image_data(&rgba)).expect("write the png");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::selection::Selection;

    fn click(cx: &mut App, win: WindowId, id: &str) {
        let b = cx.element_bounds(win, id).unwrap_or_else(|| panic!("no element #{id}"));
        cx.simulate_click(win, b.origin.x + b.size.width / 2.0, b.origin.y + b.size.height / 2.0);
    }

    fn type_text(cx: &mut App, win: WindowId, text: &str) {
        for ch in text.chars() {
            let s = ch.to_string();
            cx.simulate_key(win, &s, if ch == ' ' { "space" } else { &s });
        }
    }

    #[test]
    fn picker_end_to_end() {
        Application::headless().run(|cx: &mut App| {
            let win = open(cx);
            let id = win.id;
            cx.draw(id); // the first frame sets up the kit's event bindings
            let today = Local::now().date_naive();

            // a preset
            click(cx, id, "preset-Last week");
            assert_eq!(win.root().read(cx).query, "Last week");
            assert!(matches!(win.root().read(cx).selection, Selection::Range { .. }));
            cx.draw(id);

            // typing in the field re-reads the query at every key
            click(cx, id, "search");
            click(cx, id, "clear");
            assert_eq!(win.root().read(cx).query, "");
            cx.draw(id);
            click(cx, id, "search");
            type_text(cx, id, "last 3 frid");
            assert!(win.root().read(cx).selection.is_none());
            type_text(cx, id, "ay");
            let picker = win.root().read(cx);
            assert_eq!(picker.query, "last 3 friday");
            assert_eq!(picker.selection.len(), 3);

            // two clicks on days make a range
            let (d1, d2) = (today - chrono::Duration::days(3), today - chrono::Duration::days(1));
            cx.draw(id);
            click(cx, id, &format!("day-{d1}"));
            click(cx, id, &format!("day-{d2}"));
            let picker = win.root().read(cx);
            assert_eq!((picker.query.as_str(), &picker.selection), ("", &Selection::range(d1, d2)));

            // apply records the summary; the arrows move the months and keep the selection
            click(cx, id, "apply");
            assert_eq!(win.root().read(cx).applied, win.root().read(cx).selection.summary());
            let before = win.root().read(cx).right;
            click(cx, id, "nav-1");
            assert_ne!(win.root().read(cx).right, before);
            assert_eq!(win.root().read(cx).selection, Selection::range(d1, d2));
            let (_, _, pixels) = cx.draw(id);
            assert!(!pixels.is_empty());
        });
    }
}
