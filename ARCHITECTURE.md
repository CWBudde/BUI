# BUI Architecture

BUI is a decoupled UI toolkit extracted from the Blender project
(`source/blender/editors/interface` and its support layers). It contains **only**
reusable interface machinery — no Blender application logic (no scene graph,
depsgraph, nodes, operators-on-datablocks, file loading, Python).

```
┌──────────────────────────────────────────────────────────────────┐
│  ui/      widget toolkit: blocks/buttons, layout engine,         │
│           widget drawing, handlers, views, view2d                │
└──────┬───────────────┬──────────────┬─────────────┬──────────────┘
       │               │              │             │
┌──────▼──────┐ ┌──────▼──────┐ ┌─────▼──────────┐ ┌▼──────────────────┐
│ types/      │ │ gpu/        │ │ compat/        │ │ api/  stable C    │
│ DNA data    │ │ draw seam   │ │ RNA / CTX / WM │ │ ABI for hosts     │
│ shapes      │ │ (null back- │ │ / ED / BLF /   │ │ & bindings        │
│ (verbatim)  │ │  end + sink │ │ IMB seams      │ └───┬─────────┬─────┘
│             │ │  dispatch)  │ │ (stubs & hooks)│     │         │
└──────┬──────┘ └──────┬──────┘ └─────┬──────────┘ ┌───▼───┐ ┌───▼──────┐
       │               │              │            │buipy  │ │bui_qt    │
       │               │              │            │Python │ │QPainter  │
       │               │              │            │binding│ │backend   │
┌──────▼───────────────▼──────────────▼──────────┐└───────┘ └──────────┘
│  core/   pure C11 foundation extracted from blenlib            │
│          listbase · ghash · mempool · memarena · dynstr        │
│          math (vec/mat/rot/color/geom) · strings · rct         │
│          guarded-allocator shim · threads stubs                │
└────────────────────────────────────────────────────────────────┘
```

Dependency rule: arrows point downward only. `ui/` never includes Blender
application code; everything it needs arrives through `types/`, `gpu/`, or
`compat/`.

## Modules

| Module | Contents | Status |
|---|---|---|
| `core/` | C11 subset of blenlib, de-C++ified with mechanical transforms (namespaces stripped, casts normalized, MEM API mapped). Inline math lives in `include/intern/*.c` included by headers (`MINLINE` = `static inline` in C). | **Builds + tested** |
| `types/` | Verbatim `DNA_*_types.h` struct definitions. `DNA_listBase.h` aliases `blender::{Link,LinkData,ListBase}` onto the core's C structs so DNA structs and the C listbase API share one type. | **Builds** |
| `gpu/` | The draw surface used by the toolkit (`GPU_*`, `imm*`) behind seam headers + a null backend that *records and forwards*: with a sink installed it emits screen-space primitives (`GPU_backend_sink.h`); without one it discards them. Theme colors reach the GPU layer through function-pointer hooks. | **Builds + tested** |
| `compat/` | Seam headers carrying upstream API names (`RNA_access.hh`, `BLF_api.hh`, …) with BUI-owned opaque types and stub implementations (`intern/bui_stubs.cc`). | **Builds** |
| `ui/` | The toolkit sources from `editors/interface`: interface.cc family, widgets, handlers, regions, view2d, views, textbox. Option-gated (`-DBUI_BUILD_UI=ON`). | **Compiles + links** |
| `kit/` | Blender's UI as a retained-mode C runtime: node tree, uiLayout-style layout, `interface_handlers.cc` interaction rules, widget drawing ported from `interface_widgets.cc` (widget-base shader parameters rendered by `gpu/` raster), menus, color picker, views, editors; Tk-style pack/grid/place geometry managers, canvas display list, multi-line text area; UI undo/redo, Tab field traversal, vertical multi-drag, panel animation; damage-driven rendering; native window hosts for Win32, X11 and Cocoa (`kit_window_win32.c`, `kit_window_x11.c`, `kit_window_cocoa.m`; `kit_window_null.c` elsewhere). C API: `kit/include/BUI_kit.h`. | **Builds + tested** |
| `api/` | `bui_c_api.h` — small stable C ABI: version info + sink/viewport plumbing for hosts and bindings. | **Builds + tested** |
| `bindings/python/` | `buipy` CPython extension (no third-party deps) + `bui` package: `bui.components` (Blender component library), `bui.qtsyntax` / `bui.tksyntax` (Qt / tkinter APIs as thin facades over the kit), `bui.host` (native or Tk windows), `SvgSink` headless renderer, `bui.qt.BuiCanvas` PySide6/PyQt6 widget. | **Builds + tested** |
| `bindings/imgui/` | Dear ImGui (vendored, 1.93.1 WIP) on BUI: `imgui_impl_bui` platform + renderer backend hosted in a kit `custom` node (input, cursors, clipboard, DPI, damage-driven frames) drawing through `BUI_raster_mesh` and BUI textures; Blender style and Inter font. Gate: `-DBUI_BUILD_IMGUI` (ON with C++). | **Builds + tested** |
| `backends/qt/` | C++ Qt backend: `BUI::QtSink` (BuiGpuSink → QPainter) + `BUI::BuiWidget` (QWidget host, no moc needed). Gate: `-DBUI_BUILD_QT_BACKEND=ON`. | **Shipped** (builds where Qt exists) |
| `examples/` | `bui_svg_demo.c` (C, sink → SVG), `python_demo.py` (buipy → SVG). | **Builds + run** |

