/* BUI kit from C: a slider and a button in a native window.
 *
 *   cmake -S . -B build -DBUI_BUILD_EXAMPLES=ON && cmake --build build --target bui_kit_hello
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

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
  BK_node_add(root, size, -1);
  BK_geom_set_manager(size, BK_GEOM_PACK);
  BK_geom_set(size, "fill", 1); /* fill x */

  BKNode *apply = BK_node_new("button");
  BK_set_str(apply, "text", "Apply");
  BK_node_add(root, apply, -1);
  BK_geom_set_manager(apply, BK_GEOM_PACK);
  BK_geom_set(apply, "fill", 1);
  BK_geom_set(apply, "pady0", 6);

  BKContext *ctx = BK_context_new(root);
  BKHost host = {0};
  host.emit = on_event; /* callbacks; the window adds the rest */
  BK_context_set_host(ctx, &host);

  if (BK_window_supported()) { /* Win32, X11 or Cocoa (experimental) */
    BKWindow *win = BK_window_new(ctx, "BUI from C", 320, 120);
    BK_window_show(win);
    BK_window_run();
    BK_window_free(win);
  }
  else { /* no window system (headless): render a frame into a raster */
    BuiRaster *r = BUI_raster_create(320, 120);
    BK_render(ctx, r, 320, 120, 1);
    BUI_raster_free(r);
  }
  BK_context_free(ctx);
  BK_node_free(root);
  return 0;
}
