/* Color picker, eyedropper, color ramp, curve mapping, normal ball and
 * histogram.
 *
 * The picker follows interface_region_color_picker.cc: an HSV wheel (or
 * SV square + hue strip) with a value strip, RGB/HSV/HSL sliders, alpha,
 * hex and an eyedropper; it keeps its own HSV so hue survives grays. The
 * wheel geometry and cursor come from interface_widgets.cc
 * (hsvcircle_vals_from_pos / hsvcircle_pos_from_vals / hsv_cursor).
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "kit_intern.h"

/* Blender's color conversions (blenlib, linked from bui_core). */
void hsv_to_rgb_v(const float hsv[3], float r_rgb[3]);
void rgb_to_hsv_v(const float rgb[3], float r_hsv[3]);
void rgb_to_hsl_v(const float rgb[3], float r_hsl[3]);
void hsl_to_rgb_v(const float hsl[3], float r_rgb[3]);

#ifndef M_PI
#  define M_PI 3.14159265358979323846
#endif

static int u(void)
{
  return bk_unit();
}

/* -------------------------------------------------------------------- */
/** \name Color picker popup
 * \{ */

typedef struct PickerData {
  BKPopupPlace place;
  BKNode *swatch;
  float hsv[3];
  float alpha;
  bool square;
  BKNode *area, *strip, *tabs, *sl[3], *alpha_sl, *hex, *dropper;
} PickerData;

#define PICKER(n) ((PickerData *)(n)->ext)

static void picker_rgba(const PickerData *p, float r_rgba[4])
{
  hsv_to_rgb_v(p->hsv, r_rgba);
  r_rgba[3] = p->alpha;
}

static void picker_sync(BKNode *picker)
{
  PickerData *p = PICKER(picker);
  float rgba[4];
  picker_rgba(p, rgba);
  const int mode = (int)p->tabs->value; /* 0 RGB, 1 HSV, 2 HSL */
  float vals[3];
  static const char *names[3][3] = {{"R", "G", "B"}, {"H", "S", "V"}, {"H", "S", "L"}};
  if (mode == 0) {
    memcpy(vals, rgba, sizeof(vals));
  }
  else if (mode == 1) {
    memcpy(vals, p->hsv, sizeof(vals));
  }
  else {
    float hsl[3];
    rgb_to_hsl_v(rgba, hsl);
    vals[0] = hsl[1] == 0.0f ? p->hsv[0] : hsl[0];
    vals[1] = hsl[1];
    vals[2] = hsl[2];
  }
  for (int i = 0; i < 3; i++) {
    bk_set_text(&p->sl[i]->label, names[mode][i]);
    p->sl[i]->value = vals[i];
    bk_invalidate(p->sl[i]);
  }
  p->alpha_sl->value = p->alpha;
  char hex[16];
  snprintf(hex, sizeof(hex), "#%02X%02X%02X", bk_iround(rgba[0] * 255), bk_iround(rgba[1] * 255),
           bk_iround(rgba[2] * 255));
  bk_set_text(&p->hex->text, hex);
  bk_invalidate(picker);
}

static void picker_apply(BKNode *picker)
{
  PickerData *p = PICKER(picker);
  float rgba[4];
  picker_rgba(p, rgba);
  BK_set_color(p->swatch, "set_value", rgba);
  picker_sync(picker);
}

static void picker_set_hsv(BKNode *picker, float h, float s, float v)
{
  PickerData *p = PICKER(picker);
  p->hsv[0] = h;
  p->hsv[1] = BK_CLAMP(s, 0.0f, 1.0f);
  p->hsv[2] = BK_CLAMP(v, 0.0f, 1.0f);
  picker_apply(picker);
}

static void picker_set_rgb(BKNode *picker, const float rgb[3])
{
  PickerData *p = PICKER(picker);
  float hsv[3];
  rgb_to_hsv_v(rgb, hsv);
  if (hsv[1] == 0.0f) {
    hsv[0] = p->hsv[0]; /* keep hue through grays (color_picker_rgb_to_hsv_compat) */
  }
  if (hsv[2] == 0.0f) {
    hsv[1] = p->hsv[1];
  }
  picker_set_hsv(picker, hsv[0], hsv[1], hsv[2]);
}

static void picker_slider_changed(BKNode *slider, void *data)
{
  BKNode *picker = (BKNode *)data;
  PickerData *p = PICKER(picker);
  const float a = (float)p->sl[0]->value, b = (float)p->sl[1]->value, c = (float)p->sl[2]->value;
  const int mode = (int)p->tabs->value;
  if (mode == 0) {
    const float rgb[3] = {a, b, c};
    picker_set_rgb(picker, rgb);
  }
  else if (mode == 1) {
    picker_set_hsv(picker, a, b, c);
  }
  else {
    const float hsl[3] = {a, b, c};
    float rgb[3];
    hsl_to_rgb_v(hsl, rgb);
    picker_set_rgb(picker, rgb);
  }
}

static void picker_alpha_changed(BKNode *slider, void *data)
{
  BKNode *picker = (BKNode *)data;
  PICKER(picker)->alpha = (float)slider->value;
  picker_apply(picker);
}

static void picker_tabs_changed(BKNode *tabs, void *data)
{
  picker_sync((BKNode *)data);
}

static void picker_hex_changed(BKNode *hex, void *data)
{
  BKNode *picker = (BKNode *)data;
  float c[4];
  if (bk_parse_hex(hex->text ? hex->text : "", c, PICKER(picker)->alpha)) {
    picker_set_rgb(picker, c);
  }
  else {
    picker_sync(picker);
  }
}

static void picker_eyedropper_pick(void *data, const float rgb[3])
{
  picker_set_rgb((BKNode *)data, rgb);
}

static void picker_dropper_clicked(BKNode *button, void *data)
{
  BKContext *ctx = bk_ctx(button);
  if (ctx) {
    bk_eyedropper_start(ctx, picker_eyedropper_pick, data);
  }
}

static int area_h(void)
{
  return bk_iround(7.5f * u());
}

static void picker_measure(BKNode *n, int *rw, int *rh)
{
  PickerData *p = PICKER(n);
  const int uu = u(), pad = 2 * bk_iround(0.2f * uu);
  int rows = 5 + ((p->swatch->flags & NF_ALPHA) ? 1 : 0); /* tabs, 3 sliders, (alpha), hex */
  int h = area_h() + (int)(1.0f * uu) + rows * uu;
  if (p->square) {
    h += (int)(1.2f * uu);
  }
  *rw = 10 * uu + 2 * pad;
  *rh = h + 2 * pad;
}

