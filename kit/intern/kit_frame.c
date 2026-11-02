/* Frames with Tk's geometry managers (pack, grid, place), radio buttons,
 * canvases (Tk Canvas / QPainter display lists) and images.
 *
 * The packer is Tk's ArrangePacking / ComputePackRequest (generic/tkPack.c):
 * each child takes a parcel off one side of the remaining cavity, `expand`
 * shares what is left, fill/anchor place the child inside its parcel. The
 * gridder follows tkGrid.c's model (column/row minimums from the children,
 * extra space by weight, sticky placement). Qt box/grid layouts map onto
 * the same code.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <math.h>
#include <stdlib.h>
#include <string.h>

#include "kit_intern.h"

static int u(void)
{
  return bk_unit();
}

/* -------------------------------------------------------------------- */
/** \name Geometry info
 * \{ */

static BKGeom *geom_ensure(BKNode *n)
{
  if (!n->geom) {
    n->geom = (BKGeom *)calloc(1, sizeof(BKGeom));
    n->geom->rowspan = n->geom->columnspan = 1;
  }
  return n->geom;
}

void BK_geom_set_manager(BKNode *n, int manager)
{
  geom_ensure(n)->manager = manager;
  bk_relayout(n);
}

int BK_geom_manager(const BKNode *n)
{
  return n->geom ? n->geom->manager : BK_GEOM_NONE;
}

enum { HAS_W = 1, HAS_H = 2, HAS_RW = 4, HAS_RH = 8 };

void BK_geom_set(BKNode *n, const char *key, double v)
{
  BKGeom *g = geom_ensure(n);
  const int iv = (int)floor(v + 0.5);
#define K(name) (strcmp(key, name) == 0)
  if (K("side")) g->side = iv;
  else if (K("fill")) g->fill = iv;
  else if (K("expand")) g->expand = iv;
  else if (K("anchor")) g->anchor = iv;
  else if (K("padx0")) g->padx[0] = iv;
  else if (K("padx1")) g->padx[1] = iv;
  else if (K("pady0")) g->pady[0] = iv;
  else if (K("pady1")) g->pady[1] = iv;
  else if (K("ipadx")) g->ipadx = iv;
  else if (K("ipady")) g->ipady = iv;
  else if (K("row")) g->row = iv;
  else if (K("column")) g->column = iv;
  else if (K("rowspan")) g->rowspan = BK_MAX(1, iv);
  else if (K("columnspan")) g->columnspan = BK_MAX(1, iv);
  else if (K("sticky")) g->sticky = iv;
  else if (K("x")) g->x = (float)v;
  else if (K("y")) g->y = (float)v;
  else if (K("relx")) g->relx = (float)v;
  else if (K("rely")) g->rely = (float)v;
  else if (K("width")) { g->width = (float)v; g->has |= HAS_W; }
  else if (K("height")) { g->height = (float)v; g->has |= HAS_H; }
  else if (K("relwidth")) { g->relwidth = (float)v; g->has |= HAS_RW; }
  else if (K("relheight")) { g->relheight = (float)v; g->has |= HAS_RH; }
  else if (K("reqwidth")) g->reqw = iv;
  else if (K("reqheight")) g->reqh = iv;
  else if (K("reset_place")) { g->x = g->y = g->relx = g->rely = 0; g->has = 0; }
#undef K
  bk_relayout(n);
}

double BK_geom_get(const BKNode *n, const char *key)
{
  const BKGeom *g = n->geom;
  if (!g) {
    return 0.0;
  }
#define K(name) (strcmp(key, name) == 0)
  if (K("manager")) return g->manager;
  if (K("side")) return g->side;
  if (K("fill")) return g->fill;
  if (K("expand")) return g->expand;
  if (K("anchor")) return g->anchor;
  if (K("row")) return g->row;
  if (K("column")) return g->column;
  if (K("rowspan")) return g->rowspan;
  if (K("columnspan")) return g->columnspan;
  if (K("sticky")) return g->sticky;
  if (K("reqwidth")) return g->reqw;
  if (K("reqheight")) return g->reqh;
#undef K
  return 0.0;
}

static void child_req(BKNode *c, int *w, int *h)
{
  bk_natural(c, w, h); /* honors reqwidth/reqheight */
  if (c->geom) {
    *w += 2 * c->geom->ipadx;
    *h += 2 * c->geom->ipady;
  }
}

