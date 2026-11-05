//! Dear ImGui on BUI from Rust: ImGui's demo window plus a window of our own, in a native
//! BUI window with Blender's look.
//!
//!     cargo run -p blenderui-imgui --example demo
//!     cargo run -p blenderui-imgui --example demo -- --screenshot demo.png   (headless)
//!
//! SPDX-License-Identifier: GPL-2.0-or-later

use blenderui_imgui as imgui;

fn main() {
    let shot = std::env::args().skip_while(|a| a != "--screenshot").nth(1);

    let mut show_demo = true;
    let mut value = 0.5f32;
    let mut counter = 0;
    let mut text = String::from("Edit me");
    let mut color = [0.278f32, 0.447, 0.702];
    let frame = move || {
        if show_demo {
            imgui::show_demo_window_ex(Some(&mut show_demo));
        }
        let dpi = imgui::get_style().FontScaleDpi;
        imgui::set_next_window_pos_ex([20.0 * dpi, 20.0 * dpi], imgui::ImGuiCond_FirstUseEver, [0.0, 0.0]);
        imgui::set_next_window_size_ex([340.0 * dpi, 300.0 * dpi], imgui::ImGuiCond_FirstUseEver);
        imgui::begin("BUI from Rust");
        imgui::text(&format!("Dear ImGui {} on BUI, from Rust", imgui::get_version()));
        imgui::text_disabled(&format!("UI scale {:.2}", imgui::ui_scale()));
        imgui::separator_text("Widgets");
        imgui::checkbox("Demo window", &mut show_demo);
        imgui::slider_float("Value", &mut value, 0.0, 1.0);
        if imgui::button("Count") {
            counter += 1;
        }
        imgui::same_line();
        imgui::text(&counter.to_string());
        imgui::input_text("Text", &mut text);
        imgui::color_edit3("Color", &mut color);
        imgui::progress_bar_ex(value, [-1.0, 0.0], None);
        if let Some(dl) = imgui::get_window_draw_list() {
            let p = imgui::get_cursor_screen_pos();
            dl.add_rect_filled(p, [p.x + 60.0 * dpi, p.y + 20.0 * dpi], imgui::im_col32(71, 114, 179, 255));
            dl.add_circle([p.x + 90.0 * dpi, p.y + 10.0 * dpi], 9.0 * dpi, imgui::im_col32(255, 255, 255, 255));
        }
        imgui::dummy([100.0 * dpi, 22.0 * dpi]);
        imgui::end();
    };

    match shot {
        Some(path) => {
            let mut app = imgui::App::headless([1280, 800], frame);
            let frames = app.render(8);
            println!("{} {} ({} frames)", if app.save_png(&path) { "wrote" } else { "could not write" }, path, frames);
        }
        None => imgui::run("Dear ImGui on BUI (Rust)", [1280, 800], frame),
    }
}
