# Getting started

## 1. Build

Requirements:

- CMake ≥ 3.20
- A C11 / C++20 compiler: GCC (MinGW-w64 on Windows), Clang or MSVC
- Python 3.11+ with development headers, for the Python APIs (`buipy`)
- On Linux, the X11 headers (`libx11-dev` / `libX11-devel`) for native windows. No GL headers are needed.

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
| `BUI_BUILD_EXAMPLES` | `OFF` | C examples (`bui_kit_hello`, `bui_svg_demo`, the site screenshot programs) |
| `BUI_BUILD_UI` | `OFF` | Compile Blender's original `ui/` sources as well (not needed for the kit) |
| `BUI_BUILD_QT_BACKEND` | `OFF` | The C++ Qt/QPainter render-sink backend (needs Qt 5/6 Widgets) |
| `BUI_BUILD_IMGUI` | `ON` when a C++ compiler is present | Dear ImGui on BUI (`bui::imgui`), its C API (`bui_cimgui`), demo and test |
| `BUI_IMGUI_BLENDER_WIDGETS` | `OFF` | Dear ImGui's widgets become Blender's (an edited `imgui_widgets.cpp`) |
| `BUI_WARNINGS_AS_ERRORS` | `OFF` | Only affects GCC and Clang today: when `OFF` the build adds `-Wno-error`; when `ON` it doesn't. It never adds `-Werror` or `/WX` by itself. |
| `BUI_SLUG` | `ON` | GPU text from glyph outlines (Slug) by default; `OFF` makes the glyph atlas the default (switchable at run time either way) |
| `BUI_IMAGE_SVG` | `ON` | SVG images through the kit's nanosvg (no extra code: Blender's icons use it already) |
| `BUI_TEXT_FREETYPE` | `ON` | Rasterize glyphs with the vendored FreeType 2.13.3, the way Blender's BLF does (light hinting, gamma 1.43, Inter's optical size). `OFF` uses stb_truetype. Ignored when `BUI_TEXT_GDI` is on. |
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
| `BUI_HOST=tk` | Force the stdlib-Tk window host even where the native host exists |
| `BUI_UI_SCALE` | Fix the UI scale (for example `1.5`) instead of following the monitor's DPI |
| `BUI_GPU=1` | Draw native windows on the GPU (Direct3D 11 on Windows, OpenGL 3.3 on Linux). Overrides the app-wide defaults `bui.set_gpu()`, `Application::gpu()` and `BUI_gpu_set_default()`. `BUI_GPU=0` turns those defaults off, but a window that asks for the GPU itself (`Tk(gpu=True)`, `WindowOptions { gpu: Some(true) }`, `BK_window_set_gpu`) still gets it. |
| `BUI_GPU_DEBUG=1` | Print why the GPU renderer could not start |
| `BUI_FONT`, `BUI_FONT_DIR` | A font file to replace the embedded Inter UI font, and an extra font folder |
| `BUI_SLUG` | `0` draws GPU text from a per-glyph bitmap atlas instead of glyph outlines (Slug); `1` forces Slug. Overrides `bui.set_slug()` / `.slug()` / `BUI_gpu_set_slug()` |
| `BUI_PROFILE` | Print frame times every 60 frames (per phase in Rust and JavaScript; raster and present in the Windows host). **Any** value turns it on, even `0`. |
| `BUI_THEME` | `dark` (default) or `light`: Blender's Dark or Light theme preset, when the app doesn't call `bui.set_theme()` / `blenderui::set_theme()` / `BK_set_theme()` |
| `BUI_FILEDIALOG=native` (or `system`, `tk`) | File dialogs (`tkinter.filedialog`, `QFileDialog`) use the operating system's dialogs instead of Blender's File Browser |
| `BUI_BOOKMARKS` | The file browser's bookmarks file (default `$XDG_CONFIG_HOME/bui/bookmarks.txt`, or `%APPDATA%\bui\bookmarks.txt` on Windows) |

