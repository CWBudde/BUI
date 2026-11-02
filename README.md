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

All three share one C runtime (`kit/`). Python builds the widget tree and
runs your callbacks. Layout, input, drawing and presenting stay in C.

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
- [How it works](#how-it-works)
- [Performance](#performance)
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
| Packaging (`pip install`)            | ❌ Not yet            | Build from source with CMake (below)                                                                                                                                               |

## Install and build

Requirements: **CMake ≥ 3.20**, a **C11/C++17 compiler** (GCC / MinGW, Clang
or MSVC), and **Python 3** with development headers (for the Python APIs).

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
  shapes and a cached native text engine). Only the changed tiles are
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

## Documentation

| Guide                                              | What's inside                                                                            |
| -------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| [docs/getting-started.md](docs/getting-started.md) | Building, `PYTHONPATH`, environment variables, choosing the window host, troubleshooting |
| [docs/tkinter.md](docs/tkinter.md)                 | Running tkinter apps on BUI, module map, behaviour notes, the Blender look, limitations  |
| [docs/qt.md](docs/qt.md)                           | Qt-syntax guide: classes, signals, layouts, painting, timers, coverage and gaps          |
| [docs/components.md](docs/components.md)           | `bui.components` reference: layout, values, views, menus, color, editors, `App`          |
| [docs/c-api.md](docs/c-api.md)                     | The C kit (`BUI_kit.h`): nodes, properties, events, windows                              |
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
| `python_demo.py`, `svg_demo.c` | Low-level drawing into an SVG render sink                       |

## Repository layout

| Path                  | Contents                                                                                                                                                                                                                                                                                                                                     |
| --------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `kit/`                | **The C runtime**: `kit_core.c` (nodes, input, rendering, undo, views), `kit_draw.c` (theme and the `interface_widgets.cc` port), `kit_widgets.c`, `kit_views.c` (lists, trees), `kit_menus.c`, `kit_color.c`, `kit_frame.c` (geometry managers, canvas), `kit_textarea.c`, `kit_window.c` (Win32 host). Public API: `kit/include/BUI_kit.h` |
| `gpu/`                | Drawing: software raster (`gpu_raster.c`), render-sink dispatch                                                                                                                                                                                                                                                                              |
| `core/`, `types/`     | Blender's `blenlib` subset (C11) and data types                                                                                                                                                                                                                                                                                              |
| `ui/`, `compat/`      | Blender's original interface sources and the seams replacing Blender's application APIs                                                                                                                                                                                                                                                      |
| `api/`                | Stable C ABI for render-sink hosts (`bui_c_api.h`)                                                                                                                                                                                                                                                                                           |
| `bindings/python/`    | `buipy` extension (`src/`) and the `bui` package: `tksyntax/`, `qtsyntax.py`, `components/`, `host.py`                                                                                                                                                                                                                                       |
| `backends/qt/`        | C++ Qt/QPainter render-sink backend                                                                                                                                                                                                                                                                                                          |
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

## License

Derived from Blender sources: **GPL-2.0-or-later** (see `LICENSE` and the SPDX
headers kept in every file).
