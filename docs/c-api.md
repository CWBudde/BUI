# The C kit (`kit/include/BUI_kit.h`)

Everything the Python APIs draw is a tree of kit nodes. You can use the kit
directly from C: link `bui::kit` (it pulls in `bui::gpu` and `bui::core`).

```cmake
add_subdirectory(BUI)
target_link_libraries(myapp PRIVATE bui::kit)
```

## Hello from C

`examples/kit_hello.c` (`cmake --build build --target bui_kit_hello`, with
`-DBUI_BUILD_EXAMPLES=ON`):

```c
#include <stdio.h>
#include "BUI_kit.h"

static void on_event(void *data, BKNode *node, int kind, void *ref, double a, double b)
{
  if (kind == BK_EV_CLICK) {
    printf("clicked %s\n", BK_get_str(node, "text"));
  }
  else if (kind == BK_EV_CHANGE) {
    printf("%s = %g\n", BK_get_str(node, "label"), BK_get_num(node, "value"));
  }
}

int main(void)
{
  BKNode *root = BK_node_new("frame");
  BK_set_num(root, "padding", 8);

  BKNode *size = BK_node_new("slider");
  BK_set_str(size, "label", "Size");
  BK_set_num(size, "min", 0);
  BK_set_num(size, "max", 10);
  BK_set_num(size, "value", 2.5);
  BK_node_add(root, size, -1);              /* -1: append */
  BK_geom_set_manager(size, BK_GEOM_PACK);  /* Tk's packer */
  BK_geom_set(size, "fill", 1);             /* fill x */

  BKNode *apply = BK_node_new("button");
  BK_set_str(apply, "text", "Apply");
  BK_node_add(root, apply, -1);
  BK_geom_set_manager(apply, BK_GEOM_PACK);
  BK_geom_set(apply, "fill", 1);
  BK_geom_set(apply, "pady0", 6);

  BKContext *ctx = BK_context_new(root);
  BKHost host = {0};
  host.emit = on_event;                     /* the window fills in the rest */
  BK_context_set_host(ctx, &host);

  if (BK_window_supported()) {              /* Win32, X11, Cocoa (experimental) */
    BKWindow *win = BK_window_new(ctx, "BUI from C", 320, 120);
    BK_window_show(win);
    BK_window_run();
    BK_window_free(win);
  }
  else {                                    /* elsewhere: render to a raster */
    BuiRaster *r = BUI_raster_create(320, 120);
    BK_render(ctx, r, 320, 120, 1);
    BUI_raster_free(r);
  }
  BK_context_free(ctx);
  BK_node_free(root);
  return 0;
}
```

## Concepts

