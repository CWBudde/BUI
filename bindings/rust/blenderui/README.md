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

**GPU rendering:** windows draw on the CPU by default. Use
`Application::new().gpu(true)` (or `WindowOptions { gpu: Some(true), .. }`, or
run with `BUI_GPU=1`) to draw them with Direct3D 11. This produces the same
pixels at a fraction of the CPU cost: 11.6 ms vs 22.0 ms per frame for
3000 animated cells, where gpui-fast needs 31.9 ms.

**Text** works on every platform: GDI on Windows and a bundled stb_truetype
engine elsewhere (the `portable-text` feature selects it on Windows).
Register fonts with `blenderui::add_font` where there are none (wasm).

Guide, CSS coverage and limitations:
[docs/rust.md](https://github.com/ppmpreetham/BUI/blob/main/docs/rust.md).
