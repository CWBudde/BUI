# blenderui

Blender's UI for Rust, with [gpui](https://www.gpui.rs)'s syntax, every HTML
element and CSS. Form controls look like a browser's, or are Blender's own
widgets inside `styled` and as the capitalized Blender components (`<NumberSlider>`,
`<Button>` ...); layout is
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

Install it from [crates.io](https://crates.io/crates/blenderui):

```sh
cargo add blenderui
```

**Two syntaxes.** The example above uses gpui-style builders. The same UI can
be written as HTML-like markup with `rsx!` (with `#[component]` functions and
hooks such as `use_state`), and Tailwind classes work in both.

**Three looks.** Plain HTML looks like a browser's; add `styled` to any
element for Blender's look; use the Blender components (`NumberSlider`,
`Button`, `Checkbox`, `ColorSwatch` ...) for Blender's real widgets anywhere.
`blenderui::set_theme("light")` or `Application::new().theme("light")` (or
`BUI_THEME=light`) switches Blender's theme from Dark to Light.

**GPU rendering:** windows draw on the CPU by default. Use
`Application::new().gpu(true)` (or `WindowOptions { gpu: Some(true), .. }`, or
run with `BUI_GPU=1`) to draw them on the GPU: Direct3D 11 on Windows, OpenGL
3.3 on Linux. This produces the same pixels at a fraction of the CPU cost:
11.6 ms vs 22.0 ms per frame for 3000 animated cells, where gpui-fast needs
31.9 ms (Windows, Direct3D 11; measured before the latest features landed).

**Text** works the same on every platform: the default portable engine embeds
Inter (Blender's UI font), renders it with FreeType like Blender does, and
draws it with Slug on the GPU. The opt-in `gdi-text` feature uses Windows' GDI
engine and system fonts instead. (`portable-text` is a deprecated no-op.)
Register extra fonts with `blenderui::add_font`.

**Platforms:** Windows (Win32) and Linux (X11) native windows; macOS (Cocoa)
is experimental. You need Rust 1.88+, CMake and a C compiler (the first build
compiles BUI's C kit), and the X11 headers on Linux.

Dear ImGui on BUI is a separate crate, [`blenderui-imgui`](https://crates.io/crates/blenderui-imgui):
`cargo add blenderui-imgui`.

Guide, CSS coverage and limitations:
[docs/rust.md](https://github.com/ppmpreetham/BUI/blob/main/docs/rust.md),
and the website: <https://bui.preetham.top>.
