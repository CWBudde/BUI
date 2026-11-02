# Host integration guide

> **High-level syntax layers (recommended):** you may not need any of the
> plumbing below. `bindings/python` ships two application-facing APIs that
> render through BUI — **Qt syntax** (`bui.qtsyntax`) and **tkinter syntax**
> (`bui.tksyntax`). See "High-level syntax layers" at the bottom, and
> "Testing the Python bindings" for the one setup step they need.

BUI draws; you present. The toolkit never opens a window, creates a GL
context, or picks a UI framework. Instead, every draw call funnels through the
**GPU seam** into a render **sink** you install. This document covers the
three supported paths: plain C/C++, Qt (C++), and Python (PySide6/PyQt6 or
headless).

```
                    ┌────────────────────────┐
   your host        │   sink (BuiGpuSink)    │
 ┌────────────────┐ │  triangles()           │      renderer
 │ ui toolkit ────┼─┼─▶ lines()  ────────────┼──▶  Qt QPainter /
 │ (bui_ui)       │ │  points()              │      PySide6 QPainter /
 │ gpu seam       │ │  text()  scissor()     │      OpenGL / SVG / console
 │ (gpu_null.c)   │ │  begin/end_frame()     │      — anything
 └────────────────┘ └────────────────────────┘
```

## 1. Plain C / C++

```c
#include "bui_c_api.h"

static void my_triangles(void *user, const float (*coords)[2],
                         const float (*colors)[4], int count) {
  /* rasterize / record / forward `count` vertices */
}

BuiGpuSink sink = {0};
sink.user = my_renderer_state;
sink.triangles = my_triangles;
/* sink.lines, sink.points, sink.text, sink.scissor,
   sink.begin_frame, sink.end_frame are optional */
bui_set_render_sink(&sink);

bui_set_viewport(1280, 720);   /* pixels */
/* ... drive BUI drawing ... */
bui_set_render_sink(NULL);     /* restore the discarding backend */
```

The coordinate contract: 2D positions transformed by the current model matrix
(identity matrices → pixel coordinates), colors as RGBA floats 0..1.

A complete minimal example is `examples/svg_demo.c` — it installs an SVG
recorder and draws through the immediate-mode seam:

```sh
cmake --build build --target bui_svg_demo
./build/examples/bui_svg_demo out.svg
```

## 2. Qt (C++)

Enable the backend and link it:

```sh
cmake -S . -B build-qt -DBUI_BUILD_QT_BACKEND=ON   # needs Qt5/Qt6 Widgets
cmake --build build-qt
```

```cpp
#include "BuiWidget.h"   // backends/qt

BUI::BuiWidget *canvas = new BUI::BuiWidget(window);
canvas->render_func = [&](int w, int h) {
  // BUI draw calls; primitives land on QPainter automatically.
};
```

`BUI::BuiWidget` handles paint/resize/mouse plumbing; `BUI::QtSink` maps BUI
primitives onto `QPainter` (per-vertex colors reduce to the first vertex color
per primitive, which matches how the toolkit draws shapes). The backend does
not require moc or OpenGL.

## 3. Python

The `buipy` extension (built by default when Python development files are
found) exposes the same sink protocol to Python — no third-party packages.

### Headless / CI — SVG output

```python
import buipy
from bui.svg import SvgSink

sink = SvgSink(640, 480)
buipy.set_backend(sink)
buipy.demo_scene(640, 480)   # or your own draw calls
sink.save("frame.svg")
```

### PySide6 / PyQt6 window

```python
import sys
from PySide6.QtWidgets import QApplication   # PyQt6 works identically
import buipy
from bui.qt import BuiCanvas

app = QApplication(sys.argv)
canvas = BuiCanvas()               # default renders the demo scene
# canvas.render_func = lambda: <BUI draw calls>
canvas.resize(800, 600)
canvas.show()
sys.exit(app.exec())
```

`BuiCanvas` installs a `QtPainterBackend` for each paint pass, so the toolkit
renders through the same QPainter path as the C++ backend — pure Python, no
shiboken build.

## High-level syntax layers

