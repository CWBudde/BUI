# Dear ImGui on BUI: C++, C, Python and Rust

The real [Dear ImGui](https://github.com/ocornut/imgui) (1.93.1 WIP, vendored in
`bindings/imgui/extern/imgui`) runs on BUI: BUI's window hosts give it input, BUI's
renderers (software raster, Direct3D 11, OpenGL 3.3) draw it, and it looks like Blender
(Blender's theme colors and metrics, Blender's UI font). The backend itself is described in
[bindings/imgui/README.md](../bindings/imgui/README.md).

Every public Dear ImGui function and type can be used from four languages:

| Language | What you use | Built by |
|---|---|---|
| C++ | `imgui.h` + `imgui_impl_bui.h` | CMake target `bui_imgui` |
| C | `cimgui.h` + `cimgui_impl_bui.h` (`bindings/imgui/c`) | CMake target `bui_cimgui` |
| Python | `import blenderui.imgui as imgui` (or `bui.imgui`) | CMake target `_buiimgui` (next to `buipy`) |
| Rust | crate `blenderui-imgui` (`bindings/rust/blenderui-imgui`) | Cargo (blenderui-sys builds the C++ through CMake) |

The C, Python and Rust bindings are **generated** from `imgui.h`; see
[Regenerating](#regenerating).

- [C++](#c)
- [C](#c-1)
- [Python](#python)
- [Rust](#rust)
- [Headless rendering and tests](#headless-rendering-and-tests)
- [Blender widgets](#blender-widgets)
- [Coverage](#coverage)
- [Regenerating](#regenerating)
- [Limitations](#limitations)

## C++

```cpp
#include "imgui.h"
#include "imgui_impl_bui.h"

static void frame(void*) { ImGui::ShowDemoWindow(); }

int main() {
  ImGui::CreateContext();
  ImGui_ImplBUI_App* app = ImGui_ImplBUI_CreateApp("Dear ImGui on BUI", 1280, 800, frame, nullptr);
  ImGui_ImplBUI_RunApp(app);       // BUI's message loop until the window closes
  ImGui_ImplBUI_DestroyApp(app);
  ImGui::DestroyContext();
}
```

Link `bui::imgui`. Example: `examples/imgui_demo.cpp` (`bui_imgui_demo`).

## C

The C API follows cimgui / dear_bindings conventions:

- `ImGui::Button` is `ImGui_Button`; methods take the object first:
  `ImDrawList_AddLine(draw_list, p1, p2, col)`.
- C has no default arguments: `ImGui_Button(label)` takes the required ones,
  `ImGui_ButtonEx(label, size)` all of them.
- Overloads get a suffix from the parameter types that differ: `ImGui_PushID(str)`,
  `ImGui_PushIDInt(int)`, `ImGui_PushIDPtr(ptr)`, `ImGui_MenuItemBoolPtr(...)`,
  `ImGui_RadioButtonIntPtr(...)`, `ImGui_PushStyleVarVec2(...)`.
- References become pointers (`ImGui_GetIO()` returns `ImGuiIO*`); `const ImVec2&` is a
  plain `ImVec2`.
- `ImGui_Text(fmt, ...)` and the other printf-style functions keep their `...`; the
  `...V` variants take a `va_list`.
- Every struct has its C++ layout (checked by `static_assert`s at build time):
  `io->DisplaySize`, `style->Colors[ImGuiCol_Text]`, `ImVector_ImDrawCmd` (`Size`,
  `Capacity`, `Data`).
- Constructors and destructors: `ImGuiListClipper_New()`, `ImGuiTextFilter_NewEx("x")`,
  `X_Delete(obj)`.

```c
#include "cimgui.h"
#include "cimgui_impl_bui.h"

static void frame(void* user)
{
    float* value = (float*)user;
    ImGui_ShowDemoWindow();
    ImGui_Begin("Hello");
    ImGui_SliderFloat("Value", value, 0.0f, 1.0f);
    ImGui_Text("%.2f", *value);
    ImGui_End();
}

int main(void)
{
    float value = 0.5f;
    ImGui_CreateContext();
    cImGui_ImplBUI_App* app = cImGui_ImplBUI_CreateApp("Hello", 1280, 800, frame, &value, NULL);
    cImGui_ImplBUI_RunApp(app);
    cImGui_ImplBUI_DestroyApp(app);
    ImGui_DestroyContext();
}
```

Link `bui::cimgui` (CMake). `cimgui_impl_bui.h` is the backend for C: apps (window or
headless), `cImGui_ImplBUI_Init(node, frame, user, config)` for a node in an existing BUI
UI, `cImGui_ImplBUI_StyleBlender`, the UI scale, and headless input helpers. Example:
`examples/imgui_demo_c.c` (`bui_imgui_demo_c`, `--screenshot out.png` for headless).

## Python

```python
import blenderui.imgui as imgui

state = {"value": 0.5, "text": "Edit me", "opened": True}

def frame():
    imgui.show_demo_window()
    expanded, state["opened"] = imgui.begin("Hello", state["opened"])
    changed, state["value"] = imgui.slider_float("Value", state["value"], 0.0, 1.0)
    changed, state["text"] = imgui.input_text("Text", state["text"])
    if imgui.button("Reset"):
        state["value"] = 0.5
    imgui.text("value %.2f" % state["value"])
    imgui.end()

imgui.run(frame, title="Hello", size=(1280, 800))
```

pyimgui-style:

- functions in snake_case; an in/out pointer is an argument, and its new value comes back
  after the result: `changed, v = imgui.slider_float(...)`,
  `changed, (r, g, b) = imgui.color_edit3("Color", (r, g, b))`; `None` passes `NULL`
  (`imgui.begin("Window")` has no close button). Pure outputs are returned:
  `h, s, v = imgui.color_convert_rgb_to_hsv(1, 0, 0)`.
- `ImVec2` / `ImVec4` arguments are tuples; results are `imgui.Vec2` / `imgui.Vec4` named
  tuples (`pos.x`).
- printf-style functions take one string: `imgui.text("%d items" % n)`.
- lists replace (pointer, count) pairs: `imgui.combo("Combo", current, ["a", "b"])`,
  `imgui.plot_lines("Plot", values)`, `draw_list.add_polyline(points, col)`.
- `input_text*` take and return a `str` of any length (the buffer grows).
- one name per overload set, trying each overload in turn: `imgui.push_id("x")`,
  `imgui.push_id(3)`; each also keeps its own name (`push_id_int`, `push_style_var_vec2`).
- enums: `imgui.WINDOW_NO_TITLE_BAR` and the C++ name `imgui.ImGuiWindowFlags_NoTitleBar`.
- structs are objects whose fields have snake_case and C++ names:
  `imgui.get_io().display_size`, `io.config_flags |= imgui.CONFIG_NAV_ENABLE_KEYBOARD`,
  `io.ini_filename = None`, `imgui.get_style().colors[imgui.COL_TEXT] = (1, 1, 1, 1)`,
  `io.fonts.fonts[0]`; arrays and `ImVector`s are indexable views.
- methods: `imgui.get_window_draw_list().add_line(p1, p2, imgui.im_col32(255, 0, 0))`.
- constructors: `imgui.ImGuiListClipper()`, `imgui.ImGuiTextFilter("x")`,
  `imgui.ImGuiStorage()` (deleted with the Python object).
- `imgui.run(frame, title=..., size=...)` opens a BUI window (creating the ImGui context
  if none is current); an exception in `frame` closes the window and propagates.
  `imgui.HeadlessApp` renders without a window ([below](#headless-rendering-and-tests)).
- `help(imgui.begin)` shows the Python signature and the C++ function.

The extension `_buiimgui` is built next to `buipy` (`cmake --build build --target
_buiimgui`) and installed with the wheel.

## Rust

`blenderui-imgui` is on [crates.io](https://crates.io/crates/blenderui-imgui) (`cargo add blenderui-imgui`);
you can also take it from git, or by path from a clone. Add `features = ["blender-widgets"]` for Blender's widgets.

```toml
[dependencies]
blenderui-imgui = "0.2"
# or from git: blenderui-imgui = { git = "https://github.com/ppmpreetham/BUI" }
# or, from a clone: blenderui-imgui = { path = "bindings/rust/blenderui-imgui" }
```

```rust
use blenderui_imgui as imgui;

fn main() {
    let mut value = 0.5f32;
    let mut text = String::from("Edit me");
    imgui::run("Hello", [1280, 800], move || {
        imgui::show_demo_window();
        imgui::begin("Hello");
        imgui::slider_float("Value", &mut value, 0.0, 1.0);
        imgui::input_text("Text", &mut text);
        if imgui::button("Reset") {
            value = 0.5;
        }
        imgui::text(&format!("value {value:.2}"));
        if let Some(dl) = imgui::get_window_draw_list() {
            let p = imgui::get_cursor_screen_pos();
            dl.add_circle_filled([p.x + 10.0, p.y + 10.0], 8.0, imgui::im_col32(71, 114, 179, 255));
        }
        imgui::end();
    });
}
```

`cargo run -p blenderui-imgui --example demo` (`-- --screenshot demo.png` headless).

- `imgui::sys`: the raw FFI, generated from the same description as `cimgui.h` (no
  bindgen or libclang): every struct with its layout (`ImVector<T>`, unions, bit fields as
  storage words), every enum value as a constant, every C function. `sys` is re-exported
  at the crate root, so `imgui::ImGuiWindowFlags_NoTitleBar` and `imgui::ImVec2` work.
- the safe layer: `button(label)` / `button_ex(label, size)` like C; `&str` for strings,
  `&mut T` for in/out values (`Option<&mut T>` where C++ accepts `NULL`), `&mut [f32; 3]`
  for arrays, slices for (pointer, count) pairs, `&mut String` for `input_text*`,
  `impl Into<ImVec2>` (`[x, y]`, `(x, y)`) for vectors; methods on the structs
  (`draw_list.add_line(...)`, `clipper.step()`); constructors return
  `Owned<T>` (dropped with `X_Delete`): `ImGuiTextFilter::new_ex("x")`.
- references returned by imgui (`get_io()`, `get_style()`, draw lists) are `'static` in
  the signatures but only valid while their context lives.
- `imgui::run(title, size, frame)`, `imgui::App::new / headless / with_config`:
  creates the ImGui context if none is current; a panic in `frame` closes the window and
  resumes from `run()` / `render()`.

The crate depends on `blenderui-sys` with its `imgui` feature, which builds Dear ImGui,
the BUI backend and the C API with the project's CMake files (as for the C kit; a published
crate carries them in `csrc/`, see `scripts/vendor_c.py`). Nothing else is downloaded.

## Images as textures

ImGui's user textures are BUI textures. Decode any image the kit reads and
get its id: Python `tex, w, h = blenderui.image_texture(data)` then
`imgui.image(tex, (w, h))` and `blenderui.texture_free(tex)`; Rust
`blenderui::Image::open(path)?.texture()`; C `BK_imbuf_texture(ib, 0)`.
Animated GIF and WebP: Python `blenderui.AnimatedTexture(data)` (call
`update()` each frame, draw `texture`), Rust `blenderui::Animation` with
`Image::update_texture`, C `BK_anim_next` with `BK_imbuf_texture_update`.

## Headless rendering and tests

All three bindings can run Dear ImGui without a window: frames render into an offscreen
BUI raster and input is fed the way the window hosts feed it.

```python
with imgui.HeadlessApp(frame, size=(800, 600)) as app:
    app.render(3)                 # request and run 3 frames
    app.click(100, 40)            # mouse_move / mouse_down / mouse_up / scroll
    app.type_text("hello")        # key("Return"), key("a", imgui.BK_CTRL)
    w, h, pixels = app.pixels()   # BGRA bytes; app.save_png("shot.png")
```

C: `cImGui_ImplBUI_CreateHeadlessApp`, `cImGui_ImplBUI_Render`, `cImGui_ImplBUI_Click`,
`cImGui_ImplBUI_TypeText`, `cImGui_ImplBUI_Pixels`. Rust: `App::headless(size, frame)`,
`app.render(n)`, `app.click(x, y)`, `app.type_text(..)`, `app.pixels()`.

Tests:

| Test | Runs |
|---|---|
| `imgui_c_api` (ctest) | `bindings/imgui/tests/test_cimgui.c`: widgets, tables, list clipper, draw list, popups, IO/style structs, helpers, text filter/storage/text buffer, clicks, a slider, typing |
| `buipy_imgui` (ctest) | `bindings/python/tests/test_imgui.py` (unittest): the same from Python, plus drags, the wheel, buffer growth, overload dispatch, struct fields and array views, the demo/metrics/style windows, errors |
| `cargo test -p blenderui-imgui` | `tests/headless.rs`: Rust/C++ struct sizes, widgets and input, owned objects, a panic in the frame |
| `imgui_backend` (ctest) | the backend itself, in C++ (phase 1); with Blender widgets also their interaction (drag steps, Shift, Ctrl snapping, arrows, click / double-click / Ctrl+click to type, relative sliders, drag-toggle) |
| `imgui_blender_widgets_match_kit` (ctest, `BUI_IMGUI_BLENDER_WIDGETS=ON`) | `examples/imgui_blender_compare.cpp`: ImGui's Blender widgets and the kit's native widgets are the same pixels |

The C, Python and C++ tests run in both widget variants; expectations that differ with
Blender widgets (sliders do not jump to the pointer, a click types a value) are marked
"variant" in them.

## Blender widgets

Built with `-DBUI_IMGUI_BLENDER_WIDGETS=ON` (Rust: `blenderui-imgui` feature
`blender-widgets`), Dear ImGui's widgets are Blender's widgets: drawn by the BUI kit with
its own widget drawing and handled with Blender's rules where they differ (number fields,
sliders, checkboxes). Nothing changes in the API; the switch is also available at runtime:

| | C++ | C | Python | Rust |
|---|---|---|---|---|
| built with them | `ImGui_ImplBUI_HasBlenderWidgets()` | `cImGui_ImplBUI_HasBlenderWidgets()` | `imgui.has_blender_widgets()` | `has_blender_widgets()` |
| in use | `ImGui_ImplBUI_GetBlenderWidgets()` | `cImGui_ImplBUI_GetBlenderWidgets()` | `imgui.get_blender_widgets()` | `blender_widgets()` |
| switch | `ImGui_ImplBUI_SetBlenderWidgets(on)` | `cImGui_ImplBUI_SetBlenderWidgets(on)` | `imgui.set_blender_widgets(on)` | `set_blender_widgets(on)` |

The widget mapping, the behavior differences and the gaps are in
[bindings/imgui/blender/README.md](../bindings/imgui/blender/README.md).

## Coverage

Counted from `bindings/imgui/gen/coverage.json` (written by the generator). Dear ImGui
1.93.1 WIP has **399 functions in `namespace ImGui`** (overloads counted separately,
obsolete ones excluded; 14 of them are `...V` functions taking a `va_list`) and **264
methods and constructors** on its structs (223 methods, 41 constructors); the bindings
add a default constructor to the 9 structs that declare none.

| | `namespace ImGui` | Methods + constructors | Notes |
|---|---|---|---|
| C | 399 / 399 | 273 / 273 | 894 C functions with the `Ex` variants; all 41 structs with their layout, 5 opaque; 737 enum values |
| Python | 366 / 399 (366 / 385 without `va_list`) | 264 / 273 | 1474 constants (both spellings); fields of all structs |
| Rust FFI | 385 / 399 (all but `va_list`) | 272 / 273 | 879 of the 894 C functions |
| Rust safe layer | 366 / 399 | 264 / 273 | the rest is in `sys` |

Not in Python or the Rust safe layer (all are in C and Rust `sys`):

- the 14 `...V` functions taking a `va_list` (use the `...` ones, or format first);
- functions taking an untyped `void*` value: `PushID(const void*)`, `GetID(const void*)`,
  `TreeNode/TreeNodeEx/TreePush(const void*, ...)`, the 8 `*Scalar` / `*ScalarN` widgets
  (`DragScalar`, `SliderScalar`, `InputScalar`... use the typed `drag_float`, `input_int`
  ...), `MemFree`, `ImGuiStorage::SetVoidPtr`, `ImGuiStoragePair(key, void*)`,
  `ImFontAtlas::AddFontFromMemoryTTF` / `AddFontFromMemoryCompressedTTF` (use
  `add_font_from_file_ttf`);
- functions taking C callbacks: the getter overloads of `Combo`, `ListBox`, `PlotLines`,
  `PlotHistogram` (the list overloads are bound), `SetAllocatorFunctions`,
  `GetAllocatorFunctions`, `ImDrawList::AddCallback`, and `InputText`'s user callback
  (its buffer callback is used internally to grow strings);
- `ImFontGlyphRangesBuilder::AddRanges` / `BuildRanges`,
  `ImGuiSelectionBasicStorage::GetNextSelectedItem` (pointer-to-pointer).

## Regenerating

```sh
py -3 -I bindings/imgui/gen/gen.py
```

The generator is stdlib-only Python (`gen.py`, with `gen_python.py` and `gen_rust.py`). It
parses `bindings/imgui/extern/imgui/imgui.h` (comments, preprocessor conditionals, the
`IMGUI_API` declarations in `namespace ImGui`, enums and flags, typedefs, structs with
their fields, methods and constructors, default arguments, overloads, `fmt, ...`,
arrays, callbacks) into `bindings/imgui/gen/imgui_api.json`, then writes:

| File | |
|---|---|
| `bindings/imgui/c/cimgui.h`, `cimgui.cpp` | the C API |
| `bindings/python/src/buiimgui_gen.inc` | the Python functions, types, field tables and constants |
| `bindings/rust/blenderui-imgui/src/sys.rs`, `safe.rs` | the Rust FFI and safe layer |
| `bindings/imgui/gen/coverage.json` | what each language binds (and why not) |

Hand-written: `cimgui_impl_bui.h/.cpp` (the backend's C API), the Python runtime
(`buiimgui_rt.h`: argument parsing, conversions, struct wrappers, array views) and module
(`buiimgui_module.cpp`: the App type, init), `bui/imgui.py` (`run`, `HeadlessApp`),
the Rust `lib.rs` (helpers, `Owned`, the runner), examples and tests.

After updating the vendored Dear ImGui, run the generator and rebuild: the C++ compiler
checks the C struct layouts (`static_assert`s in `cimgui.cpp`) and
`imgui::sys::layout_mismatches()` the Rust ones.

## Limitations

- Dear ImGui's single current context: create apps and call imgui from one thread.
  Python and Rust references to imgui objects (draw lists, IO, fonts) are borrowed: do not
  keep them past their context.
- Python objects wrapping imgui structs do not keep the context alive; owned objects
  (`imgui.ImGuiListClipper()`...) are deleted with the Python object.
- In release builds `IM_ASSERT` is off and Dear ImGui recovers from most API misuse
  (`io.ConfigErrorRecovery`); with assertions on, misuse aborts the process from every
  language.
- The backend's own gaps (IME placement, gamepads, multi-viewports) are listed in
  [bindings/imgui/README.md](../bindings/imgui/README.md#known-gaps).
