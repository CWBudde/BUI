/* Editor regions: the sidebar (N-panel) and tool shelf with category tabs.
 *
 * Ported from Blender's interface_panel.cc (panel_category_tabs_draw_all) and
 * area.cc (ED_region_panels_layout_ex: panel margins). Panels carry a
 * `category`; the region shows one category's panels and draws a tab per
 * category along its edge, labels rotated, as Blender does.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include "kit_intern.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

static int u(void)
{
  return bk_unit();
}

/* -------------------------------------------------------------------- */
/** \name Rotated text (BLF_ROTATION: tab labels)
 * \{ */

typedef struct RotText {
  uint64_t hash; /* 0: empty */
  unsigned gen;
  int w, h;      /* image size */
  int ox, ascent;
  uint32_t *argb;
} RotText;

#define ROT_CACHE 64
static RotText g_rot[ROT_CACHE];

static uint64_t rot_hash(const char *s, float size, const uchar c[4])
{
  uint64_t h = 1469598103934665603ull;
  for (const char *p = s; *p; p++) {
    h = (h ^ (uchar)*p) * 1099511628211ull;
  }
  h = (h ^ (uint64_t)(size * 64.0f)) * 1099511628211ull;
  h = (h ^ ((uint64_t)c[0] << 16 | (uint64_t)c[1] << 8 | c[2])) * 1099511628211ull;
  return h | 1;
}

/* Text rotated a quarter turn clockwise: it reads top to bottom, the glyph
 * tops facing +x (BLF rotation -M_PI_2 in Blender's y-up space). The
 * baseline is the vertical line x = `bx`; the text starts at y = `by`. */
