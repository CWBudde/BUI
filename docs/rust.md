# Rust: gpui syntax, HTML and CSS

The `blenderui` crate (`bindings/rust/`) gives Rust programs Blender's UI with
the syntax of [gpui](https://www.gpui.rs): views implement `Render`, elements
are built with `div().flex().gap_2().child(...)`, and listeners come from
`cx.listener(...)`. On top of gpui's model it supports **every HTML element**
and **CSS**: `<style>` sheets, selectors, the cascade, flexbox, grid, tables,
inline text that wraps, lists, `<details>`, and form controls that are real
Blender widgets.

<p align="center"><img src="images/rust-showcase.png" width="640" alt="The Rust showcase example"></p>

- [Install](#install)
- [The app model](#the-app-model)
- [Elements](#elements)
- [Styling](#styling)
- [Layout](#layout)
- [Form controls](#form-controls)
- [Events](#events)
- [Painting your own content](#painting-your-own-content)
- [Headless rendering and tests](#headless-rendering-and-tests)
- [Coverage](#coverage)
- [Performance vs gpui](#performance-vs-gpui)
- [Limitations](#limitations)
- [How it works](#how-it-works)

## Install

```toml
[dependencies]
blenderui = { git = "https://github.com/ppmpreetham/BUI" }
```

Building needs **CMake** and a **C compiler**: `blenderui-sys` compiles the C
kit with the `cmake` crate. On Windows the default Rust target
(`x86_64-pc-windows-msvc`) uses MSVC (Visual Studio Build Tools). To build
against another checkout of the C sources, set `BUI_SOURCE_DIR`.

From the repository:

```sh
cd bindings/rust
cargo run --example showcase                        # the window above
cargo run --example showcase -- --screenshot a.png   # render it off-screen to a PNG
cargo run --example counter                         # the README example
cargo test                                          # unit, coverage and headless tests
```

The workspace uses edition 2024 (Rust 1.85 or newer).

## The app model

It follows gpui closely:

| gpui                                     | blenderui                                                     |
| ---------------------------------------- | ------------------------------------------------------------- |
| `Application::new().run(\|cx\| ...)`      | the same; `Application::headless()` renders off-screen        |
| `cx.open_window(options, \|window, cx\| cx.new(\|cx\| View))` | the same; returns `WindowHandle<View>`   |
| `impl Render for View`                    | the same: `fn render(&mut self, window, cx) -> impl IntoElement` |
| `Entity<T>`, `WeakEntity<T>`, `cx.new`    | `Copy` handles into the app's entity slots: `read` (returns `&T`), `update`, `downgrade`; `cx.release(e)` drops a value |
| `Context<T>`: `notify`, `listener`, `entity` | the same; it derefs to `App`                              |
| `WindowOptions { window_bounds, titlebar, .. }`, `WindowBounds::Windowed(..)` | the same, plus `is_resizable`, `window_min_size` |
| `cx.set_global`, `cx.global`              | the same                                                       |
| `cx.stop_propagation()`                   | the same                                                       |

```rust
use blenderui::prelude::*;

struct Todo { items: Vec<String>, draft: String }

impl Render for Todo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().p_4().flex().flex_col().gap_2()
            .child(h2().child(format!("{} items", self.items.len())))
            .child(ul().children(self.items.iter().map(|s| li().child(s.clone()))))
            .child(div().flex().gap_2()
                .child(input().id("draft").placeholder("New item").value(self.draft.clone())
                    .on_change(cx.listener(|this, e: &ChangeEvent, _, cx| {
                        this.draft = e.value.clone();
                        cx.notify();
                    })))
                .child(button().child("Add").on_click(cx.listener(|this, _, _, cx| {
                    this.items.push(std::mem::take(&mut this.draft));
                    cx.notify();
                }))))
    }
}
```

`cx.notify()` re-renders. Rendering builds a fresh element tree; elements with
an `.id(...)` keep their state across renders (scroll position, a text field's
contents, a checkbox you clicked, which `<details>` are open).

`Window` gives listeners `viewport_size()`, `scale_factor()`, `mouse_position()`,
`set_window_title()`, `focus_id()` / `focused_id()`, `is_hovered(id)`,
`scroll_offset(id)` / `set_scroll_offset(id, x, y)`, `refresh()` and
`request_animation_frame()` (call it from `render` to animate; frames are paced
to the display, `set_vsync(false)` lifts the cap).
A `<title>` element sets the window title too.

## Elements

There is **one element type, `Div`**, and a constructor for every element of
the WHATWG HTML index: `div()`, `span()`, `section()`, `table()`, `tr()`,
`details()`, `dialog()`, `input()`, `select()`, `video()` ... (113 elements,
plus `svg()` and `math()` as boxes). Tags only change the defaults: what the
user-agent stylesheet gives them and how they are realised.

| Kind      | Elements                                                                                  | Realised as                                                    |
| --------- | ----------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| Box       | `div`, `p`, `h1`-`h6`, `a`, `span`, `ul`/`ol`/`li`, `table` ..., `details`, `dialog`, `form` ... | laid out and painted from CSS                           |
| Widget    | `button`, `input`, `select`, `textarea`, `progress`, `meter` (+ `option`, `optgroup`, `datalist`) | Blender's own widgets ([below](#form-controls))        |
| Replaced  | `img`, `video`, `audio`, `iframe`, `embed`, `object`, `canvas`                            | an intrinsic size; `img` shows PNG files or RGBA pixels       |
| Metadata  | `head`, `title`, `style`, `script`, `meta`, `link`, `template` ...                        | not rendered (`<style>` and `<title>` are read)               |
| Break     | `br`, `wbr`                                                                               | line breaks                                                    |

Unknown names work too: `custom("my-widget")` is a box, like a custom element
in a browser.

Every element has `.child(..)`, `.children(..)`, `.id(..)`, `.class(..)`,
`.attr(name, value)` and typed attributes: `.href`, `.value`, `.placeholder`,
`.input_type`, `.name`, `.checked`, `.disabled`, `.selected`, `.open`, `.min`,
`.max`, `.step`, `.src`, `.alt`, `.title`, `.colspan`. `FluentBuilder` adds
`.when(cond, |el| ..)`, `.when_some(opt, |el, v| ..)` and `.map(..)`.
Text children are `&'static str`, `String` or `SharedString`.

> `main()` (the `<main>` element) has the same name as your program's
> `fn main`. Inside `fn main` call it as `blenderui::html::main()`.

The document is wrapped in `<html><body>` unless your root already is one, so
`body { margin: 0 }` and the user-agent defaults behave as in a browser. The
defaults are Blender's dark theme: `#303030` background, 13 px text, blue
links.

## Styling

Three ways, one cascade:

```rust
// 1. gpui / Tailwind methods
div().flex().items_center().gap_2().px_3().py_1().rounded_md().bg(rgb(0x3a3a3a))
    .text_color(white()).shadow_md().hover(|s| s.bg(rgb(0x4772b3)))

// 2. CSS blocks, checked at compile time (a misspelled property doesn't compile)
div().style(css!("border-left: 3px solid #4772b3; padding: 4px 8px"))

// 3. CSS built at run time
div().css("grid-template-columns", format!("repeat({n}, 1fr)"))
     .style(format!("width: {w}px"))

// 4. stylesheets, anywhere in the tree
style().child(".card:hover > h4 { color: #e6a23c } @media (max-width: 600px) { nav { display: none } }")
```

**`Styled` methods** (~1,300, generated from Tailwind's scales): spacing and
sizing on the spacing scale (`p_4`, `mx_auto`, `w_64`, `h_full`, `w_1_3`,
`size(px(20.0))`, `gap_x_2`), display and position (`flex`, `grid`, `block`,
`hidden`, `absolute`, `inset_0`, `z_index`), flexbox (`flex_col`, `flex_1`,
`items_center`, `justify_between`, `flex_wrap`), grid (`grid_cols(3)`,
`col_span(2)`), color (`bg`, `text_color`, `border_color`, `opacity`),
borders and corners (`border_1`, `border_t_2`, `rounded_lg`, `rounded_full`),
shadows (`shadow_sm` ... `shadow_2xl`), text (`text_sm` ... `text_6xl`,
`font_bold`, `italic`, `underline`, `uppercase`, `truncate`, `leading_tight`,
`whitespace_nowrap`), overflow (`overflow_y_scroll`) and cursors
(`cursor_pointer`). Colors: `rgb(0xRRGGBB)`, `rgba(0xRRGGBBAA)`,
`hsla(h, s, l, a)`, named helpers, or any CSS color string.

**State styles**: `.hover(|s| ..)`, `.active(|s| ..)` and `.focus(|s| ..)`
take a refinement with the same methods (they are CSS `:hover` / `:active` /
`:focus` rules for that element).

**Selectors**: type, `*`, `#id`, `.class`, attributes (`[a]`, `=`, `~=`,
`|=`, `^=`, `$=`, `*=`), combinators (descendant, `>`, `+`, `~`), and
pseudo-classes: `:hover`, `:active`, `:focus`, `:focus-within`,
`:focus-visible`, `:checked`, `:disabled`, `:enabled`, `:link`, `:any-link`,
`:required`, `:optional`, `:read-only`, `:read-write`, `:open`,
`:placeholder-shown`, `:first-child`, `:last-child`, `:only-child`,
`:first-of-type`, `:last-of-type`, `:nth-child()`, `:nth-last-child()`,
`:nth-of-type()`, `:nth-last-of-type()`, `:empty`, `:root`, `:not()`,
`:is()`, `:where()`. `@media` with width and height conditions is evaluated against the
window.

**Cascade**: user-agent sheet, then author sheets, then inline declarations
(methods, `.css`, `.style`), then state refinements; `!important` wins over
all of them, and specificity and order decide within a level. `inherit`,
`initial`, `unset` and `revert` are understood (they copy or reset the text,
color, size and display properties). Custom properties (`--name`) are
accepted but `var()` is not substituted. Units: `px`, `em`, `rem`, `%`, `vw`,
`vh`, `vmin`, `vmax`, `ch`, `ex`, `pt`, `pc`, `in`, `cm`, `mm`, `Q`, and
`calc()`, `min()`, `max()`, `clamp()`.

## Layout

Layout is computed with [taffy](https://github.com/DioxusLabs/taffy):

- **Block** flow with margins, padding, borders, `box-sizing`, min/max sizes
  and `aspect-ratio`.
- **Inline** content becomes line boxes: text breaks into words that wrap,
  inline elements (`b`, `a`, `code`, `mark` ...) style their words, and
  inline-blocks and widgets sit on the line. `white-space` (`normal`,
  `nowrap`, `pre`, `pre-wrap`, `pre-line`), `text-align`, `text-indent`,
  `letter-spacing`, `word-spacing`, `vertical-align: sub | super`,
  `text-transform` and `text-overflow: ellipsis` apply.
- **Flexbox** and **grid**: every property of both modules (except
  `grid-template-areas`), including `repeat()`, `minmax()`, `fr`,
  `auto-fill` / `auto-fit`, spans and line placement.
- **Tables**: rows and cells share the width (`colspan` weights cells),
  `border-spacing`, `border-collapse` and `caption-side` apply.
- **Positioning**: `relative`, `absolute` (and `fixed` as absolute), `inset`
  properties, `z-index` for painting and hit testing, `transform: translate()`.
- **Overflow**: `hidden` / `clip` clip, `scroll` / `auto` scroll with the
  wheel (Shift for horizontal) and show overlay scrollbars.
- **Lists**: `list-style-type` (`disc`, `circle`, `square`, `decimal`,
  `lower-alpha`, `upper-roman` ...), `list-style-position`, `<ol start>`.
- **`<details>` / `<summary>`**: clicking the summary toggles the details
  (`.open(true)` sets the initial state); `<dialog>` shows with `.open(true)`.

## Form controls

Form elements are Blender's widgets, with Blender's interaction (drag to
scrub numbers, Ctrl+Z, double-click to type, Escape to cancel):

| HTML                                              | Blender widget                       | `ChangeEvent`                         |
| ------------------------------------------------- | ------------------------------------ | ------------------------------------- |
| `button()` with text, `input().input_type("submit" \| "button" \| "reset")` | button          | `on_click`                            |
| `input()`: `text`, `password` (masked), `email`, `search`, `url`, `tel`, `date` ... | text field  | `value`                               |
| `input().input_type("number")`                    | number field                          | `number`, `value`                     |
| `input().input_type("range")`                     | slider                                | `number`, `value`                     |
| `input().input_type("checkbox")`                  | checkbox                              | `checked`                             |
| `input().input_type("radio")` (grouped by `name`) | radio button                          | `checked`                             |
| `input().input_type("color")`                     | color swatch (opens Blender's picker) | `value` as `#rrggbb`                  |
| `select()` with `option()` / `optgroup()`         | dropdown menu                         | `number` (index), `value` (text)      |
| `textarea()`                                      | multi-line text editor                | `value`                               |
| `progress()`, `meter()`                           | progress bar                          | (display only)                        |

`on_input` and `on_change` both fire when the value changes. Values you pass
(`.value(..)`, `.checked(..)`) are pushed to the widget only when they change
between renders, so a field the user is typing in is not reset; bind it to your
state (as in the todo example) to control it. A `button()` holding elements
other than text is laid out as an ordinary box (style it with CSS) and still
gets `on_click`.

## Events

| Listener                                   | Event                                                   | Bubbles |
| ------------------------------------------ | ------------------------------------------------------- | ------- |
| `on_click`                                 | `ClickEvent { down, up }` (press and release on the element) | yes |
| `on_mouse_down(button, ..)`, `on_any_mouse_down` | `MouseDownEvent { button, position, modifiers, click_count }` | yes |
| `on_mouse_up(button, ..)`                  | `MouseUpEvent`                                          | yes     |
| `on_mouse_move`                            | `MouseMoveEvent { position, pressed_button, modifiers }` | yes    |
| `on_hover`                                 | `bool`: entered / left                                  | no      |
| `on_scroll_wheel`                          | `ScrollWheelEvent { position, delta, modifiers }`       | yes     |
| `on_key_down`, `on_key_up`                 | `KeyDownEvent { keystroke: Keystroke { key, key_char, modifiers } }` | yes (from the focused element) |
| `on_change`, `on_input`                    | `ChangeEvent { value, checked, number }`                | yes     |

Positions are logical pixels. Clicking an element focuses it; key events go
to the focused element and bubble up. `cx.stop_propagation()` stops bubbling.

## Painting your own content

`canvas(|bounds, painter| ..)` is gpui's `canvas()`: it paints with a
`Painter` in logical pixels, clipped to its box.

```rust
canvas(|b, p| {
    p.fill(b, rgb(0x232323));
    p.quad(b, 6.0, rgba(0x4772b380), rgb(0x4772b3), 1.0);       // rounded rect + border
    p.line(&[(b.origin.x, b.origin.y), (b.origin.x + 80.0, b.origin.y + 40.0)], white(), 2.0);
    p.oval(Bounds { origin: point(10.0, 10.0), size: size(20.0, 20.0) }, red(), black(), 1.0);
    p.text(b.origin.x + 4.0, b.origin.y + 4.0, "hello", 13.0, white());
})
.w(px(200.0)).h(px(100.0))
```

`Painter` also has `polygon`, `arc` (pie slices), `image` (scaled) and
`with_clip`. Images: `img().src("file.png")` (PNG, decoded once and cached) or
`img().image_rgba(w, h, pixels)`.

## Headless rendering and tests

`Application::headless()` runs the same code without a native window (and is
what you get on platforms without one). Drive it from the `run` closure:

```rust
Application::headless().run(|cx| {
    let win = cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| Counter { count: 0 }))
        .unwrap();
    let b = cx.element_bounds(win.id, "plus").unwrap();         // the element with .id("plus")
    cx.simulate_click(win.id, b.origin.x + 2.0, b.origin.y + 2.0);
    assert_eq!(win.root().read(cx).count, 1);
    let (w, h, pixels) = cx.draw(win.id);                       // 0x00RRGGBB, device pixels
});
```

Also `simulate_mouse_move`, `simulate_scroll` and `simulate_key(text, keysym)`.
The kit has global state, so one `Application` runs at a time per process
(`run` takes a lock; parallel tests queue up).

## Coverage

From `cargo test --test parity -- --nocapture`:

| Area                                  | Coverage                                                                         |
| ------------------------------------- | -------------------------------------------------------------------------------- |
| HTML elements (WHATWG index)          | **113 / 113** constructible, plus `svg` / `math` roots; 9 are Blender widgets     |
| CSS properties                        | **180** change layout or rendering; **81** more parse but have no effect          |
| W3C property index (745 properties)   | **176** rendered (24%), **75** parsed without effect (10%), **494** not recognized (66%): most of these are drafts, speech, paged media and SVG |
| CSS 2.1, screen properties            | **80 / 90** rendered, 90 / 90 parsed                                              |
| Flexbox                               | **12 / 12**                                                                       |
| Grid                                  | **23 / 23** (including `grid-template-areas` and named lines)                     |
| Tailwind CSS v4                       | Classes compile on first use (v4 theme, variants, arbitrary values); `tw!` checks them at compile time |

Beyond properties, the engine supports:
- **Values:** every CSS math function (`calc`, `min`, `max`, `clamp`, `round`, `mod`, `rem`, trigonometry, `pow`, `hypot` ...) with mixed units, every length unit, and `var()` / `env()`.
- **Colors:** CSS Color 4/5: `oklch`, `lab`, `color()`, `color-mix()`, relative colors and `light-dark()`.
- **Selectors:** Selectors Level 4 (`:has()`, `:is()`, `:where()`, `:nth-child(of S)` ...), nesting, and `::before` / `::after` with `content`.
- **At-rules:** `@media` (Media Queries 4 ranges), `@supports`, `@layer` and `@scope`.

```rust
div().class(tw!("grid grid-cols-3 gap-4 md:grid-cols-6 hover:bg-sky-500/50"))
div().style("width: calc(100% - 2 * var(--gap)); background: color-mix(in oklch, var(--brand) 60%, white)")
```

The CSS 2.1 properties that parse without effect: `clip`, `counter-increment`, `counter-reset`, `empty-cells`, `table-layout`, `unicode-bidi`, `list-style-image` and `background-attachment` / `-position` / `-repeat` (there are no background images; gradients are supported).

How much is left, against all 745 properties in the
[W3C index](https://www.w3.org/Style/CSS/all-properties.en.html):

| | Properties | Share |
|---|---:|---:|
| Change layout or rendering | 176 | 24% |
| Parsed, no effect yet | 75 | 10% |
| Not recognized | 494 | 66% |

The 494 unrecognized properties break down as:

| Group | Properties |
|---|---:|
| Draft border and corner shapes (`corner-shape`, `border-*-clip` ...) | ~78 |
| SVG fill, stroke and markers | ~40 |
| Gap decorations (`row-rule-*` ...) | 38 |
| Speech and aural | 31 |
| Animation triggers and scroll-driven animation | 26 |
| Paged media, regions, rhythm and others | ~25 |
| Scroll snap margins and padding | 23 |
| Masking | 18 |
| Everything else | ~215 |

The everyday screen CSS is much further along: Flexbox 12/12, Grid 23/23 and
CSS 2.1 80/90. The largest remaining gaps among common properties are visual
effects (transforms, filters, blend modes, `clip-path`), animations and
transitions, and background images.

Not done yet:
- **Parsed, not applied:** `@container` (queries parse, no container sizes yet), `@font-face`, `@keyframes`, transitions and animations.
- **Rendering effects:** 2D/3D `rotate` / `skew` transforms, `filter` / `backdrop-filter`, blend modes, `clip-path`, masks, `border-image` and border styles other than solid.
- **The rest of the W3C property index:** about 490 entries this engine doesn't know, mostly drafts, speech and paged media.
- **Box sizing:** the default is `border-box` (like Tailwind's preflight), not CSS's `content-box`.

## Performance vs gpui

The same UI on blenderui (GPU renderer and default software raster),
[gpui-fast](https://github.com/longbridge/gpui-fast) 0.1.13 and gpui 0.2.2:
a window of N rounded, labelled cells that all change every frame. Ryzen 7
5800U laptop at 60 Hz, all apps back to back in one session.
[bindings/rust/bench](../bindings/rust/bench/README.md) has the full table and
how to re-run it.

| | blenderui GPU | blenderui CPU | gpui-fast | gpui |
|---|---|---|---|---|
| 1000 / 3000 animated cells | **60 / 59.9 fps** | 60 / 57.6 fps | 49.2 / 31.5 fps | 36.4 / 19.9 fps |
| CPU per frame, 100 / 1000 / 3000 cells | **1.2 / 4.7 / 11.6 ms** | 4.6 / 10.2 / 22.0 ms | 2.7 / 20.5 / 31.9 ms + GPU | 4.7 / 27.7 / 50.5 ms + GPU |
| RAM (working set) | 28-40 MB + 19-22 MB GPU | **21-30 MB**, no GPU | 45-65 MB + 19-24 MB GPU | 40-77 MB + 16-21 MB GPU |
| First frame after launch | **~5 ms** | **~5 ms** | ~200 ms | ~220 ms |
| Idle CPU | **0 %** | 0-0.9 % | 0-0.4 % | 0.9-1.4 % |

### The GPU renderer

```rust
Application::new().gpu(true).run(|cx| { /* every window on the GPU */ });
// or one window:
cx.open_window(WindowOptions { gpu: Some(true), ..Default::default() }, build);
// window.is_gpu() turns true a few frames in, once the device is ready
```

`BUI_GPU=1` / `0` overrides both. It is Direct3D 11 on Windows. Elsewhere,
or without a device, windows stay on the software raster. Every primitive
becomes one instance of a single shader, following Blender's instanced
widget-base batches. Text runs and images are cached in atlases. Frames
render into a persistent target, so only damaged regions redraw. Both
renderers produce the same pixels (the `gpu_parity` test).

The software raster draws in parallel bands (`BUI_RASTER_THREADS=1` forces
one thread). `BUI_PROFILE=1` prints per-phase frame times for either
renderer.

### Text

Windows draws text with GDI by default. Linux, macOS and wasm use the
bundled stb_truetype engine; enable it on Windows too with the
`portable-text` feature. It finds the platform's fonts, or `BUI_FONT` /
`BUI_FONT_DIR`. Where there are no system fonts (wasm), register one:

```rust
blenderui::add_font("Inter", false, false, include_bytes!("Inter-Regular.ttf"));
```

## Limitations

- **Floats** place block boxes, but text does not wrap around them; `direction: rtl`
  lays boxes out right to left, but text itself is not shaped right to left.
- **One font face**: the UI font in regular, bold and italic. `font-family`
  is parsed but `monospace` text uses the UI font.
- **Effects and animation**: see "Not done yet" under [Coverage](#coverage).
- **Transforms**: `translate` only; `scale()` and `rotate()` are ignored.
- **Inline boxes** paint their background per word; borders and padding on
  inline elements (`<kbd>`, `<code>`) are not drawn.
- **Widgets draw above page content**, so a `z-index`ed box (a `<dialog>`)
  over a form field does not cover the field.
- **Images**: PNG only, nearest-neighbour scaling. `video`, `audio`,
  `iframe` show a placeholder.
- **Windows** are native on Windows; elsewhere use `Application::headless()`
  until the Linux and macOS window hosts exist.
- `text-align: justify` aligns left; `<table>` columns are equal-width
  (weighted by `colspan`) rather than sized to their content.

## How it works

```
 Render::render ─► element tree (one Div type for every tag)
        │
  cascade: UA sheet ▸ <style> sheets ▸ inline ▸ :hover/:active/:focus ▸ !important
        │          one handler table applies every property
  box tree: blocks, line boxes of words, flex/grid items, tables as flex
        │
  taffy layout ─► absolute boxes, clips, scroll offsets
        │
  paint ─► one display list on a kit canvas      form controls ─► kit widget nodes,
        │  (rects, rounded rects, text, images)                 placed at their boxes
        ▼
  C kit: input, hit testing, Blender widget drawing, damage rendering, Win32 window
```

- `blenderui-sys` is the raw FFI over `kit/include/BUI_kit.h`; its build script
  builds the C kit with CMake and links it statically.
- Every style source (methods, `css!`, `.css`, `.style`, sheets, the
  user-agent sheet, state refinements) becomes CSS declarations applied by
  one table of property handlers, so a property has exactly one
  implementation.
- **Compile time**: the property table is a `const`. Style methods and
  `css!` resolve property names to indices when the program compiles (a
  typo is a compile error), typed methods (`.w(px(40.0))`, `.bg(rgb(..))`)
  build their value token directly, and fixed values (`.flex()`,
  `.shadow_md()`, `css!` values) are tokenized once per program.
- **No `Rc` / `RefCell`**: a frame is a set of flat arenas (boxes, element
  data, taffy nodes) linked by index, owned by the window and reused every
  frame. Computed styles live in a per-window store, shared by every element
  with the same inputs. Listeners are owned `Box`es; while one runs, the
  frame is moved out of the window so it can take `&mut Window`. Entities
  are `Copy` handles into an app-owned slot map; the app itself lives on
  `Application::run`'s stack. Small per-element lists (matched rules,
  declarations) stay on the stack. The only shared ownership left is
  `Arc<Image>` for image pixels.
- **Reuse across frames**: `render()` builds a fresh element tree each frame,
  but the work behind it is reused. Matched selectors are memoized by an
  element fingerprint (what selectors can see: tag, id, attributes, position,
  state, ancestors, previous siblings). Computed styles are shared by input.
  The taffy tree persists and is reconciled by key, so only changed subtrees
  are laid out again. Widget nodes are reconciled by key too.
- Rendering happens on the next turn of the native message loop after
  events.
