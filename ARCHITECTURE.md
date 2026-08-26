# BUI Architecture

BUI is a decoupled UI toolkit extracted from the Blender project
(`source/blender/editors/interface` and its support layers). It contains **only**
reusable interface machinery — no Blender application logic (no scene graph,
depsgraph, nodes, operators-on-datablocks, file loading, Python).

```
┌─────────────────────────────────────────────────────────────┐
│  ui/      widget toolkit: blocks/buttons, layout engine,    │
│           widget drawing, handlers, views, view2d           │
└──────┬───────────────┬──────────────┬───────────────────────┘
       │               │              │
┌──────▼──────┐ ┌──────▼──────┐ ┌─────▼───────────────────────┐
│ types/      │ │ gpu/        │ │ compat/                     │
│ DNA data    │ │ draw seam   │ │ RNA / CTX / WM / ED / BLF / │
│ shapes      │ │ (null       │ │ IMB seams (stubs & hooks)   │
│ (verbatim   │ │  backend;   │ │                             │
│  structs)   │ │  plug real  │ │                             │
│             │ │  renderer)  │ │                             │
└──────┬──────┘ └──────┬──────┘ └─────┬───────────────────────┘
       │               │              │
┌──────▼───────────────▼──────────────▼───────────────────────┐
│  core/   pure C11 foundation extracted from blenlib         │
│          listbase · ghash · mempool · memarena · dynstr     │
│          math (vec/mat/rot/color/geom) · strings · rct      │
│          guarded-allocator shim · threads stubs             │
└─────────────────────────────────────────────────────────────┘
```

Dependency rule: arrows point downward only. `ui/` never includes Blender
application code; everything it needs arrives through `types/`, `gpu/`, or
`compat/`.

## Modules

| Module | Contents | Status |
|---|---|---|
| `core/` | C11 subset of blenlib, de-C++ified with mechanical transforms (namespaces stripped, casts normalized, MEM API mapped). Inline math lives in `include/intern/*.c` included by headers (`MINLINE` = `static inline` in C). | **Builds + tested** |
| `types/` | Verbatim `DNA_*_types.h` struct definitions (regions, areas, themes, events). Data shapes only; no serialization runtime. | Headers ready |
| `gpu/` | The 90-symbol draw surface used by the toolkit (`GPU_*`, `imm*`) behind seam headers + a null backend. Real renderers implement the same headers. Theme colors reach the GPU layer through function-pointer hooks to avoid a `gpu→ui` dependency. | **Builds** |
| `compat/` | Seam headers carrying upstream API names (`RNA_access.hh`, `BLF_api.hh`, …) with BUI-owned opaque types/stubs. | Skeleton staged |
| `ui/` | The toolkit sources from `editors/interface`: interface.cc family, widgets, handlers, regions, view2d, views, textbox. Option-gated (`-DBUI_BUILD_UI=ON`). | Sources staged; compile WIP |

## Build

```sh
cmake -S . -B build -G "MinGW Makefiles"     # or any generator
cmake --build build
ctest --test-dir build                       # core tests
cmake -S . -B build-ui -DBUI_BUILD_UI=ON     # work on the ui module
cmake --build build-ui --target bui_ui
```

## Decoupling inventory (what was cut, what remains)

Cut entirely (Blender application logic): scene/mesh/node/depsgraph/modifiers,
animrig, asset system, sequencer, render, Python, file I/O (BLO), eyedroppers,
app-specific UI templates, window manager backends (GHOST), operator system.

Remaining seams before `bui_ui` compiles end-to-end (call-site counts measured
from the copied sources):

| Seam | Touchpoints | Notes |
|---|---|---|
| WM (events/operators/keymaps) | ~183 | Needs lite `wmEvent`/`wmOperator` structs + handler plumbing in `compat/` |
| ED (screen/region services) | ~120 | Region redraw tagging, area/region lookup — small BUI-owned implementations |
| CTX (context) | ~46 | Minimal `bContext` replacement |
| BLF (fonts) | ~45 | Text measurement/draw hook; null impl first |
| IMB (images/icons) | ~23 | Icon texture registration hook |
| RNA (properties) | ~16 entry points | Opaque `PropertyRNA` + accessor vtable |

Plus link-stage items: BLI C++ utilities used by `views/*.cc` (Vector/Map/Set —
header-only templates already vendored under `core/include`),
`DNA_*` runtime (only needed if persistence is wanted), icon/font assets.

## Porting notes

- Upstream files keep their names and SPDX headers (GPL-2.0-or-later). The
  mechanical de-C++ pass is intentionally boring so future syncs stay easy.
- `core/include/fmt/*` is a declaration-level stand-in for the external {fmt}
  library; swap for real {fmt} if you need formatted strings.
- `MEM_guardedalloc.h` is a plain-malloc shim; leak tracking is out of scope.