Two facades speak the syntax of established toolkits on top of the C kit
(`kit/`, exposed to Python as `buipy.KitNode` / `buipy.KitContext`): widgets
are kit nodes, layouts are the kit's C geometry managers, and windows come
from `bui.host` (native Win32 on Windows, Tk elsewhere or with
`BUI_HOST=tk`):

### Qt syntax — `bui.qtsyntax`

```python
from bui.qtsyntax import QApplication, QWidget, QPushButton, QLabel, QVBoxLayout

app = QApplication([])
window = QWidget()
window.setWindowTitle("BUI — Qt syntax")
layout = QVBoxLayout()
status = QLabel("Press the button.")
button = QPushButton("Click me")
button.clicked.connect(lambda: status.setText("Clicked!"))
layout.addWidget(status)
layout.addWidget(button)
window.setLayout(layout)
window.show()
app.exec()
```

Supported surface: see [SYNTAX_COVERAGE.md](SYNTAX_COVERAGE.md) for the
exact, honest list (the daily-driver core: windows, labels, buttons,
checkboxes, box layouts, signals, a `setStyleSheet` subset). Anything not
yet implemented raises a clear error naming that file. The window host is
chosen at runtime: **PySide6/PyQt6** if installed, otherwise a stdlib
tkinter window — the same code runs either way, always rendered by BUI.

### tkinter syntax — `bui.tksyntax`

```python
from bui.tksyntax import Tk, Label, Button, Checkbutton

root = Tk()
root.title("BUI — tkinter syntax")
root.geometry("420x260")
status = Label(root, text="")
status.pack()
Button(root, text="Click me", command=lambda: status.setText("Clicked!")).pack(pady=4)
Checkbutton(root, text="Enable feature").pack()
root.mainloop()
```

Runs on the Python standard library alone. `pack(side=, padx=, pady=)`,
`place(x=, y=)`, `config(**kw)`, `invoke()` are supported; widgets render onto
a tk.Canvas **via the BUI sink**, so swapping the host renderer (Qt, SVG,
custom) changes nothing in app code.

## Testing the Python bindings

`buipy` is a **compiled extension** — it lives in the CMake build tree and is
built for one specific interpreter. `ModuleNotFoundError: No module named
'buipy'` means Python cannot find (or cannot load) that file. Three ways to
fix, fastest first:

**1. Nothing to install (auto-locate).** `import bui` finds the extension in
`build*/bindings/python` automatically. Just build and run:

```sh
cmake -S . -B build -G "MinGW Makefiles"
cmake --build build --target buipy
python examples/python_demo.py            # needs the interpreter CMake used
```

**2. Match the interpreter.** The extension only loads in the interpreter it
was built for (CMake found Python 3.14 on this machine →
`buipy.cp314-win_amd64.pyd`). Check which one CMake picked and run that one:

```sh
grep Python3_EXECUTABLE build/CMakeCache.txt
C:/Users/ppmpr/AppData/Local/Programs/Python/Python314/python.exe examples/python_demo.py
```

To target a different interpreter, point CMake at it explicitly:

```sh
cmake -S . -B build-py312 -DPython3_EXECUTABLE="C:/Path/Python312/python.exe"
cmake --build build-py312 --target buipy
```

**3. pip install (no PYTHONPATH ever again):**

```sh
cmake --build build --target buipy
python -m pip install bindings/python     # stages bui/ + the .pyd into site-packages
python -c "import bui; print(bui.version())"
```

Sanity check either way:

```sh
python -c "import bui; print(bui.version())"     # -> 0.1.0
python -m pytest bindings/python/tests           # or: ctest --test-dir build
```

## Wire format reference

| Callback | Payload |
|---|---|
| `begin_frame(w, h)` | Start of a frame batch; viewport in pixels |
| `scissor(x, y, w, h)` | Clip rect; `w`/`h` ≤ 0 clears clipping |
| `triangles(coords, colors, n)` | Non-indexed triangles, 3 vertices each |
| `lines(coords, colors, n)` | Line list, endpoints in pairs |
| `points(coords, colors, n, size)` | Points at `size` pixels |
| `text(x, y, text, size, rgba)` | UTF-8 text run at baseline |
| `end_frame()` | End of frame batch |

All coordinates are 2D floats; colors RGBA floats 0..1. Callbacks may be
invoked with or without the GIL held — the Python bindings handle this
internally.