static void picker_arrange(BKNode *n)
{
  PickerData *p = PICKER(n);
  const int uu = u(), pad = 2 * bk_iround(0.2f * uu), iw = n->w - 2 * pad;
  int y = n->y + pad;
  const int ah = area_h();
  if (p->square) {
    bk_arrange(p->area, n->x + pad, y, iw, ah);
    y += ah + (int)(0.3f * uu);
    bk_arrange(p->strip, n->x + pad, y, iw, (int)(0.8f * uu));
    y += (int)(0.8f * uu);
  }
  else {
    const int sw = bk_iround(0.9f * uu), gap = bk_iround(0.5f * uu);
    bk_arrange(p->area, n->x + pad + (iw - sw - gap - ah) / 2, y, ah, ah);
    bk_arrange(p->strip, n->x + pad + iw - sw, y, sw, ah);
    y += ah;
  }
  y += (int)(0.4f * uu);
  bk_arrange(p->tabs, n->x + pad, y, iw, uu);
  y += uu + (int)(0.2f * uu);
  BKNode *col[4] = {p->sl[0], p->sl[1], p->sl[2], p->alpha_sl};
  const int ncol = (p->swatch->flags & NF_ALPHA) ? 4 : 3;
  p->alpha_sl->flags = (ncol == 4) ? (p->alpha_sl->flags | NF_VISIBLE) :
                                     (p->alpha_sl->flags & ~NF_VISIBLE);
  for (int i = 0; i < ncol; i++) {
    col[i]->corners = (i == 0 ? (CNR_TL | CNR_TR) : 0) | (i == ncol - 1 ? (CNR_BL | CNR_BR) : 0);
    bk_arrange(col[i], n->x + pad, y, iw, uu);
    y += uu;
  }
  y += (int)(0.2f * uu);
  bk_arrange(p->hex, n->x + pad, y, iw - uu - (int)(0.2f * uu), uu);
  bk_arrange(p->dropper, n->x + pad + iw - uu, y, uu, uu);
}

static void picker_draw(BKNode *n)
{
  BKRect r = bk_node_rect(n);
  bk_widget_menu_back(&r, CNR_ALL);
}

static void picker_free(BKNode *n)
{
  free(n->ext);
}

const BKClass bk_cls_picker = {.name = "picker",
                               .flags = CLS_INTERACTIVE | CLS_POPUP,
                               .free_ext = picker_free,
                               .measure = picker_measure,
                               .arrange = picker_arrange,
                               .draw = picker_draw};

static BKNode *picker_child(BKNode *picker, const BKClass *cls)
{
  BKNode *c = bk_node_new_cls(cls);
  bk_node_add(picker, c);
  return c;
}

BKNode *bk_picker_new(BKNode *swatch)
{
  BKNode *n = bk_node_new_cls(&bk_cls_picker);
  PickerData *p = (PickerData *)calloc(1, sizeof(PickerData));
  p->place.at_x = p->place.at_y = -100000;
  p->place.side_x = p->place.side_y = p->place.side_xl = -100000;
  p->swatch = swatch;
  p->square = (swatch->flags & NF_SQUARE) != 0;
  rgb_to_hsv_v(swatch->color, p->hsv);
  p->alpha = swatch->color[3];
  n->ext = p;
  p->area = picker_child(n, p->square ? &bk_cls_hsv_square : &bk_cls_hsv_circle);
  p->strip = picker_child(n, p->square ? &bk_cls_hue_strip : &bk_cls_value_strip);
  p->area->ext = n;
  p->strip->ext = n;
  p->tabs = picker_child(n, &bk_cls_enum);
  const char *modes[3] = {"RGB", "HSV", "HSL"};
  BK_set_strs(p->tabs, "items", modes, 3);
  p->tabs->value = 1; /* HSV */
  p->tabs->on_change = picker_tabs_changed;
  p->tabs->on_change_data = n;
  for (int i = 0; i < 3; i++) {
    p->sl[i] = picker_child(n, &bk_cls_slider);
    p->sl[i]->step = 0.01;
    p->sl[i]->on_change = picker_slider_changed;
    p->sl[i]->on_change_data = n;
  }
  p->alpha_sl = picker_child(n, &bk_cls_slider);
  bk_set_text(&p->alpha_sl->label, "A");
  p->alpha_sl->step = 0.01;
  p->alpha_sl->on_change = picker_alpha_changed;
  p->alpha_sl->on_change_data = n;
  p->hex = picker_child(n, &bk_cls_text);
  bk_set_text(&p->hex->label, "Hex");
  p->hex->on_change = picker_hex_changed;
  p->hex->on_change_data = n;
  p->dropper = picker_child(n, &bk_cls_button);
  p->dropper->icon = bk_icons_lookup("EYEDROPPER");
  bk_set_text(&p->dropper->desc, "Sample a color from the window");
  p->dropper->on_click = picker_dropper_clicked;
  p->dropper->on_click_data = n;
  picker_sync(n);
  return n;
}

