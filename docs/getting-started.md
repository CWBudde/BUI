# Getting started

## 1. Build

Requirements:

- CMake ≥ 3.20
- A C11 / C++17 compiler: GCC (MinGW-w64 on Windows), Clang or MSVC
- Python 3 with development headers, for the Python APIs (`buipy`)

```sh
cmake -S . -B build
cmake --build build
ctest --test-dir build --output-on-failure
```

On Windows with MSYS2 / MinGW-w64:

```sh
cmake -S . -B build -G "MinGW Makefiles"
cmake --build build
```

### Build options

| Option | Default | Effect |
|---|---|---|
| `BUI_BUILD_PYTHON_BINDINGS` | `ON` | Build the `buipy` CPython extension. It is skipped with a message if the Python dev files are missing. |
| `BUI_BUILD_TESTS` | `ON` | C test executables and the ctest suites |
| `BUI_BUILD_EXAMPLES` | `OFF` | C examples (`bui_svg_demo`) |
| `BUI_BUILD_UI` | `OFF` | Compile Blender's original `ui/` sources as well (not needed for the kit) |
| `BUI_BUILD_QT_BACKEND` | `OFF` | The C++ Qt/QPainter render-sink backend (needs Qt 5/6 Widgets) |
| `BUI_WARNINGS_AS_ERRORS` | `OFF` | `-Werror` / `/WX` |
| `BUI_TEXT_GDI` | `OFF` | Use Windows' GDI text engine and system fonts instead of the portable engine (embedded Inter, Slug on the GPU) |

## 2. Use it from Python

The pure-Python package is `bindings/python/bui`. Put that directory on the
path:

```sh
export PYTHONPATH=/path/to/BUI/bindings/python    # Windows: set PYTHONPATH=C:\path\to\BUI\bindings\python
```

`import bui` (and anything under it) finds the compiled `buipy` extension
in your CMake build directory by itself (`build*/bindings/python/`). If you
keep the build somewhere else, point to it:

```sh
export BUIPY_BUILD_DIR=/path/to/build/bindings/python
```

Check the setup:

```sh
python -c "import bui, buipy; print(buipy.kit_window_supported(), len(buipy.kit_icon_names()))"
python examples/components_demo.py
python -m bui.tksyntax examples/tk_unchanged_app.py
python examples/qt_syntax_demo.py
```

## 3. Pick an API

| You have / want | Use | Guide |
|---|---|---|
| An existing tkinter app | `python -m bui.tksyntax app.py`, or `bui.tksyntax.install()` | [tkinter.md](tkinter.md) |
| New code, tkinter style | `import bui.tksyntax as tk` | [tkinter.md](tkinter.md) |
| Qt-style code | `from bui.qtsyntax import ...` | [qt.md](qt.md) |
| A Blender-like tool panel | `from bui.components import ...` | [components.md](components.md) |
| BUI drawing inside your own renderer | Render sinks | [../INTEGRATION.md](../INTEGRATION.md) |
| C only | `kit/include/BUI_kit.h` | [c-api.md](c-api.md) |

You can mix them in one process. Windows, timers and the event loop are
shared (`bui.host`).

## Environment variables

| Variable | Effect |
|---|---|
| `BUIPY_BUILD_DIR` | Directory containing the built `buipy` extension |
| `BUI_HOST=tk` | Force the stdlib-Tk window host even where the native host exists (Windows) |
| `BUI_UI_SCALE` | Fix the UI scale (for example `1.5`) instead of following the monitor's DPI |
| `BUI_GPU=1` | Draw native windows on the GPU (Direct3D 11). `BUI_GPU=0` forces the software raster. Overrides `bui.set_gpu()` / `.gpu(true)` |
| `BUI_GPU_DEBUG=1` | Print why the GPU renderer could not start |
| `BUI_FONT`, `BUI_FONT_DIR` | A font file to replace the embedded Inter UI font, and an extra font folder |
| `BUI_SLUG` | `0` draws GPU text from a per-glyph bitmap atlas instead of glyph outlines (Slug) |
| `BUI_PROFILE=1` | Print per-phase frame times (raster and present) |

## Window hosts

`bui.host.Window` puts a kit context in a top-level window.

- **Native (Windows).** A Win32 window written in C. Input goes straight
  into the kit, and frames are presented by blitting only the changed tiles.
  It follows per-monitor DPI and syncs animations to vsync.
- **Tk (everywhere else, or with `BUI_HOST=tk`).** A stdlib `tkinter`
  window that shows the kit's frame buffer and forwards input to it.
  The UI and the API are identical, but presenting is a little slower.

### GPU rendering

Native windows draw on the CPU by default. To draw them on the GPU
(Direct3D 11) instead, use one of these:

| Where | Flag |
|---|---|
| tkinter syntax | `tk.Tk(gpu=True)` |
| Any Python API | `bui.set_gpu(True)` before creating windows |
| Rust | `Application::new().gpu(true)` or `WindowOptions { gpu: Some(true), .. }` |
| C | `BUI_gpu_set_default(1)` or `BK_window_set_gpu(win, 1)` |
| Any app, no code change | `BUI_GPU=1` |

Both renderers draw the same pixels. The GPU uses far less CPU per frame
(about 2-4x less in the
[benchmark](../bindings/rust/bench/README.md)), but the process needs about
10 MB more RAM plus GPU memory. The device starts in the background, so the
first frames are drawn on the CPU and the switch happens a few frames in.
Without a GPU, or on Linux/macOS for now, windows stay on the software
raster.

`bui.host.run()` runs one loop for all windows, `bui.host.update()` processes
pending events once, and `bui.host.quit()` stops the loop.

## Troubleshooting

| Symptom | Fix |
|---|---|
| `ImportError: buipy` | Build first (`cmake --build build`) or set `BUIPY_BUILD_DIR`. The extension must match your Python version: a `cp314` build needs Python 3.14. |
| Window doesn't appear | Call `mainloop()` / `app.exec()` / `App.run()`. Without a loop, windows only show during `update()`. |
| Blurry or tiny UI | Set `BUI_UI_SCALE`. On Windows the native host is DPI-aware by default. |
| `gpu=True` has no effect | Run with `BUI_GPU_DEBUG=1` to see why the device failed. Windows without Direct3D 11 stay on the software raster. |
| Monospace/serif text uses the UI font | Install a TrueType font family (DejaVu, Liberation or Noto). The UI font itself (Inter) is embedded. |
| A tkinter feature behaves differently | See the behaviour notes in [tkinter.md](tkinter.md#behaviour). If stdlib tkinter does something else, that is a bug; `tests/test_tk_compat.py` shows how to write a side-by-side check. |
