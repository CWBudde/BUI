# BUI (Blender UI)

> [!NOTE]
> This repository is built entirely using LLMs. No human-written code.

BUI is the user interface of [Blender](https://www.blender.org), extracted
into a standalone, fast, C-first UI library. You get Blender's look _and_
Blender's interactions, scrubbable number fields, drag-toggles, pie menus,
the color wheel, Ctrl+Z on any edit and you can drive it three ways:

- **tkinter syntax**: run existing tkinter programs unchanged, now looking
  like Blender.
- **Qt syntax**: write `QWidget` / `QVBoxLayout` / `clicked.connect` code
  (a common subset today).
- **Blender components**: a native component library (`Panel`,
  `NumberSlider`, `ColorSwatch`, `TreeView`, `PieMenu` ...).
- **Rust, gpui syntax**: the `blenderui` crate. `div().flex().gap_2().child(...)`,
  `cx.listener`, `Render`, plus every HTML element and CSS (stylesheets,
  selectors, flexbox, grid, tables), with form controls as Blender widgets.

All of them share one C runtime (`kit/`). Python (or Rust) builds the widget
tree and runs your callbacks. Input, widget drawing and presenting stay in C.

```sh
pip install blenderui          # on PyPI: https://pypi.org/project/blenderui/
```

<p align="center">
  <img src="docs/images/components.png" width="340" alt="Blender components">
  <img src="docs/images/tkinter_items.png" width="300" alt="A plain tkinter program on BUI">
</p>
<p align="center"><sub>Left: the component library (<code>examples/components_demo.py</code>).
Right: an unmodified tkinter program (<code>examples/tk_unchanged_app.py</code>) running on BUI.</sub></p>

---

## Contents

- [Why BUI](#why-bui)
- [Status](#status)
- [Install and build](#install-and-build)
- [Quick start](#quick-start)
  - [tkinter syntax](#tkinter-syntax)
  - [Qt syntax](#qt-syntax)
  - [Blender components](#blender-components)
  - [Rust (gpui syntax, HTML and CSS)](#rust-gpui-syntax-html-and-css)
- [How it works](#how-it-works)
- [Performance](#performance) (including [blenderui vs gpui](#rust-blenderui-vs-gpui) and the [GPU renderer](#gpu-renderer))
- [Roadmap: more languages and platforms](#roadmap-more-languages-and-platforms)
- [Packaging](#packaging)
- [Documentation](#documentation)
- [Repository layout](#repository-layout)
- [Testing](#testing)
- [License](#license)

---

## Why BUI

|                            |                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| -------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Blender's real UI code** | Widget drawing is a C port of Blender's `interface_widgets.cc` (`round_box_edges`, `widgetbase_draw`, `widget_state`, the shape tables). Color math, rects and text cursors are Blender's `blenlib`, linked. The HSV wheel uses Blender's own functions.                                                                                                                                                                                       |
| **Blender's interactions** | Drag number fields with a hidden, continuous cursor (Shift is precise, Ctrl snaps). Ctrl+Wheel steps values, Ctrl+C/V copies and pastes them, Backspace resets. Drag across checkboxes to set them all. Press-drag-release picks menu items, and typing in a menu searches it. Ctrl+Z / Ctrl+Shift+Z undoes UI edits. Tab moves between fields. Drag vertically over number fields to edit several at once. Panels collapse with an animation. |
| **Familiar APIs**          | Use the tkinter or Qt code you already know, or the component library.                                                                                                                                                                                                                                                                                                                                                                         |
| **Fast**                   | Rendering is damage-driven: an idle UI draws nothing, and a hover or scrub redraws only the widget it touches. A full window renders in under a millisecond (see [Performance](#performance)).                                                                                                                                                                                                                                                 |
| **Small dependencies**     | C11 plus a CPython extension. No Qt or Tk widgets are used to draw anything.                                                                                                                                                                                                                                                                                                                                                                   |

## Status

| Part                                 | State                 | Details                                                                                                                                                                            |
| ------------------------------------ | --------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C kit (`kit/`)                       | ✅ Working            | Nodes, Tk-style geometry managers, input, Blender widget drawing, damage rendering, undo, animation, text area, canvas, scroll views, multi-select                                 |
| Native window (Windows)              | ✅ Working            | Win32 window in C: DPI-aware, vsync'd animation, clipboard, cursors, window-manager features                                                                                       |
| Other platforms                      | ⚠️ Fallback           | The window is hosted by the stdlib `tkinter`, which draws the frame buffer; the UI is identical. Linux and macOS are not regularly tested.                                         |
| **tkinter syntax** (`bui.tksyntax`)  | ✅ **Complete API**   | Every public name of every tkinter and ttk widget class exists (7846/7846). Nine real tkinter programs behave identically on the stdlib and on BUI.                                |
| **Qt syntax** (`bui.qtsyntax`)       | 🟡 **Partial (~25%)** | The common subset works: widgets, layouts, signals, `QPainter`, menus, timers. Model/view, docks, toolbars, splitters and stylesheets are missing. [Details](docs/qt.md#coverage). |
| Component library (`bui.components`) | ✅ Working            | 45 Blender components, including editors (color ramp, curves) and templates                                                                                                        |
| C++ / Qt render-sink backends        | ✅ Working            | For embedding BUI drawing into your own renderer ([INTEGRATION.md](INTEGRATION.md))                                                                                                |
| Packaging (`pip install blenderui`)  | ✅ Published          | [`blenderui` 0.1.0 on PyPI](https://pypi.org/project/blenderui/): a prebuilt wheel for Windows / CPython 3.12, and a source package that other platforms compile on install (needs CMake and a C compiler). Prebuilt wheels for all platforms come from CI on the next tagged release ([docs/publishing.md](docs/publishing.md)) |
| **Rust** (`blenderui` crate)         | ✅ **Working**        | gpui syntax (`Render`, `Context`, `Entity`, `cx.listener`, ~1,300 `Styled` methods) over **113/113 HTML elements** and CSS: **180 properties rendered** (+81 parsed without effect); against the 745-property W3C index, 24% rendered, 10% parsed only and 66% left (mostly drafts, speech, print and SVG; [details](docs/rust.md#coverage)), CSS 2.1 screen properties 80/90 rendered, Flexbox 12/12, Grid 23/23, plus `calc()` and every math function, `var()`, CSS Color 4/5 (`oklch`, `color-mix`), Selectors 4 (`:has()`), nesting, `@layer`, and **Tailwind CSS v4** classes (`tw!` checks them at compile time). Layout by [taffy](https://github.com/DioxusLabs/taffy). Native window on Windows, headless rendering everywhere. Animating 3000 elements: 59.9 fps on 11.6 ms CPU per frame with the opt-in GPU renderer (same session: gpui-fast 31.5 fps on 31.9 ms, gpui 19.9 fps on 50.5 ms), with less memory and a ~5 ms first frame ([benchmark](bindings/rust/bench/README.md)). [docs/rust.md](docs/rust.md) |
| **egui integration**                 | 🗺️ Planned             | BUI panels drawn inside egui (raster → `egui::TextureHandle`, input forwarded) and a Blender-styled `egui::Visuals` theme. Builds on the Rust crate |
| **WebAssembly build**                | 🗺️ Planned             | The kit compiled with Emscripten: the raster blits to `<canvas>` (damage rectangles only), DOM events feed `BK_press` / `BK_key` |
| **React** (`@blenderui/react`)       | 🗺️ Planned             | A JSX reconciler over the WASM build (`<Panel>`, `<NumberSlider>` ...); `@blenderui/web` for plain JS/TS, N-API for Node |
| Other languages and hosts            | 🗺️ Planned             | C#, Go, Zig, Swift, Kotlin, Lua; Dear ImGui; Godot, Unity, Bevy; Flutter; native Linux/macOS windows; Vulkan/Metal/WebGPU renderers ([roadmap](#roadmap-more-languages-and-platforms)) |

## Install and build

### From PyPI

```sh
pip install blenderui
```

```python
import blenderui.tksyntax as tk          # tkinter syntax
from blenderui.components import App     # Blender components
from blenderui.qtsyntax import QWidget   # Qt syntax
```

`blenderui` and `bui` are the same package (`blenderui.X` *is* `bui.X`);
the examples below use `bui` because that's how the source tree is
organised. Use `blenderui` in published code: the `bui` name on PyPI
belongs to an unrelated project. A console script comes with it:
`blenderui-tk app.py` runs a tkinter app on BUI.

### From source

Requirements: **CMake ≥ 3.20**, a **C11/C++17 compiler** (GCC / MinGW or
Clang; MSVC is untested), and **Python 3.11+** with development headers.

```sh
git clone <this repository> BUI
cd BUI
cmake -S . -B build
cmake --build build
ctest --test-dir build            # C + Python test suites
```

The Python package lives in `bindings/python/bui`. Its compiled extension
(`buipy`) stays in the build tree, and `import bui` finds it there
automatically. Put `bindings/python` on `PYTHONPATH` and you are done:

```sh
export PYTHONPATH=$PWD/bindings/python        # Windows: set PYTHONPATH=%CD%\bindings\python
python examples/components_demo.py
```

More detail, including environment variables and troubleshooting:
[docs/getting-started.md](docs/getting-started.md).

## Quick start

### tkinter syntax

Run any tkinter program on BUI **without changing it**:

```sh
python -m bui.tksyntax examples/tk_unchanged_app.py
```

Or opt in from code, before anything imports `tkinter`:

```python
import bui.tksyntax
bui.tksyntax.install()          # `import tkinter` now returns BUI's tkinter

import tkinter as tk
from tkinter import ttk, messagebox

root = tk.Tk()
root.title("Hello")
name = tk.StringVar(value="Cube")
ttk.Entry(root, textvariable=name).pack(fill="x", padx=8, pady=8)
ttk.Button(root, text="Greet",
           command=lambda: messagebox.showinfo("Hi", "Hello " + name.get())).pack()
root.mainloop()
```

Or import it directly, under its own name:

```python
import bui.tksyntax as tk
from bui.tksyntax import ttk
```

<p align="center">
  <img src="docs/images/tkinter_notes.png" width="260" alt="Text with tags">
  <img src="docs/images/tkinter_canvas.png" width="260" alt="Canvas">
  <img src="docs/images/tkinter_messagebox.png" width="260" alt="messagebox">
</p>

Everything is supported, including ttk, `messagebox`, `filedialog`,
`simpledialog`, `colorchooser`, `font` and `scrolledtext`. That covers
`pack`/`grid`/`place`, scrollbars, `Canvas` (tags, `create_window`,
scroll regions), `Text` (indices, tags, marks, search, undo), `Treeview`,
bindings, variables and traces, `after`, and window-manager calls.
Full guide: **[docs/tkinter.md](docs/tkinter.md)**.

> **GPU rendering:** add `gpu=True` to `Tk(...)` (or call `bui.set_gpu(True)`,
> or run with `BUI_GPU=1`) to draw on the GPU. See [GPU renderer](#gpu-renderer).

### Qt syntax

```python
from bui.qtsyntax import (QApplication, QWidget, QVBoxLayout, QFormLayout,
                          QLineEdit, QSlider, QPushButton, QLabel)

app = QApplication([])
window = QWidget()
window.setWindowTitle("Qt syntax on BUI")
layout = QVBoxLayout(window)

form = QFormLayout()
name = QLineEdit()
name.setPlaceholderText("type here…")
level = QSlider()
level.setRange(0, 100)
form.addRow("Name:", name)
form.addRow("Level:", level)
layout.addLayout(form)

status = QLabel("Press the button.")
button = QPushButton("Click me")
button.clicked.connect(lambda: status.setText("Hello " + (name.text() or "BUI")))
layout.addWidget(status)
layout.addWidget(button)

window.show()
app.exec()
```

<p align="center"><img src="docs/images/qt.png" width="460" alt="Qt syntax on BUI"></p>

Supported classes, signals, layouts and the known gaps:
**[docs/qt.md](docs/qt.md)**.

### Blender components

```python
from bui.components import (App, Panel, NumberSlider, VectorField, EnumRow,
                            Checkbox, ColorSwatch, Button)

App("Object", (420, 520),
    Panel("Transform",
          VectorField("Location", (0.0, 1.25, 0.0)),
          NumberSlider("Scale", 1.0, min=0, max=10, on_change=print)),
    Panel("Shading",
          EnumRow("Mode", ["Solid", "Material", "Rendered"]),
          Checkbox("Shadows", True),
          ColorSwatch("Color", (0.8, 0.3, 0.1, 1.0))),
    Button("Apply", on_click=lambda: print("apply")),
).run()
```

Component reference and Blender interaction details:
**[docs/components.md](docs/components.md)**.

### Rust (gpui syntax, HTML and CSS)

```toml
[dependencies]
blenderui = { git = "https://github.com/ppmpreetham/BUI" }   # needs CMake and a C compiler
```

```rust
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
```

> **GPU rendering:** `Application::new().gpu(true)` (or `BUI_GPU=1`) draws every
> window with Direct3D 11: same pixels, about half the CPU of the default
> software raster, and less than gpui-fast. See [GPU renderer](#gpu-renderer).

<p align="center"><img src="docs/images/rust-showcase.png" width="560" alt="The Rust showcase example"></p>
<p align="center"><sub><code>bindings/rust/blenderui/examples/showcase.rs</code>: a stylesheet, grid cards with
gradients and shadows, Blender form widgets, a table, lists, <code>&lt;details&gt;</code> and <code>&lt;pre&gt;</code>.</sub></p>

Elements, the CSS that is supported, events and testing headless:
**[docs/rust.md](docs/rust.md)**.

## How it works

```
 your code:  tkinter syntax      Qt syntax       bui.components
                  │                 │                  │
                  └──────── Python: options, signals, variables, callbacks
                                    │
 C kit (kit/):  node tree ─ geometry (pack/grid/place) ─ input & interaction
                ─ Blender widget drawing ─ damage-driven raster rendering
                                    │
 window host:   native Win32 window (Windows)   │   stdlib Tk window (elsewhere)
```

- **One node tree.** Every widget wraps one kit node: `button`, `number`,
  `slider`, `text`, `textarea`, `list`, `tree`, `canvas`, `scrollbar` and so
  on. The C kit owns layout, hit testing, hover, focus, drag grabs, menus,
  tooltips, undo and animation.
- **Tk's geometry managers in C.** `pack` and `grid` are ports of Tk's own
  packer and gridder. Qt's box, grid and form layouts are mapped onto them.
- **Damage rendering.** A state change marks only its widget's rectangle.
  A frame redraws just those regions into a frame buffer (anti-aliased SDF
  shapes and a cached glyph-run text engine: GDI on Windows, stb_truetype
  elsewhere), on the CPU or, opt-in, on the GPU. Only the changed tiles are
  presented to the window.
- **Python stays thin.** Facades keep what tkinter and Qt keep in their own
  scripting layers: option values, variables, traces, signals and bindings.

More: [ARCHITECTURE.md](ARCHITECTURE.md).

## Performance

Measured on the full component demo (520×900) on a desktop CPU:

| Frame                                           | Time     |
| ----------------------------------------------- | -------- |
| Complete frame (everything redrawn)             | ~0.7 ms  |
| Hover or number-scrub frame (damage only)       | ~0.05 ms |
| Native window: input event → render → on screen | ~0.06 ms |
| Tk-hosted window, the same path                 | ~0.18 ms |

The test suite enforces a 120 Hz budget: full frames under 8.3 ms, and
interaction frames under 1 ms.

### Rust: blenderui vs gpui

The same UI built with the `blenderui` crate and with
[gpui](https://crates.io/crates/gpui) 0.2.2 (Zed's GPU-rendered framework)
and its faster fork [gpui-fast](https://github.com/longbridge/gpui-fast): a
1000×800 window of N rounded, labelled cells that all change every frame.
Ryzen 7 5800U laptop on mains power, 60 Hz display, release builds, all apps
back to back in one session
([method and full data](bindings/rust/bench/README.md)). blenderui is shown
with its opt-in GPU renderer (`.gpu(true)` / `BUI_GPU=1`) and its default
software raster.

| | blenderui (GPU) | blenderui (CPU raster) | gpui-fast | gpui |
|---|---:|---:|---:|---:|
| FPS, 3000 animated cells | **59.9** | 57.6 | 31.5 | 19.9 |
| FPS, 1000 animated cells | **60** | **60** | 49.2 | 36.4 |
| CPU per frame, 3000 cells | **11.6 ms** | 22.0 ms | 31.9 ms + GPU | 50.5 ms + GPU |
| CPU per frame, 1000 cells | **4.7 ms** | 10.2 ms | 20.5 ms + GPU | 27.7 ms + GPU |
| CPU per frame, 100 cells | **1.2 ms** | 4.6 ms | 2.7 ms + GPU | 4.7 ms + GPU |
| Uncapped FPS, 100 / 1000 / 3000 cells | **1491 / 309 / 94** | 649 / 202 / 76 | - | - |
| RAM (working set), 3000 / 1000 / 100 cells | 40 / 35 / 28 MB | **30 / 24 / 21 MB** | 65 / 45 / 47 MB | 77 / 47 / 40 MB |
| GPU memory | 19-22 MB | **none** | 19-24 MB | 16-21 MB |
| First frame after launch | **~5 ms** | **~5 ms** | ~200 ms | ~220 ms |
| Idle CPU | **0 %** | 0-0.9 % | 0-0.4 % | 0.9-1.4 % |

With the GPU renderer, blenderui beats gpui-fast on every row. It does the
least CPU work per frame at every scene size (2.2-4.4x less than gpui-fast),
holds 60 fps where both gpuis drop, and uses less RAM with the same GPU
memory. It still starts about 40x faster. The software raster remains the
default, because it needs no GPU memory or driver. gpui-fast's heavy-scene
frame rate varies between sessions on this laptop (30-60 fps at 3000 cells),
so compare within a session.

### GPU renderer

Off by default; turn it on per app or per window:

```rust
Application::new().gpu(true).run(|cx| { /* ... */ });               // every window
cx.open_window(WindowOptions { gpu: Some(true), ..Default::default() }, build); // one window
```

```python
from bui import tksyntax as tk
root = tk.Tk(gpu=True)        # or bui.set_gpu(True) for every window
```

Or set `BUI_GPU=1` for any BUI app (C, Python, Rust); `BUI_GPU=0` forces the
software raster. It is Direct3D 11 on Windows. On other platforms, or when
no device can be created, windows stay on the software raster. The design
follows Blender's instanced widget-base batches (`interface_widgets.cc`):
- Every primitive is one 128-byte instance of a single uber-shader
  ([`gpu/shaders/bui_ui.hlsl`](gpu/shaders/bui_ui.hlsl)). The primitives are
  rects, Blender widget bases with outline and emboss, circles, lines,
  shadows, gradients, HSV wheels, images and text runs.
- A frame of thousands of widgets is a few `DrawInstanced` calls.
- Text runs and images are cached in atlases.
- Frames render into a persistent target, so damage tracking still limits
  redraws to what changed. Presentation uses a flip-model swap chain with
  dirty rects.
- The device is created on a worker thread, so startup never waits for the
  driver.

The shaders evaluate the software raster's own coverage formulas, so both
renderers draw the same pixels: the `gpu_parity` test finds 10 of 172,800
pixels that differ by more than 3/255, all on hard triangle edges.

### tkinter canvas

[`bindings/python/bench/tk_canvas_bench.py`](bindings/python/bench/tk_canvas_bench.py) (2000 rectangles, ovals, lines, polygons, arcs and
labels on one tkinter-syntax `Canvas`, rendered headless):

| Per frame | before | now |
|---|---:|---:|
| Render, every item moved | 11.1 ms | **3.6 ms** (parallel raster for all item types) |
| Render, one item moved | 9.4 ms | **0.08 ms** (only the item's old and new boxes redraw) |
| Python, 2000 `Canvas.move` calls | 22.5 ms | **15.0 ms** (cached colors, O(1) item lookup) |

## Roadmap: more languages and platforms

BUI's core is plain C with a small, stable surface (`kit/include/BUI_kit.h`
for widgets, `api/bui_c_api.h` for render sinks). Its output is either a
frame buffer (`BuiRaster`, ARGB pixels plus damage rectangles) or a stream
of draw primitives. That makes every target below the same job: bind the C
API, feed in input events, and present the pixels or primitives. None of
them needs Python.

| Target | Plan | Status |
|---|---|---|
| **Python**: tkinter, Qt, components | `bui.tksyntax`, `bui.qtsyntax`, `bui.components` | ✅ Done (Qt partial) |
| **C / C++** | `BUI_kit.h`, render sinks, Qt/QPainter backend | ✅ Done |
| **Rust** | `blenderui-sys` (FFI over `BUI_kit.h`, builds the kit with CMake) and `blenderui`: gpui syntax, all HTML elements, CSS, taffy layout ([docs/rust.md](docs/rust.md)) | ✅ Done (native window on Windows; Linux/macOS render headless until their window hosts land) |
| **egui** | Two ways in: draw a BUI panel inside egui (raster to an `egui::TextureHandle`, with input forwarded), and a Blender-styled `egui::Visuals` theme | 🗺️ Planned |
| **WebAssembly** | Compile the kit with rust. The raster blits to a `<canvas>` (`putImageData` of the damage rectangles only), and DOM pointer and keyboard events go to `BK_press` / `BK_key` | 🗺️ Planned |
| **React** | `@blenderui/react` on top of the WASM build: a reconciler that maps JSX to kit nodes (`<Panel>`, `<NumberSlider>` ...) | 🗺️ Planned |
| **JavaScript / TypeScript** | `@blenderui/web` (the plain WASM API), and Node through N-API | 🗺️ Planned |
| **Tauri / Electron** | The WASM build in the webview, or the native kit through the host language | 🗺️ Planned |
| **C# / .NET** | P/Invoke over `BUI_kit.h` (WinForms / WPF / Avalonia hosts) | 🗺️ Planned |
| **Go**, **Zig**, **Swift**, **Kotlin/JVM**, **Lua** | cgo / `@cImport` / Swift C interop / Panama FFI / LuaJIT FFI over the same header | 🗺️ Planned |
| **Dear ImGui** | A backend that shows BUI panels in an ImGui window, plus a Blender-styled ImGui theme | 🗺️ Planned |
| **Game engines**: Godot, Unity, Bevy | Editor-style tool panels: GDExtension, a native Unity plugin, a Bevy plugin through the Rust crate | 🗺️ Planned |
| **Flutter** | `dart:ffi` bindings, with the frame buffer shown as a `Texture` | 🗺️ Planned |
| **Native windows on Linux / macOS** | Wayland / X11 and Cocoa hosts in C, like the Win32 one (today the stdlib Tk window hosts them) | 🗺️ Planned |
| **GPU renderers** | Direct3D 11 done (opt-in, see [GPU renderer](#gpu-renderer)); Vulkan / Metal / WebGPU backends for the same instance stream | 🟡 D3D11 done |

<details>
<summary>Intended API sketches (planned, not implemented yet)</summary>

egui:

```rust
egui::Window::new("Properties").show(ctx, |ui| {
    blenderui_egui::panel(ui, &mut state.bui_panel);   // a BUI subtree inside egui
});
```

React (WebAssembly):

```jsx
import { render } from "@blenderui/react";

export default function Properties() {
  const [size, setSize] = useState(1);
  return (
    render(<App />, {title: "Bla bla", width: 1920, height: 1080})
  );
}
```

</details>

Work order: the Rust crate is done (it also unlocks egui and Bevy); next is
the WebAssembly build (it unlocks React, JS/TS, Tauri and Electron), then
native Linux / macOS windows.

## Packaging

The project is packaged as **`blenderui`** for PyPI: `pyproject.toml` with
scikit-build-core, which drives the CMake build. One wheel contains the
`buipy` extension plus the `bui` and `blenderui` packages.

```sh
python -m pip install build
python -m build                     # dist/blenderui-<version>.tar.gz and a wheel
```

- **Version:** a single source, `bindings/python/bui/_version.py`, read by
  CMake and by the packaging.
- **CI:** `.github/workflows/wheels.yml` runs `cibuildwheel` for CPython
  3.11–3.14 on Windows (MinGW), Linux (manylinux) and macOS (arm64 and
  x86_64), plus an sdist and the full ctest run. Pushing a `v*` tag
  publishes to PyPI with trusted publishing (no tokens stored).
- **Verified locally:** a wheel built on Windows installs into a clean
  virtualenv and passes the component, syntax, render, tkinter-compat and
  input suites.
- **Release steps and one-time PyPI setup:** [docs/publishing.md](docs/publishing.md).
  Changes are recorded in [CHANGELOG.md](CHANGELOG.md).

## Documentation

| Guide                                              | What's inside                                                                            |
| -------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| [docs/getting-started.md](docs/getting-started.md) | Building, `PYTHONPATH`, environment variables, choosing the window host, troubleshooting |
| [docs/tkinter.md](docs/tkinter.md)                 | Running tkinter apps on BUI, module map, behaviour notes, the Blender look, limitations  |
| [docs/qt.md](docs/qt.md)                           | Qt-syntax guide: classes, signals, layouts, painting, timers, coverage and gaps          |
| [docs/components.md](docs/components.md)           | `bui.components` reference: layout, values, views, menus, color, editors, `App`          |
| [docs/c-api.md](docs/c-api.md)                     | The C kit (`BUI_kit.h`): nodes, properties, events, windows                              |
| [docs/rust.md](docs/rust.md)                       | The Rust crate: gpui syntax, HTML elements, CSS coverage, events, headless testing       |
| [docs/publishing.md](docs/publishing.md)           | Packaging as `blenderui`, building wheels, CI, releasing to PyPI                          |
| [CHANGELOG.md](CHANGELOG.md)                       | Release notes                                                                             |
| [SYNTAX_COVERAGE.md](SYNTAX_COVERAGE.md)           | Member-by-member coverage tables for both syntax layers                                  |
| [INTEGRATION.md](INTEGRATION.md)                   | Embedding BUI drawing in C, C++, Qt or PySide hosts through render sinks                 |
| [ARCHITECTURE.md](ARCHITECTURE.md)                 | Modules, dependency rules, what was taken from Blender and how                           |

Examples (`examples/`):

| File                           | Shows                                                           |
| ------------------------------ | --------------------------------------------------------------- |
| `tk_unchanged_app.py`          | A plain tkinter app: menus, notebook, form, table, text, canvas |
| `tk_syntax_demo.py`            | tkinter syntax laid out like a Blender properties editor        |
| `qt_syntax_demo.py`            | Qt syntax: layouts, form widgets, signals                       |
| `components_demo.py`           | Every Blender component, in a properties-editor layout          |
| `kit_hello.c`                  | The C kit from C: a slider and a button in a native window      |
| `bindings/rust/blenderui/examples/showcase.rs` | Rust: HTML, a stylesheet, flex/grid, tables, Blender form widgets |
| `python_demo.py`, `svg_demo.c` | Low-level drawing into an SVG render sink                       |

## Repository layout

| Path                  | Contents                                                                                                                                                                                                                                                                                                                                     |
| --------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `kit/`                | **The C runtime**: `kit_core.c` (nodes, input, rendering, undo, views), `kit_draw.c` (theme and the `interface_widgets.cc` port), `kit_widgets.c`, `kit_views.c` (lists, trees), `kit_menus.c`, `kit_color.c`, `kit_frame.c` (geometry managers, canvas), `kit_textarea.c`, `kit_window.c` (Win32 host). Public API: `kit/include/BUI_kit.h` |
| `gpu/`                | Drawing: software raster (`gpu_raster.c`), D3D11 GPU renderer (`gpu_d3d11.c`, `shaders/`), text (GDI / stb_truetype), render-sink dispatch                                                                                                                                                                                                                                                                            |
| `core/`, `types/`     | Blender's `blenlib` subset (C11) and data types                                                                                                                                                                                                                                                                                              |
| `ui/`, `compat/`      | Blender's original interface sources and the seams replacing Blender's application APIs                                                                                                                                                                                                                                                      |
| `api/`                | Stable C ABI for render-sink hosts (`bui_c_api.h`)                                                                                                                                                                                                                                                                                           |
| `bindings/python/`    | `buipy` extension (`src/`) and the `bui` package: `tksyntax/`, `qtsyntax.py`, `components/`, `host.py`                                                                                                                                                                                                                                       |
| `backends/qt/`        | C++ Qt/QPainter render-sink backend                                                                                                                                                                                                                                                                                                          |
| `bindings/rust/`      | Cargo workspace: `blenderui-sys` (FFI, builds the kit with CMake) and `blenderui` (gpui syntax, HTML, CSS, taffy layout)                                                                                                                                                                                                                  |
| `bindings/python/blenderui/` | The `blenderui` import alias (the PyPI name)                                                                                                                                                                                                                                                                                       |
| `pyproject.toml`, `.github/workflows/` | PyPI packaging (scikit-build-core) and the wheel and release CI                                                                                                                                                                                                                                                        |
| `examples/`, `tools/` | Demos, the icon generator, the docs screenshot generator                                                                                                                                                                                                                                                                                     |

## Testing

```sh
ctest --test-dir build --output-on-failure
```

| Suite                            | Covers                                                                             |
| -------------------------------- | ---------------------------------------------------------------------------------- |
| `core`                           | blenlib subset                                                                     |
| `buipy`, `buipy_render`          | Raster primitives, damage tracking, the native window against the frame buffer     |
| `buipy_components`               | Every component, Blender interactions, undo, multi-drag, animation, 120 Hz budgets |
| `buipy_syntax`, `buipy_advanced` | Qt and tkinter facades: layouts, painting, menus, dialogs, images                  |
| `buipy_tk_compat`                | Nine tkinter programs run on stdlib tkinter **and** BUI; results must match        |
| `buipy_tk_input`                 | tkinter widgets driven by real pointer and keyboard input through the kit          |

Rust (`cd bindings/rust && cargo test`): CSS parsing and selectors, the HTML and CSS
coverage report (`cargo test --test parity -- --nocapture`), and headless windows
checking layout positions, painted pixels, clicks, `:hover` and Blender widgets.

## License

Derived from Blender sources: **GPL-2.0-or-later** (see `LICENSE` and the SPDX
headers kept in every file).