## Window hosts

`bui.host.Window` puts a kit context in a top-level window.

- **Native (Windows and Linux).** A window written in C: BUI's own Win32 host on Windows,
  X11 on Linux (and an experimental native Wayland host; X11 also runs through XWayland). Input goes
  straight into the kit, and frames are presented by blitting only the
  changed tiles. It follows the display's DPI (per-monitor on Windows,
  `Xft.dpi` on X11) and paces animations to the display's refresh rate.
- **Native (macOS).** A Cocoa window (a port of Blender's GHOST
  code; checked by CI and tried by hand on three Macs (two Apple Silicon, one Intel)) with Retina scaling. It needs the main
  thread. See [macos-testing.md](macos-testing.md).
- **Tk (with `BUI_HOST=tk`, or where no native host works).** A stdlib `tkinter` window that
  shows the kit's frame buffer and forwards input to it. The UI and the API
  are identical, but presenting is a little slower.

### GPU rendering

Native windows draw on the CPU by default. To draw them on the GPU
(Direct3D 11 on Windows, OpenGL 3.3 on Linux) instead, use one of these:

| Where | Flag |
|---|---|
| tkinter syntax | `tk.Tk(gpu=True)` |
| Any Python API | `bui.set_gpu(True)` before creating windows |
| Rust | `Application::new().gpu(true)` or `WindowOptions { gpu: Some(true), .. }` |
| C | `BUI_gpu_set_default(1)` or `BK_window_set_gpu(win, 1)` |
| Any app, no code change | `BUI_GPU=1` |

GPU text is drawn with Slug (from glyph outlines, sharp at any size) or from
a per-glyph bitmap atlas like Blender's (about half the GPU work):

| Where | Slug (default) / atlas |
|---|---|
| Python | `bui.set_slug(True)` / `bui.set_slug(False)` |
| Rust | `Application::new().slug(..)`, `blenderui::set_slug(..)`, or build without the default `slug` feature |
| C | `BUI_gpu_set_slug(1 / 0)`, or CMake `-DBUI_SLUG=OFF` for the atlas by default |
| Any app | `BUI_SLUG=1` / `BUI_SLUG=0` |

Both renderers draw the same pixels. The GPU uses far less CPU per frame
(about 2-4x less in the
[benchmark](../bindings/rust/bench/README.md)), but the process needs about
10 MB more RAM plus GPU memory. The device starts in the background, so the
first frames are drawn on the CPU and the switch happens a few frames in.
The GPU renderer is Direct3D 11 on Windows and OpenGL 3.3 on Linux (libGL
is loaded at run time). Without a GPU or OpenGL, and on macOS, windows stay
on the software raster.

`bui.host.run()` runs one loop for all windows, `bui.host.update()` processes
pending events once, and `bui.host.quit()` stops the loop.

## Troubleshooting

| Symptom | Fix |
|---|---|
| `ImportError: buipy` | Build first (`cmake --build build`) or set `BUIPY_BUILD_DIR`. The extension must match your Python version: a `cp314` build needs Python 3.14. |
| Window doesn't appear | Call `mainloop()` / `app.exec()` / `App.run()`. Without a loop, windows only show during `update()`. |
| Blurry or tiny UI | Set `BUI_UI_SCALE`. On Windows the native host is DPI-aware by default. |
| `gpu=True` has no effect | Run with `BUI_GPU_DEBUG=1` to see why the device failed. Machines without Direct3D 11 (Windows) or OpenGL 3.3 (Linux) stay on the software raster. |
| Monospace/serif text uses the UI font | Install a TrueType font family (DejaVu, Liberation or Noto). The UI font itself (Inter) is embedded. |
| A tkinter feature behaves differently | See the behaviour notes in [tkinter.md](tkinter.md#behaviour). If stdlib tkinter does something else, that is a bug; `bindings/python/tests/test_tk_compat.py` shows how to write a side-by-side check. |
