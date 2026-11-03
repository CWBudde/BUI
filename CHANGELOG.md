# Changelog

All notable changes to **blenderui** (BUI). The format follows
[Keep a Changelog](https://keepachangelog.com); versions follow
[Semantic Versioning](https://semver.org).

## [Unreleased]

### Added
- **Rust crates** (`bindings/rust/`): `blenderui-sys` (FFI over `BUI_kit.h`,
  builds the kit with CMake) and `blenderui`: gpui syntax (`Application`,
  `Render`, `Context`, `Entity`, `cx.listener`, ~1,300 `Styled` methods), all
  113 HTML elements, CSS (stylesheets, selectors, cascade, 173 rendered
  properties), taffy layout (block, inline line boxes, flexbox, grid, tables),
  form controls as Blender widgets, headless rendering for tests. See
  [docs/rust.md](docs/rust.md).
- Kit canvas: per-item clip rectangles and text font flags, and
  `BK_canvas_add_item` (append without an id lookup) for immediate-mode hosts.
- **Parallel canvas rasterisation**: large display lists draw in horizontal
  bands on the Win32 thread pool, with pixels identical to serial drawing
  (`BUI_RASTER_THREADS` sets the thread count). New raster APIs back it:
  `BUI_raster_view`, `BUI_text_prepare`, `BUI_raster_text_run` and
  `BUI_text_generation`.
- `BK_window_request_frame` and the `on_frame` window callback: animation
  frames that rebuild right before the render. They are vsync-paced, and a
  frame that is already late doesn't wait for another vblank.
- The `opaque` node flag: the kit skips its region clear under an opaque root.
- The Rust crate gains `Window::request_animation_frame`, `set_vsync` and
  `presented_frames`, and `BUI_PROFILE=1` per-phase timings.
- `bindings/rust/bench`: blenderui vs gpui vs gpui-fast benchmark (FPS, CPU, GPU, RAM).

- Rust: `css!` (crate `blenderui-macros`) checks CSS blocks at compile time;
  `.style()` takes its output or run-time text.

### Changed
- Rust: no `Rc` / `RefCell`. Frames are index-linked arenas reused across
  frames, styles live in a per-window store, listeners are owned `Box`es.
  `Entity<T>` is a `Copy` handle into an app-owned slot map: `read`
  returns `&T`, `WeakEntity::upgrade` became `upgrade_in(cx)`, and
  `cx.release(entity)` drops a value. Elements are no longer `Clone`.
- Rust: style methods resolve property names at compile time (a `const`
  property table) and build typed values without formatting or parsing.

### Performance
- Kit canvas (tkinter, Qt, Rust): every item type rasterizes in parallel
  (lines, arcs and outlines included); item edits redraw only the item's old
  and new boxes; items are found by id through a hash index; replacing an
  item reuses its buffers. tkinter: 2000 items, one moving: 9.4 → 0.08 ms
  render per frame; all moving: 11.1 → 3.6 ms.
- tkinter facade: colors parsed once; lines delete only arrowheads they made.
- Rust: frame-to-frame reuse: selector matches memoized by element
  fingerprint, a persistent taffy tree reconciled by key (unchanged subtrees
  keep their cached layout). 3000 elements of which one changes: 42 % less
  CPU per frame than when all change.
- Rust: fewer allocations: 32-byte tokens, smaller elements with inline
  children, reused painter / text buffer / layout scratch, stack-backed
  small lists.
- Fill-only rounded canvas rects use the span-filling rounded-rect routine
  (about 2.5× faster).
- Left-anchored canvas text is no longer measured before drawing.
- Rust crate: style sharing between identical elements, interned and typed
  CSS values, an indexed stylesheet, allocation-free text measuring.
  Animating 3000 elements went from 12 to about 50 fps, at 29 ms CPU per
  frame (from 40).
- Lone text runs fold into their box as one layout leaf. Canvas raster
  threads adapt to the frame deadline, items are bucketed by band, and
  `BK_canvas_put_item` / `BK_canvas_truncate` update a canvas in place
  (the latter invalidates once).

### Changed
- The C sources build with MSVC (`/utf-8`, no `typeof` / `[[nodiscard]]`
  outside GCC and Clang).

## [0.1.0] - first release

### Added
- **C kit** (`kit/`): Blender's UI as a C runtime. It has the node tree,
  Tk's packer and gridder in C, input and interaction (hover, drag grabs,
  continuous cursor, menus, tooltips), widget drawing ported from Blender's
  `interface_widgets.cc`, and damage-driven raster rendering. Also: undo and
  redo, animation, the text area, canvas, scroll views, scrollbars and
  multi-select.
- **Native window host** for Windows (Win32): DPI-aware, vsync'd
  animation, clipboard, cursors, window-manager features. Other platforms use
  a stdlib-Tk-hosted window.
- **`bui.tksyntax`**: the complete tkinter API (7846/7846 public names of
  the tkinter and ttk classes), plus `ttk`, `messagebox`, `filedialog`,
  `simpledialog`, `colorchooser`, `font` and `scrolledtext`. Run existing apps
  unchanged with `python -m bui.tksyntax app.py`, `blenderui-tk app.py` or
  `install()`.
- **`bui.qtsyntax`**: a Qt-style API subset (widgets, layouts, signals,
  `QPainter`, menus, timers).
- **`bui.components`**: 45 Blender components (layouts, values, views,
  menus, pie menus, the color picker, editors, templates).
- Packaging as `blenderui` (scikit-build-core), with `import blenderui` as
  an alias of `bui`.