## Render-sink dispatch

The null backend is the integration seam. `gpu/include/GPU_backend_sink.h`
defines `BuiGpuSink` — a function-pointer table (`triangles`, `lines`,
`points`, `text`, `scissor`, `begin_frame`, `end_frame`). The null backend:

- records immediate-mode vertices (`immBegin/immAttr*/immVertex*/immEnd`),
- decodes vertex batches (pos/color attributes, index buffers),
- applies the model matrix and forwards screen-space primitives to the sink,
- or, with no sink installed, discards everything (pure null).

### Raster and GPU renderer

The kit itself draws through `GPU_raster.h`:
- **Software raster** (`gpu/intern/gpu_raster.c`): the default. It renders
  anti-aliased SDF primitives into a CPU frame buffer, in parallel bands.
  Text is a glyph-run cache over stb_truetype, with Inter (Blender's UI
  font) embedded as the default face (`gpu/extern/font_inter.h`).
  `BUI_TEXT_GDI=ON` switches Windows to GDI and system fonts.
  The GPU renderer draws text one quad per glyph. Slug (`K_SLUG` in
  `bui_ui.hlsl`) is the default: glyph outlines are uploaded once as
  quadratic curves with horizontal and vertical bands, and the pixel shader
  integrates exact coverage. With `BUI_SLUG=0`, it uses glyph bitmaps from
  the shared glyph cache in the mask atlas instead.
- **GPU renderer**: Direct3D 11 on Windows (`gpu/intern/gpu_d3d11.c`,
  `gpu/shaders/bui_ui.hlsl`) and OpenGL 3.3 core on Linux
  (`gpu/intern/gpu_gl.c`: the same instances, atlases and Slug textures, the
  uber-shader translated to GLSL, GLX on the X11 window or EGL offscreen,
  libGL loaded at run time). Opt-in with `BUI_GPU=1`, `BUI_gpu_set_default`,
  Rust `.gpu(true)` or tkinter `Tk(gpu=True)`.

Textured triangle meshes (`BUI_raster_mesh`, what Dear ImGui emits) are the
one non-analytic primitive: indexed vertices with UV and color sampling a
`BUI_texture_*` texture. The software raster fills them with a fixed-point
top-left rule and bilinear sampling; the GPU paths draw them indexed with a
scissor (`vs_mesh`/`ps_mesh` in `bui_ui.hlsl`, the GLSL twin in `gpu_gl.c`),
uploading each texture once and then only its logged update rectangles.

