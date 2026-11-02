# BUI

> [!NOTE]
> This repository is built entirely using LLMs. No Human written code.

A modular UI toolkit extracted from the Blender project. Core in C, widgets in
C++, no Blender application logic — just the interface machinery. Renderable
from C++, **Qt**, and **Python (PySide6 / PyQt6)** through one stable
integration point.

See [ARCHITECTURE.md](ARCHITECTURE.md) for the module layout and dependency
rules, and [INTEGRATION.md](INTEGRATION.md) for host-application guides.

## Quick start

```sh
cmake -S . -B build
cmake --build build
ctest --test-dir build        # core + Python binding tests
```

With the full widget toolkit and examples:

```sh
cmake -S . -B build-ui -DBUI_BUILD_UI=ON -DBUI_BUILD_EXAMPLES=ON
cmake --build build-ui
cmake --build build-ui --target bui_svg_demo
./build-ui/examples/bui_svg_demo demo.svg    # renders a showcase scene to SVG
```

## Modules

- `core/` – pure C11 foundation (containers, memory, math, strings)
- `types/` – shared data-type shapes (DNA structs, verbatim)
- `gpu/` – drawing abstraction: null backend + primitive **sink dispatch**
- `compat/` – seam headers replacing Blender-app APIs (RNA/WM/ED/BLF/IMB/CTX)
- `ui/` – the widget toolkit sources extracted from Blender (blocks/buttons, layout, handlers, views)
- `kit/` – **Blender's UI as a C runtime**: nodes, layout, input, widget drawing ported from `interface_widgets.cc`, damage-driven rendering
- `api/` – stable C ABI (`bui_c_api.h`) for hosts and bindings
- `bindings/python/` – `buipy` CPython extension + `bui` package (SVG sink, Qt canvas)
- `backends/qt/` – C++ Qt/QPainter backend (`BuiWidget`, `BUI::QtSink`)

## Rendering from any stack

BUI never opens a window itself. Hosts install a **render sink** — a table of
function pointers (`BuiGpuSink`) — and receive every primitive the toolkit
draws (triangles, lines, points, text, scissor). One API, any renderer:

| Host stack             | What to use                                             |
| ---------------------- | ------------------------------------------------------- |
| C / C++ (any renderer) | `bui_set_render_sink()` from `api/bui_c_api.h`          |
| Qt (C++)               | `BUI::QtSink` + `BUI::BuiWidget` in `backends/qt/`      |
| Python (PySide6/PyQt6) | `bui.qt.BuiCanvas` from `bindings/python/bui/qt.py`     |
| Python (headless/CI)   | `bui.svg.SvgSink` renders frames to SVG                 |
| Any (no GPU)           | `buipy.Raster` / `GPU_raster.h` — native frame renderer |

### The fast path: native raster + damage-only present

The tkinter- and Qt-syntax layers render the way Blender does, adapted to a
CPU host: the whole UI is drawn into **one frame buffer in C**
(`gpu/intern/gpu_raster.c` — anti-aliased SDF boxes/circles/ellipses/lines,
polygons, and a cached native text engine), then presented with **one blit of
just the tiles that changed** (direct GDI blit on Windows, dirty-rect
`PhotoImage.put` elsewhere, a single `drawImage` under Qt). Redraw requests
are coalesced into one frame per idle, and hover with no handler costs
nothing. A 60-widget window renders a changed frame in ~1.5 ms.

```python
import bui                      # locates the built buipy extension
import buipy
r = buipy.Raster(640, 480)
buipy.set_backend(r)            # draw_* calls now rasterize natively
buipy.begin_frame(640, 480)
buipy.draw_roundrect(20, 20, 120, 32, 6, (0.2, 0.4, 0.8, 1.0))
buipy.draw_text(32, 41, 13, "Hello", (1, 1, 1, 1))
buipy.end_frame()
r.damage()                      # [(x, y, w, h), ...] changed since last frame
r.present_native(hwnd)          # Windows; or r.ppm(...) / r.rgb32()
```

## Blender component library — `bui.components` (runs in C)

Blender's UI as composable components: Blender's look _and_ Blender's
interactions. Every component is a node of the C kit (`kit/`); Python only
builds the tree and receives callbacks.

