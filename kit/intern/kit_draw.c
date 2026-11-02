/* Drawing: Blender's default theme, the widget base and the widget draw
 * functions, ported from editors/interface/interface_widgets.cc (same names,
 * same math, same shape tables), rendered into a BuiRaster. Screen space is
 * y-down, so Blender's "ymin" (bottom) is our ymax.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <math.h>
#include <stdlib.h>
#include <string.h>

#include "kit_icons_data.h"
#include "kit_intern.h"

/* Blender's color math (blenlib, linked from bui_core). */
void rgb_to_hsl_v(const float rgb[3], float r_hsl[3]);
void hsl_to_rgb_v(const float hsl[3], float r_rgb[3]);

/* -------------------------------------------------------------------- */
/** \name Theme: Blender 4.x defaults (userdef_default_theme.c, ThemeUI)
 * \{ */

#define RGBA(v) {(uchar)((v) >> 24), (uchar)((v) >> 16), (uchar)((v) >> 8), (uchar)(v)}
#define WCOL(outline_, outline_sel_, inner_, inner_sel_, item_, text_, text_sel_, round_) \
  {RGBA(outline_), RGBA(outline_sel_), RGBA(inner_), RGBA(inner_sel_), RGBA(item_), \
   RGBA(text_), RGBA(text_sel_), 0, 0, 0, round_}

BKTheme bk_theme = {
    /* regular */ WCOL(0x3d3d3dff, 0x656565ff, 0x545454ff, 0x4772b3ff, 0x191919ff, 0xe6e6e6ff,
                       0xffffffff, 0.2f),
    /* tool */ WCOL(0x3d3d3dff, 0x656565ff, 0x545454ff, 0x4772b3ff, 0x191919ff, 0xe6e6e6ff,
                    0xffffffff, 0.2f),
    /* toggle */ WCOL(0x3d3d3dff, 0x656565ff, 0x545454ff, 0x4772b3ff, 0x191919ff, 0xe6e6e6ff,
                      0xffffffff, 0.2f),
    /* radio */ WCOL(0x3d3d3dff, 0x656565ff, 0x545454ff, 0x4772b3ff, 0xffffffff, 0xe6e6e6ff,
                     0xffffffff, 0.2f),
    /* option */ WCOL(0x3d3d3dff, 0x656565ff, 0x545454ff, 0x4772b3ff, 0xffffffff, 0xe6e6e6ff,
                      0xffffffff, 0.3f),
    /* text */ WCOL(0x3d3d3dff, 0x656565ff, 0x1d1d1dff, 0x181818ff, 0x4772b3ff, 0xe6e6e6ff,
                    0xffffffff, 0.2f),
    /* num */ WCOL(0x3d3d3dff, 0x656565ff, 0x545454ff, 0x656565ff, 0xe6e6e6ff, 0xe6e6e6ff,
                   0xffffffff, 0.2f),
    /* numslider */ WCOL(0x3d3d3dff, 0x656565ff, 0x545454ff, 0x999999ff, 0x4772b3ff,
                         0xe6e6e6ff, 0xffffffff, 0.2f),
    /* menu */ WCOL(0x3d3d3dff, 0x656565ff, 0x282828ff, 0x303030ff, 0xe6e6e6ff, 0xe6e6e6ff,
                    0xffffffff, 0.2f),
    /* pulldown */ WCOL(0x3d3d3d00, 0x3d3d3d00, 0x00000000, 0x4772b3ff, 0xffffffff,
                        0xd9d9d9ff, 0xffffffff, 0.2f),
    /* menu_back */ WCOL(0x242424ff, 0x242424ff, 0x181818efu, 0x2d2d2dff, 0x656565ff,
                         0xa5a5a5ff, 0xffffffff, 0.25f),
    /* menu_item */ WCOL(0x00000000, 0x00000000, 0x00000000, 0x4772b3ff, 0xffffff8f,
                         0xe6e6e6ff, 0xffffffff, 0.2f),
    /* list_item */ WCOL(0x00000000, 0x00000000, 0x00000000, 0x334d80ff, 0x4772b3ff,
                         0xe6e6e6ff, 0xffffffff, 0.2f),
    /* box */ WCOL(0x3d3d3dff, 0x656565ff, 0x282828ff, 0x4772b3ff, 0x191919ff, 0xe6e6e6ff,
                   0xffffffff, 0.2f),
    /* scroll */ WCOL(0x00000000, 0x00000000, 0x00000000, 0x646464b4, 0x545454ff, 0xe6e6e6ff,
                      0xffffffff, 0.5f),
    /* progress */ WCOL(0x3d3d3dff, 0x656565ff, 0x1d1d1dff, 0x4772b3ff, 0x4772b3ff,
                        0xe6e6e6ff, 0xffffffff, 0.2f),
    /* tooltip */ WCOL(0x3d3d3dff, 0x3d3d3dff, 0x1d1d1dff, 0x1d1d1dff, 0x1d1d1dff, 0xe6e6e6ff,
                       0xffffffff, 0.25f),
    /* tab */ WCOL(0x1d1d1dff, 0x1d1d1dff, 0x2a2a2aff, 0x3d3d3dff, 0x28292dff, 0x989898ff,
                   0xffffffff, 0.2f),
    /* pie_menu */ WCOL(0x0a0a0aff, 0x0a0a0aff, 0x181818ffu, 0x4772b3ff, 0x4772b3ff,
                        0xd9d9d9ff, 0xffffffff, 0.5f),
    RGBA(0x303030ff), /* region_bg */
    RGBA(0x181818ff), /* header_bg (top bar) */
    RGBA(0x1d1d1dff), /* tabs_bg (navigation bar) */
    RGBA(0x3d3d3dff), /* panel_header */
    RGBA(0x3d3d3dff), /* panel_back */
    RGBA(0x0000001fu), /* panel_sub_back */
    RGBA(0xffffff0du), /* panel_outline */
    RGBA(0xe6e6e6ff), /* text */
    RGBA(0xffffffff), /* text_hi */
    RGBA(0x00000026u), /* widget_emboss */
    RGBA(0x71a8ffff), /* text cursor */
    RGBA(0x4772b380u), /* text selection */
    0.4f,             /* menu_shadow_fac */
    6,                /* menu_shadow_width */
    0.4f,             /* panel_roundness */
};

float bk_scale_fac = 1.0f;
static char g_font_family[64] = "";

void BK_set_scale(float scale)
{
  bk_scale_fac = BK_CLAMP(scale, 0.5f, 4.0f);
}
float BK_scale(void)
{
  return bk_scale_fac;
}
int BK_unit(void)
{
  return bk_unit();
}
void BK_set_font(const char *family)
{
  strncpy(g_font_family, family ? family : "", sizeof(g_font_family) - 1);
}

int bk_unit(void)
{
  return (int)floorf(20.0f * bk_scale_fac + 0.5f); /* U.widget_unit */
}
float bk_px(void)
{
  return bk_scale_fac >= 1.5f ? 2.0f : 1.0f; /* U.pixelsize */
}
float bk_text_scale = 1.0f;

float bk_font_size(void)
{
  return floorf(11.0f * bk_scale_fac * bk_text_scale + 0.5f); /* UI_DEFAULT_TEXT_POINTS */
}