static void text_rotated(float bx, float by, const char *s, const uchar color[4])
{
  const float size = bk_font_size();
  const uint64_t key = rot_hash(s, size, color);
  const unsigned gen = BUI_text_generation();
  RotText *e = &g_rot[key % ROT_CACHE];
  if (e->hash != key || e->gen != gen) {
    free(e->argb);
    memset(e, 0, sizeof(*e));
    float width;
    const BuiTextRun *run = BUI_text_prepare(bk_font_family(), 0, size, s, (int)strlen(s), &width);
    int mw, mh, ox, ascent;
    const uint8_t *mask = run ? BUI_text_run_mask(run, &mw, &mh, &ox, &ascent) : NULL;
    if (!mask) {
      return;
    }
    uint32_t *img = (uint32_t *)malloc(sizeof(uint32_t) * (size_t)mw * (size_t)mh);
    if (!img) {
      return;
    }
    const uint32_t rgb = (uint32_t)color[0] << 16 | (uint32_t)color[1] << 8 | color[2];
    /* the image is mh wide and mw tall: mask (mx, my) -> (mh - 1 - my, mx) */
    for (int my = 0; my < mh; my++) {
      for (int mx = 0; mx < mw; mx++) {
        const uint32_t m = mask[(size_t)my * mw + mx]; /* premultiplied */
        img[(size_t)mx * mh + (size_t)(mh - 1 - my)] =
            m << 24 | ((rgb >> 16) * m / 255) << 16 | (((rgb >> 8) & 0xFF) * m / 255) << 8 |
            ((rgb & 0xFF) * m / 255);
      }
    }
    e->hash = key;
    e->gen = gen;
    e->w = mh;
    e->h = mw;
    e->ox = ox;
    e->ascent = ascent;
    e->argb = img;
  }
  if (!e->argb) {
    return;
  }
  const float a = color[3] / 255.0f;
  const float x0 = floorf(bx + 0.5f) + (float)e->ascent - (float)e->w, y0 = floorf(by + 0.5f) - (float)e->ox;
  const BuiPixmap pm = {e->argb, e->w, e->h, (e->hash ^ (uint64_t)e->gen << 32) | 1ull << 63};
  BUI_raster_image(bk_draw.r, x0, y0, (float)e->w, (float)e->h, &pm, NULL, a, 0);
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Sidebar region
 * \{ */

#define MAX_CATEGORIES 32

static int sidebar_categories(const BKNode *n, const char *out[MAX_CATEGORIES])
{
  int count = 0;
  for (int i = 0; i < n->nkids; i++) {
    const char *c = n->kids[i]->category ? n->kids[i]->category : "";
    bool seen = false;
    for (int k = 0; k < count; k++) {
      seen |= strcmp(out[k], c) == 0;
    }
    if (!seen && count < MAX_CATEGORIES) {
      out[count++] = c;
    }
  }
  return count;
}

static const char *sidebar_active(const BKNode *n, const char *cats[], int count)
{
  return count > 0 ? cats[BK_CLAMP(n->current, 0, count - 1)] : "";
}

/* UI_PANEL_CATEGORY_MARGIN_WIDTH: 1 unit, when there is more than one category */
static int sidebar_tabs_w(const BKNode *n)
{
  const char *cats[MAX_CATEGORIES];
  return sidebar_categories(n, cats) > 1 ? u() : 0;
}

static bool in_active(const BKNode *n, const BKNode *c, const char *active)
{
  return strcmp(c->category ? c->category : "", active) == 0;
}

static void sidebar_measure(BKNode *n, int *rw, int *rh)
{
  const char *cats[MAX_CATEGORIES];
  const char *active = sidebar_active(n, cats, sidebar_categories(n, cats));
  const int mx = bk_iround(0.4f * u()), my = bk_iround(0.1f * u());
  int w = 0, h = my;
  for (int i = 0; i < n->nkids; i++) {
    BKNode *c = n->kids[i];
    if (in_active(n, c, active)) {
      int cw, ch;
      bk_natural(c, &cw, &ch);
      w = BK_MAX(w, cw);
      h += ch + my;
    }
  }
  *rw = w + 2 * mx + sidebar_tabs_w(n);
  *rh = h;
}

static void sidebar_arrange(BKNode *n)
{
  const char *cats[MAX_CATEGORIES];
  const char *active = sidebar_active(n, cats, sidebar_categories(n, cats));
  /* UI_PANEL_MARGIN_X / _Y (area.cc: ED_region_panels_layout_ex) */
  const int mx = bk_iround(0.4f * u()), my = bk_iround(0.1f * u());
  const int x0 = n->x + mx, w = n->w - 2 * mx - sidebar_tabs_w(n);
  int y = n->y + my;
  for (int i = 0; i < n->nkids; i++) {
    BKNode *c = n->kids[i];
    if (!in_active(n, c, active)) {
      c->flags &= ~NF_VISIBLE;
      continue;
    }
    c->flags |= NF_VISIBLE;
    int cw, ch;
    bk_natural(c, &cw, &ch);
    bk_arrange(c, x0, y, w, ch);
    y += ch + my;
  }
}

/* The tab rectangles, top to bottom (panel_category_tabs_draw_all). */
static int sidebar_tab_rects(const BKNode *n, BKRect rects[MAX_CATEGORIES], const char *cats[])
{
  const int count = sidebar_categories(n, cats);
  if (count < 2) {
    return 0;
  }
  const float s = bk_scale_fac;
  const int px = BK_MAX(1, (int)(s + 0.5f));
  const int pad_text = (int)floorf(6.0f * s + 0.5f) + 2 * px; /* TABS_PADDING_TEXT_FACTOR */
  const int pad = (int)floorf(4.0f * s + 0.5f);               /* TABS_PADDING_BETWEEN_FACTOR */
  const int xmax = n->x + n->w;
  int y = n->y + pad;
  for (int i = 0; i < count; i++) {
    const int tw = (int)floorf(bk_text_width(cats[i], -1, false) + 0.5f);
    rects[i].xmin = xmax - sidebar_tabs_w(n);
    rects[i].xmax = xmax - 3;
    rects[i].ymin = y;
    rects[i].ymax = y + tw + 2 * pad_text;
    y += tw + pad + 2 * pad_text;
  }
  return count;
}

static int sidebar_tab_at(BKNode *n, int x, int y)
{
  const char *cats[MAX_CATEGORIES];
  BKRect r[MAX_CATEGORIES];
  const int count = sidebar_tab_rects(n, r, cats);
  for (int i = 0; i < count; i++) {
    if (x >= r[i].xmin && y >= r[i].ymin && y < r[i].ymax) {
      return i;
    }
  }
  return -1;
}

static BKNode *sidebar_hit(BKNode *n, int x, int y)
{
  if (x >= n->x + n->w - sidebar_tabs_w(n)) {
    return n;
  }
  for (int i = n->nkids - 1; i >= 0; i--) {
    BKNode *f = bk_hit(n->kids[i], x, y);
    if (f) {
      return f;
    }
  }
  return n;
}

static int sidebar_press(BKNode *n, BKEvent *e)
{
  const int i = sidebar_tab_at(n, e->x, e->y);
  if (i >= 0 && i != n->current) {
    n->current = i;
    bk_relayout(n);
    bk_invalidate(n);
    bk_emit(n, BK_EV_CHANGE, NULL, i, 0);
  }
  return i >= 0;
}

static int sidebar_wheel(BKNode *n, BKEvent *e)
{
  /* Ctrl+wheel cycles the categories (handle_panel_category_cycling) */
  const char *cats[MAX_CATEGORIES];
  const int count = sidebar_categories(n, cats);
  if (!EV_CTRL(e) || count < 2) {
    return 0;
  }
  n->current = (n->current + (e->delta > 0 ? count - 1 : 1)) % count;
  bk_relayout(n);
  bk_invalidate(n);
  bk_emit(n, BK_EV_CHANGE, NULL, n->current, 0);
  return 1;
}

static void sidebar_draw(BKNode *n)
{
  bk_fill(n->x, n->y, n->w, n->h, bk_theme.region_bg);
  const char *cats[MAX_CATEGORIES];
  BKRect r[MAX_CATEGORIES];
  const int count = sidebar_tab_rects(n, r, cats);
  if (count == 0) {
    return;
  }
  const int tw = sidebar_tabs_w(n);
  bk_fill(n->x + n->w - tw, n->y, tw, n->h, bk_theme.tabs_bg); /* TH_TAB_BACK */
  const float rad = bk_theme.tab.roundness * (float)u();
  const int px = BK_MAX(1, (int)(bk_scale_fac + 0.5f));
  for (int i = 0; i < count; i++) {
    const bool active = i == BK_CLAMP(n->current, 0, count - 1);
    float fill[4], line[4];
    bk_rgba(active ? bk_theme.tab.inner_sel : bk_theme.tab.inner, fill);
    bk_rgba(active ? bk_theme.tab.outline_sel : bk_theme.tab.outline, line);
    const int x = r[i].xmin, y = r[i].ymin, w = bk_rw(&r[i]), h = bk_rh(&r[i]);
    bk_roundbox(x, y, w, h, rad, CNR_TR | CNR_BR, fill);
    BuiWidgetBase wb = {{(float)x, (float)y, (float)w, (float)h}, rad, rad - 1, CNR_TR | CNR_BR};
    memcpy(wb.outline, line, sizeof(line));
    wb.pixelsize = 1.0f;
    BUI_raster_widget_base(bk_draw.r, &wb);
    /* disguise the outline on the panel side to join the tab to the panel */
    bk_fill_f(x, y + px, px, h - 2 * px, fill);
    /* the label: baseline left of the middle by 0.35 of the font size */
    const int v_ofs = (int)floorf((float)(r[i].xmax - r[i].xmin) * 0.5f + 0.5f);
    const int size_ofs = (int)floorf(bk_font_size() * 0.35f + 0.5f);
    const int pad_text = (int)floorf(6.0f * bk_scale_fac + 0.5f) + 2 * px;
    const uchar black[4] = {0, 0, 0, 128};
    const float bx = (float)(x + v_ofs - size_ofs), by = (float)(y + pad_text);
    text_rotated(bx - 1.0f, by, cats[i], black); /* the widget style's shadow */
    text_rotated(bx, by, cats[i], active ? bk_theme.tab.text_sel : bk_theme.tab.text);
  }
}

const BKClass bk_cls_sidebar = {.name = "sidebar",
                                .flags = CLS_INTERACTIVE,
                                .measure = sidebar_measure,
                                .arrange = sidebar_arrange,
                                .draw = sidebar_draw,
                                .hit = sidebar_hit,
                                .press = sidebar_press,
                                .wheel = sidebar_wheel};

/** \} */

/* -------------------------------------------------------------------- */
/** \name Area header (ED_region_header_layout)
 * \{ */

/* HEADERY: 20 + HEADER_PADDING_Y (6), scaled */
static int header_h(void)
{
  return bk_iround(26.0f * bk_scale_fac);
}

static bool is_spacer(const BKNode *c)
{
  return c->cls == &bk_cls_separator && c->weight > 0.0f;
}

static void areaheader_measure(BKNode *n, int *rw, int *rh)
{
  const int gap = bk_iround(0.2f * u());
  int w = bk_iround(8.0f * bk_scale_fac) * 2;
  for (int i = 0; i < n->nkids; i++) {
    if (NODE_VISIBLE(n->kids[i]) && !is_spacer(n->kids[i])) {
      int cw, ch;
      bk_natural(n->kids[i], &cw, &ch);
      w += cw + gap;
    }
  }
  *rw = w;
  *rh = header_h();
}

/* Items from UI_HEADER_OFFSET, left to right, buttons one unit tall and
 * centered; spacers (layout.separator_spacer) share the spare width. */
static void areaheader_arrange(BKNode *n)
{
  const int gap = bk_iround(0.2f * u()), ofs = bk_iround(8.0f * bk_scale_fac);
  int used = 0, spacers = 0;
  for (int i = 0; i < n->nkids; i++) {
    BKNode *c = n->kids[i];
    if (!NODE_VISIBLE(c)) {
      continue;
    }
    if (is_spacer(c)) {
      spacers++;
      continue;
    }
    int cw, ch;
    bk_natural(c, &cw, &ch);
    used += cw + gap;
  }
  const int spare = BK_MAX(0, n->w - 2 * ofs - used);
  const int y = n->y + (n->h - u()) / 2;
  int x = n->x + ofs;
  for (int i = 0; i < n->nkids; i++) {
    BKNode *c = n->kids[i];
    if (!NODE_VISIBLE(c)) {
      continue;
    }
    if (is_spacer(c)) {
      const int sw = spacers ? spare / spacers : 0;
      bk_arrange(c, x, y, sw, u());
      x += sw;
      continue;
    }
    int cw, ch;
    bk_natural(c, &cw, &ch);
    bk_arrange(c, x, y, cw, u());
    x += cw + gap;
  }
}

static void areaheader_draw(BKNode *n)
{
  /* TH_HEADER of the editors (space_view3d.header and friends) */
  bk_fill(n->x, n->y, n->w, n->h, bk_theme.area_header);
}

const BKClass bk_cls_areaheader = {.name = "areaheader",
                                   .measure = areaheader_measure,
                                   .arrange = areaheader_arrange,
                                   .draw = areaheader_draw};

/** \} */
