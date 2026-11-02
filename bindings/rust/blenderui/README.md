# blenderui

Blender's UI for Rust, with [gpui](https://www.gpui.rs)'s syntax, every HTML
element and CSS. Form controls are Blender's own widgets; layout is
[taffy](https://github.com/DioxusLabs/taffy); drawing and input come from the
[BUI](https://github.com/ppmpreetham/BUI) C kit (built from source, so CMake
and a C compiler are needed).

```rust
use blenderui::prelude::*;

struct Counter { count: i32 }

impl Render for Counter {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().flex().flex_col().gap_2().p_4()
            .child(h1().child(format!("Count: {}", self.count)))
            .child(button().child("Add one").on_click(cx.listener(|this, _, _, cx| {
                this.count += 1;
                cx.notify();
            })))
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| Counter { count: 0 }))
            .unwrap();
    });
}
```

Guide, CSS coverage and limitations:
[docs/rust.md](https://github.com/ppmpreetham/BUI/blob/main/docs/rust.md).
