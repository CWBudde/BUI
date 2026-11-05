//! Dear ImGui on BUI from Rust, headless: the generated FFI's struct layouts match C++,
//! the safe layer drives widgets, simulated input changes values, owned objects work,
//! a panic in the frame comes back to the caller.
//!
//! SPDX-License-Identifier: GPL-2.0-or-later

use std::cell::RefCell;
use std::rc::Rc;

use blenderui_imgui as imgui;
use imgui::ImVec2;

#[test]
fn layouts_match_cpp() {
    let bad = imgui::sys::layout_mismatches();
    assert!(bad.is_empty(), "Rust/C++ size mismatches: {bad:?}");
}

#[derive(Default)]
struct State {
    checked: bool,
    slider: f32,
    slider_w: f32,
    text: String,
    clicks: i32,
    combo: i32,
    color: [f32; 4],
    frames: i32,
    table_cells: i32,
    clipped: i32,
    rects: std::collections::HashMap<&'static str, (ImVec2, ImVec2)>,
    panic_now: bool,
}

fn rect() -> (ImVec2, ImVec2) {
    (imgui::get_item_rect_min(), imgui::get_item_rect_max())
}

// one test: Dear ImGui's context is global (tests run on several threads)
#[test]
fn headless_widgets_and_input() {
    imgui::set_ui_scale(1.0);
    let st = Rc::new(RefCell::new(State { slider: 0.25, color: [0.2, 0.4, 0.6, 1.0], ..Default::default() }));
    let s2 = st.clone();
    let mut clipper = imgui::ImGuiListClipper::new();
    let frame = move || {
        let mut s = s2.borrow_mut();
        s.frames += 1;
        if s.panic_now {
            s.panic_now = false;
            drop(s);
            panic!("boom");
        }
        imgui::set_next_window_pos_ex([10.0, 10.0], imgui::ImGuiCond_Always, [0.0, 0.0]);
        imgui::set_next_window_size_ex([560.0, 760.0], imgui::ImGuiCond_Always);
        imgui::begin_ex("Rust", None, imgui::ImGuiWindowFlags_NoMove | imgui::ImGuiWindowFlags_NoSavedSettings);
        imgui::text(&format!("Hello from Rust {}", 42));
        imgui::text_colored([1.0, 0.5, 0.0, 1.0], "colored");
        imgui::bullet_text("bullet");
        imgui::checkbox("Check", &mut s.checked);
        s.rects.insert("check", rect());
        s.slider_w = imgui::calc_item_width();
        let mut v = s.slider;
        imgui::slider_float("Slider", &mut v, 0.0, 1.0);
        s.slider = v;
        s.rects.insert("slider", rect());
        let mut t = std::mem::take(&mut s.text);
        imgui::input_text("Text", &mut t);
        s.text = t;
        s.rects.insert("input", rect());
        if imgui::button("Button") {
            s.clicks += 1;
        }
        s.rects.insert("button", rect());
        let mut c = s.combo;
        imgui::combo("Combo", &mut c, &["one", "two", "three"]);
        s.combo = c;
        let mut col = s.color;
        imgui::color_edit4("Color", &mut col);
        s.color = col;
        imgui::plot_lines("Plot", &[0.0, 0.5, 1.0, 0.5, 0.0]);
        if imgui::tree_node("Tree") {
            imgui::text("inside");
            imgui::tree_pop();
        }
        if imgui::begin_table("table", 3) {
            s.table_cells = 0;
            for r in 0..3 {
                imgui::table_next_row();
                for col in 0..3 {
                    imgui::table_set_column_index(col);
                    imgui::text(&format!("{r},{col}"));
                    s.table_cells += 1;
                }
            }
            imgui::end_table();
        }
        let dl = imgui::get_window_draw_list().expect("draw list");
        let p = imgui::get_cursor_screen_pos();
        dl.add_line(p, [p.x + 40.0, p.y + 10.0], imgui::im_col32(255, 255, 255, 255));
        dl.add_rect_filled_ex(p, [p.x + 10.0, p.y + 10.0], imgui::im_col32(255, 0, 0, 255), 2.0, 0);
        dl.add_polyline_ex(&[p, ImVec2 { x: p.x + 5.0, y: p.y + 9.0 }], 0xFF00FF00, 1.5, 0);
        dl.add_text(p, 0xFFFFFFFF, "dl");
        imgui::dummy([50.0, 12.0]);
        imgui::begin_child("child");
        clipper.begin(1000);
        s.clipped = 0;
        while clipper.step() {
            for i in clipper.DisplayStart..clipper.DisplayEnd {
                imgui::text(&format!("row {i}"));
                s.clipped += 1;
            }
        }
        clipper.end();
        imgui::end_child();
        imgui::end();
    };
    let mut app = imgui::App::headless([1280, 800], frame);
    assert!(app.is_headless());
    app.render(4);
    assert!(st.borrow().frames > 0);
    assert_eq!(st.borrow().table_cells, 9);
    let clipped = st.borrow().clipped;
    assert!(clipped > 0 && clipped < 1000, "clipped {clipped}");

    let center = |st: &Rc<RefCell<State>>, n: &str| {
        let (a, b) = st.borrow().rects[n];
        ((a.x + b.x) / 2.0, (a.y + b.y) / 2.0)
    };
    // checkbox, button
    let (x, y) = center(&st, "check");
    app.click(x, y);
    assert!(st.borrow().checked, "click toggles the checkbox");
    let (x, y) = center(&st, "button");
    app.click(x, y);
    assert_eq!(st.borrow().clicks, 1, "click presses the button");
    // slider
    let (a, b) = st.borrow().rects["slider"];
    let w = st.borrow().slider_w;
    app.click(a.x + w * 0.9, (a.y + b.y) / 2.0);
    if blenderui_imgui::blender_widgets() {
        // variant (Blender widgets): a click types a value instead of jumping to the pointer
        app.key("a", 2); // BK_CTRL
        app.type_text("0.9");
        app.key("Return", 0);
    }
    assert!(st.borrow().slider > 0.8, "slider {}", st.borrow().slider);
    // text
    let (x, y) = center(&st, "input");
    app.click(x, y);
    app.type_text("Hi Rust");
    assert_eq!(st.borrow().text, "Hi Rust");
    app.key("BackSpace", 0);
    assert_eq!(st.borrow().text, "Hi Rus");
    app.type_text(&"z".repeat(100)); // grows the buffer (resize callback)
    assert_eq!(st.borrow().text.len(), 106);
    app.key("Return", 0);

    // structs: IO / style / fonts
    let io = imgui::get_io();
    assert_eq!((io.DisplaySize.x, io.DisplaySize.y), (1280.0, 800.0));
    let style = imgui::get_style();
    assert!((style.Colors[imgui::ImGuiCol_WindowBg as usize].x - 0x30 as f32 / 255.0).abs() < 0.01, "Blender style");
    let fonts = unsafe { &*io.Fonts };
    assert!(fonts.Fonts.Size >= 1);

    // helpers
    let (mut h, mut s, mut v) = (0.0, 0.0, 0.0);
    imgui::color_convert_rgb_to_hsv(1.0, 0.0, 0.0, &mut h, &mut s, &mut v);
    assert!(h.abs() < 1e-4 && (s - 1.0).abs() < 1e-4 && (v - 1.0).abs() < 1e-4);
    assert_eq!(imgui::color_convert_float4_to_u32([1.0, 0.0, 0.0, 1.0]), 0xFF0000FF);
    assert_eq!(imgui::get_key_name(imgui::ImGuiKey_A), "A");
    assert!(imgui::calc_text_size("Hello").x > 10.0);

    // owned objects
    let mut filter = imgui::ImGuiTextFilter::new_ex("two");
    assert!(filter.pass_filter("one two"));
    assert!(!filter.pass_filter("three"));
    filter.clear();
    let mut storage = imgui::ImGuiStorage::new();
    storage.set_int(7, 42);
    assert_eq!(storage.get_int(7), 42);
    let mut tb = imgui::ImGuiTextBuffer::new();
    tb.append("abc");
    assert_eq!(tb.c_str(), "abc");

    // pixels: Blender's region grey
    let (px, w, h) = app.pixels().expect("pixels");
    assert_eq!((w, h), (1280, 800));
    assert!(px.iter().filter(|&&p| p & 0xFFFFFF == 0x303030).count() > 10000);

    // a panic in the frame comes back from render()
    st.borrow_mut().panic_now = true;
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        app.render(2);
    }));
    assert!(r.is_err(), "panic propagates");
    app.render(2); // and the app keeps working
    drop(app);
}