In GPU mode the same `BUI_raster_*` calls record one 128-byte instance per
primitive, after the same clip test. This follows Blender's
`GPU_SHADER_2D_WIDGET_BASE_INST` batching in `interface_widgets.cc`:
- One uber-shader evaluates the software raster's coverage formulas, so the
  pixels match. `gpu/tests/test_gpu_parity.c` checks this.
- Text runs and images are cached in atlases. Polygons are covered on the
  CPU and uploaded as masks.
- Frames render into a persistent target, so damage rectangles still limit
  redraws. Presentation uses a flip-model swap chain with dirty rects.
- The device is created on a worker thread, and a lost device falls back
  to software.

Real renderers plug in at three levels: C (`bui_set_render_sink`), Qt/C++
(`BUI::QtSink`), Python (`buipy.set_backend`). See INTEGRATION.md.

## Build

```sh
cmake -S . -B build -G "MinGW Makefiles"     # or any generator
cmake --build build
ctest --test-dir build                       # core tests + buipy tests
cmake -S . -B build-ui -DBUI_BUILD_UI=ON -DBUI_BUILD_EXAMPLES=ON
cmake --build build-ui                       # + widget toolkit (bui_ui)
```

Options: `BUI_BUILD_PYTHON_BINDINGS` (ON; auto-skips without Python dev
files), `BUI_BUILD_QT_BACKEND` (OFF; needs Qt5/Qt6 Widgets),
`BUI_BUILD_EXAMPLES`, `BUI_BUILD_TESTS`.

Windows note: the toolchain must resolve MinGW runtime DLLs from
`/c/msys64/ucrt64/bin` **before** `/mingw64/bin` or other vendors' paths
(cc1.exe loads zlib1/gmp from PATH). If builds fail silently, prepend the
ucrt64 dir: `export PATH="/c/msys64/ucrt64/bin:$PATH"`.

## Decoupling inventory (what was cut, what remains)

Cut entirely (Blender application logic): scene/mesh/node/depsgraph/modifiers,
animrig (only the DNA structs + a minimal `animrig::Slot` wrapper remain),
asset system, sequencer, render, Python-in-Blender, file I/O (BLO),
eyedroppers, app-specific UI templates, window manager backends (GHOST),
operator system.

Remaining seams before a *hosted* toolkit (real widgets on screen) works —
call-site counts measured from the copied sources:

| Seam | Touchpoints | Notes |
|---|---|---|
| WM (events/operators/keymaps) | ~183 | Needs lite `wmEvent`/`wmOperator` structs + handler plumbing in `compat/` |
| ED (screen/region services) | ~120 | Region redraw tagging, area/region lookup — small BUI-owned implementations |
| CTX (context) | ~46 | Minimal `bContext` replacement |
| BLF (fonts) | ~45 | Text measurement/draw hook; null impl first |
| IMB (images/icons) | ~23 | Icon texture registration hook |
| RNA (properties) | ~16 entry points | Opaque `PropertyRNA` + accessor vtable |

These seams block *widget-tree hosting* (creating blocks/buttons driven by
events), not compilation: all five libraries — `bui_core`, `bui_gpu`,
`bui_compat`, `bui_api`, `bui_ui` — build and link cleanly today, and the
render pipeline is exercisable end-to-end through the sink (see
`examples/`).

## Porting notes

- Upstream files keep their names and SPDX headers (GPL-2.0-or-later). The
  mechanical de-C++ pass is intentionally boring so future syncs stay easy.
- `core/include/fmt/*` is a declaration-level stand-in for the external {fmt}
  library; swap for real {fmt} if you need formatted strings.
- `MEM_guardedalloc.h` is a plain-malloc shim with the upstream C++ template
  surface (`MEM_new<T>`, `MEM_new_zeroed<T>`, `MEM_new_array_uninitialized<T>`,
  `MEM_delete<T>`, `MEM_SAFE_DELETE`).
- `GPU_batch.hh` carries a layout contract: the first four members of
  `blender::gpu::VertBuf` must match `GPUVertBufInternal` in `gpu_null.c`.