BKNode *BK_picker_part(BKNode *picker, const char *name)
{
  if (picker->cls != &bk_cls_picker) {
    return NULL;
  }
  PickerData *p = PICKER(picker);
  if (strcmp(name, "area") == 0) {
    return p->area;
  }
  if (strcmp(name, "strip") == 0) {
    return p->strip;
  }
  if (strcmp(name, "tabs") == 0) {
    return p->tabs;
  }
  if (strncmp(name, "slider", 6) == 0) {
    const int i = atoi(name + 6);
    return (i >= 0 && i < 3) ? p->sl[i] : NULL;
  }
  if (strcmp(name, "alpha") == 0) {
    return p->alpha_sl;
  }
  if (strcmp(name, "hex") == 0) {
    return p->hex;
  }
  if (strcmp(name, "dropper") == 0) {
    return p->dropper;
  }
  return NULL;
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Picker parts: HSV wheel, SV square, hue strip, value strip
 * \{ */

static BKNode *part_picker(const BKNode *n)
{
  return (BKNode *)n->ext; /* the picker popup that owns this part */
}

static void hsv_cursor(float x, float y, const BKNode *part)
{
  /* interface_widgets.cc hsv_cursor(): a disc of the current color, 12 px
   * (20 px while pressed), outlines whose alpha follows the value. */
  const PickerData *p = PICKER(part_picker(part));
  float rgba[4];
  picker_rgba(p, rgba);
  const float radius = ((bk_pressed(part) ? 20.0f : 12.0f) * bk_scale_fac + bk_px()) * 0.5f;
  const float v = p->hsv[2];
  const float bg[4] = {0, 0, 0, v / 2.0f};
  const float fg[4] = {1, 1, 1, BK_MIN(1.0f - v + 0.2f, 0.8f)};
  const float col[4] = {rgba[0], rgba[1], rgba[2], 1.0f};
  bk_circle(x, y, radius + 1, bg);
  bk_circle(x, y, radius, fg);
  bk_circle(x, y, radius - 1, col);
}

static int part_drag(BKNode *n, BKEvent *e);

static int part_press(BKNode *n, BKEvent *e)
{
  return part_drag(n, e);
}

static int part_wheel(BKNode *n, BKEvent *e)
{
  /* Blender: wheel over the picker changes the value */
  BKNode *picker = part_picker(n);
  if (!picker) {
    return 0;
  }
  PickerData *p = PICKER(picker);
  picker_set_hsv(picker, p->hsv[0], p->hsv[1], p->hsv[2] + 0.05f * e->delta);
  return 1;
}

static void circle_geom(const BKNode *n, float *cx, float *cy, float *r)
{
  *r = BK_MIN(n->w, n->h) * 0.5f - 1.0f;
  *cx = n->x + n->w * 0.5f;
  *cy = n->y + n->h * 0.5f;
}

static int part_drag(BKNode *n, BKEvent *e)
{
  BKNode *picker = part_picker(n);
  if (!picker) {
    return 0;
  }
  PickerData *p = PICKER(picker);
  if (n->cls == &bk_cls_hsv_circle) {
    /* hsvcircle_vals_from_pos: hue from the angle (red at the bottom),
     * saturation from the distance; y is flipped for screen space */
    float cx, cy, r;
    circle_geom(n, &cx, &cy, &r);
    const float dx = e->x - cx, dy = cy - e->y;
    float hue = atan2f(dx, dy) / (2.0f * (float)M_PI) + 0.5f;
    hue -= floorf(hue);
    const float dist = sqrtf(dx * dx + dy * dy) / BK_MAX(1.0f, r);
    picker_set_hsv(picker, hue, BK_MIN(1.0f, dist), p->hsv[2]);
  }
  else if (n->cls == &bk_cls_hsv_square) {
    const float s = (e->x - n->x) / (float)BK_MAX(1, n->w);
    const float v = 1.0f - (e->y - n->y) / (float)BK_MAX(1, n->h);
    picker_set_hsv(picker, p->hsv[0], s, v);
  }
  else if (n->cls == &bk_cls_hue_strip) {
    const float h = BK_CLAMP((e->x - n->x) / (float)BK_MAX(1, n->w), 0.0f, 0.9999f);
    picker_set_hsv(picker, h, p->hsv[1], p->hsv[2]);
  }
  else {
    const float v = 1.0f - (e->y - n->y) / (float)BK_MAX(1, n->h);
    picker_set_hsv(picker, p->hsv[0], p->hsv[1], v);
  }
  return 1;
}

static void outline_rect(const BKNode *n)
{
  BuiWidgetBase wb = {{(float)n->x, (float)n->y, (float)n->w, (float)n->h}, 0, 0, CNR_ALL};
  bk_rgba(bk_theme.regular.outline, wb.outline);
  wb.pixelsize = 1.0f;
  BUI_raster_widget_base(bk_draw.r, &wb);
}

static void hsv_rgba(float h, float s, float v, float out[4])
{
  const float hsv[3] = {h - floorf(h), s, v};
  hsv_to_rgb_v(hsv, out);
  out[3] = 1.0f;
}

static void hsv_circle_draw(BKNode *n)
{
  BKNode *picker = part_picker(n);
  if (!picker) {
    return;
  }
  PickerData *p = PICKER(picker);
  float cx, cy, r;
  circle_geom(n, &cx, &cy, &r);
  BUI_raster_hsv_circle(bk_draw.r, cx, cy, r, p->hsv[2]);
  float oc[4];
  bk_rgba(bk_theme.regular.outline, oc);
  bk_ring(cx, cy, r, oc, 1.0f);
  /* hsvcircle_pos_from_vals: ang = 2 pi h + pi/2, (cos(-ang), sin(-ang)) */
  const float ang = 2.0f * (float)M_PI * p->hsv[0] + (float)M_PI_2;
  const float rad = BK_CLAMP(p->hsv[1], 0.0f, 1.0f) * r;
  hsv_cursor(cx + cosf(-ang) * rad, cy - sinf(-ang) * rad, n);
}

static void hsv_square_draw(BKNode *n)
{
  BKNode *picker = part_picker(n);
  if (!picker) {
    return;
  }
  PickerData *p = PICKER(picker);
  float hue[4];
  hsv_rgba(p->hsv[0], 1, 1, hue);
  const float white[4] = {1, 1, 1, 1}, black[4] = {0, 0, 0, 1};
  BUI_raster_gradient(bk_draw.r, n->x, n->y, n->w, n->h, white, hue, black, black);
  outline_rect(n);
  hsv_cursor(n->x + p->hsv[1] * n->w, n->y + (1.0f - p->hsv[2]) * n->h, n);
}

static void hue_strip_draw(BKNode *n)
{
  BKNode *picker = part_picker(n);
  if (!picker) {
    return;
  }
  const float seg = n->w / 6.0f;
  for (int i = 0; i < 6; i++) {
    float a[4], b[4];
    hsv_rgba(i / 6.0f, 1, 1, a);
    hsv_rgba((i + 1) / 6.0f, 1, 1, b);
    const int x0 = bk_iround(n->x + i * seg), x1 = bk_iround(n->x + (i + 1) * seg);
    BUI_raster_gradient(bk_draw.r, x0, n->y, x1 - x0, n->h, a, b, b, a);
  }
  outline_rect(n);
  hsv_cursor(n->x + PICKER(picker)->hsv[0] * n->w, n->y + n->h * 0.5f, n);
}

static void value_strip_draw(BKNode *n)
{
  BKNode *picker = part_picker(n);
  if (!picker) {
    return;
  }
  PickerData *p = PICKER(picker);
  float top[4];
  hsv_rgba(p->hsv[0], p->hsv[1], 1, top);
  const float black[4] = {0, 0, 0, 1};
  BUI_raster_gradient(bk_draw.r, n->x, n->y, n->w, n->h, top, top, black, black);
  outline_rect(n);
  hsv_cursor(n->x + n->w * 0.5f, n->y + (1.0f - p->hsv[2]) * n->h, n);
}

#define PART_CLASS(name_, draw_) \
  {.name = name_, .flags = CLS_INTERACTIVE, .cursor = BK_CURSOR_CROSSHAIR, .draw = draw_, \
   .press = part_press, .drag = part_drag, .wheel = part_wheel}

const BKClass bk_cls_hsv_circle = PART_CLASS("hsv_circle", hsv_circle_draw);
const BKClass bk_cls_hsv_square = PART_CLASS("hsv_square", hsv_square_draw);
const BKClass bk_cls_hue_strip = PART_CLASS("hue_strip", hue_strip_draw);
const BKClass bk_cls_value_strip = PART_CLASS("value_strip", value_strip_draw);

/** \} */

/* -------------------------------------------------------------------- */
/** \name Eyedropper (modal: the next click samples the window)
 * \{ */

typedef struct EyedropperData {
  void (*pick)(void *data, const float rgb[3]);
  void *data;
} EyedropperData;

static int eyedropper_press(BKNode *n, BKEvent *e)
{
  BKContext *ctx = bk_ctx(n);
  EyedropperData d = *(EyedropperData *)n->ext;
  float rgb[3];
  bool ok = false;
  if (ctx && ctx->raster) {
    const int w = BUI_raster_width(ctx->raster), h = BUI_raster_height(ctx->raster);
    if (e->x >= 0 && e->y >= 0 && e->x < w && e->y < h) {
      const uint32_t px = BUI_raster_pixels(ctx->raster)[(size_t)e->y * w + e->x];
      rgb[0] = ((px >> 16) & 255) / 255.0f;
      rgb[1] = ((px >> 8) & 255) / 255.0f;
      rgb[2] = (px & 255) / 255.0f;
      ok = true;
    }
  }
  if (ctx) {
    bk_end_modal(ctx); /* frees n */
  }
  if (ok) {
    d.pick(d.data, rgb);
  }
  return 1;
}

static void eyedropper_free(BKNode *n)
{
  free(n->ext);
}

const BKClass bk_cls_eyedropper = {.name = "eyedropper",
                                   .cursor = BK_CURSOR_CROSSHAIR,
                                   .free_ext = eyedropper_free,
                                   .press = eyedropper_press};

void bk_eyedropper_start(BKContext *ctx, void (*pick)(void *data, const float rgb[3]), void *data)
{
  BKNode *n = bk_node_new_cls(&bk_cls_eyedropper);
  EyedropperData *d = (EyedropperData *)calloc(1, sizeof(EyedropperData));
  d->pick = pick;
  d->data = data;
  n->ext = d;
  bk_begin_modal(ctx, n);
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Color ramp band (ui_draw_but_COLORBAND)
 * \{ */

#define RAMP_MAX 32
enum { INTERP_LINEAR, INTERP_EASE, INTERP_BSPLINE, INTERP_CONSTANT };

typedef struct RampData {
  float pos[RAMP_MAX];
  float col[RAMP_MAX][4];
  int n, active, interp, drag;
} RampData;

#define RAMP(n) ((RampData *)(n)->ext)

static void ramp_sorted(const RampData *d, int *order)
{
  for (int i = 0; i < d->n; i++) {
    order[i] = i;
  }
  for (int i = 1; i < d->n; i++) { /* insertion sort by position */
    const int k = order[i];
    int j = i - 1;
    while (j >= 0 && d->pos[order[j]] > d->pos[k]) {
      order[j + 1] = order[j];
      j--;
    }
    order[j + 1] = k;
  }
}

void BK_ramp_evaluate(BKNode *n, float t, float out[4])
{
  RampData *d = RAMP(n);
  if (!d->n) {
    out[0] = out[1] = out[2] = 0;
    out[3] = 1;
    return;
  }
  int order[RAMP_MAX];
  ramp_sorted(d, order);
  if (t <= d->pos[order[0]]) {
    memcpy(out, d->col[order[0]], sizeof(float[4]));
    return;
  }
  for (int i = 0; i + 1 < d->n; i++) {
    const int a = order[i], b = order[i + 1];
    if (t <= d->pos[b]) {
      if (d->interp == INTERP_CONSTANT) {
        memcpy(out, d->col[a], sizeof(float[4]));
        return;
      }
      float f = (t - d->pos[a]) / BK_MAX(1e-9f, d->pos[b] - d->pos[a]);
      if (d->interp != INTERP_LINEAR) {
        f = f * f * (3 - 2 * f);
      }
      for (int k = 0; k < 4; k++) {
        out[k] = d->col[a][k] + (d->col[b][k] - d->col[a][k]) * f;
      }
      return;
    }
  }
  memcpy(out, d->col[order[d->n - 1]], sizeof(float[4]));
}

static void ramp_init(BKNode *n)
{
  RampData *d = (RampData *)calloc(1, sizeof(RampData));
  d->n = 2;
  d->pos[1] = 1.0f;
  d->col[0][3] = 1.0f;
  d->col[1][0] = d->col[1][1] = d->col[1][2] = d->col[1][3] = 1.0f;
  d->drag = -1;
  n->ext = d;
}

static void ramp_free(BKNode *n)
{
  free(n->ext);
}

static void ramp_changed(BKNode *n)
{
  bk_invalidate(n);
  bk_emit(n, BK_EV_CHANGE, NULL, RAMP(n)->active, 0);
}

static int ramp_add(BKNode *n, float pos)
{
  RampData *d = RAMP(n);
  if (d->n == RAMP_MAX) {
    return d->active;
  }
  float c[4];
  BK_ramp_evaluate(n, pos, c);
  d->pos[d->n] = pos;
  memcpy(d->col[d->n], c, sizeof(c));
  d->active = d->n++;
  ramp_changed(n);
  return d->active;
}

static void ramp_remove(BKNode *n)
{
  RampData *d = RAMP(n);
  if (d->n <= 1) {
    return;
  }
  const int a = d->active;
  memmove(d->pos + a, d->pos + a + 1, sizeof(float) * (size_t)(d->n - a - 1));
  memmove(d->col + a, d->col + a + 1, sizeof(float[4]) * (size_t)(d->n - a - 1));
  d->n--;
  int order[RAMP_MAX];
  ramp_sorted(d, order);
  d->active = order[0];
  ramp_changed(n);
}

static int band_h(void)
{
  return bk_iround(1.1f * u());
}

static float ramp_pos_at(const BKNode *n, int x)
{
  return BK_CLAMP((x - n->x) / (float)BK_MAX(1, n->w), 0.0f, 1.0f);
}

static int ramp_press(BKNode *n, BKEvent *e)
{
  RampData *d = RAMP(n);
  const float pos = ramp_pos_at(n, e->x);
  if (EV_CTRL(e) || !d->n) { /* Ctrl+click: add a stop */
    d->drag = ramp_add(n, pos);
    return 1;
  }
  int best = 0;
  for (int i = 1; i < d->n; i++) {
    if (fabsf(d->pos[i] - pos) < fabsf(d->pos[best] - pos)) {
      best = i;
    }
  }
  d->active = best;
  d->drag = best;
  ramp_changed(n);
  return 1;
}

static int ramp_drag(BKNode *n, BKEvent *e)
{
  RampData *d = RAMP(n);
  if (d->drag < 0) {
    return 0;
  }
  float pos = ramp_pos_at(n, e->x);
  if (EV_CTRL(e)) {
    pos = floorf(pos * 10.0f + 0.5f) / 10.0f;
  }
  d->pos[d->drag] = pos;
  ramp_changed(n);
  return 1;
}

static int ramp_release(BKNode *n, BKEvent *e)
{
  RAMP(n)->drag = -1;
  return 1;
}

static int ramp_key(BKNode *n, BKEvent *e)
{
  if ((bk_keysym_is(e->keysym, "Delete") || bk_keysym_is(e->keysym, "x")) && !EV_CTRL(e)) {
    ramp_remove(n);
    return 1;
  }
  return 0;
}

static void ramp_measure(BKNode *n, int *rw, int *rh)
{
  *rw = 6 * u();
  *rh = bk_iround(1.6f * u());
}

static void ramp_draw(BKNode *n)
{
  RampData *d = RAMP(n);
  const int bh = band_h();
  bk_checker(n->x, n->y, n->w, bh);
  /* draw the band in thin vertical slices: exact for every interpolation */
  const int slice = 2;
  for (int x = 0; x < n->w; x += slice) {
    float c0[4], c1[4];
    BK_ramp_evaluate(n, (x + 0.5f) / n->w, c0);
    BK_ramp_evaluate(n, BK_MIN(n->w, x + slice + 0.5f) / n->w, c1);
    if (d->interp == INTERP_CONSTANT) {
      memcpy(c1, c0, sizeof(c0));
    }
    BUI_raster_gradient(bk_draw.r, n->x + x, n->y, BK_MIN(slice, n->w - x), bh, c0, c1, c1, c0);
  }
  outline_rect(&(BKNode){.x = n->x, .y = n->y, .w = n->w, .h = bh});
  const float uu = (float)u();
  int order[RAMP_MAX];
  ramp_sorted(d, order);
  for (int k = 0; k < d->n; k++) {
    const int i = order[k];
    const float sx = floorf(n->x + d->pos[i] * n->w + 0.5f);
    const bool active = i == d->active;
    const float dark[4] = {0, 0, 0, 0.6f}, white[4] = {1, 1, 1, 1}, black[4] = {0, 0, 0, 0.8f};
    bk_fill_f(sx - 1, n->y, 3, bh, dark);
    bk_fill_f(sx, n->y, 1, bh, active ? white : black);
    const float ty = n->y + bh + 1, ts = 0.28f * uu;
    const float tri[3][2] = {{sx - ts, ty + ts * 1.6f}, {sx + ts, ty + ts * 1.6f}, {sx, ty}};
    const float frame[4] = {active ? 1.0f : 0.1f, active ? 1.0f : 0.1f, active ? 1.0f : 0.1f, 1};
    bk_polygon(tri, 3, frame);
    const float inner[3][2] = {
        {sx - ts * 0.55f, ty + ts * 1.35f}, {sx + ts * 0.55f, ty + ts * 1.35f}, {sx, ty + ts * 0.5f}};
    const float c[4] = {d->col[i][0], d->col[i][1], d->col[i][2], 1};
    bk_polygon(inner, 3, c);
  }
}

static int ramp_set_num(BKNode *n, const char *key, double v)
{
  RampData *d = RAMP(n);
  if (strcmp(key, "interpolation") == 0) {
    d->interp = BK_CLAMP((int)v, 0, 3);
    bk_invalidate(n);
    return 1;
  }
  if (strcmp(key, "active") == 0) {
    d->active = BK_CLAMP((int)v, 0, BK_MAX(0, d->n - 1));
    bk_invalidate(n);
    return 1;
  }
  if (strcmp(key, "active_pos") == 0 && d->n) {
    d->pos[d->active] = BK_CLAMP((float)v, 0.0f, 1.0f);
    bk_invalidate(n);
    return 1;
  }
  if (strcmp(key, "add") == 0) {
    ramp_add(n, (float)v);
    return 1;
  }
  if (strcmp(key, "remove") == 0) {
    ramp_remove(n);
    return 1;
  }
  return 0;
}

static int ramp_get_num(BKNode *n, const char *key, double *r)
{
  RampData *d = RAMP(n);
  if (strcmp(key, "interpolation") == 0) {
    *r = d->interp;
    return 1;
  }
  if (strcmp(key, "active") == 0) {
    *r = d->active;
    return 1;
  }
  if (strcmp(key, "count") == 0) {
    *r = d->n;
    return 1;
  }
  return 0;
}

static int ramp_color_set(BKNode *n, const float c[4])
{
  RampData *d = RAMP(n);
  if (!d->n) {
    return 0;
  }
  memcpy(d->col[d->active], c, sizeof(float[4]));
  bk_invalidate(n);
  return 1;
}

const BKClass bk_cls_ramp = {.name = "ramp",
                             .flags = CLS_INTERACTIVE,
                             .init = ramp_init,
                             .free_ext = ramp_free,
                             .measure = ramp_measure,
                             .draw = ramp_draw,
                             .press = ramp_press,
                             .drag = ramp_drag,
                             .release = ramp_release,
                             .key = ramp_key,
                             .set_num = ramp_set_num,
                             .get_num = ramp_get_num};

int BK_ramp_count(BKNode *n)
{
  return RAMP(n)->n;
}

void BK_ramp_get(BKNode *n, int i, float *pos, float rgba[4])
{
  RampData *d = RAMP(n);
  if (i >= 0 && i < d->n) {
    *pos = d->pos[i];
    memcpy(rgba, d->col[i], sizeof(float[4]));
  }
}

void BK_ramp_set(BKNode *n, const float *pos, const float (*rgba)[4], int count)
{
  RampData *d = RAMP(n);
  d->n = BK_MIN(count, RAMP_MAX);
  for (int i = 0; i < d->n; i++) {
    d->pos[i] = BK_CLAMP(pos[i], 0.0f, 1.0f);
    memcpy(d->col[i], rgba[i], sizeof(float[4]));
  }
  d->active = 0;
  bk_invalidate(n);
}

int BK_ramp_active(BKNode *n)
{
  return RAMP(n)->active;
}

void BK_ramp_set_active_color(BKNode *n, const float rgba[4])
{
  ramp_color_set(n, rgba);
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Curve mapping (ui_draw_but_CURVE)
 * \{ */

#define CURVE_MAX_PTS 64

typedef struct CurveData {
  float pts[4][CURVE_MAX_PTS][2];
  int npts[4];
  int nchannels, channel, selected, drag;
} CurveData;

#define CURVE(n) ((CurveData *)(n)->ext)

static const float g_channel_colors[4][4] = {
    {0.9f, 0.9f, 0.9f, 1}, {0.95f, 0.35f, 0.35f, 1}, {0.4f, 0.85f, 0.4f, 1}, {0.45f, 0.55f, 1, 1}};

/* Sort points by x, then evaluate with Fritsch-Carlson monotone tangents
 * (smooth, never overshoots; like Blender's auto-clamped handles). */
static int curve_sorted(const CurveData *d, int ch, float out[][2])
{
  const int n = d->npts[ch];
  memcpy(out, d->pts[ch], sizeof(float[2]) * (size_t)n);
  for (int i = 1; i < n; i++) {
    float p[2] = {out[i][0], out[i][1]};
    int j = i - 1;
    while (j >= 0 && out[j][0] > p[0]) {
      out[j + 1][0] = out[j][0];
      out[j + 1][1] = out[j][1];
      j--;
    }
    out[j + 1][0] = p[0];
    out[j + 1][1] = p[1];
  }
  return n;
}

static float curve_eval(const CurveData *d, int ch, float x)
{
  float p[CURVE_MAX_PTS][2];
  const int n = curve_sorted(d, ch, p);
  if (n == 0) {
    return x;
  }
  if (x <= p[0][0]) {
    return p[0][1];
  }
  if (x >= p[n - 1][0] || n < 2) {
    return p[n - 1][1];
  }
  float dd[CURVE_MAX_PTS] = {0}, m[CURVE_MAX_PTS];
  for (int i = 0; i + 1 < n; i++) {
    dd[i] = (p[i + 1][1] - p[i][1]) / BK_MAX(1e-9f, p[i + 1][0] - p[i][0]);
  }
  m[0] = dd[0];
  m[n - 1] = dd[n - 2];
  for (int i = 1; i + 1 < n; i++) {
    m[i] = (dd[i - 1] + dd[i]) * 0.5f;
  }
  for (int i = 0; i + 1 < n; i++) {
    if (dd[i] == 0.0f) {
      m[i] = m[i + 1] = 0.0f;
    }
    else {
      const float a = m[i] / dd[i], b = m[i + 1] / dd[i], s = a * a + b * b;
      if (s > 9.0f) {
        const float t = 3.0f / sqrtf(s);
        m[i] = t * a * dd[i];
        m[i + 1] = t * b * dd[i];
      }
    }
  }
  for (int i = 0; i + 1 < n; i++) {
    if (x <= p[i + 1][0]) {
      const float h = BK_MAX(1e-9f, p[i + 1][0] - p[i][0]), t = (x - p[i][0]) / h;
      const float t2 = t * t, t3 = t2 * t;
      return (2 * t3 - 3 * t2 + 1) * p[i][1] + (t3 - 2 * t2 + t) * h * m[i] +
             (-2 * t3 + 3 * t2) * p[i + 1][1] + (t3 - t2) * h * m[i + 1];
    }
  }
  return p[n - 1][1];
}

static void curve_init(BKNode *n)
{
  CurveData *d = (CurveData *)calloc(1, sizeof(CurveData));
  d->nchannels = 1;
  for (int c = 0; c < 4; c++) {
    d->npts[c] = 2;
    d->pts[c][1][0] = d->pts[c][1][1] = 1.0f;
  }
  d->selected = d->drag = -1;
  n->ext = d;
  n->height_units = 8.0f;
}

static void curve_free(BKNode *n)
{
  free(n->ext);
}

static void curve_to_px(const BKNode *n, float x, float y, float *sx, float *sy)
{
  *sx = n->x + x * n->w;
  *sy = n->y + (1.0f - y) * n->h;
}

static void curve_changed(BKNode *n)
{
  bk_invalidate(n);
  bk_emit(n, BK_EV_CHANGE, NULL, CURVE(n)->channel, 0);
}

static int curve_press(BKNode *n, BKEvent *e)
{
  CurveData *d = CURVE(n);
  const int ch = d->channel;
  int near = -1;
  float best = 0.45f * u();
  for (int i = 0; i < d->npts[ch]; i++) {
    float sx, sy;
    curve_to_px(n, d->pts[ch][i][0], d->pts[ch][i][1], &sx, &sy);
    const float dist = hypotf(sx - e->x, sy - e->y);
    if (dist < best) {
      near = i;
      best = dist;
    }
  }
  if (near < 0) {
    const float fx = BK_CLAMP((e->x - n->x) / (float)BK_MAX(1, n->w), 0.0f, 1.0f);
    const float fy = BK_CLAMP(1.0f - (e->y - n->y) / (float)BK_MAX(1, n->h), 0.0f, 1.0f);
    float cx, cy;
    curve_to_px(n, fx, curve_eval(d, ch, fx), &cx, &cy);
    const bool on_curve = fabsf(cy - e->y) < 0.5f * u();
    if ((on_curve || EV_CTRL(e)) && d->npts[ch] < CURVE_MAX_PTS) {
      /* click the curve (or Ctrl+click anywhere) adds a point */
      near = d->npts[ch]++;
      d->pts[ch][near][0] = fx;
      d->pts[ch][near][1] = on_curve ? curve_eval(d, ch, fx) : fy;
      curve_changed(n);
    }
  }
  d->selected = near;
  d->drag = near;
  bk_invalidate(n);
  return 1;
}

static int curve_drag(BKNode *n, BKEvent *e)
{
  CurveData *d = CURVE(n);
  if (d->drag < 0) {
    return 0;
  }
  float fx = BK_CLAMP((e->x - n->x) / (float)BK_MAX(1, n->w), 0.0f, 1.0f);
  float fy = BK_CLAMP(1.0f - (e->y - n->y) / (float)BK_MAX(1, n->h), 0.0f, 1.0f);
  if (EV_CTRL(e)) { /* snap to the grid */
    fx = floorf(fx * 8.0f + 0.5f) / 8.0f;
    fy = floorf(fy * 8.0f + 0.5f) / 8.0f;
  }
  d->pts[d->channel][d->drag][0] = fx;
  d->pts[d->channel][d->drag][1] = fy;
  curve_changed(n);
  return 1;
}

static int curve_release(BKNode *n, BKEvent *e)
{
  CURVE(n)->drag = -1;
  return 1;
}

static void curve_remove_selected(BKNode *n)
{
  CurveData *d = CURVE(n);
  const int ch = d->channel, s = d->selected;
  if (s >= 0 && d->npts[ch] > 2) {
    memmove(d->pts[ch][s], d->pts[ch][s + 1], sizeof(float[2]) * (size_t)(d->npts[ch] - s - 1));
    d->npts[ch]--;
    d->selected = -1;
    curve_changed(n);
  }
}

static int curve_key(BKNode *n, BKEvent *e)
{
  if ((bk_keysym_is(e->keysym, "Delete") || bk_keysym_is(e->keysym, "x")) && !EV_CTRL(e)) {
    curve_remove_selected(n);
    return 1;
  }
  return 0;
}

static void curve_measure(BKNode *n, int *rw, int *rh)
{
  *rw = 6 * u();
  *rh = (int)(n->height_units * u());
}

static void curve_draw(BKNode *n)
{
  CurveData *d = CURVE(n);
  const float back[4] = {0.16f, 0.16f, 0.16f, 1};
  bk_roundbox(n->x, n->y, n->w, n->h, bk_widget_radius(&bk_theme.regular), CNR_ALL, back);
  bk_clip_push(n->x, n->y, n->w, n->h);
  for (int k = 1; k < 8; k++) {
    const float g[4] = {1, 1, 1, (k % 2 == 0) ? 0.1f : 0.05f};
    bk_fill_f(floorf(n->x + n->w * k / 8.0f + 0.5f), n->y, 1, n->h, g);
    bk_fill_f(n->x, floorf(n->y + n->h * k / 8.0f + 0.5f), n->w, 1, g);
  }
  for (int pass = 0; pass < 2; pass++) {
    for (int ch = 0; ch < d->nchannels; ch++) {
      const bool current = ch == d->channel;
      if ((pass == 0) == current) {
        continue; /* other channels first, dimmed */
      }
      float col[4];
      memcpy(col, g_channel_colors[d->nchannels > 1 ? ch : 0], sizeof(col));
      if (!current) {
        col[3] = 0.25f;
      }
      float px, py;
      curve_to_px(n, 0, curve_eval(d, ch, 0), &px, &py);
      const int steps = 96;
      for (int i = 1; i <= steps; i++) {
        const float x = i / (float)steps;
        float sx, sy;
        curve_to_px(n, x, curve_eval(d, ch, x), &sx, &sy);
        bk_line(px, py, sx, sy, col, current ? BK_MAX(1.2f, 0.08f * u()) : 1.0f);
        px = sx;
        py = sy;
      }
    }
  }
  const int ch = d->channel;
  const float r = 0.17f * u();
  for (int i = 0; i < d->npts[ch]; i++) {
    float sx, sy;
    curve_to_px(n, d->pts[ch][i][0], d->pts[ch][i][1], &sx, &sy);
    const float black[4] = {0, 0, 0, 1}, sel[4] = {1, 1, 1, 1}, grey[4] = {0.55f, 0.55f, 0.55f, 1};
    bk_circle(sx, sy, r + 1, black);
    bk_circle(sx, sy, r, i == d->selected ? sel : grey);
  }
  bk_clip_pop();
  outline_rect(n);
}

static int curve_set_num(BKNode *n, const char *key, double v)
{
  CurveData *d = CURVE(n);
  if (strcmp(key, "channels") == 0) {
    d->nchannels = BK_CLAMP((int)v, 1, 4);
    bk_invalidate(n);
    return 1;
  }
  if (strcmp(key, "channel") == 0) {
    d->channel = BK_CLAMP((int)v, 0, d->nchannels - 1);
    d->selected = -1;
    bk_invalidate(n);
    return 1;
  }
  if (strcmp(key, "remove_selected") == 0) {
    curve_remove_selected(n);
    return 1;
  }
  return 0;
}

static int curve_get_num(BKNode *n, const char *key, double *r)
{
  CurveData *d = CURVE(n);
  if (strcmp(key, "channel") == 0) {
    *r = d->channel;
    return 1;
  }
  if (strcmp(key, "selected") == 0) {
    *r = d->selected;
    return 1;
  }
  return 0;
}

const BKClass bk_cls_curve = {.name = "curve",
                              .flags = CLS_INTERACTIVE,
                              .init = curve_init,
                              .free_ext = curve_free,
                              .measure = curve_measure,
                              .draw = curve_draw,
                              .press = curve_press,
                              .drag = curve_drag,
                              .release = curve_release,
                              .key = curve_key,
                              .set_num = curve_set_num,
                              .get_num = curve_get_num};

int BK_curve_count(BKNode *n, int ch)
{
  return CURVE(n)->npts[BK_CLAMP(ch, 0, 3)];
}

void BK_curve_get(BKNode *n, int ch, int i, float xy[2])
{
  CurveData *d = CURVE(n);
  ch = BK_CLAMP(ch, 0, 3);
  if (i >= 0 && i < d->npts[ch]) {
    xy[0] = d->pts[ch][i][0];
    xy[1] = d->pts[ch][i][1];
  }
}

void BK_curve_set(BKNode *n, int ch, const float (*xy)[2], int count)
{
  CurveData *d = CURVE(n);
  ch = BK_CLAMP(ch, 0, 3);
  d->npts[ch] = BK_MIN(count, CURVE_MAX_PTS);
  memcpy(d->pts[ch], xy, sizeof(float[2]) * (size_t)d->npts[ch]);
  d->selected = -1;
  bk_invalidate(n);
}

float BK_curve_evaluate(BKNode *n, int ch, float x)
{
  return curve_eval(CURVE(n), BK_CLAMP(ch, 0, 3), x);
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Normal ball (UNITVEC) and histogram
 * \{ */

static void normal_init(BKNode *n)
{
  n->color[0] = n->color[1] = 0;
  n->color[2] = 1;
  n->height_units = 4.0f;
}

static void normal_measure(BKNode *n, int *rw, int *rh)
{
  *rw = *rh = (int)(n->height_units * u());
}

static void normal_geom(const BKNode *n, float *cx, float *cy, float *r)
{
  *r = BK_MIN(n->w, n->h) * 0.5f - 2.0f;
  *cx = n->x + n->w * 0.5f;
  *cy = n->y + n->h * 0.5f;
}

static int normal_aim(BKNode *n, BKEvent *e)
{
  float cx, cy, r;
  normal_geom(n, &cx, &cy, &r);
  float nx = (e->x - cx) / r, ny = -(e->y - cy) / r;
  const float d = hypotf(nx, ny);
  if (d > 1.0f) {
    nx /= d;
    ny /= d;
  }
  const float v[4] = {nx, ny, sqrtf(BK_MAX(0.0f, 1.0f - nx * nx - ny * ny)), 1};
  memcpy(n->color, v, sizeof(v));
  bk_invalidate(n);
  bk_emit(n, BK_EV_CHANGE, NULL, 0, 0);
  return 1;
}

static void normal_draw(BKNode *n)
{
  float cx, cy, r;
  normal_geom(n, &cx, &cy, &r);
  const float rim[4] = {0.1f, 0.1f, 0.1f, 1};
  bk_circle(cx, cy, r + 1.5f, rim);
  const float base[4] = {0.75f, 0.75f, 0.75f, 1}, dis[4] = {0.4f, 0.4f, 0.4f, 1};
  BUI_raster_sphere(bk_draw.r, cx, cy, r, n->color, NODE_ENABLED(n) ? base : dis);
  if (bk_hovered(n)) {
    float c[4];
    bk_rgba_alpha(bk_theme.text, 0.4f, c);
    bk_ring(cx, cy, r + 1.5f, c, 1.0f);
  }
}

const BKClass bk_cls_normal = {.name = "normal",
                               .flags = CLS_INTERACTIVE,
                               .cursor = BK_CURSOR_CROSSHAIR,
                               .init = normal_init,
                               .measure = normal_measure,
                               .draw = normal_draw,
                               .press = normal_aim,
                               .drag = normal_aim};

typedef struct HistData {
  float *bins;
  int n, channels;
} HistData;

static void hist_init(BKNode *n)
{
  n->ext = calloc(1, sizeof(HistData));
  n->height_units = 5.0f;
}

static void hist_free(BKNode *n)
{
  HistData *h = (HistData *)n->ext;
  free(h->bins);
  free(h);
}

void BK_histogram_set(BKNode *n, const float *bins, int count, int channels)
{
  HistData *h = (HistData *)n->ext;
  free(h->bins);
  channels = BK_CLAMP(channels, 1, 3);
  h->bins = (float *)malloc(sizeof(float) * (size_t)BK_MAX(1, count * channels));
  memcpy(h->bins, bins, sizeof(float) * (size_t)(count * channels));
  h->n = count;
  h->channels = channels;
  bk_invalidate(n);
}

static void hist_measure(BKNode *n, int *rw, int *rh)
{
  *rw = 6 * u();
  *rh = (int)(n->height_units * u());
}

static void hist_draw(BKNode *n)
{
  HistData *h = (HistData *)n->ext;
  const float back[4] = {0.12f, 0.12f, 0.12f, 1};
  bk_roundbox(n->x, n->y, n->w, n->h, bk_widget_radius(&bk_theme.regular), CNR_ALL, back);
  bk_clip_push(n->x, n->y, n->w, n->h);
  for (int k = 1; k < 4; k++) {
    const float g[4] = {1, 1, 1, 0.06f};
    bk_fill_f(floorf(n->x + n->w * k / 4.0f + 0.5f), n->y, 1, n->h, g);
    bk_fill_f(n->x, floorf(n->y + n->h * k / 4.0f + 0.5f), n->w, 1, g);
  }
  if (h->n >= 2) {
    float(*pts)[2] = (float(*)[2])malloc(sizeof(float[2]) * (size_t)(h->n + 2));
    for (int ch = 0; ch < h->channels; ch++) {
      const float *col = h->channels == 3 ? g_channel_colors[ch + 1] : g_channel_colors[0];
      pts[0][0] = n->x;
      pts[0][1] = n->y + n->h;
      for (int i = 0; i < h->n; i++) {
        const float v = BK_CLAMP(h->bins[i * h->channels + ch], 0.0f, 1.0f);
        pts[i + 1][0] = n->x + n->w * i / (float)(h->n - 1);
        pts[i + 1][1] = n->y + n->h * (1.0f - v);
      }
      pts[h->n + 1][0] = n->x + n->w;
      pts[h->n + 1][1] = n->y + n->h;
      const float fill[4] = {col[0], col[1], col[2], 0.35f};
      const float line[4] = {col[0], col[1], col[2], 0.9f};
      bk_polygon((const float(*)[2])pts, h->n + 2, fill);
      for (int i = 1; i < h->n; i++) {
        bk_line(pts[i][0], pts[i][1], pts[i + 1][0], pts[i + 1][1], line, 1.0f);
      }
    }
    free(pts);
  }
  bk_clip_pop();
  outline_rect(n);
}

const BKClass bk_cls_histogram = {.name = "histogram",
                                  .init = hist_init,
                                  .free_ext = hist_free,
                                  .measure = hist_measure,
                                  .draw = hist_draw};

/** \} */
