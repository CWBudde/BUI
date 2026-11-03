# Changelog

All notable changes to **blenderui** (BUI). The format follows
[Keep a Changelog](https://keepachangelog.com); versions follow
[Semantic Versioning](https://semver.org).

## [Unreleased]

### Added (JavaScript)
- **`@blenderui/native`** (`bindings/node`, napi-rs crate `blenderui-node`): BUI from Node.js in native windows, GPUIX style.
  - A DOM-like API: `createElement`, `setAttribute`, `appendChild`, `addEventListener`, `requestAnimationFrame`, `getBoundingClientRect`, `screenshot`.
  - BUI runs on its own UI thread, so Node is never blocked and an idle UI uses no CPU.
  - Each tick sends only the changes: one `Int32Array` of ops and a string table per window. Events return through a thread-safe function.
  - Animation frames are vsync-paced with one rebuild per frame.
  - Headless mode for tests.
- **`@blenderui/react`** (`bindings/react`): a React 19 renderer (react-reconciler 0.31) with HTML host components, `className` / `style` objects / events, and one native batch per commit.
- **Signals in JavaScript**: the Rust hooks' `Signal` / `Memo` / `Effect` as `signal`, `memo`, `effect`, `batch` (plain JS) and `useSignal`, `useComputed`, `useSignalEffect`, `useSignalValue` (React). A signal passed as an attribute, `style`, text, React prop or React child updates that node alone, without a re-render: React at 3,000 cells drops from ~184 MB to ~95 MB.
- **Rust:**
  - `UiSender` / `App::ui_sender()`: run closures on the UI thread from any thread.
  - `Application::keep_alive`, `App::run_pending`, `App::on_window_closed`, `App::close_window`, `Div::key`, `blenderui::native_windows()`.
  - `css::Value::Shared`, and `StyleBlock` for `&[Decl]`, for pre-parsed styles.
- **C:** `BK_window_wake_init` / `BK_window_wake` (a cross-thread wake through a message-only window) and `BK_window_set_keep_alive`.

### Fixed
- **GPU text:** `BUI_gpu_slug()` read the environment on every text run, which pushed the GPU raster from 1.6 to 9.4 ms per frame at 3,000 cells. It now reads it once.
- **`WindowHandle::update`:** `cx.notify()` inside the closure didn't re-render that window.

### Changed
- **Text looks like Blender's:** Inter, Blender's UI font, is embedded and is the default face on every platform. The portable stb_truetype engine is now the default on Windows too. `-DBUI_TEXT_GDI=ON` (Rust: the `gdi-text` feature) restores GDI and system fonts. `BUI_TEXT_STB` and `portable-text` are now no-ops.

### Added
- **Slug as a flag:** GPU text is Slug or the per-glyph atlas, chosen at build time (CMake `BUI_SLUG`, Rust feature `slug`, both on by default) or at run time (`BUI_gpu_set_slug`, `bui.set_slug`, `Application::slug` / `blenderui::set_slug`, `BUI_SLUG=1/0`).
- **Optional props and keys** (Rust): `Option<T>` props and `children` may be left out in `rsx!`, and a missing required prop is a compile error that names it. `key={..}` ties a component's hook state to the key instead of its position. Struct components opt in with `#[derive(Props)]`.
- **CSS** (Rust):
  - **Applied at-rules:** `@container` size queries (`container-type`, `container-name`, `container`), `@font-face` (loads local font files) and `@property` initial values.
  - **Images:** `background-image: url()` with `background-size`, `-position` (including `-x` / `-y`) and `-repeat`, plus the `background` shorthand; `object-fit` and `object-position` on `<img>`.
  - **Font families:** `font-family` now selects the face (registered, installed, or generic `monospace` / `serif`).
  - **Longhands and aliases:** logical border longhands, `text-wrap-mode`, `white-space-collapse`, `scale` and `all`.
  - **Parsed (no effect yet):** about 100 more properties, including scroll margins and padding, overscroll, OpenType font features, masks and `contain-intrinsic-*`.
  - **Coverage:** 207 of 745 W3C properties now render (was 176), 177 more parse (was 75), and CSS 2.1 is at 82 of 90.
- **Slug GPU text:** with the GPU renderer, glyphs are drawn from their outlines with Eric Lengyel's Slug algorithm: one instance per glyph, exact coverage in the pixel shader, cached curve and band textures, no CPU rasterization. `BUI_SLUG=0` falls back to per-glyph bitmaps in the GPU atlas (Blender's approach).
- **Components and hooks** (Rust, compatible with zopra, React and Leptos):
  - `#[component]` turns a function into a component, with lowercase or uppercase names and `&str` props.
  - `use_state` returns `count()` / `set_count(..)` closures that need no `cx`.
  - Signals: `use_signal`, `Signal`/`RwSignal`, and `signal`/`create_signal` with read and write halves.
  - Auto-tracked `Memo` and `Effect`; `use_effect` / `use_effect!` and `use_memo` with explicit dependencies.
  - Context: `provide_context` / `use_context`.
  - State is tied to the call site, like React, and freed when its component stops rendering. Setters re-render their window. No `Rc` or `RefCell`.
- **`rsx!` / `view!` markup** (Rust): JSX syntax lowered to builder calls at compile time, in about 470 lines with no dependencies.
  - **Syntax:** any tag or custom element; text, `{expr}` and `{...spread}` children; `for`, `if` and `match` in markup; fragments; and struct components with children.
  - **Uniform lowering:** `{expr}` attributes call the builder method of the same name, so the whole API is reachable without per-attribute code.
  - **Events:** listener closures may omit the `(event, window, cx)` parameters.
  - **Attributify:** bare Tailwind utilities are attributes too: `<div p-4 bg-[#4772b3] hover:bg-sky-500/50>`. They compile to classes, while HTML boolean attributes stay attributes.
  - **Did-you-mean errors:** a misspelled utility, variant, root or value is a compile error with suggestions, in `rsx!` and `tw!` (`blenderui_tailwind::suggest`).
  - **Variant attributes:** every Tailwind variant works as an attribute prefix: `hover:class="..."`, `md:dark:hover:style="..."`, `group-hover:color="red"`, `active:bg={red()}`.
- **Modern CSS in the Rust crate**:
  - **Layout:** [taffy](https://github.com/DioxusLabs/taffy) 0.6 → 0.15 behind a layout tree of our own. taffy's built-in tree resolves `calc()` to 0; ours evaluates it. That brings mixed-unit `calc()` everywhere (`calc(100% - 2rem)`, grid tracks, `fit-content(calc())`), floats and `clear`, `direction: rtl`, `grid-template-areas` with named lines and areas, the sizing keywords (`min-content`, `max-content`, `fit-content`, `stretch`), `display: flow-root`, real `position: static` / `fixed` with containing blocks, `contain`, `flex-wrap: balance`, and `safe` / `unsafe` alignment.
  - **Math:** `calc()`, `min()`, `max()`, `clamp()`, `round()` (all four strategies), `mod()`, `rem()`, `abs()`, `sign()`, trigonometry, `pow()`, `sqrt()`, `hypot()`, `log()`, `exp()`, the constants `e`/`pi`/`infinity`, and every length unit (`vmin`/`vmax`, `dvh`/`svh`/`lvh`, `ch`, `ex`, `lh`, `rlh`, `cap`, `cq*` ...) plus angles. Expressions are interned once.
  - **Custom properties:** `var()` with fallbacks, nesting inside functions, inheritance and cycle detection. `env()` is supported too. Variable maps live in a shared arena, not in reference-counted values.
  - **Colors (CSS Color 4/5):** modern `rgb()`/`hsl()` syntax with `none`; `hwb()`, `lab()`, `lch()`, `oklab()` and `oklch()`; `color()` in every predefined space; `color-mix()` with every interpolation space and hue method; relative colors (`oklch(from var(--c) calc(l + .1) c h)`); `light-dark()`; system colors; CSS gamut mapping into sRGB.
  - **Selectors Level 4:** full selector lists in `:is()`, `:where()` and `:not()`; relative `:has()` (evaluated on the element's declarative subtree before it is built); `:nth-child(An+B of S)`; the `*-of-type` family; `:lang()`; `:dir()`; attribute `i`/`s` flags; CSS escapes; nesting with `&`; and pseudo-elements, with `::before` / `::after` generating boxes (the `content` property with strings, `attr()` and quotes).
  - **At-rules:**
    - Media Queries 4: ranges, `not`/`or`, every desktop feature.
    - `@supports`, including `selector()`.
    - `@layer`: cascade layers, both the statement and the block form.
    - `@scope`.
    - Parsed but not applied yet: `@container`, `@font-face`, `@keyframes` and `@property`.
- **Tailwind CSS v4** (`blenderui-tailwind` crate): class names compile to CSS on first use, like Tailwind's JIT, using the v4 default theme.
  - **Variants:** state and structural ones, pseudo-elements, `group-*` and `peer-*` (including named), `has-*`, `not-*`, `in-*`, `data-*`, `aria-*`, `nth-*`, `*` and `**`, the breakpoints with their `max-*` and `min-[…]` forms, container queries, `dark`, `motion-*`, `contrast-*`, `print`, orientation, `supports-*`, `rtl`/`ltr`, `open`, `inert`, and arbitrary `[&…]`.
  - **Values:** arbitrary values and properties, `(--var)`, negatives, `/opacity` and `!important`.
  - **Order:** utilities sort the way Tailwind sorts them.
  - **Usage:** `.class("md:hover:bg-sky-500/50")` just works; `tw!("…")` checks classes at compile time; `blenderui::set_tailwind(false)` opts out.
- **GPU renderer** (Direct3D 11, opt-in): Rust `Application::new().gpu(true)`
  or `WindowOptions { gpu: Some(true), .. }`, tkinter `Tk(gpu=True)` or
  `bui.set_gpu()`, C `BUI_gpu_set_default` / `BK_window_set_gpu`, or
  `BUI_GPU=1` for any app. Modeled on Blender's instanced widget-base
  batches (`interface_widgets.cc`): every primitive (rects, Blender widget
  bases, circles, lines, shadows, gradients, HSV wheels, images, text runs)
  is one 128-byte instance of a single uber-shader, so a frame of thousands
  of widgets is a few `DrawInstanced` calls. Text runs and images live in
  atlases. Frames render into a persistent target, so the kit's damage
  rectangles still bound the work, and they present through a flip-model
  swap chain with dirty rects. The device is created on a worker thread
  (the first frame never waits for the driver), and a lost device falls
  back to the software raster. The pixel shader evaluates the software
  raster's coverage formulas, so both renderers draw the same pixels
  (`gpu_parity` test: 10 of 172,800 pixels differ by more than 3/255, all on
  hard triangle edges). Shaders live in `gpu/shaders/bui_ui.hlsl`, with
  precompiled bytecode checked in, so builds need no shader compiler.
- **Portable text engine**: stb_truetype behind the same glyph-run cache as
  GDI, so text renders on Linux, macOS and wasm. It is also available on
  Windows with `-DBUI_TEXT_STB=ON` or the `portable-text` Rust feature.
  Fonts come from the platform's font folders (Segoe UI, DejaVu, Liberation,
  Noto, Helvetica and others, with generic `sans`/`serif`/`mono`), from
  `BUI_FONT` / `BUI_FONT_DIR`, or from `BUI_text_add_font` (Rust
  `blenderui::add_font`), which wasm needs. Missing bold and italic faces
  are synthesized. A per-glyph bitmap cache keeps changing strings cheap.
  Layout matches GDI, and the 3000-cell benchmark costs the same CPU.
- `BUI_text_run_mask`, `BUI_raster_gpu_attach`/`_poll`/`is_gpu`.
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
