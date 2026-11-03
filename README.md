<div align="center">

# BUI

**Blender's user interface, as a library you can use from Python, Rust and C.**

Scrubbable number fields, drag-toggles, pie menus, the color wheel and Ctrl+Z on every edit:
the real Blender widgets, driven by the tkinter or Qt code you already know, by HTML + CSS + Tailwind in Rust, or by plain C.

[![PyPI](https://img.shields.io/pypi/v/blenderui?label=pypi%20blenderui)](https://pypi.org/project/blenderui/)
[![Python](https://img.shields.io/pypi/pyversions/blenderui)](https://pypi.org/project/blenderui/)
[![Wheels](https://github.com/ppmpreetham/BUI/actions/workflows/wheels.yml/badge.svg)](https://github.com/ppmpreetham/BUI/actions/workflows/wheels.yml)
[![License: GPL-2.0-or-later](https://img.shields.io/badge/license-GPL--2.0--or--later-blue)](#license)
![Platforms](https://img.shields.io/badge/platforms-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey)

[Quick start](#quick-start) ·
[What it can do](#what-it-can-do) ·
[Status](#status-whats-done-and-whats-not) ·
[Docs](#documentation) ·
[Performance](#performance) ·
[Contributing](#contributing)

<img src="https://raw.githubusercontent.com/ppmpreetham/BUI/main/docs/images/components.png" width="330" alt="BUI's Blender components: panels, number sliders, vector fields, checkboxes and a color swatch">
<img src="https://raw.githubusercontent.com/ppmpreetham/BUI/main/docs/images/tkinter_items.png" width="300" alt="An unmodified tkinter program running on BUI">

<sub>Left: the component library. Right: an unmodified tkinter program, now looking like Blender.</sub>

</div>

> [!NOTE]
> This repository is built entirely using LLMs. No human-written code.

## What is BUI?

BUI takes the user interface of [Blender](https://www.blender.org) (its widget
drawing, its interaction rules and its look) and turns it into a standalone
UI library with a small C core. Your code builds the widget tree and handles
callbacks. Layout, input, drawing and presenting all happen in fast native
code.

You can drive it four ways, and all of them share the same C runtime:

| You write | You get | Status |
|---|---|---|
| **tkinter** code, even existing programs, unchanged | Blender-styled apps; every tkinter and ttk API | ✅ Complete |
| **Qt-style** Python (`QWidget`, layouts, `clicked.connect`) | The common subset of Qt widgets | 🟡 Partial |
| **Blender components** in Python (`Panel`, `NumberSlider`, `PieMenu` ...) | 45 native Blender widgets and editors | ✅ Complete |
| **Rust**: HTML + CSS + Tailwind, JSX (`rsx!`), hooks, or gpui syntax | A web-like framework rendered with Blender widgets | ✅ Working |

## Quick start

### Python

```sh
pip install blenderui
```

Run any tkinter program on BUI **without changing a line**:

```sh
blenderui-tk my_app.py          # same as: python -m bui.tksyntax my_app.py
```

Or opt in from code:

```python
import blenderui.tksyntax as tk
from blenderui.tksyntax import ttk, messagebox

root = tk.Tk()
name = tk.StringVar(value="Cube")
ttk.Entry(root, textvariable=name).pack(fill="x", padx=8, pady=8)
ttk.Button(root, text="Greet",
           command=lambda: messagebox.showinfo("Hi", "Hello " + name.get())).pack()
root.mainloop()
```

Or build with Blender's own components:

```python
from blenderui.components import App, Panel, NumberSlider, VectorField, Checkbox, ColorSwatch, Button

App("Object", (420, 520),
    Panel("Transform",
          VectorField("Location", (0.0, 1.25, 0.0)),
          NumberSlider("Scale", 1.0, min=0, max=10, on_change=print)),
    Panel("Shading",
          Checkbox("Shadows", True),
          ColorSwatch("Color", (0.8, 0.3, 0.1, 1.0))),
    Button("Apply", on_click=lambda: print("apply")),
).run()
```

### Rust

```bash
cargo add blenderui
```

```rust
use blenderui::prelude::*;

#[component]
fn counter(label: &str) {
    let (count, set_count) = use_state(0);
    rsx! {
        <div flex items-center gap-3 p-4 rounded-xl bg-zinc-800>
            <span text-zinc-300>{label.to_string()} ": " {count().to_string()}</span>
            <div px-3 py-1 rounded-md bg-sky-700 hover:bg-sky-500 onClick={move || set_count(count() + 1)}>
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
                <input type="range" min={0.0} max={10.0} />   // a real Blender slider
            </body>
        }
    }
}

fn main() {
    Application::new().run(|cx| {
        cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| Hello)).unwrap();
    });
}
```

<p align="center">
  <img src="https://raw.githubusercontent.com/ppmpreetham/BUI/main/docs/images/rust-hello.png" width="420" alt="The Rust hello example: a heading, a counter component and a Blender slider">
</p>

This example lives at [`bindings/rust/blenderui/examples/hello.rs`](bindings/rust/blenderui/examples/hello.rs)
(`cargo run --release --example hello`).

### C

```c
#include "BUI_kit.h"   /* see examples/kit_hello.c and docs/c-api.md */
```

Build from source with CMake (see [Building from source](#building-from-source)).

## What it can do

<table>
<tr><td width="50%" valign="top">

### 🎛️ Blender's widgets and interactions
- Number fields you **drag with a hidden, continuous cursor**: Shift for precision, Ctrl to snap.
- Ctrl+Wheel steps values, Ctrl+C / Ctrl+V copy values, Backspace resets.
- **Drag across checkboxes** to set them all.
- Press-drag-release menus, **type to search** in menus, pie menus.
- **Ctrl+Z / Ctrl+Shift+Z** on every UI edit, Tab between fields.
- Multi-field vertical drags, animated collapsing panels.
- The HSV color wheel, color ramps, curves, trees, lists, text areas and canvases.

</td><td width="50%" valign="top">

### 🐍 Python, three ways
- **tkinter**: all 7,846 public names of tkinter and ttk. Nine real programs behave identically on the standard library and on BUI.
- **Qt syntax**: widgets, layouts, signals, `QPainter`, menus and timers.
- **Components**: 45 Blender widgets, editors and templates.
- One `pip install`, no Qt or Tk widgets drawing anything.

</td></tr>
<tr><td valign="top">

### 🦀 Rust: the web platform, natively
- **All 113 HTML elements**; form controls are real Blender widgets.
- **Modern CSS**: flexbox, grid with named areas, `calc()` and every math function, `var()`, `oklch()` and `color-mix()`, `:has()`, nesting, `@layer`, `@media` ranges, `::before`/`::after`.
- **Tailwind CSS v4**: `class="md:hover:bg-sky-500/50"` or bare attributes (`<div p-4 bg-[#4772b3]>`), with **"did you mean" compile errors**.
- **`rsx!` / `view!`** JSX, **`#[component]`**, and **hooks** in zopra, React or Leptos style (`use_state`, `signal`, `Memo`, `Effect`, context).
- **gpui-compatible syntax** too: `div().flex().gap_2()`, `Render`, `cx.listener`.

</td><td valign="top">

### ⚡ Fast by design
- **Damage rendering**: an idle UI costs 0% CPU, and a hover redraws only the widget it touches.
- **Opt-in GPU renderer** (Direct3D 11), modeled on Blender's instanced widget drawing. It draws the same pixels as the CPU raster.
- In the benchmark, **2–4× less CPU per frame than gpui-fast**, with **less RAM** and a **~5 ms first frame**.
- **Blender's text**: Inter (Blender's UI font) is embedded, so text looks the same on every platform. On the GPU, glyphs are drawn from their outlines with [Slug](https://github.com/EricLengyel/Slug), falling back to a per-glyph atlas like Blender's.

</td></tr>
</table>

<details>
<summary><b>More screenshots</b></summary>
<p align="center">
  <img src="https://raw.githubusercontent.com/ppmpreetham/BUI/main/docs/images/rust-showcase.png" width="560" alt="The Rust showcase: a stylesheet, grid cards, Blender form widgets, a table and lists">
  <br><sub><code>examples/showcase.rs</code>: HTML + CSS in Rust</sub><br><br>
  <img src="https://raw.githubusercontent.com/ppmpreetham/BUI/main/docs/images/qt.png" width="420" alt="Qt syntax on BUI">
  <br><sub>Qt syntax</sub><br><br>
  <img src="https://raw.githubusercontent.com/ppmpreetham/BUI/main/docs/images/tkinter_notes.png" width="260" alt="tkinter Text with tags">
  <img src="https://raw.githubusercontent.com/ppmpreetham/BUI/main/docs/images/tkinter_canvas.png" width="260" alt="tkinter Canvas">
  <img src="https://raw.githubusercontent.com/ppmpreetham/BUI/main/docs/images/tkinter_messagebox.png" width="260" alt="tkinter messagebox">
  <br><sub>tkinter programs: text with tags, canvas, dialogs</sub>
</p>
</details>

### A taste of the Rust side

```rust
rsx! {
    // Tailwind as attributes, any variant, arbitrary values; typos fail to compile
    <button px-3 py-1 rounded-md bg-zinc-800 hover:bg-sky-600 md:dark:hover:bg-sky-400
            active:style="transform: scale(0.97)" onClick={move || save()}>
        "Save"
    </button>

    // control flow right in the markup
    <ul>{for item in items { <li class="py-1">{item}</li> }}</ul>
    {if dirty { <p text-amber-400>"Unsaved changes"</p> }}

    // real CSS, evaluated by the engine
    <div style="width: calc(100% - 2 * var(--gap)); background: color-mix(in oklch, var(--brand) 60%, white)" />
}
```

```text
error: rsx!: `bg-sky-50O` is not a Tailwind utility; did you mean `bg-sky-500` or `bg-sky-50` or `bg-sky-100`?
```

## Status: what's done and what's not

### ✅ Done

| Area | What works |
|---|---|
| C runtime (`kit/`) | Nodes, Tk-style geometry managers, input, Blender widget drawing, damage rendering, undo, animation, text area, canvas, scroll views |
| Windows | A native Win32 window in C: DPI-aware, vsync-paced animation, clipboard, cursors, window-manager features |
| tkinter syntax | **Complete**: every public tkinter and ttk name (7,846 of 7,846) |
| Components | 45 Blender components, including the color ramp and curve editors |
| Rust | HTML (113/113 elements), Flexbox 12/12, Grid 23/23, CSS 2.1 80/90, math functions, `var()`, CSS Color 4/5, Selectors 4, `@layer`/`@media`/`@supports`/`@scope`, Tailwind v4, `rsx!`, `#[component]`, hooks, headless testing |
| Rendering | Software raster (parallel, damage-tracked), GPU renderer (Direct3D 11), portable text engine |
| Packaging | [`blenderui` on PyPI](https://pypi.org/project/blenderui/), with a CI workflow for wheels on all platforms |

### 🟡 Partial or in progress

| Area | What's missing |
|---|---|
| Qt syntax (~25%) | Model/view classes, docks, toolbars, splitters, Qt stylesheets ([details](docs/qt.md#coverage)) |
| CSS (176 of 745 W3C properties render) | Rendering effects: `rotate`/`skew` transforms, `filter`/`backdrop-filter`, blend modes, `clip-path`, masks, `border-image`, non-solid borders. Also animations and transitions, background images, applying `@container`, `@font-face`. Most of the remaining W3C list is drafts, speech, print and SVG ([breakdown](docs/rust.md#coverage)) |
| Text | No text wrapping around floats; no right-to-left shaping |
| Rust components | No optional props; no async resources (`use_resource`); no generic components |
| Linux / macOS | They work through a fallback window (the stdlib Tk window shows BUI's frame), but they're less tested. No native window host and no GPU renderer yet |
| crates.io | The crates are ready to publish but not published yet; use the git dependency for now |

### 🗺️ Planned

| Target | Plan |
|---|---|
| **WebAssembly** | The kit in the browser: the raster blits to `<canvas>`, DOM events feed the kit. This unlocks the React, JS/TS and Tauri targets |
| **React** (`@blenderui/react`) | A JSX reconciler over the WASM build |
| **egui** | BUI panels inside egui, plus a Blender-styled theme |
| **Native Linux / macOS windows** | Wayland / X11 and Cocoa hosts in C, like the Win32 one |
| **More GPU backends** | Vulkan / Metal / WebGPU drawing the same instance stream as Direct3D 11 |
| **More languages** | C#, Go, Zig, Swift, Kotlin and Lua over the same C header; Dear ImGui; Godot, Unity and Bevy; Flutter |

## How it works

```
 your code:   tkinter syntax   Qt syntax   components        Rust: rsx! / HTML / CSS / Tailwind / hooks
                     │              │            │                     │
                     └──── Python: options, signals, callbacks         └── style cascade + taffy layout
                                         │                                       │
 C kit (kit/):   node tree · geometry · input & interaction · Blender widget drawing · undo · animation
                                         │
 renderer:       damage-driven software raster (parallel)   or   GPU (Direct3D 11, instanced, opt-in)
                                         │
 window:         native Win32 window   ·   stdlib Tk window (elsewhere)   ·   headless (tests, screenshots)
```

- **Blender's real UI code.** Widget drawing is a C port of Blender's
  `interface_widgets.cc` (`round_box_edges`, `widgetbase_draw`, the shape
  tables). Color math, rects and text cursors are Blender's `blenlib`.
- **One node tree.** Every widget is one kit node. The C kit owns layout, hit
  testing, hover, focus, drags, menus, tooltips, undo and animation.
- **Damage rendering.** A change marks only its widget's rectangle, and only
  changed tiles reach the screen.
- **Thin bindings.** Python and Rust keep only what the syntax layer needs.

More: [ARCHITECTURE.md](ARCHITECTURE.md).

## Performance

Python and C, on the full component demo (520×900):

| Frame | Time |
|---|---|
| Complete frame (everything redrawn) | ~0.7 ms |
| Hover or number-scrub frame (damage only) | ~0.05 ms |
| Native window: input event → on screen | ~0.06 ms |

Rust, against Zed's [gpui](https://crates.io/crates/gpui) and the faster fork
[gpui-fast](https://github.com/longbridge/gpui-fast). The scene is 3000
animated cells, the machine a Ryzen 7 5800U laptop, and all apps ran in the
same session:

| | blenderui (GPU) | blenderui (CPU) | gpui-fast | gpui |
|---|---:|---:|---:|---:|
| FPS | **59.9** | 57.6 | 31.5 | 19.9 |
| CPU per frame | **11.6 ms** | 22.0 ms | 31.9 ms + GPU | 50.5 ms + GPU |
| RAM | 40 MB | **30 MB** | 65 MB | 77 MB |
| First frame | **~5 ms** | **~5 ms** | ~200 ms | ~220 ms |
| Idle CPU | **0 %** | 0–0.9 % | 0–0.4 % | 0.9–1.4 % |

<details>
<summary>Full results, the GPU renderer, and the method</summary>

| | blenderui (GPU) | blenderui (CPU) | gpui-fast | gpui |
|---|---:|---:|---:|---:|
| FPS, 1000 animated cells | **60** | **60** | 49.2 | 36.4 |
| CPU per frame, 1000 / 100 cells | **4.7 / 1.2 ms** | 10.2 / 4.6 ms | 20.5 / 2.7 ms + GPU | 27.7 / 4.7 ms + GPU |
| Uncapped FPS, 100 / 1000 / 3000 cells | **1491 / 309 / 94** | 649 / 202 / 76 | - | - |
| RAM, 1000 / 100 cells | 35 / 28 MB | **24 / 21 MB** | 45 / 47 MB | 47 / 40 MB |
| GPU memory | 19–22 MB | **none** | 19–24 MB | 16–21 MB |

The GPU renderer is off by default. Turn it on with
`Application::new().gpu(true)`, `tk.Tk(gpu=True)`, `bui.set_gpu(True)` or
`BUI_GPU=1`:
- Every primitive is one instance of a single shader, so a frame is a few
  `DrawInstanced` calls.
- Text and images are cached in atlases.
- Frames render into a persistent target, so only damaged regions redraw.
- Presentation uses a flip-model swap chain with dirty rects.

Its shaders evaluate the software raster's formulas: the parity test finds
10 of 172,800 pixels that differ by more than 3/255.

gpui-fast's heavy-scene frame rate varies between sessions on this laptop
(30–60 fps), so compare within one session. Method, raw data and how to rerun:
[bindings/rust/bench/README.md](bindings/rust/bench/README.md).
</details>

## Documentation

| Guide | What's inside |
|---|---|
| [Getting started](docs/getting-started.md) | Building, environment variables, window hosts, GPU rendering, troubleshooting |
| [tkinter](docs/tkinter.md) | Running tkinter apps on BUI, behavior notes, limitations |
| [Qt syntax](docs/qt.md) | Classes, signals, layouts, painting, coverage |
| [Components](docs/components.md) | The `bui.components` reference |
| [Rust](docs/rust.md) | `rsx!`, hooks, HTML, CSS and Tailwind coverage, events, headless tests |
| [C API](docs/c-api.md) | The C kit (`BUI_kit.h`) |
| [Integration](INTEGRATION.md) | Embedding BUI drawing in C, C++, Qt or PySide hosts |
| [Architecture](ARCHITECTURE.md) | Modules and what was taken from Blender |
| [Publishing](docs/publishing.md) | Releasing to PyPI and crates.io |
| [Changelog](CHANGELOG.md) | What changed |

<details>
<summary><b>Examples</b></summary>

| File | Shows |
|---|---|
| [`examples/tk_unchanged_app.py`](examples/tk_unchanged_app.py) | A plain tkinter app: menus, notebook, form, table, text, canvas |
| [`examples/components_demo.py`](examples/components_demo.py) | Every Blender component in a properties-editor layout |
| [`examples/qt_syntax_demo.py`](examples/qt_syntax_demo.py) | Qt syntax: layouts, form widgets, signals |
| [`examples/tk_syntax_demo.py`](examples/tk_syntax_demo.py) | tkinter syntax laid out like a Blender properties editor |
| [`examples/kit_hello.c`](examples/kit_hello.c) | The C kit: a slider and a button in a native window |
| [`hello.rs`](bindings/rust/blenderui/examples/hello.rs) | Rust: a component, hooks and Tailwind attributes |
| [`showcase.rs`](bindings/rust/blenderui/examples/showcase.rs) | Rust: HTML, a stylesheet, flex/grid, tables, Blender form widgets |
</details>

## Building from source

You need **CMake ≥ 3.20**, a **C11 compiler** (GCC/MinGW or Clang; MSVC
works for the Rust build) and **Python 3.11+** with headers.

```sh
git clone https://github.com/ppmpreetham/BUI && cd BUI
cmake -S . -B build && cmake --build build
ctest --test-dir build                      # C and Python test suites

export PYTHONPATH=$PWD/bindings/python      # Windows: set PYTHONPATH=%CD%\bindings\python
python examples/components_demo.py

cd bindings/rust && cargo test              # Rust: CSS, Tailwind, rsx!, hooks, headless windows
```

<details>
<summary>Repository layout</summary>

| Path | Contents |
|---|---|
| `kit/` | The C runtime: nodes, input, widget drawing (the `interface_widgets.cc` port), menus, geometry managers, canvas, the Win32 host. API: `kit/include/BUI_kit.h` |
| `gpu/` | Software raster, Direct3D 11 renderer (`gpu_d3d11.c`, `shaders/`, Slug text), text engine (stb_truetype + embedded Inter, or GDI) |
| `core/`, `types/` | Blender's `blenlib` subset and data types |
| `ui/`, `compat/` | Blender's original interface sources and the seams replacing Blender's application APIs |
| `api/` | Stable C ABI for render-sink hosts |
| `bindings/python/` | The `buipy` extension and the `bui` / `blenderui` packages |
| `bindings/rust/` | `blenderui`, `blenderui-sys`, `blenderui-macros`, `blenderui-tailwind`, and the benchmark |
| `backends/qt/` | C++ Qt/QPainter render-sink backend |
| `examples/`, `tools/` | Demos, the icon generator, the docs screenshot generator |
</details>

## FAQ

<details>
<summary><b>Do I need Blender installed?</b></summary>
No. BUI contains the parts of Blender's UI code it needs and builds on its own.
</details>

<details>
<summary><b>Is it really my unchanged tkinter app?</b></summary>
Yes. <code>blenderui-tk app.py</code> makes <code>import tkinter</code> return BUI's
implementation, which covers every public tkinter and ttk name. Nine real
programs are tested to behave identically on the standard library and on BUI.
See <a href="docs/tkinter.md#limitations">the limitations</a> for the few
differences.
</details>

<details>
<summary><b>Is it a web browser in Rust?</b></summary>
No. There's no DOM, no JavaScript and no network. The Rust crate gives you
HTML elements and CSS as a <i>UI language</i>: they are laid out with
<a href="https://github.com/DioxusLabs/taffy">taffy</a> and drawn with Blender's
widgets, in a native window.
</details>

<details>
<summary><b>Do the macros cost performance?</b></summary>
<code>rsx!</code>, <code>#[component]</code>, <code>css!</code> and <code>tw!</code>
expand at compile time into the same builder calls you would write by hand.
Tailwind classes are compiled to CSS once, on first use. Hooks are a few
nanoseconds per read or write.
</details>

<details>
<summary><b>Why GPL?</b></summary>
BUI is derived from Blender's source code, which is GPL-2.0-or-later, so BUI is too.
</details>

## Contributing

Contributions are welcome: bug reports, missing tkinter/Qt behavior, CSS
features, platform hosts and docs. Read [CONTRIBUTING.md](CONTRIBUTING.md) first,
and please follow the [Code of Conduct](CODE_OF_CONDUCT.md). Report security
issues privately, as [SECURITY.md](SECURITY.md) describes.

## Acknowledgments

- [Blender](https://www.blender.org): the UI this project is built from.
- [taffy](https://github.com/DioxusLabs/taffy): flexbox, grid and block layout for the Rust crate.
- [Tailwind CSS](https://tailwindcss.com): the v4 theme (MIT) behind the Tailwind support.
- [stb_truetype](https://github.com/nothings/stb): the text engine (public domain).
- [Inter](https://rsms.me/inter) by Rasmus Andersson: the embedded UI font, as in Blender (SIL Open Font License 1.1).
- [Slug](https://github.com/EricLengyel/Slug) by Eric Lengyel: the GPU glyph coverage algorithm (shader code MIT, "Copyright 2017 by Eric Lengyel").
- [gpui](https://github.com/zed-industries/zed), [zopra](https://github.com/ppmpreetham/zopra) and [Leptos](https://leptos.dev): the APIs the Rust syntax follows.

## License

BUI is derived from Blender sources and is licensed **GPL-2.0-or-later**. The
SPDX header is kept in every file. Third-party parts keep their own licenses:
stb_truetype is public domain, the Tailwind theme values are MIT, Inter is
OFL-1.1 (`gpu/extern/Inter-LICENSE.txt`), and the Slug shader port is MIT.