/* Offset of a child inside spare space for an anchor (0 = center). */
static int anchor_off(int anchor, int extra, int lo_bit, int hi_bit)
{
  const bool lo = anchor & lo_bit, hi = anchor & hi_bit;
  if (lo && !hi) {
    return 0;
  }
  if (hi && !lo) {
    return extra;
  }
  return extra / 2;
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Frame
 * \{ */

#define GRID_MAX 64

typedef struct FrameData {
  float colweight[GRID_MAX], rowweight[GRID_MAX];
  int colmin[GRID_MAX], rowmin[GRID_MAX];
} FrameData;

static void frame_init(BKNode *n)
{
  n->ext = calloc(1, sizeof(FrameData));
}

static void frame_free(BKNode *n)
{
  free(n->ext);
}

void BK_frame_grid_weight(BKNode *n, int column_axis, int index, double weight, int minsize)
{
  if (n->cls != &bk_cls_frame || index < 0 || index >= GRID_MAX) {
    return;
  }
  FrameData *d = (FrameData *)n->ext;
  if (column_axis) {
    d->colweight[index] = (float)weight;
    d->colmin[index] = minsize;
  }
  else {
    d->rowweight[index] = (float)weight;
    d->rowmin[index] = minsize;
  }
  bk_relayout(n);
}

static int frame_header_h(const BKNode *n)
{
  return (n->text && n->text[0]) ? bk_iround(1.25f * u()) : 0;
}

static void frame_insets(const BKNode *n, int *l, int *t, int *r, int *b)
{
  const int p = n->padding;
  *l = *r = *b = p;
  *t = p + frame_header_h(n);
}

static bool managed(const BKNode *c, int manager)
{
  return NODE_VISIBLE(c) && c->geom && c->geom->manager == manager;
}

static int padsum(const int p[2])
{
  return p[0] + p[1];
}

/* Tk ComputePackRequest. */
static void pack_request(BKNode *n, int *rw, int *rh)
{
  int width = 0, height = 0, maxw = 0, maxh = 0;
  for (int i = 0; i < n->nkids; i++) {
    BKNode *c = n->kids[i];
    if (!managed(c, BK_GEOM_PACK)) {
      continue;
    }
    int cw, ch;
    child_req(c, &cw, &ch);
    const BKGeom *g = c->geom;
    if (g->side <= 1) { /* top / bottom */
      maxw = BK_MAX(maxw, cw + padsum(g->padx) + width);
      height += ch + padsum(g->pady);
    }
    else {
      maxh = BK_MAX(maxh, ch + padsum(g->pady) + height);
      width += cw + padsum(g->padx);
    }
  }
  *rw = BK_MAX(maxw, width);
  *rh = BK_MAX(maxh, height);
}

static int x_expansion(BKNode *n, int from, int cavity_w)
{
  int num = 0, min_exp = cavity_w;
  for (int i = from; i < n->nkids; i++) {
    BKNode *c = n->kids[i];
    if (!managed(c, BK_GEOM_PACK)) {
      continue;
    }
    int cw, ch;
    child_req(c, &cw, &ch);
    const int child_w = cw + padsum(c->geom->padx);
    if (c->geom->side <= 1) {
      if (num) {
        min_exp = BK_MIN(min_exp, (cavity_w - child_w) / num);
      }
    }
    else {
      cavity_w -= child_w;
      if (c->geom->expand) {
        num++;
      }
    }
  }
  if (num) {
    min_exp = BK_MIN(min_exp, cavity_w / num);
  }
  return BK_MAX(0, min_exp);
}

static int y_expansion(BKNode *n, int from, int cavity_h)
{
  int num = 0, min_exp = cavity_h;
  for (int i = from; i < n->nkids; i++) {
    BKNode *c = n->kids[i];
    if (!managed(c, BK_GEOM_PACK)) {
      continue;
    }
    int cw, ch;
    child_req(c, &cw, &ch);
    const int child_h = ch + padsum(c->geom->pady);
    if (c->geom->side >= 2) {
      if (num) {
        min_exp = BK_MIN(min_exp, (cavity_h - child_h) / num);
      }
    }
    else {
      cavity_h -= child_h;
      if (c->geom->expand) {
        num++;
      }
    }
  }
  if (num) {
    min_exp = BK_MIN(min_exp, cavity_h / num);
  }
  return BK_MAX(0, min_exp);
}

/* Tk ArrangePacking. */
static void do_pack(BKNode *n, int x0, int y0, int w, int h)
{
  int cx = x0, cy = y0, cw = w, ch = h;
  for (int i = 0; i < n->nkids; i++) {
    BKNode *c = n->kids[i];
    if (!managed(c, BK_GEOM_PACK)) {
      continue;
    }
    const BKGeom *g = c->geom;
    int rw, rh;
    child_req(c, &rw, &rh);
    int fx, fy, fw, fh;
    if (g->side <= 1) {
      fw = cw;
      fh = rh + padsum(g->pady);
      if (g->expand) {
        fh += y_expansion(n, i, ch);
      }
      fh = BK_CLAMP(fh, 0, ch);
      fx = cx;
      if (g->side == 0) {
        fy = cy;
        cy += fh;
      }
      else {
        fy = cy + ch - fh;
      }
      ch -= fh;
    }
    else {
      fw = rw + padsum(g->padx);
      fh = ch;
      if (g->expand) {
        fw += x_expansion(n, i, cw);
      }
      fw = BK_CLAMP(fw, 0, cw);
      fy = cy;
      if (g->side == 2) {
        fx = cx;
        cx += fw;
      }
      else {
        fx = cx + cw - fw;
      }
      cw -= fw;
    }
    const int aw = BK_MAX(0, fw - padsum(g->padx)), ah = BK_MAX(0, fh - padsum(g->pady));
    const int ww = ((g->fill & 1) || rw > aw) ? aw : rw;
    const int hh = ((g->fill & 2) || rh > ah) ? ah : rh;
    bk_arrange(c, fx + g->padx[0] + anchor_off(g->anchor, aw - ww, BK_ANCHOR_W, BK_ANCHOR_E),
               fy + g->pady[0] + anchor_off(g->anchor, ah - hh, BK_ANCHOR_N, BK_ANCHOR_S), ww,
               hh);
  }
}

/* -- grid -- */

static void grid_sizes(BKNode *n, int *colw, int *rowh, int *r_ncols, int *r_nrows)
{
  FrameData *d = (FrameData *)n->ext;
  int ncols = 0, nrows = 0;
  memset(colw, 0, sizeof(int) * GRID_MAX);
  memset(rowh, 0, sizeof(int) * GRID_MAX);
  /* single-span children first, then spanning ones grow the last cell */
  for (int pass = 0; pass < 2; pass++) {
    for (int i = 0; i < n->nkids; i++) {
      BKNode *c = n->kids[i];
      if (!managed(c, BK_GEOM_GRID)) {
        continue;
      }
      const BKGeom *g = c->geom;
      const int c0 = BK_CLAMP(g->column, 0, GRID_MAX - 1), r0 = BK_CLAMP(g->row, 0, GRID_MAX - 1);
      const int cs = BK_CLAMP(g->columnspan, 1, GRID_MAX - c0),
                rs = BK_CLAMP(g->rowspan, 1, GRID_MAX - r0);
      ncols = BK_MAX(ncols, c0 + cs);
      nrows = BK_MAX(nrows, r0 + rs);
      int cw, ch;
      child_req(c, &cw, &ch);
      cw += padsum(g->padx);
      ch += padsum(g->pady);
      if (pass == 0 && cs == 1) {
        colw[c0] = BK_MAX(colw[c0], cw);
      }
      if (pass == 0 && rs == 1) {
        rowh[r0] = BK_MAX(rowh[r0], ch);
      }
      if (pass == 1 && cs > 1) {
        int sum = 0;
        for (int k = c0; k < c0 + cs; k++) {
          sum += colw[k];
        }
        if (sum < cw) {
          colw[c0 + cs - 1] += cw - sum;
        }
      }
      if (pass == 1 && rs > 1) {
        int sum = 0;
        for (int k = r0; k < r0 + rs; k++) {
          sum += rowh[k];
        }
        if (sum < ch) {
          rowh[r0 + rs - 1] += ch - sum;
        }
      }
    }
  }
  for (int k = 0; k < ncols; k++) {
    colw[k] = BK_MAX(colw[k], d->colmin[k]);
  }
  for (int k = 0; k < nrows; k++) {
    rowh[k] = BK_MAX(rowh[k], d->rowmin[k]);
  }
  *r_ncols = ncols;
  *r_nrows = nrows;
}

static void grid_request(BKNode *n, int *rw, int *rh)
{
  int colw[GRID_MAX], rowh[GRID_MAX], nc, nr;
  grid_sizes(n, colw, rowh, &nc, &nr);
  *rw = *rh = 0;
  for (int k = 0; k < nc; k++) {
    *rw += colw[k];
  }
  for (int k = 0; k < nr; k++) {
    *rh += rowh[k];
  }
}

static void distribute(int *sizes, int count, const float *weights, int extra)
{
  float total = 0.0f;
  for (int k = 0; k < count; k++) {
    total += weights[k];
  }
  if (extra <= 0 || total <= 0.0f) {
    return;
  }
  int given = 0, last = -1;
  for (int k = 0; k < count; k++) {
    if (weights[k] > 0.0f) {
      const int add = (int)(extra * weights[k] / total);
      sizes[k] += add;
      given += add;
      last = k;
    }
  }
  if (last >= 0) {
    sizes[last] += extra - given; /* rounding remainder */
  }
}

static void do_grid(BKNode *n, int x0, int y0, int w, int h)
{
  FrameData *d = (FrameData *)n->ext;
  int colw[GRID_MAX], rowh[GRID_MAX], nc, nr;
  grid_sizes(n, colw, rowh, &nc, &nr);
  if (!nc) {
    return;
  }
  int sw = 0, sh = 0;
  for (int k = 0; k < nc; k++) {
    sw += colw[k];
  }
  for (int k = 0; k < nr; k++) {
    sh += rowh[k];
  }
  distribute(colw, nc, d->colweight, w - sw);
  distribute(rowh, nr, d->rowweight, h - sh);
  int colx[GRID_MAX + 1], rowy[GRID_MAX + 1];
  colx[0] = x0;
  for (int k = 0; k < nc; k++) {
    colx[k + 1] = colx[k] + colw[k];
  }
  rowy[0] = y0;
  for (int k = 0; k < nr; k++) {
    rowy[k + 1] = rowy[k] + rowh[k];
  }
  for (int i = 0; i < n->nkids; i++) {
    BKNode *c = n->kids[i];
    if (!managed(c, BK_GEOM_GRID)) {
      continue;
    }
    const BKGeom *g = c->geom;
    const int c0 = BK_CLAMP(g->column, 0, GRID_MAX - 1), r0 = BK_CLAMP(g->row, 0, GRID_MAX - 1);
    const int c1 = BK_MIN(nc, c0 + g->columnspan), r1 = BK_MIN(nr, r0 + g->rowspan);
    const int cx = colx[c0] + g->padx[0], cy = rowy[r0] + g->pady[0];
    const int cw = BK_MAX(0, colx[c1] - colx[c0] - padsum(g->padx));
    const int ch = BK_MAX(0, rowy[r1] - rowy[r0] - padsum(g->pady));
    int rw, rh;
    child_req(c, &rw, &rh);
    const int st = g->sticky;
    const int ww = ((st & BK_ANCHOR_W) && (st & BK_ANCHOR_E)) ? cw : BK_MIN(rw, cw);
    const int hh = ((st & BK_ANCHOR_N) && (st & BK_ANCHOR_S)) ? ch : BK_MIN(rh, ch);
    bk_arrange(c, cx + anchor_off(st, cw - ww, BK_ANCHOR_W, BK_ANCHOR_E),
               cy + anchor_off(st, ch - hh, BK_ANCHOR_N, BK_ANCHOR_S), ww, hh);
  }
}

static void do_place(BKNode *c, int x0, int y0, int pw, int ph)
{
  const BKGeom *g = c->geom;
  int rw, rh;
  child_req(c, &rw, &rh);
  float x = g->relx * pw + g->x, y = g->rely * ph + g->y;
  float ww = g->relwidth * pw + g->width, hh = g->relheight * ph + g->height;
  if (!(g->has & (HAS_W | HAS_RW))) {
    ww = (float)rw;
  }
  if (!(g->has & (HAS_H | HAS_RH))) {
    hh = (float)rh;
  }
  /* anchor: which point of the child sits at (x, y); place defaults to nw */
  const int a = g->anchor ? g->anchor : (BK_ANCHOR_N | BK_ANCHOR_W);
  x -= anchor_off(a, (int)ww, BK_ANCHOR_W, BK_ANCHOR_E);
  y -= anchor_off(a, (int)hh, BK_ANCHOR_N, BK_ANCHOR_S);
  bk_arrange(c, x0 + (int)x, y0 + (int)y, (int)ww, (int)hh);
}

static void frame_measure(BKNode *n, int *rw, int *rh)
{
  int l, t, r, b;
  frame_insets(n, &l, &t, &r, &b);
  int pw = 0, ph = 0, gw = 0, gh = 0;
  if (!(n->flags & NF_NOPROPAGATE)) {
    pack_request(n, &pw, &ph);
    grid_request(n, &gw, &gh);
  }
  int w = BK_MAX(pw, gw), h = BK_MAX(ph, gh);
  if (frame_header_h(n)) {
    w = BK_MAX(w, (int)bk_text_width(n->text, -1, false) + 2 * u());
  }
  *rw = w + l + r;
  *rh = h + t + b;
}

static void frame_arrange(BKNode *n)
{
  int l, t, r, b;
  frame_insets(n, &l, &t, &r, &b);
  const int x0 = n->x + l, y0 = n->y + t;
  const int w = BK_MAX(0, n->w - l - r), h = BK_MAX(0, n->h - t - b);
  do_pack(n, x0, y0, w, h);
  do_grid(n, x0, y0, w, h);
  for (int i = 0; i < n->nkids; i++) {
    BKNode *c = n->kids[i];
    if (managed(c, BK_GEOM_PLACE)) {
      do_place(c, x0, y0, w, h);
    }
    else if (!c->geom || c->geom->manager == BK_GEOM_NONE) {
      c->x = c->y = c->w = c->h = 0; /* unmanaged: not shown (Tk) */
    }
  }
}

static bool shown(const BKNode *c)
{
  return NODE_VISIBLE(c) && c->geom && c->geom->manager != BK_GEOM_NONE;
}

static void frame_draw(BKNode *n)
{
  float c[4];
  const int hh = frame_header_h(n);
  if (hh) { /* LabelFrame / QGroupBox: a Blender panel (header + body) */
    const float rad = bk_theme.panel_roundness * 0.5f * u();
    bk_rgba(bk_theme.panel_back, c);
    bk_roundbox(n->x, n->y, n->w, n->h, rad, CNR_ALL, c);
    bk_rgba(bk_theme.panel_header, c);
    bk_roundbox(n->x, n->y, n->w, hh, rad, CNR_TL | CNR_TR, c);
    BKRect tr = bk_rect(n->x + (int)(0.4f * u()), n->y, n->w, hh);
    bk_text_in(&tr, n->text, bk_theme.text, BK_ALIGN_LEFT, false);
  }
  else if (n->flags & NF_HAS_BG) {
    bk_fill_f(n->x, n->y, n->w, n->h, n->color);
  }
}

/* Shared by frames and canvases: own drawing, then managed children clipped
 * to the widget (Tk children never spill out). */
static void frame_draw_tree(BKNode *n)
{
  if (!NODE_VISIBLE(n)) {
    return;
  }
  BKRect r = bk_rect(n->x - 3, n->y - 3, n->w + 6, n->h + 6);
  if (!bk_clip_visible(&r)) {
    return;
  }
  n->cls->draw(n);
  bk_clip_push(n->x, n->y, n->w, n->h + 2); /* children never spill out (Tk) */
  for (int i = 0; i < n->nkids; i++) {
    if (shown(n->kids[i])) {
      bk_draw_tree(n->kids[i]);
    }
  }
  bk_clip_pop();
}

static BKNode *frame_hit(BKNode *n, int x, int y)
{
  for (int i = n->nkids - 1; i >= 0; i--) {
    BKNode *c = n->kids[i];
    if (shown(c)) {
      BKNode *f = bk_hit(c, x, y);
      if (f) {
        return f;
      }
    }
  }
  return (n->flags & NF_EVENTS) ? n : NULL;
}

const BKClass bk_cls_frame = {.name = "frame",
                              .init = frame_init,
                              .free_ext = frame_free,
                              .measure = frame_measure,
                              .arrange = frame_arrange,
                              .draw = frame_draw,
                              .draw_tree = frame_draw_tree,
                              .hit = frame_hit};

/** \} */

/* -------------------------------------------------------------------- */
/** \name Radio button
 * \{ */

static int radio_press(BKNode *n, BKEvent *e)
{
  if (!n->value) {
    bk_set_value(n, 1); /* a radio only ever turns itself on */
  }
  return 1;
}

static void radio_measure(BKNode *n, int *rw, int *rh)
{
  *rw = u() + (int)bk_text_width(n->label, -1, false) + bk_pad();
  *rh = u();
}

static void radio_draw(BKNode *n)
{
  BKRect r = bk_node_rect(n);
  int st = NODE_ENABLED(n) ? (bk_hovered(n) ? ST_HOVER : 0) : ST_DISABLED;
  if (n->value) {
    st |= ST_SELECT;
  }
  BKWCol w;
  bk_widget_state(&w, &bk_theme.option, st);
  w.roundness = 0.5f; /* round option box */
  BKRect box = r;
  bk_widget_optionbut(&w, &box, st & ~ST_SELECT); /* no checkmark: a dot instead */
  if (n->value) {
    const int s = bk_rh(&r);
    const float cx = r.xmin + s * 0.5f - (s - 2 * bk_px()) / 6.0f, cy = r.ymin + s * 0.5f;
    float dot[4];
    bk_rgba(w.item, dot);
    bk_circle(cx, cy, s * 0.14f, dot);
  }
  uchar tc[4];
  memcpy(tc, bk_theme.option.text, 4);
  if (!NODE_ENABLED(n)) {
    tc[3] /= 2;
  }
  box.xmin -= bk_pad() / 2;
  bk_text_in(&box, n->label, tc, BK_ALIGN_LEFT, false);
}

static int radio_to_text(BKNode *n, char *buf, int size)
{
  buf[0] = n->value ? '1' : '0';
  buf[1] = 0;
  return 1;
}

const BKClass bk_cls_radio = {.name = "radio",
                              .flags = CLS_INTERACTIVE,
                              .measure = radio_measure,
                              .draw = radio_draw,
                              .press = radio_press,
                              .to_text = radio_to_text};

/** \} */

/* -------------------------------------------------------------------- */
/** \name Images
 * \{ */

void bk_image_free(BKImage *img)
{
  if (img) {
    free(img->argb);
    free(img);
  }
}

static BKImage *image_from_rgba(const unsigned char *rgba, int w, int h)
{
  if (!rgba || w <= 0 || h <= 0) {
    return NULL;
  }
  BKImage *img = (BKImage *)calloc(1, sizeof(BKImage));
  img->w = w;
  img->h = h;
  img->argb = (uint32_t *)malloc(sizeof(uint32_t) * (size_t)w * h);
  for (int i = 0; i < w * h; i++) {
    const unsigned char *p = rgba + 4 * (size_t)i;
    img->argb[i] = ((uint32_t)p[3] << 24) | ((uint32_t)p[0] << 16) | ((uint32_t)p[1] << 8) | p[2];
  }
  return img;
}

void bk_image_draw(const BKImage *img, float x, float y, float w, float h, float alpha)
{
  if (img) {
    BUI_raster_image(bk_draw.r, x, y, w, h, img->argb, img->w, img->h, alpha);
  }
}

void BK_node_set_image(BKNode *n, const unsigned char *rgba, int w, int h)
{
  bk_image_free((BKImage *)n->image);
  n->image = image_from_rgba(rgba, w, h);
  bk_relayout(n);
  bk_invalidate(n);
}

static void image_measure(BKNode *n, int *rw, int *rh)
{
  const BKImage *img = (const BKImage *)n->image;
  *rw = img ? img->w : u();
  *rh = img ? img->h : u();
}

static void image_draw(BKNode *n)
{
  const BKImage *img = (const BKImage *)n->image;
  if (img) {
    bk_image_draw(img, n->x + (n->w - img->w) / 2, n->y + (n->h - img->h) / 2, img->w, img->h,
                  NODE_ENABLED(n) ? 1.0f : 0.5f);
  }
}

const BKClass bk_cls_image = {.name = "image", .measure = image_measure, .draw = image_draw};

/** \} */

/* -------------------------------------------------------------------- */
/** \name Canvas (display list)
 * \{ */

typedef struct CItem {
  BKCanvasItem it;
  float *coords;
  char *text;
  BKImage *img;
} CItem;

typedef struct CanvasData {
  CItem *items;
  int n, cap;
} CanvasData;

#define CANVAS(n) ((CanvasData *)(n)->ext)

static void canvas_init(BKNode *n)
{
  n->ext = calloc(1, sizeof(CanvasData));
}

static void citem_free(CItem *c)
{
  free(c->coords);
  free(c->text);
  bk_image_free(c->img);
}

static void canvas_free(BKNode *n)
{
  CanvasData *d = CANVAS(n);
  for (int i = 0; i < d->n; i++) {
    citem_free(&d->items[i]);
  }
  free(d->items);
  free(d);
}

static int canvas_find(CanvasData *d, int id)
{
  for (int i = 0; i < d->n; i++) {
    if (d->items[i].it.id == id) {
      return i;
    }
  }
  return -1;
}

void BK_canvas_set_item(BKNode *n, const BKCanvasItem *item)
{
  if (n->cls != &bk_cls_canvas) {
    return;
  }
  CanvasData *d = CANVAS(n);
  int i = canvas_find(d, item->id);
  if (i < 0) {
    if (d->n == d->cap) {
      d->cap = d->cap ? d->cap * 2 : 16;
      d->items = (CItem *)realloc(d->items, sizeof(CItem) * (size_t)d->cap);
    }
    i = d->n++;
    memset(&d->items[i], 0, sizeof(CItem));
  }
  else {
    citem_free(&d->items[i]);
  }
  CItem *c = &d->items[i];
  c->it = *item;
  c->coords = (float *)malloc(sizeof(float) * (size_t)BK_MAX(1, item->ncoords));
  memcpy(c->coords, item->coords, sizeof(float) * (size_t)item->ncoords);
  c->it.coords = c->coords;
  c->text = item->text ? bk_strdup(item->text) : NULL;
  c->it.text = c->text;
  c->img = image_from_rgba(item->rgba, item->iw, item->ih);
  c->it.rgba = NULL;
  bk_invalidate(n);
}

void BK_canvas_delete(BKNode *n, int id)
{
  CanvasData *d = CANVAS(n);
  for (int i = d->n - 1; i >= 0; i--) {
    if (id < 0 || d->items[i].it.id == id) {
      citem_free(&d->items[i]);
      memmove(d->items + i, d->items + i + 1, sizeof(CItem) * (size_t)(d->n - i - 1));
      d->n--;
    }
  }
  bk_invalidate(n);
}

void BK_canvas_raise(BKNode *n, int id, int to_top)
{
  CanvasData *d = CANVAS(n);
  const int i = canvas_find(d, id);
  if (i < 0) {
    return;
  }
  CItem tmp = d->items[i];
  memmove(d->items + i, d->items + i + 1, sizeof(CItem) * (size_t)(d->n - i - 1));
  if (to_top) {
    d->items[d->n - 1] = tmp;
  }
  else {
    memmove(d->items + 1, d->items, sizeof(CItem) * (size_t)(d->n - 1));
    d->items[0] = tmp;
  }
  bk_invalidate(n);
}

int BK_canvas_count(BKNode *n)
{
  return CANVAS(n)->n;
}

static void canvas_measure(BKNode *n, int *rw, int *rh)
{
  *rw = 10 * u();
  *rh = 5 * u();
}

static void draw_polyline(const float *pts, int npts, float ox, float oy, const float c[4],
                          float width, bool closed)
{
  for (int k = 0; k + 1 < npts; k++) {
    bk_line(ox + pts[2 * k], oy + pts[2 * k + 1], ox + pts[2 * k + 2], oy + pts[2 * k + 3], c,
            width);
  }
  if (closed && npts > 2) {
    bk_line(ox + pts[2 * npts - 2], oy + pts[2 * npts - 1], ox + pts[0], oy + pts[1], c, width);
  }
}

static void canvas_item_draw(const CItem *ci, float ox, float oy)
{
  const BKCanvasItem *it = &ci->it;
  if (it->hidden) {
    return;
  }
  const float *p = ci->coords;
  const float lw = it->width > 0 ? it->width : 1.0f;
  const bool fill = it->fill[3] > 0, outline = it->outline[3] > 0;
  BuiRaster *r = bk_draw.r;
  switch (it->type) {
    case BK_ITEM_RECT:
    case BK_ITEM_ROUNDRECT: {
      if (it->ncoords < 4) {
        return;
      }
      const float x0 = BK_MIN(p[0], p[2]), y0 = BK_MIN(p[1], p[3]);
      const float w = fabsf(p[2] - p[0]), h = fabsf(p[3] - p[1]);
      if (it->type == BK_ITEM_ROUNDRECT || it->radius > 0) {
        BuiWidgetBase wb = {{ox + x0, oy + y0, w, h}, it->radius, BK_MAX(0, it->radius - lw), CNR_ALL};
        if (fill) {
          memcpy(wb.inner1, it->fill, sizeof(float[4]));
          memcpy(wb.inner2, it->fill, sizeof(float[4]));
        }
        if (outline) {
          memcpy(wb.outline, it->outline, sizeof(float[4]));
        }
        wb.pixelsize = lw;
        BUI_raster_widget_base(r, &wb);
      }
      else {
        if (fill) {
          BUI_raster_fill_rect(r, ox + x0, oy + y0, w, h, it->fill);
        }
        if (outline) {
          BUI_raster_frame_rect(r, ox + x0, oy + y0, w, h, lw, it->outline);
        }
      }
      break;
    }
    case BK_ITEM_OVAL: {
      if (it->ncoords < 4) {
        return;
      }
      const float cx = ox + (p[0] + p[2]) * 0.5f, cy = oy + (p[1] + p[3]) * 0.5f;
      const float rx = fabsf(p[2] - p[0]) * 0.5f, ry = fabsf(p[3] - p[1]) * 0.5f;
      if (fill) {
        BUI_raster_ellipse(r, cx, cy, rx, ry, 0.0f, it->fill);
      }
      if (outline) {
        BUI_raster_ellipse(r, cx, cy, rx, ry, lw, it->outline);
      }
      break;
    }
    case BK_ITEM_LINE: {
      const float *c = it->fill[3] > 0 ? it->fill : it->outline;
      draw_polyline(p, it->ncoords / 2, ox, oy, c, lw, false);
      break;
    }
    case BK_ITEM_POLYGON: {
      const int np = it->ncoords / 2;
      if (np >= 3 && fill) {
        float(*pts)[2] = (float(*)[2])malloc(sizeof(float[2]) * (size_t)np);
        for (int k = 0; k < np; k++) {
          pts[k][0] = ox + p[2 * k];
          pts[k][1] = oy + p[2 * k + 1];
        }
        BUI_raster_polygon(r, (const float(*)[2])pts, np, it->fill);
        free(pts);
      }
      if (outline) {
        draw_polyline(p, np, ox, oy, it->outline, lw, true);
      }
      break;
    }
    case BK_ITEM_ARC: {
      if (it->ncoords < 4) {
        return;
      }
      /* Tk pieslice: center + points along the arc (degrees ccw from 3 o'clock) */
      const float cx = ox + (p[0] + p[2]) * 0.5f, cy = oy + (p[1] + p[3]) * 0.5f;
      const float rx = fabsf(p[2] - p[0]) * 0.5f, ry = fabsf(p[3] - p[1]) * 0.5f;
      const int seg = 48;
      float pts[50][2];
      pts[0][0] = cx;
      pts[0][1] = cy;
      for (int k = 0; k <= seg; k++) {
        const float a = (it->start + it->extent * k / seg) * 3.14159265f / 180.0f;
        pts[k + 1][0] = cx + rx * cosf(a);
        pts[k + 1][1] = cy - ry * sinf(a);
      }
      if (fill) {
        BUI_raster_polygon(r, (const float(*)[2])pts, seg + 2, it->fill);
      }
      if (outline) {
        for (int k = 1; k <= seg; k++) {
          bk_line(pts[k][0], pts[k][1], pts[k + 1][0], pts[k + 1][1], it->outline, lw);
        }
      }
      break;
    }
    case BK_ITEM_TEXT: {
      if (!ci->text || it->ncoords < 2) {
        return;
      }
      const float size = it->size > 0 ? it->size : bk_font_size();
      const int len = (int)strlen(ci->text);
      float tw = 0;
      BUI_text_measure("", 0, size, ci->text, len, &tw);
      float x = ox + p[0], y = oy + p[1];
      x -= anchor_off(it->anchor, (int)tw, BK_ANCHOR_W, BK_ANCHOR_E);
      /* y: baseline from the anchor (cap height ~0.72 em) */
      const float asc = size * 0.72f;
      y += asc - anchor_off(it->anchor, (int)asc, BK_ANCHOR_N, BK_ANCHOR_S);
      const float *c = it->fill[3] > 0 ? it->fill : it->outline;
      BUI_raster_set_font(r, "", 0);
      BUI_raster_text(r, floorf(x + 0.5f), floorf(y + 0.5f), ci->text, len, size, c);
      break;
    }
    case BK_ITEM_IMAGE: {
      if (!ci->img || it->ncoords < 2) {
        return;
      }
      const float w = (float)ci->img->w, h = (float)ci->img->h;
      const float x = ox + p[0] - anchor_off(it->anchor, (int)w, BK_ANCHOR_W, BK_ANCHOR_E);
      const float y = oy + p[1] - anchor_off(it->anchor, (int)h, BK_ANCHOR_N, BK_ANCHOR_S);
      bk_image_draw(ci->img, x, y, w, h, 1.0f);
      break;
    }
  }
}

static void canvas_draw(BKNode *n)
{
  if (n->flags & NF_HAS_BG) {
    bk_fill_f(n->x, n->y, n->w, n->h, n->color);
  }
  CanvasData *d = CANVAS(n);
  bk_clip_push(n->x, n->y, n->w, n->h);
  const float ox = (float)(n->x - n->scroll_x), oy = (float)(n->y - n->scroll);
  for (int i = 0; i < d->n; i++) {
    canvas_item_draw(&d->items[i], ox, oy);
  }
  bk_clip_pop();
}

/* create_window: place-managed children sit at canvas coordinates, scrolled
 * with the items. */
static void canvas_arrange(BKNode *n)
{
  for (int i = 0; i < n->nkids; i++) {
    BKNode *c = n->kids[i];
    if (managed(c, BK_GEOM_PLACE)) {
      do_place(c, n->x - n->scroll_x, n->y - n->scroll, n->w, n->h);
    }
    else {
      c->x = c->y = c->w = c->h = 0;
    }
  }
}

const BKClass bk_cls_canvas = {.name = "canvas",
                               .init = canvas_init,
                               .free_ext = canvas_free,
                               .measure = canvas_measure,
                               .arrange = canvas_arrange,
                               .draw = canvas_draw,
                               .draw_tree = frame_draw_tree,
                               .hit = frame_hit};

/** \} */