- **Blender's code**: the widget drawing is a C port of
  `interface_widgets.cc` — `round_box_edges`, `widgetbase_draw`, the shape
  tables (number arrows, checkmark, menu chevron), `widget_state` /
  `widget_active_color` (Blender's HSL hover tint), `widget_numbut`,
  `widget_numslider`, `widget_optionbut`, `widget_menu_back` …, rendered by a
  software implementation of the `GPU_SHADER_2D_WIDGET_BASE` parameters.
  The HSV wheel uses `hsvcircle_vals_from_pos` / `hsv_cursor`; color math,
  rects and the UTF-8 word-jump text cursor are blenlib (`core/`), linked.
- **Fast**: rendering is damage driven — state changes mark only the
  widget's rect, a frame redraws and diffs just those regions, and an idle
  UI draws nothing. Measured on the full demo (520×900): **~0.7 ms** for a
  complete frame, **~0.05 ms** for a hover/scrub frame, **~0.18 ms**
  including the Tk event and the window blit — far inside a 120 Hz (8.3 ms)
  budget, i.e. thousands of frames per second.

```python
from bui.components import App, Panel, NumberSlider, EnumRow, Checkbox, Button

App("Demo", (420, 600),
    Panel("Object",
          NumberSlider("Size", 1.0, min=0, max=10, on_change=print,
                       description="Object size"),
          EnumRow("Mode", ["Object", "Edit"]),
          Checkbox("Smooth", True),
          Button("Apply", on_click=lambda: print("apply"))),
).run()
```

|           |                                                                                                                                                                                                                      |
| --------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Layout    | `Column`, `Row` (`align=True` joins; `weight=0` keeps natural width), `Split`, `Grid`, `Box`, `Panel` (sub-panels, header checkbox, icon), `Tabs`, `SideTabs` (properties-editor icon tabs), `ScrollView`            |
| Values    | `NumberField`, `NumberSlider`, `VectorField`, `TextField`, `SearchField`, `HotkeyField`, `Checkbox`, `Toggle` (icon toggles), `Decorator`, `Dropdown` (icons), `EnumRow` (icons, vertical, icon-only), `ColorSwatch` |
| Buttons   | `Button` (icon, `emboss=False`), `IconButton`, `Popover`, `MenuButton`                                                                                                                                               |
| Views     | `ListView` (UI list), `TreeView` + `TreeItem` (outliner, toggle columns), `GridView` (preview tiles)                                                                                                                 |
| Menus     | `Menu` / `MenuItem` (icons, shortcuts, checkboxes, submenus, type-to-search), `MenuBar`, `PieMenu`, right-click context menus                                                                                        |
| Color     | `ColorSwatch` picker: HSV wheel + value strip or SV square + hue strip, RGB/HSV/HSL sliders, hex, eyedropper                                                                                                         |
| Editors   | `ColorRamp`, `CurveMapping` (channels), `NormalBall`, `Histogram`                                                                                                                                                    |
| Templates | `IDSelector` (`template_ID`), `PathField`                                                                                                                                                                            |
| Other     | `Label` (icon), `ProgressBar`, `Separator`, `Spacer`, tooltips, 120 vector icon names from Blender (`buipy.kit_icon_names()`)                                                                                        |

Interactions, as in Blender:

- **Numbers** — the cursor turns into Blender's ↔ arrows over number
  fields; dragging hides it with a continuous grab (it never hits the
  window edge and returns where it started). Shift = precise, Ctrl = snap,
  Alt = edit X/Y/Z together. Click to type (`2*pi` works).
- **Any value** — Ctrl+Wheel steps, Ctrl+C / Ctrl+V copy/paste, Backspace
  resets, Minus negates, right-click for Copy/Paste/Reset; Ctrl+Alt+C/V
  copies a whole vector.
- **Toggles & panels** — drag across checkboxes or panel headers to set them
  all; Ctrl+click a header to solo it; drag a panel's grip to reorder.
- **Menus** — press-drag-release picks; hover switches between menu-bar
  menus and opens submenus; typing searches every item; `PieMenu.show()` on
  a hotkey (`app.bind("q", ...)`).
- **Views** — double-click / F2 renames, arrows navigate, Left/Right
  collapse/expand trees, drag down outliner toggle columns.
- **Color** — wheel with red at the bottom and Blender's cursor; the
  eyedropper samples any pixel of the window.

See `examples/components_demo.py` (every component, properties-editor style).

C kit (`kit/intern/`): `kit_core.c` (nodes, properties, input context,
damage rendering, text editing, number expressions), `kit_draw.c` (theme +
the `interface_widgets.cc` port, text, icons), `kit_widgets.c` (layouts and
widgets), `kit_views.c`, `kit_menus.c`, `kit_color.c` (picker, eyedropper,
ramp, curves, normal, histogram). Icons are generated into
`kit_icons_data.c` by `tools/gen_kit_icons.py`. Python wrappers:
`bui/components/` (`core`, `widgets`, `containers`, `views`, `menus`,
`editors`, `templates`, `app`); the C API is `kit/include/BUI_kit.h`.

## High-level syntax layers — write Qt or tkinter, render BUI

BUI ships two Python facades that speak the syntax of established toolkits
while BUI does all the rendering:

```python
# Qt syntax  (examples/qt_syntax_demo.py)
from bui.qtsyntax import QApplication, QWidget, QPushButton, QLabel, QVBoxLayout
app = QApplication([])
window = QWidget()
layout = QVBoxLayout()
status = QLabel("Press the button.")
button = QPushButton("Click me")
button.clicked.connect(lambda: status.setText("Clicked!"))
layout.addWidget(status); layout.addWidget(button)
window.setLayout(layout); window.show()
app.exec()

# tkinter syntax  (examples/tk_syntax_demo.py) - the full tkinter API, incl.
# ttk, messagebox, filedialog, simpledialog, font, scrolledtext
from bui.tksyntax import Tk, Label, Button
root = Tk(); root.geometry("420x260")
Label(root, text="Hello BUI").pack()
Button(root, text="Click me", command=on_click).pack()
root.mainloop()
```

Existing tkinter programs run unchanged, Blender-styled:
`python -m bui.tksyntax app.py` (try `examples/tk_unchanged_app.py`), or call
`bui.tksyntax.install()` before `import tkinter`.

No Qt or Tk widgets are involved: both facades are thin Python over the C kit
(`kit/`). Every widget is a kit node, `pack`/`grid`/`place` and Qt's
box/grid/form layouts run on the kit's C geometry managers (a port of Tk's
packer and gridder), `Canvas` and `QPainter` record into a kit display list,
and windows are native Win32 windows (`kit/intern/kit_window.c`) — input,
layout, rendering and presenting never leave C; Python only runs your
callbacks. Elsewhere (or with `BUI_HOST=tk`) a stdlib Tk window hosts the
frame buffer. See [SYNTAX_COVERAGE.md](SYNTAX_COVERAGE.md).

Blender's interaction polish comes for free in all three APIs: Ctrl+Z /
Ctrl+Shift+Z undo of UI edits, Tab / Shift+Tab between fields, vertical
multi-drag over number fields, animated panel collapse.

## Low-level Python quick start

```python
import buipy
from bui.svg import SvgSink

sink = SvgSink(640, 480)
buipy.set_backend(sink)
buipy.demo_scene(640, 480)          # drives the toolkit's draw seam
sink.save("demo.svg")               # or render into QPainter via bui.qt.BuiCanvas
```

See [INTEGRATION.md](INTEGRATION.md) for installing/testing `buipy`.

## Qt quick start (C++)

```cmake
set(BUI_BUILD_QT_BACKEND ON)        # requires Qt5/Qt6 Widgets
add_subdirectory(BUI)
target_link_libraries(myapp PRIVATE bui::qt)
```

```cpp
BUI::BuiWidget canvas(parent);
canvas.render_func = [&](int w, int h) { /* BUI draw calls */ };
```

The pure-Python path needs no C++ build at all — see
`bindings/python/bui/qt.py`.

## License

Derived from Blender sources: GPL-2.0-or-later (see upstream `COPYING` and the
SPDX headers preserved in every file).
