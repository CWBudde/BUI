//! The README example: `cargo run --example counter`.

use blenderui::prelude::*;

struct Counter { count: i32 }

impl Render for Counter {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().flex().flex_col().gap_2().p_4()
            .child(h1().child(format!("Count: {}", self.count)))
            .child(p().child("Any ").child(b().child("HTML element")).child(", styled with ")
                .child(code().child(".css()")).child(" or a <style> sheet."))
            .child(button().child("Add one").on_click(cx.listener(|this, _, _, cx| {
                this.count += 1;
                cx.notify();
            })))
            .child(input().input_type("range").min(0.0).max(10.0))   // a Blender slider
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| Counter { count: 0 }))
            .unwrap();
    });
}
