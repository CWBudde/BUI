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

  if (BK_window_supported()) {              /* Windows: native window */
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
| **Text** | `BUI_text_measure`, `BUI_raster_text`, `BUI_text_add_font(family, flags, data, len)` (register a font; needed on wasm) |
| **Specialised** | Canvas display list (`BK_canvas_set_item`), text area (`BK_textarea_*`), trees (`BK_tree_item_*`), menus (`BK_open_menu`, `BK_open_pie`), color ramps and curves, `BK_undo` / `BK_redo` |

Node types: `frame column row grid box panel tabs sidetabs scroll scrollbar
label separator progress button toggle checkbox radio decorator text search
hotkey number slider dropdown enum swatch popover menubutton menubar list
tree gridview canvas textarea image ramp curve normal histogram`.

Read the header for the full list. It is the reference: every function
has a one-line comment.

## Lower level: render sinks

The kit draws through `gpu/` into a `BuiRaster`. If you have your own
renderer (OpenGL, Vulkan, a game engine, Qt's `QPainter`), you can receive
BUI's primitives instead through a render sink. See
[../INTEGRATION.md](../INTEGRATION.md).