| Concept | API |
|---|---|
| **Nodes** | `BK_node_new(type)`, `BK_node_add(parent, child, index)`, `BK_node_remove`, `BK_node_child(_count)`, `BK_node_parent`, `BK_node_rect`, `BK_node_natural`, `BK_node_invalidate`, `BK_node_relayout`, `BK_node_free` |
| **Properties** | `BK_set_num / BK_set_str / BK_set_color / BK_set_strs` and `BK_get_*`, by name (`"value"`, `"text"`, `"label"`, `"min"`, `"items"`, `"enabled"` ...). `"set_value"` sets a value *and* notifies. |
| **Geometry** | `BK_geom_set_manager(node, BK_GEOM_PACK / GRID / PLACE)`, `BK_geom_set(node, key, v)` (`side`, `fill`, `expand`, `anchor`, `padx0`..., `row`, `column`, `sticky`, `x`, `relx`, `reqwidth` ...), `BK_frame_grid_weight`. Children of `frame` and `canvas` nodes. |
| **Context** | `BK_context_new(root)`: input state, popups, focus, undo. Input goes in with `BK_press`, `BK_release`, `BK_motion`, `BK_wheel`, `BK_key`, `BK_key_release`, `BK_button`, `BK_context_menu`; frames come out of `BK_render(ctx, raster, w, h, full)`, which returns the number of redrawn regions (0 means nothing changed). |
| **Host** | `BKHost`: `request` (schedule a render), `set_cursor`, `warp`, clipboard, `schedule` (timers), `emit` (events). A `BKWindow` provides all of them. |
| **Events** | `emit(data, node, kind, ref, a, b)`. Kinds: `BK_EV_CHANGE`, `CLICK`, `CALL` (menu item or hotkey), `RENAME`, `ACTIVATE`, `TOGGLE`, `BROWSE`, `SUBMIT`. With the `"events"` property you also get raw `PRESS` / `RELEASE` / `DOUBLE` / `MOTION` / `ENTER` / `LEAVE` / `KEY` / `KEYRELEASE` / `WHEEL` / `CONFIGURE`, plus `VIEW` and `SCROLL` for scrolling. |
| **Windows** | `BK_window_new`, `show`, `close`, `set_title`, `set_size`, `wm(key, a, b)` (position, topmost, alpha, resizable, minsize, state ...), `timer`, `run`, `pump`, `quit` |
| **GPU rendering** | `BUI_gpu_set_default(1)` (all new windows), `BK_window_set_gpu(win, 1)` (one window), `BK_window_gpu(win)` (1 once frames draw on the GPU). `BUI_GPU=1/0` overrides. For your own rasters: `BUI_raster_gpu_attach(r, hwnd_or_NULL, wait)`, `BUI_raster_gpu_poll`, `BUI_raster_is_gpu`. |
| **Theme** | `BK_set_theme("dark" / "light")` (Blender's Dark or Light preset; every context relayouts and redraws), `BK_theme_name()`, `BK_theme_color(key, rgba)` (`"region_bg"`, `"text"`, `"regular.inner"` ...). Without a call, `BUI_THEME=dark|light` picks the theme. `BK_set_scale` sets the UI scale. |
| **Text** | `BUI_text_measure`, `BUI_raster_text`, `BUI_text_add_font(family, flags, data, len)` (register a font; needed on wasm) |
| **Specialised** | Canvas display list (`BK_canvas_set_item`), text area (`BK_textarea_*`), trees (`BK_tree_item_*`), menus (`BK_open_menu`, `BK_open_pie`), color ramps and curves, `BK_undo` / `BK_redo` |
| **Custom nodes** | A `custom` node draws and handles input itself: `BK_custom_set(node, &cls, data)` with `event` (pointer with all three buttons and a grab, precise wheel via `BK_scroll`, keys and text while focused, key releases, focus), `frame` (called by `BK_render` when requested with `BK_custom_request_frame(node, ms)`: immediate-mode UIs build their frame there) and `draw` (raster + rect). `BK_custom_set_cursor`, `BK_node_context`. Dear ImGui runs on it: [../bindings/imgui](../bindings/imgui/README.md). |
| **File browser** | `BK_node_new("filebrowser")`: Blender's File Browser in any window. Strings `dir`, `file`, `button`, `search`; numbers `mode` (0 open, 1 save, 2 directory), `multiple`, `hidden`, `sort`, `invert`; list `filters` (`"Label\tpattern ..."`); read `state` and `result`. `BK_file_dialog(title, mode, dir, file, filters, n, multiple, &paths)` runs one in its own window and returns the number of chosen paths (free them with `BK_file_dialog_free`). |
| **Meshes and textures** | `BUI_texture_create/update/free` (RGBA8, shared by every raster and GPU device), `BUI_raster_mesh(r, vertices, n, indices, n_idx, flags, tex, clip, ox, oy)`: indexed triangles with position, UV and color, straight-alpha blending, per-call clip; the same pixels on the software raster, Direct3D 11 and OpenGL. |

Node types: `frame column row grid box panel tabs sidetabs scroll scrollbar
label separator progress button toggle checkbox radio decorator text search
hotkey number slider dropdown enum swatch popover menubutton menubar list
tree gridview canvas textarea image ramp curve normal histogram custom`.

Read the header for the full list. It is the reference: every function
has a one-line comment.

## Showing the kit in your own window

The kit always draws into a `BuiRaster`: the software raster, or Direct3D 11
/ OpenGL instances when the window is on the GPU. To put kit widgets inside
your own renderer or framework, render the context into a raster with
`BK_render(ctx, raster, w, h, full)`, upload or blit its pixels (32-bit
`0x00RRGGBB`, rows top-down), and feed input back with `BK_press`,
`BK_motion`, `BK_key` and friends. `bui.qt.BuiCanvas` does exactly this
inside a Qt widget.

## A different layer: render sinks

Render sinks (`api/bui_c_api.h`, `bui_set_render_sink`) are **not** how the
kit draws. They receive the triangles, lines, points and text runs of the
immediate-mode GPU seam (`gpu/intern/gpu_null.c`), which the Qt/QPainter
backend, the SVG recorder and `buipy.demo_scene` use. Kit widgets never pass
through a sink. See [../INTEGRATION.md](../INTEGRATION.md).
