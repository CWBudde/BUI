/* BUI kit from C: a modifier stack (Blender's Properties editor, Modifiers tab).
 *
 *   cmake -S . -B build -DBUI_BUILD_EXAMPLES=ON && cmake --build build --target bui_site_kit_app
 *   bui_site_kit_app                         (native window)
 *   bui_site_kit_app --screenshot out.png    (off-screen; BUI_THEME, BUI_UI_SCALE)
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <stdio.h>
#include <string.h>

#include "BUI_kit.h"
#include "GPU_raster.h"
#include "site_png.h"

static BKNode *add(BKNode *parent, const char *type)
{
  BKNode *n = BK_node_new(type);
  BK_node_add(parent, n, -1);
  return n;
}

/* Blender's property row: right-aligned label (40%) | value. */
static BKNode *prop(BKNode *parent, const char *label, BKNode *value)
{
  BKNode *row = add(parent, "split");
  BKNode *l = add(row, "label");
  BK_set_str(l, "text", label);
  BK_set_str(l, "text_align", "right");
  BK_set_num(l, "weight", 0.4);
  BK_node_add(row, value, -1);
  BK_set_num(value, "weight", 0.6);
  return row;
}

static BKNode *number(double value, double min, double max, int integer, const char *suffix)
{
  BKNode *n = BK_node_new("number");
  if (integer) {
    BK_set_num(n, "integer", 1);
  }
  BK_set_num(n, "min", min);
  BK_set_num(n, "max", max);
  BK_set_num(n, "step", integer ? 1 : 0.1);
  BK_set_num(n, "precision", integer ? 0 : 3);
  BK_set_num(n, "value", value);
  if (suffix) {
    BK_set_str(n, "suffix", suffix);
  }
  return n;
}

static BKNode *slider(double value, double min, double max)
{
  BKNode *s = BK_node_new("slider");
  BK_set_num(s, "min", min);
  BK_set_num(s, "max", max);
  BK_set_num(s, "step", 0.05);
  BK_set_num(s, "precision", 3);
  BK_set_num(s, "value", value);
  return s;
}

static BKNode *enumrow(const char **items, int n, int current)
{
  BKNode *e = BK_node_new("enum");
  BK_set_strs(e, "items", items, n);
  BK_set_num(e, "value", current);
  return e;
}

static BKNode *dropdown(const char **items, int n, int current)
{
  BKNode *e = BK_node_new("dropdown");
  BK_set_strs(e, "items", items, n);
  BK_set_num(e, "value", current);
  return e;
}

static BKNode *checkbox(BKNode *parent, const char *label, int on)
{
  BKNode *c = add(parent, "checkbox");
  BK_set_str(c, "label", label);
  BK_set_num(c, "value", on);
  return c;
}

/* A collapsible panel; returns its body (the panel's first child), where its rows go. */
static BKNode *panel(BKNode *parent, const char *title, const char *icon, BKNode **r_panel)
{
  BKNode *p = add(parent, "panel");
  BK_set_str(p, "text", title);
  BK_set_str(p, "icon", icon);
  if (r_panel) {
    *r_panel = p;
  }
  return BK_node_child(p, 0);
}

static void on_event(void *data, BKNode *node, int kind, void *ref, double a, double b)
{
  (void)data, (void)ref, (void)a, (void)b;
  if (kind == BK_EV_CLICK) {
    printf("clicked %s\n", BK_get_str(node, "text"));
  }
}

static BKNode *build(void)
{
  BKNode *root = BK_node_new("column");
  BK_set_num(root, "padding", 8);
  BK_set_num(root, "spacing", 2);

  BKNode *top = add(root, "row");
  BKNode *addb = add(top, "button");
  BK_set_str(addb, "text", "Add Modifier");
  BK_set_str(addb, "icon", "ADD");
  BKNode *opts = add(top, "button");
  BK_set_str(opts, "icon", "DOWNARROW_HLT");
  BK_set_num(opts, "weight", 0);

  BKNode *sub = panel(root, "Subdivision", "MOD_SUBSURF", NULL);
  const char *algo[] = {"Catmull-Clark", "Simple"};
  BK_node_add(sub, enumrow(algo, 2, 0), -1);
  prop(sub, "Levels Viewport", number(2, 0, 6, 1, NULL));
  prop(sub, "Render", number(3, 0, 6, 1, NULL));
  checkbox(sub, "Optimal Display", 1);

  BKNode *bevel = panel(root, "Bevel", "MOD_BEVEL", NULL);
  const char *affect[] = {"Vertices", "Edges"};
  BK_node_add(bevel, enumrow(affect, 2, 1), -1);
  const char *width_type[] = {"Offset", "Width", "Depth", "Percent", "Absolute"};
  prop(bevel, "Width Type", dropdown(width_type, 5, 0));
  prop(bevel, "Amount", number(0.02, 0, 1, 0, " m"));
  prop(bevel, "Segments", number(3, 1, 100, 1, NULL));
  prop(bevel, "Shape", slider(0.5, 0, 1));
  checkbox(bevel, "Harden Normals", 1);
  checkbox(bevel, "Clamp Overlap", 1);

  BKNode *array = panel(root, "Array", "MOD_ARRAY", NULL);
  const char *fit[] = {"Fixed Count", "Fit Length", "Fit Curve"};
  prop(array, "Fit Type", dropdown(fit, 3, 0));
  prop(array, "Count", number(6, 1, 1000, 1, NULL));
  checkbox(array, "Relative Offset", 1);
  prop(array, "Factor X", number(1.1, -10, 10, 0, NULL));
  checkbox(array, "Merge", 0);

  BKNode *weld;
  panel(root, "Weld", "AUTOMERGE_OFF", &weld);
  BK_set_num(weld, "collapsed", 1);

  BKNode *apply = add(root, "row");
  BKNode *b1 = add(apply, "button");
  BK_set_str(b1, "text", "Apply All");
  BKNode *b2 = add(apply, "button");
  BK_set_str(b2, "text", "Copy to Selected");
  return root;
}

int main(int argc, char **argv)
{
  const char *shot = NULL;
  for (int i = 1; i + 1 < argc; i++) {
    if (strcmp(argv[i], "--screenshot") == 0) {
      shot = argv[i + 1];
    }
  }
  BKNode *root = build();
  BKContext *ctx = BK_context_new(root);
  BKHost host = {0};
  host.emit = on_event;
  BK_context_set_host(ctx, &host);
  const int W = 420, H = 528;
  if (!shot && BK_window_supported()) {
    BKWindow *win = BK_window_new(ctx, "Modifiers", W, H);
    BK_window_show(win);
    BK_window_run();
    BK_window_free(win);
  }
  else {
    BK_set_scale(site_scale());
    const int w = (int)(W * BK_scale()), h = (int)(H * BK_scale());
    BuiRaster *r = BUI_raster_create(w, h);
    BK_render(ctx, r, w, h, 1);
    if (shot) {
      site_write_png(shot, BUI_raster_pixels(r), w, h);
    }
    BUI_raster_free(r);
  }
  BK_context_free(ctx);
  BK_node_free(root);
  return 0;
}