/* Password fields: every UTF-8 character shows as `mask`. */
int bk_mask_text(const char *s, int len, const char *mask, char *buf, int size)
{
  const int ml = (int)strlen(mask);
  int out = 0;
  for (int i = 0; i < len; i++) {
    if (((unsigned char)s[i] & 0xC0) != 0x80 && out + ml < size) {
      memcpy(buf + out, mask, (size_t)ml);
      out += ml;
    }
  }
  buf[out] = 0;
  return out;
}

float bk_masked_width(const char *s, int len, const char *mask, bool bold)
{
  if (!mask || !mask[0]) {
    return bk_text_width(s, len, bold);
  }
  char buf[1024];
  return bk_text_width(buf, bk_mask_text(s, len, mask, buf, sizeof(buf)), bold);
}
int bk_pad(void)
{
  return (int)floorf(0.4f * bk_unit() + 0.5f); /* UI_TEXT_MARGIN_X */
}
int bk_iround(float v)
{
  return (int)floorf(v + 0.5f);
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Clip stack & primitives
 * \{ */

BKDraw bk_draw;
static int g_clip_stack[32][4];
static int g_clip_n = 0;

static void clip_apply(void)
{
  const int *c = bk_draw.clip;
  BUI_raster_set_clip(bk_draw.r, c[0], c[1], BK_MAX(1, c[2] - c[0]), BK_MAX(1, c[3] - c[1]));
  if (c[2] <= c[0] || c[3] <= c[1]) {
    BUI_raster_set_clip(bk_draw.r, -10, -10, 1, 1); /* empty */
  }
}

void bk_clip_reset(int w, int h)
{
  g_clip_n = 0;
  bk_draw.clip[0] = 0;
  bk_draw.clip[1] = 0;
  bk_draw.clip[2] = w;
  bk_draw.clip[3] = h;
  clip_apply();
}

void bk_clip_push(int x, int y, int w, int h)
{
  if (g_clip_n < 32) {
    memcpy(g_clip_stack[g_clip_n], bk_draw.clip, sizeof(int[4]));
  }
  g_clip_n++;
  int *c = bk_draw.clip;
  c[0] = BK_MAX(c[0], x);
  c[1] = BK_MAX(c[1], y);
  c[2] = BK_MIN(c[2], x + w);
  c[3] = BK_MIN(c[3], y + h);
  clip_apply();
}

void bk_clip_pop(void)
{
  if (g_clip_n > 0) {
    g_clip_n--;
    if (g_clip_n < 32) {
      memcpy(bk_draw.clip, g_clip_stack[g_clip_n], sizeof(int[4]));
    }
  }
  clip_apply();
}

bool bk_clip_visible(const BKRect *r)
{
  const int *c = bk_draw.clip;
  return r->xmax > c[0] && r->xmin < c[2] && r->ymax > c[1] && r->ymin < c[3];
}

void bk_rgba(const uchar c[4], float o[4])
{
  o[0] = c[0] / 255.0f;
  o[1] = c[1] / 255.0f;
  o[2] = c[2] / 255.0f;
  o[3] = c[3] / 255.0f;
}

void bk_rgba_alpha(const uchar c[4], float alpha, float o[4])
{
  bk_rgba(c, o);
  o[3] *= alpha;
}

void bk_fill(float x, float y, float w, float h, const uchar c[4])
{
  float f[4];
  bk_rgba(c, f);
  BUI_raster_fill_rect(bk_draw.r, x, y, w, h, f);
}

void bk_fill_f(float x, float y, float w, float h, const float c[4])
{
  BUI_raster_fill_rect(bk_draw.r, x, y, w, h, c);
}

void bk_roundbox(float x, float y, float w, float h, float rad, int corners, const float c[4])
{
  BuiWidgetBase p = {{x, y, w, h}, rad, rad, corners};
  memcpy(p.inner1, c, sizeof(float[4]));
  memcpy(p.inner2, c, sizeof(float[4]));
  memcpy(p.outline, c, sizeof(float[4]));
  p.pixelsize = 1.0f;
  BUI_raster_widget_base(bk_draw.r, &p);
}

void bk_shadow(const BKRect *r, float rad)
{
  /* widget_softshadow(): inset by the outline, Blender's shadow width/alpha */
  const float px = bk_px();
  BUI_raster_shadow(bk_draw.r, r->xmin + px, r->ymin + px, bk_rw(r) - 2 * px,
                    bk_rh(r) - 2 * px, rad, bk_theme.menu_shadow_width * bk_scale_fac,
                    bk_theme.menu_shadow_fac);
}

void bk_line(float x0, float y0, float x1, float y1, const float c[4], float width)
{
  BUI_raster_line(bk_draw.r, x0, y0, x1, y1, width, c);
}

void bk_circle(float cx, float cy, float rad, const float c[4])
{
  BUI_raster_circle(bk_draw.r, cx, cy, rad, 0.0f, c);
}

void bk_ring(float cx, float cy, float rad, const float c[4], float width)
{
  BUI_raster_ellipse(bk_draw.r, cx, cy, rad, rad, width, c);
}

void bk_polygon(const float (*pts)[2], int n, const float c[4])
{
  BUI_raster_polygon(bk_draw.r, pts, n, c);
}

void bk_checker(float x, float y, float w, float h)
{
  /* UI_ALPHA_CHECKER_DARK / LIGHT, 8 px cells (scaled) */
  const float dark[4] = {100 / 255.0f, 100 / 255.0f, 100 / 255.0f, 1.0f};
  const float light[4] = {160 / 255.0f, 160 / 255.0f, 160 / 255.0f, 1.0f};
  const float cell = floorf(8.0f * bk_scale_fac + 0.5f);
  bk_fill_f(x, y, w, h, dark);
  for (float yy = y; yy < y + h; yy += cell) {
    const int row = (int)((yy - y) / cell);
    for (float xx = x + ((row & 1) ? cell : 0); xx < x + w; xx += 2 * cell) {
      bk_fill_f(xx, yy, fminf(cell, x + w - xx), fminf(cell, y + h - yy), light);
    }
  }
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Color utilities (interface_widgets.cc)
 * \{ */

static void color_blend_v3_v3(uchar cp[3], const uchar cpstate[3], const float fac)
{
  if (fac != 0.0f) {
    cp[0] = (uchar)((1.0f - fac) * cp[0] + fac * cpstate[0]);
    cp[1] = (uchar)((1.0f - fac) * cp[1] + fac * cpstate[1]);
    cp[2] = (uchar)((1.0f - fac) * cp[2] + fac * cpstate[2]);
  }
}

static void color_blend_v4_v4v4(uchar r_col[4], const uchar col1[4], const uchar col2[4],
                                const float fac)
{
  const int faci = (int)BK_CLAMP(fac * 255.0f + 0.5f, 0.0f, 255.0f);
  const int facm = 255 - faci;
  for (int k = 0; k < 4; k++) {
    r_col[k] = (uchar)((faci * col1[k] + facm * col2[k]) / 256);
  }
}

static int srgb_to_grayscale_byte(const uchar c[3])
{
  /* BLI srgb_to_grayscale_byte: Rec.709 luma weights */
  return (int)((255 * 0 + 54 * c[0] + 182 * c[1] + 19 * c[2]) / 255);
}

static void color_mul_hsl_v3(uchar ch[3], float h_factor, float s_factor, float l_factor)
{
  float rgb[3] = {ch[0] / 255.0f, ch[1] / 255.0f, ch[2] / 255.0f}, hsl[3];
  rgb_to_hsl_v(rgb, hsl);
  hsl[0] *= h_factor;
  hsl[1] *= s_factor;
  hsl[2] *= l_factor;
  hsl_to_rgb_v(hsl, rgb);
  for (int k = 0; k < 3; k++) {
    ch[k] = (uchar)BK_CLAMP(rgb[k] * 255.0f + 0.5f, 0.0f, 255.0f);
  }
}

static void shadecolors4(const uchar *color, short shadetop, short shadedown, uchar r_top[4],
                         uchar r_down[4])
{
  for (int k = 0; k < 3; k++) {
    r_top[k] = (uchar)BK_CLAMP(color[k] + shadetop, 0, 255);
    r_down[k] = (uchar)BK_CLAMP(color[k] + shadedown, 0, 255);
  }
  r_top[3] = r_down[3] = color[3];
}

void bk_widget_active_color(BKWCol *wcol)
{
  const bool dark = srgb_to_grayscale_byte(wcol->text) > srgb_to_grayscale_byte(wcol->inner);
  color_mul_hsl_v3(wcol->inner, 1.0f, 1.15f, dark ? 1.2f : 1.1f);
  color_blend_v4_v4v4(wcol->outline, wcol->outline, wcol->outline_sel, 0.5f);
  color_mul_hsl_v3(wcol->outline_sel, 1.0f, 1.15f, 1.15f);
  color_mul_hsl_v3(wcol->text, 1.0f, 1.15f, dark ? 1.25f : 0.8f);
}

static void widget_color_disabled(BKWCol *w)
{
  const float f = 0.5f; /* widget_alpha_factor() */
  w->outline[3] = (uchar)(w->outline[3] * f);
  w->outline_sel[3] = (uchar)(w->outline_sel[3] * f);
  w->inner[3] = (uchar)(w->inner[3] * f);
  w->inner_sel[3] = (uchar)(w->inner_sel[3] * f);
  w->item[3] = (uchar)(w->item[3] * f);
  w->text[3] = (uchar)(w->text[3] * f);
  w->text_sel[3] = (uchar)(w->text_sel[3] * f);
}

void bk_widget_state(BKWCol *wcol, const BKWCol *theme, int state)
{
  *wcol = *theme;
  if (state & ST_DISABLED) {
    widget_color_disabled(wcol);
  }
  if (state & ST_SELECT) {
    memcpy(wcol->inner, wcol->inner_sel, 4);
    memcpy(wcol->outline, wcol->outline_sel, 4);
    memcpy(wcol->text, wcol->text_sel, 3);
    const short t = wcol->shadetop;
    wcol->shadetop = wcol->shadedown;
    wcol->shadedown = t;
  }
  else {
    if (state & ST_ACTIVE_DEFAULT) {
      memcpy(wcol->inner, wcol->inner_sel, 4);
      memcpy(wcol->outline, wcol->outline_sel, 4);
      memcpy(wcol->text, wcol->text_sel, 4);
    }
    if (state & ST_HOVER) {
      bk_widget_active_color(wcol);
    }
  }
}

void bk_widget_state_menu_item(BKWCol *wcol, const BKWCol *theme, int state)
{
  *wcol = *theme;
  if ((state & ST_DISABLED) && (state & ST_HOVER)) {
    wcol->text[3] = 128;
    color_blend_v3_v3(wcol->inner, wcol->text, 0.5f);
    wcol->inner[3] = 64;
  }
  else if (state & ST_DISABLED) {
    wcol->text[3] = 128;
  }
  else if (state & (ST_ACTIVE_DEFAULT | ST_SELECT)) {
    memcpy(wcol->inner, wcol->inner_sel, 4);
    memcpy(wcol->outline, wcol->outline_sel, 4);
    memcpy(wcol->text, wcol->text_sel, 4);
  }
  else if (state & ST_HOVER) {
    color_blend_v3_v3(wcol->inner, wcol->text, 0.2f);
    color_blend_v3_v3(wcol->outline, wcol->outline_sel, 0.5f);
    memcpy(wcol->text, wcol->text_sel, 3);
    wcol->inner[3] = 255;
    wcol->text[3] = 255;
  }
}

float bk_widget_radius(const BKWCol *wcol)
{
  return wcol->roundness * bk_unit(); /* widget_radius_from_zoom */
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Widget base: round_box_edges, shape presets, widgetbase_draw
 * \{ */

enum { TRIA_NONE, TRIA_ARROWS, TRIA_SCROLL, TRIA_MENU, TRIA_CHECK, TRIA_DASH };

typedef struct WidgetTrias {
  int type;
  float center[2], size;
  int npoly;         /* outline polygons */
  int nv[2];
  float v[2][16][2]; /* screen-space polygon outlines */
} WidgetTrias;

typedef struct WidgetBase {
  BuiWidgetBase p;
  bool draw_inner, draw_outline, draw_emboss;
  WidgetTrias tria1, tria2;
} WidgetBase;

static const float g_shape_preset_number_arrow_vert[3][2] = {
    {-0.352077f, 0.532607f}, {-0.352077f, -0.549313f}, {0.330000f, -0.008353f}};
static const float g_shape_preset_scroll_circle_vert[16][2] = {
    {0.382684f, 0.923879f},   {0.000001f, 1.000000f},   {-0.382683f, 0.923880f},
    {-0.707107f, 0.707107f},  {-0.923879f, 0.382684f},  {-1.000000f, 0.000000f},
    {-0.923880f, -0.382684f}, {-0.707107f, -0.707107f}, {-0.382683f, -0.923880f},
    {0.000000f, -1.000000f},  {0.382684f, -0.923880f},  {0.707107f, -0.707107f},
    {0.923880f, -0.382684f},  {1.000000f, -0.000000f},  {0.923880f, 0.382683f},
    {0.707107f, 0.707107f}};
static const float g_shape_preset_menu_arrow_vert[6][2] = {
    {-0.33f, 0.16f}, {0.33f, 0.16f}, {0, 0.82f}, {0, -0.82f}, {-0.33f, -0.16f}, {0.33f, -0.16f}};
static const float g_shape_preset_checkmark_vert[6][2] = {
    {-0.578579f, 0.253369f}, {-0.392773f, 0.412794f}, {-0.004241f, -0.328551f},
    {-0.003001f, 0.034320f}, {1.055313f, 0.864744f},  {0.866408f, 1.026895f}};
/* Outline order of the checkmark's 4 triangles {3,2,4},{3,4,5},{1,0,3},{0,2,3}. */
static const int g_checkmark_outline[6] = {0, 2, 4, 5, 3, 1};

static void widget_init(WidgetBase *wtb)
{
  memset(wtb, 0, sizeof(*wtb));
  wtb->draw_inner = wtb->draw_outline = wtb->draw_emboss = true;
  wtb->p.pixelsize = bk_px();
}

static void round_box_edges(WidgetBase *wt, int roundboxalign, const BKRect *rect, float rad)
{
  const float px = bk_px();
  float radi = rad - px;
  const int hnum = ((roundboxalign & (CNR_TL | CNR_TR)) == (CNR_TL | CNR_TR) ||
                    (roundboxalign & (CNR_BR | CNR_BL)) == (CNR_BR | CNR_BL)) ?
                       1 :
                       2;
  const int vnum = ((roundboxalign & (CNR_TL | CNR_BL)) == (CNR_TL | CNR_BL) ||
                    (roundboxalign & (CNR_TR | CNR_BR)) == (CNR_TR | CNR_BR)) ?
                       1 :
                       2;
  const int minsize = BK_MIN(bk_rw(rect) * hnum, bk_rh(rect) * vnum);
  if (2.0f * rad > minsize) {
    rad = 0.5f * minsize;
  }
  if (2.0f * (radi + 1.0f) > minsize) {
    radi = 0.5f * minsize - px;
  }
  wt->p.rect[0] = (float)rect->xmin;
  wt->p.rect[1] = (float)rect->ymin;
  wt->p.rect[2] = (float)bk_rw(rect);
  wt->p.rect[3] = (float)bk_rh(rect);
  wt->p.rad = rad;
  wt->p.radi = BK_MAX(0.0f, radi);
  wt->p.corners = roundboxalign;
}

/* Map a preset vertex (Blender's y-up local space) to the screen. */
static void tria_vert(float *out, float cx, float cy, float sx, float sy, const float v[2], int i1,
                      int i2)
{
  out[0] = sx * v[i1] + cx;
  out[1] = cy - sy * v[i2];
}

static void shape_preset_init_trias_ex(WidgetTrias *tria, const BKRect *rect, float triasize,
                                       char where, const float (*verts)[2], int verts_tot)
{
  int i1 = 0, i2 = 1;
  const float minsize = (where == 'r' || where == 'l') ? bk_rh(rect) : bk_rw(rect);
  float centx = rect->xmin + 0.4f * minsize;
  float centy = rect->ymax - 0.5f * minsize; /* Blender: ymin(bottom) + 0.5 * minsize */
  float sizex, sizey;
  sizex = sizey = -0.5f * triasize * minsize;
  if (where == 'r') {
    centx = rect->xmax - 0.4f * minsize;
    sizex = -sizex;
  }
  else if (where == 't') {
    centx = rect->xmin + 0.5f * minsize;
    centy = rect->ymin + 0.5f * minsize;
    sizey = -sizey;
    i2 = 0;
    i1 = 1;
  }
  else if (where == 'b') {
    centx = rect->xmin + 0.5f * minsize;
    sizex = -sizex;
    i2 = 0;
    i1 = 1;
  }
  tria->npoly = 1;
  tria->nv[0] = verts_tot;
  for (int a = 0; a < verts_tot; a++) {
    tria_vert(tria->v[0][a], centx, centy, sizex, sizey, verts[a], i1, i2);
  }
}

static void shape_preset_init_number_arrows(WidgetTrias *tria, const BKRect *rect,
                                            float triasize, char where)
{
  tria->type = TRIA_ARROWS;
  shape_preset_init_trias_ex(tria, rect, triasize, where, g_shape_preset_number_arrow_vert, 3);
}

static void shape_preset_init_scroll_circle(WidgetTrias *tria, const BKRect *rect,
                                            float triasize, char where)
{
  tria->type = TRIA_SCROLL;
  shape_preset_init_trias_ex(tria, rect, triasize, where, g_shape_preset_scroll_circle_vert, 16);
}

static void shape_preset_trias_from_rect_menu(WidgetTrias *tria, const BKRect *rect)
{
  const float width = bk_rw(rect), height = bk_rh(rect);
  if ((width / height) < 0.5f) {
    return; /* too narrow */
  }
  tria->type = TRIA_MENU;
  float centx = rect->xmin + 0.52f * height;
  const float centy = rect->ymin + 0.5f * height;
  const float size = 0.4f * height;
  if (width > height * 1.1f) {
    centx = rect->xmax - 0.32f * height; /* wider buttons: align tighter right */
  }
  /* Blender 4.x draws the menu indicator as a down chevron at this spot. */
  tria->center[0] = centx;
  tria->center[1] = centy;
  tria->size = size;
  (void)g_shape_preset_menu_arrow_vert;
}

static void draw_menu_chevron(const WidgetTrias *t, const float c[4])
{
  const float s = t->size * 0.42f, lw = BK_MAX(1.2f, 0.075f * bk_unit());
  const float cx = t->center[0], cy = t->center[1];
  BUI_raster_line(bk_draw.r, cx - s, cy - s * 0.4f, cx, cy + s * 0.5f, lw, c);
  BUI_raster_line(bk_draw.r, cx, cy + s * 0.5f, cx + s, cy - s * 0.4f, lw, c);
}

static void shape_preset_trias_from_rect_checkmark(WidgetTrias *tria, const BKRect *rect)
{
  tria->type = TRIA_CHECK;
  const float h = bk_rh(rect);
  const float centx = rect->xmin + 0.5f * h, centy = rect->ymax - 0.5f * h, size = 0.5f * h;
  tria->npoly = 1;
  tria->nv[0] = 6;
  for (int a = 0; a < 6; a++) {
    tria_vert(tria->v[0][a], centx, centy, size, size,
              g_shape_preset_checkmark_vert[g_checkmark_outline[a]], 0, 1);
  }
}

static void shape_preset_trias_from_rect_dash(WidgetTrias *tria, const BKRect *rect)
{
  tria->type = TRIA_DASH;
  const float h = bk_rh(rect);
  const float cx = rect->xmin + 0.5f * h, cy = rect->ymin + 0.5f * h, s = 0.5f * h;
  const float hw = 0.5f * s, hh = 0.1f * s;
  tria->npoly = 1;
  tria->nv[0] = 4;
  const float q[4][2] = {{cx - hw, cy - hh}, {cx + hw, cy - hh}, {cx + hw, cy + hh}, {cx - hw, cy + hh}};
  memcpy(tria->v[0], q, sizeof(q));
}

static void draw_trias(const WidgetTrias *t, const float color[4])
{
  for (int k = 0; k < t->npoly; k++) {
    if (t->nv[k] >= 3) {
      BUI_raster_polygon(bk_draw.r, (const float(*)[2])t->v[k], t->nv[k], color);
    }
  }
}

static void widgetbase_draw_ex(WidgetBase *wtb, const BKWCol *wcol, const float *color_override,
                               bool alpha_checker)
{
  BuiWidgetBase *p = &wtb->p;
  memset(p->inner1, 0, sizeof(float[4]) * 4);
  if (color_override) {
    memcpy(p->inner1, color_override, sizeof(float[4]));
    memcpy(p->inner2, color_override, sizeof(float[4]));
  }
  else if (wtb->draw_inner) {
    if (wcol->shaded == 0) {
      bk_rgba(wcol->inner, p->inner1);
      bk_rgba(wcol->inner, p->inner2);
    }
    else {
      uchar top[4], down[4];
      shadecolors4(wcol->inner, wcol->shadetop, wcol->shadedown, top, down);
      bk_rgba(top, p->inner1);
      bk_rgba(down, p->inner2);
    }
  }
  if (wtb->draw_outline) {
    bk_rgba(wcol->outline, p->outline);
  }
  /* Emboss only if the outline is not fully transparent (it looks like a gap). */
  if (wtb->draw_emboss && wcol->outline[3] != 0) {
    bk_rgba(bk_theme.emboss, p->emboss);
  }
  p->alpha_checker = alpha_checker;
  BUI_raster_widget_base(bk_draw.r, p);
  if (wtb->tria1.type != TRIA_NONE || wtb->tria2.npoly) {
    float tc[4];
    bk_rgba(wcol->item, tc);
    if (wtb->tria1.type == TRIA_MENU) {
      draw_menu_chevron(&wtb->tria1, tc);
    }
    draw_trias(&wtb->tria1, tc);
    draw_trias(&wtb->tria2, tc);
  }
}

static void widgetbase_draw(WidgetBase *wtb, const BKWCol *wcol)
{
  widgetbase_draw_ex(wtb, wcol, NULL, false);
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Widget types (interface_widgets.cc)
 * \{ */

#define NUM_BUT_PADDING_FACTOR 0.425f

static void swap_shade(BKWCol *w)
{
  const short t = w->shadetop;
  w->shadetop = w->shadedown;
  w->shadedown = t;
}

void bk_widget_but(BKWCol *wcol, BKRect *rect, int state, int corners)
{
  WidgetBase wtb;
  widget_init(&wtb);
  round_box_edges(&wtb, corners, rect, bk_widget_radius(wcol));
  wtb.draw_emboss = !(state & ST_ALIGN_DOWN);
  if (state & ST_NO_EMBOSS) {
    wtb.draw_outline = wtb.draw_emboss = false;
    if (!(state & (ST_HOVER | ST_SELECT))) {
      wtb.draw_inner = false;
    }
  }
  widgetbase_draw(&wtb, wcol);
}

void bk_widget_numbut(BKWCol *wcol, BKRect *rect, int state, int corners)
{
  const float rad = bk_widget_radius(wcol);
  const float px = bk_px();
  const int handle_width = BK_MIN(bk_rw(rect) / 3, (int)(bk_rh(rect) * 0.7f));
  if (state & ST_SELECT) {
    swap_shade(wcol);
  }
  WidgetBase wtb;
  widget_init(&wtb);
  round_box_edges(&wtb, corners, rect, rad);

  if ((state & ST_HOVER) && !(state & ST_TEXT_INPUT)) {
    BKWCol wcol_zone;
    WidgetBase wtb_zone;
    BKRect rz;

    /* left arrow zone */
    widget_init(&wtb_zone);
    wtb_zone.draw_outline = wtb_zone.draw_emboss = false;
    wcol_zone = *wcol;
    memcpy(wcol_zone.item, wcol->text, 3);
    if (state & ST_HOVER_LEFT) {
      bk_widget_active_color(&wcol_zone);
    }
    rz = *rect;
    rz.xmax = rect->xmin + handle_width + (int)px;
    round_box_edges(&wtb_zone, corners & ~(CNR_TR | CNR_BR), &rz, rad);
    shape_preset_init_number_arrows(&wtb_zone.tria1, &rz, 0.6f, 'l');
    widgetbase_draw(&wtb_zone, &wcol_zone);

    /* right arrow zone */
    widget_init(&wtb_zone);
    wtb_zone.draw_outline = wtb_zone.draw_emboss = false;
    wcol_zone = *wcol;
    memcpy(wcol_zone.item, wcol->text, 3);
    if (state & ST_HOVER_RIGHT) {
      bk_widget_active_color(&wcol_zone);
    }
    rz = *rect;
    rz.xmin = rect->xmax - handle_width - (int)px;
    round_box_edges(&wtb_zone, corners & ~(CNR_TL | CNR_BL), &rz, rad);
    shape_preset_init_number_arrows(&wtb_zone.tria1, &rz, 0.6f, 'r');
    widgetbase_draw(&wtb_zone, &wcol_zone);

    /* middle highlight zone */
    widget_init(&wtb_zone);
    wtb_zone.draw_outline = wtb_zone.draw_emboss = false;
    wcol_zone = *wcol;
    memcpy(wcol_zone.item, wcol->text, 3);
    if (!(state & (ST_HOVER_LEFT | ST_HOVER_RIGHT))) {
      bk_widget_active_color(&wcol_zone);
    }
    rz = *rect;
    rz.xmin = rect->xmin + handle_width - (int)px;
    rz.xmax = rect->xmax - handle_width + (int)px;
    round_box_edges(&wtb_zone, 0, &rz, 0);
    widgetbase_draw(&wtb_zone, &wcol_zone);

    /* outline */
    wtb.draw_inner = false;
    wtb.draw_emboss = !(state & ST_ALIGN_DOWN);
    widgetbase_draw(&wtb, wcol);
  }
  else {
    wtb.draw_emboss = !(state & ST_ALIGN_DOWN);
    widgetbase_draw(&wtb, wcol);
  }
  if (!(state & ST_TEXT_INPUT)) {
    const int text_padding = (int)(NUM_BUT_PADDING_FACTOR * bk_rh(rect));
    rect->xmin += text_padding;
    rect->xmax -= text_padding;
  }
}

void bk_widget_numslider(BKWCol *wcol, BKRect *rect, int state, int corners, float factor)
{
  WidgetBase wtb, wtb1;
  widget_init(&wtb);
  widget_init(&wtb1);
  const float rad = bk_widget_radius(wcol);
  round_box_edges(&wtb, corners, rect, rad);
  wtb.draw_outline = false;
  wtb.draw_emboss = !(state & ST_ALIGN_DOWN);
  widgetbase_draw(&wtb, wcol);

  if (!(state & ST_TEXT_INPUT)) {
    int corners_slider = corners;
    uchar outline[3];
    memcpy(outline, wcol->outline, 3);
    memcpy(wcol->outline, wcol->item, 3);
    memcpy(wcol->inner, wcol->item, 3);
    if (!(state & ST_SELECT)) {
      swap_shade(wcol);
    }
    BKRect rect1 = *rect;
    float factor_discard = 1.0f;
    factor = BK_CLAMP(factor, 0.0f, 1.0f);
    const float width = (float)bk_rw(rect);
    const float factor_ui = factor * width;
    const float min_width = 2.0f * rad;
    if (factor_ui > width - rad) {
      factor_discard = factor; /* left + middle + right */
    }
    else if (factor_ui > min_width) {
      corners_slider &= ~(CNR_TR | CNR_BR); /* left + middle */
      rect1.xmax = rect1.xmin + (int)factor_ui;
    }
    else {
      corners_slider &= ~(CNR_TR | CNR_BR); /* left part */
      rect1.xmax = rect1.xmin + (int)min_width;
      factor_discard = factor_ui / min_width;
    }
    round_box_edges(&wtb1, corners_slider, &rect1, rad);
    wtb1.draw_outline = false;
    wtb1.draw_emboss = false;
    /* alpha_discard: keep the left `factor_discard` of the slider shape */
    bk_clip_push(rect1.xmin, rect1.ymin - 1, (int)ceilf(bk_rw(&rect1) * factor_discard),
                 bk_rh(&rect1) + 2);
    widgetbase_draw(&wtb1, wcol);
    bk_clip_pop();
    memcpy(wcol->outline, outline, 3);
    if (!(state & ST_SELECT)) {
      swap_shade(wcol);
    }
  }
  /* outline */
  wtb.draw_outline = true;
  wtb.draw_inner = false;
  wtb.draw_emboss = false;
  widgetbase_draw(&wtb, wcol);
  if (!(state & ST_TEXT_INPUT)) {
    const int text_padding = (int)(NUM_BUT_PADDING_FACTOR * bk_rh(rect));
    rect->xmax -= text_padding;
    rect->xmin += text_padding;
  }
}

void bk_widget_textbut(BKWCol *wcol, BKRect *rect, int state, int corners)
{
  if (state & ST_SELECT) {
    swap_shade(wcol);
  }
  WidgetBase wtb;
  widget_init(&wtb);
  round_box_edges(&wtb, corners, rect, bk_widget_radius(wcol));
  wtb.draw_emboss = !(state & ST_ALIGN_DOWN);
  widgetbase_draw(&wtb, wcol);
}

void bk_widget_menubut(BKWCol *wcol, BKRect *rect, int state, int corners)
{
  WidgetBase wtb;
  widget_init(&wtb);
  round_box_edges(&wtb, corners, rect, bk_widget_radius(wcol));
  shape_preset_trias_from_rect_menu(&wtb.tria1, rect);
  if (state & ST_NO_EMBOSS) {
    wtb.draw_inner = wtb.draw_outline = wtb.draw_emboss = false;
  }
  wtb.draw_emboss = wtb.draw_emboss && !(state & ST_ALIGN_DOWN);
  widgetbase_draw(&wtb, wcol);
  /* text space, arrows are about 0.6 height of button */
  rect->xmax -= (6 * bk_rh(rect)) / 10;
}

void bk_widget_optionbut(BKWCol *wcol, BKRect *rect, int state)
{
  BKRect rt = *rect;
  WidgetBase wtb;
  widget_init(&wtb);
  rt.xmax = rt.xmin + bk_rh(&rt); /* square */
  /* smaller: BLI_rcti_resize + translate so the left edge stays */
  const int delta = (int)((bk_rh(&rt) - 2 * bk_px()) / 6);
  rt.xmin += delta;
  rt.xmax -= delta;
  rt.ymin += delta;
  rt.ymax -= delta;
  rt.xmin -= delta;
  rt.xmax -= delta;
  if (state & ST_INDETERMINATE) {
    color_blend_v4_v4v4(wcol->inner, wcol->inner, wcol->inner_sel, 0.75f);
  }
  const float rad = wcol->roundness * bk_rh(&rt); /* widget_radius_from_rcti */
  round_box_edges(&wtb, CNR_ALL, &rt, rad);
  if (state & ST_INDETERMINATE) {
    shape_preset_trias_from_rect_dash(&wtb.tria1, &rt);
  }
  else if (state & ST_SELECT) {
    shape_preset_trias_from_rect_checkmark(&wtb.tria1, &rt);
  }
  widgetbase_draw(&wtb, wcol);
  rect->xmin = rt.xmax + (int)(delta * 0.9f); /* text space */
}

void bk_widget_pulldownbut(BKWCol *wcol, BKRect *rect, int state, int corners)
{
  if (state & ST_HOVER) {
    WidgetBase wtb;
    memcpy(wcol->inner, wcol->inner_sel, 4);
    memcpy(wcol->text, wcol->text_sel, 3);
    widget_init(&wtb);
    round_box_edges(&wtb, corners, rect, bk_widget_radius(wcol));
    widgetbase_draw(&wtb, wcol);
  }
}

void bk_widget_menu_itembut(BKWCol *wcol, BKRect *rect)
{
  WidgetBase wtb;
  widget_init(&wtb);
  const int padding = (int)(0.125f * bk_unit()); /* padding on the sides */
  rect->xmin += padding;
  rect->xmax -= padding;
  round_box_edges(&wtb, CNR_ALL, rect, bk_widget_radius(wcol));
  widgetbase_draw(&wtb, wcol);
}

void bk_widget_list_itembut(BKWCol *wcol, BKRect *rect, int state)
{
  WidgetBase wtb;
  widget_init(&wtb);
  round_box_edges(&wtb, CNR_ALL, rect, bk_widget_radius(wcol));
  if (state & ST_HOVER) {
    color_blend_v3_v3(wcol->inner, wcol->text, 0.2f);
    wcol->inner[3] = (state & ST_SELECT) ? 255 : 20;
  }
  widgetbase_draw(&wtb, wcol);
}

void bk_widget_box(BKWCol *wcol, BKRect *rect, int corners, const uchar *color)
{
  WidgetBase wtb;
  widget_init(&wtb);
  BKWCol w = *wcol;
  if (color && color[3]) {
    memcpy(w.inner, color, 4);
  }
  round_box_edges(&wtb, corners, rect, bk_widget_radius(&w));
  widgetbase_draw(&wtb, &w);
}

void bk_widget_progress(BKWCol *wcol, BKRect *rect, int corners, float factor)
{
  BKRect rect_bar = *rect;
  WidgetBase wtb, wtb_bar;
  widget_init(&wtb);
  widget_init(&wtb_bar);
  const float ofs = bk_widget_radius(wcol);
  float w = BK_CLAMP(factor, 0.0f, 1.0f) * bk_rw(rect);
  w = BK_MAX(w, ofs); /* ensure minimum size */
  rect_bar.xmax = rect_bar.xmin + (int)w;
  round_box_edges(&wtb, corners, rect, ofs);
  round_box_edges(&wtb_bar, corners, &rect_bar, ofs);
  widgetbase_draw(&wtb, wcol);
  BKWCol bar = *wcol;
  memcpy(bar.inner, bar.item, 3); /* "slider" bar color */
  widgetbase_draw(&wtb_bar, &bar);
}

void bk_widget_swatch(BKWCol *wcol, BKRect *rect, int state, int corners, const float col_in[4])
{
  WidgetBase wtb;
  widget_init(&wtb);
  round_box_edges(&wtb, corners, rect, bk_widget_radius(wcol));
  float col[4] = {col_in[0], col_in[1], col_in[2], col_in[3]};
  const bool show_alpha_checkers = col[3] < 1.0f;
  if (state & ST_DISABLED) {
    col[3] *= 0.5f;
  }
  wtb.draw_emboss = !(state & ST_ALIGN_DOWN);
  widgetbase_draw_ex(&wtb, wcol, col, show_alpha_checkers);
}

void bk_widget_scroll(BKWCol *wcol, const BKRect *rect, const BKRect *slider, bool pressed)
{
  WidgetBase wtb;
  widget_init(&wtb);
  const bool horizontal = bk_rw(rect) > bk_rh(rect);
  const float rad = wcol->roundness * (horizontal ? bk_rh(rect) : bk_rw(rect));
  wtb.p.shade_horizontal = !horizontal;
  if (horizontal) {
    swap_shade(wcol);
  }
  round_box_edges(&wtb, CNR_ALL, rect, rad);
  widgetbase_draw(&wtb, wcol);
  if (bk_rw(slider) < 2 || bk_rh(slider) < 2) {
    return;
  }
  swap_shade(wcol);
  memcpy(wcol->inner, wcol->item, 4);
  if (wcol->shadetop > wcol->shadedown) {
    wcol->shadetop += 20;
  }
  else {
    wcol->shadedown += 20;
  }
  if (pressed) {
    for (int k = 0; k < 3; k++) {
      wcol->inner[k] = wcol->inner[k] >= 250 ? 255 : wcol->inner[k] + 5;
    }
  }
  wtb.draw_emboss = false;
  round_box_edges(&wtb, CNR_ALL, slider, rad);
  widgetbase_draw(&wtb, wcol);
  (void)shape_preset_init_scroll_circle;
}

void bk_widget_tab(BKWCol *wcol, BKRect *rect, int state, int corners)
{
  WidgetBase wtb;
  widget_init(&wtb);
  round_box_edges(&wtb, corners, rect, bk_widget_radius(wcol));
  widgetbase_draw(&wtb, wcol);
}

static void widget_menu_back(const BKWCol *wcol, const BKRect *rect, int corners)
{
  WidgetBase wtb;
  widget_init(&wtb);
  const float radius = bk_widget_radius(wcol);
  bk_shadow(rect, radius); /* widget_softshadow */
  round_box_edges(&wtb, corners, rect, radius);
  wtb.draw_emboss = false;
  widgetbase_draw(&wtb, wcol);
}

void bk_widget_menu_back(const BKRect *rect, int corners)
{
  widget_menu_back(&bk_theme.menu_back, rect, corners);
}

void bk_widget_tooltip_back(const BKRect *rect)
{
  widget_menu_back(&bk_theme.tooltip, rect, CNR_ALL);
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Text
 * \{ */

typedef struct MeasureSlot {
  uint64_t hash;
  float size;
  float width;
} MeasureSlot;
#define MEASURE_SLOTS 8192
static MeasureSlot g_measure[MEASURE_SLOTS];

static uint64_t hash_str(const char *s, int len, bool bold)
{
  uint64_t h = 1469598103934665603ull ^ (bold ? 0x9e37 : 0);
  for (int i = 0; i < len; i++) {
    h = (h ^ (uchar)s[i]) * 1099511628211ull;
  }
  return h | 1;
}

float bk_text_width(const char *s, int len, bool bold)
{
  if (!s) {
    return 0.0f;
  }
  if (len < 0) {
    len = (int)strlen(s);
  }
  if (len == 0) {
    return 0.0f;
  }
  const float size = bk_font_size();
  const uint64_t h = hash_str(s, len, bold);
  MeasureSlot *slot = &g_measure[h & (MEASURE_SLOTS - 1)];
  if (slot->hash == h && slot->size == size) {
    return slot->width;
  }
  float w = 0.0f;
  if (!BUI_text_measure(g_font_family, bold ? BUI_FONT_BOLD : 0, size, s, len, &w)) {
    w = len * size * 0.55f;
  }
  slot->hash = h;
  slot->size = size;
  slot->width = w;
  return w;
}

float bk_text_baseline(const BKRect *rect)
{
  /* UI_fontstyle_draw: centered on the ascender (Inter: ~0.72 em) */
  return rect->ymin + bk_rh(rect) * 0.5f + bk_font_size() * 0.36f;
}

void bk_text_f(float x, float baseline, const char *s, int len, const float c[4], bool bold)
{
  if (!s || c[3] <= 0.0f) {
    return;
  }
  if (len < 0) {
    len = (int)strlen(s);
  }
  if (len == 0) {
    return;
  }
  BUI_raster_set_font(bk_draw.r, g_font_family, bold ? BUI_FONT_BOLD : 0);
  BUI_raster_text(bk_draw.r, floorf(x + 0.5f), floorf(baseline + 0.5f), s, len, bk_font_size(),
                  c);
}

void bk_text(float x, float baseline, const char *s, int len, const uchar color[4], bool bold)
{
  float c[4];
  bk_rgba(color, c);
  bk_text_f(x, baseline, s, len, c, bold);
}

/* Bytes of `s` that fit in `width` with an ellipsis appended (UTF-8 safe). */
static int fit_with_ellipsis(const char *s, float width, bool bold, bool *r_ellipsis)
{
  const int len = (int)strlen(s);
  *r_ellipsis = false;
  if (bk_text_width(s, len, bold) <= width) {
    return len;
  }
  *r_ellipsis = true;
  const float ew = bk_text_width("\xe2\x80\xa6", 3, bold);
  int n = len;
  while (n > 0) {
    n--;
    while (n > 0 && ((uchar)s[n] & 0xC0) == 0x80) {
      n--;
    }
    if (bk_text_width(s, n, bold) + ew <= width) {
      break;
    }
  }
  return n;
}

float bk_line_height(void)
{
  return floorf(bk_font_size() * 1.45f + 0.5f);
}

float bk_text_in(const BKRect *rect, const char *s, const uchar color[4], int align, bool bold)
{
  if (!s || !s[0]) {
    return 0.0f;
  }
  if (strchr(s, '\n')) { /* multi-line: a centered block of lines */
    int lines = 1;
    for (const char *c = s; *c; c++) {
      lines += *c == '\n';
    }
    const float lh = bk_line_height();
    float y = rect->ymin + (bk_rh(rect) - lines * lh) * 0.5f, widest = 0.0f;
    for (const char *line = s;; y += lh) {
      const char *end = strchr(line, '\n');
      const int len = end ? (int)(end - line) : (int)strlen(line);
      char buf[512];
      memcpy(buf, line, (size_t)BK_MIN(len, 511));
      buf[BK_MIN(len, 511)] = 0;
      const BKRect lr = bk_rect(rect->xmin, (int)y, bk_rw(rect), (int)lh);
      widest = BK_MAX(widest, bk_text_in(&lr, buf, color, align, bold));
      if (!end) {
        break;
      }
      line = end + 1;
    }
    return widest;
  }
  const int pad = bk_pad();
  const float avail = (float)(bk_rw(rect) - 2 * pad);
  bool ell;
  const int n = fit_with_ellipsis(s, avail, bold, &ell);
  char buf[512];
  int blen = BK_MIN(n, (int)sizeof(buf) - 4);
  memcpy(buf, s, (size_t)blen);
  if (ell) {
    memcpy(buf + blen, "\xe2\x80\xa6", 3);
    blen += 3;
  }
  const float tw = bk_text_width(buf, blen, bold);
  float x;
  if (align == BK_ALIGN_LEFT) {
    x = rect->xmin + pad;
  }
  else if (align == BK_ALIGN_RIGHT) {
    x = rect->xmax - pad - tw;
  }
  else {
    x = rect->xmin + (bk_rw(rect) - tw) * 0.5f;
  }
  bk_clip_push(rect->xmin, rect->ymin, bk_rw(rect), bk_rh(rect));
  bk_text(x, bk_text_baseline(rect), buf, blen, color, bold);
  bk_clip_pop();
  return tw;
}

int bk_icon_size(void)
{
  return bk_iround(0.8f * bk_unit()); /* ICON_DEFAULT_HEIGHT = 16 * scale */
}

float bk_content_width(const char *s, int icon, bool bold)
{
  float w = (s && s[0]) ? bk_text_width(s, -1, bold) : 0.0f;
  if (icon) {
    w += bk_icon_size() + ((s && s[0]) ? 0.25f * bk_unit() : 0.0f);
  }
  return w;
}

void bk_text_icon(const BKRect *rect, const char *s, int icon, const uchar color[4], int align,
                  bool enabled)
{
  if (!icon) {
    bk_text_in(rect, s, color, align, false);
    return;
  }
  const float u = (float)bk_unit();
  const float is = (float)bk_icon_size();
  const float alpha = enabled ? 1.0f : 0.5f;
  if (!s || !s[0]) {
    bk_icon(icon, rect->xmin + bk_rw(rect) * 0.5f, rect->ymin + bk_rh(rect) * 0.5f, NULL, alpha,
            0);
    return;
  }
  const float tw = BK_MIN(bk_text_width(s, -1, false), bk_rw(rect) - is - 2.0f * bk_pad());
  float ix;
  if (align == BK_ALIGN_CENTER) {
    ix = rect->xmin + (bk_rw(rect) - (is + 0.25f * u + tw)) * 0.5f;
  }
  else {
    ix = rect->xmin + 0.2f * u;
  }
  bk_icon(icon, ix + is * 0.5f, rect->ymin + bk_rh(rect) * 0.5f, NULL, alpha, 0);
  BKRect tr = *rect;
  tr.xmin = (int)(ix + is + 0.25f * u) - bk_pad();
  bk_text_in(&tr, s, color, BK_ALIGN_LEFT, false);
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Icons & small glyphs
 * \{ */

const float *bk_icon_tint(int icon)
{
  if (icon <= 0 || icon >= bk_icon_def_count || !bk_icon_defs[icon].has_tint) {
    return NULL;
  }
  return bk_icon_defs[icon].tint;
}

void bk_icon(int icon, float cx, float cy, const uchar *color, float alpha, float size)
{
  if (icon <= 0 || icon >= bk_icon_def_count) {
    return;
  }
  const float s = size > 0.0f ? size : (float)bk_icon_size();
  float c[4];
  if (color) {
    bk_rgba(color, c);
  }
  else {
    const float *tint = bk_icon_tint(icon);
    if (tint) {
      memcpy(c, tint, sizeof(c));
    }
    else {
      bk_rgba(bk_theme.text, c);
      c[3] *= 0.9f;
    }
  }
  c[3] *= alpha;
  bk_icons_draw(icon, floorf(cx - s * 0.5f + 0.5f), floorf(cy - s * 0.5f + 0.5f), s, c);
}

void bk_tria(float cx, float cy, char dir, const float c[4], float size)
{
  /* draw_icon_tri(): 'h' right, 'v' down */
  const float s = size > 0 ? size : 0.18f * bk_unit();
  float p[3][2];
  if (dir == 'h') {
    const float q[3][2] = {{cx - s * 0.5f, cy - s}, {cx - s * 0.5f, cy + s}, {cx + s * 0.7f, cy}};
    memcpy(p, q, sizeof(p));
  }
  else {
    const float q[3][2] = {{cx - s, cy - s * 0.5f}, {cx + s, cy - s * 0.5f}, {cx, cy + s * 0.7f}};
    memcpy(p, q, sizeof(p));
  }
  BUI_raster_polygon(bk_draw.r, (const float(*)[2])p, 3, c);
}

void bk_chevron_down(float cx, float cy, const uchar col[4])
{
  float c[4];
  bk_rgba(col, c);
  const float s = 0.16f * bk_unit(), lw = BK_MAX(1.2f, 0.08f * bk_unit());
  BUI_raster_line(bk_draw.r, cx - s, cy - s * 0.45f, cx, cy + s * 0.55f, lw, c);
  BUI_raster_line(bk_draw.r, cx, cy + s * 0.55f, cx + s, cy - s * 0.45f, lw, c);
}

int bk_icons_lookup(const char *name)
{
  if (!name || !name[0]) {
    return 0;
  }
  for (int i = 1; i < bk_icon_def_count; i++) {
    if (strcmp(bk_icon_defs[i].name, name) == 0) {
      return i;
    }
  }
  return 0;
}

int BK_icon_id(const char *name)
{
  return bk_icons_lookup(name);
}
int BK_icon_count(void)
{
  return bk_icon_def_count - 1;
}
const char *BK_icon_name(int id)
{
  return (id > 0 && id < bk_icon_def_count) ? bk_icon_defs[id].name : "";
}

void bk_icons_draw(int id, float x, float y, float s, const float color[4])
{
  const BKIconDef *def = &bk_icon_defs[id];
  const float lw = BK_MAX(1.0f, s * 0.085f);
  BuiRaster *r = bk_draw.r;
  for (int i = 0; i < def->count; i++) {
    const BKIconOp *op = &bk_icon_ops[def->first + i];
    float c[4];
    if (op->color_mode == 2) {
      memcpy(c, op->rgba, sizeof(c));
      c[3] *= color[3];
    }
    else {
      memcpy(c, color, sizeof(c));
      if (op->color_mode == 1) {
        c[3] *= op->k;
      }
    }
    const float *a = op->a;
#define PX(v) (x + (v) * s)
#define PY(v) (y + (v) * s)
    switch (op->op) {
      case BK_ICON_LINE:
        BUI_raster_line(r, PX(a[0]), PY(a[1]), PX(a[2]), PY(a[3]), lw * a[4], c);
        break;
      case BK_ICON_POLY: {
        float pts[32][2];
        const int n = BK_MIN(op->npts, 32);
        for (int k = 0; k < n; k++) {
          pts[k][0] = PX(bk_icon_pts[op->pts_offset + k][0]);
          pts[k][1] = PY(bk_icon_pts[op->pts_offset + k][1]);
        }
        BUI_raster_polygon(r, (const float(*)[2])pts, n, c);
        break;
      }
      case BK_ICON_DISC:
        BUI_raster_circle(r, PX(a[0]), PY(a[1]), a[2] * s, 0.0f, c);
        break;
      case BK_ICON_RING:
        BUI_raster_ellipse(r, PX(a[0]), PY(a[1]), a[2] * s, a[3] * s, lw * a[4], c);
        break;
      case BK_ICON_RECT:
        BUI_raster_fill_rect(r, PX(a[0]), PY(a[1]), a[2] * s, a[3] * s, c);
        break;
      case BK_ICON_FRAME:
        BUI_raster_widget(r, PX(a[0]), PY(a[1]), a[2] * s, a[3] * s, a[4] * s, BUI_CORNER_ALL,
                          NULL, NULL, c, lw * a[5], NULL);
        break;
      case BK_ICON_BOX:
        BUI_raster_round_rect(r, PX(a[0]), PY(a[1]), a[2] * s, a[3] * s, a[4] * s, c);
        break;
    }
#undef PX
#undef PY
  }
}

/** \} */
