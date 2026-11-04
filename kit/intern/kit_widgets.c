/* Layouts and leaf widgets.
 *
 * Layout follows Blender's uiLayout: columns/rows with buttonspace gaps,
 * aligned blocks share edges (corners squared, no gaps, emboss only on the
 * last), property splits, boxes and panels (header + body, sub-panels).
 * Widgets draw with the ported interface_widgets.cc functions and follow
 * interface_handlers.cc behavior: number drag/click/type, toggles with
 * drag-toggle, text editing, enum menus.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <ctype.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "kit_intern.h"

/* -------------------------------------------------------------------- */
/** \name Shared helpers
 * \{ */

static int u(void)
{
  return bk_unit();
}

/* Blender's uiStyle: buttonspacex/y, boxspace, panel margins. */
static int space_y(void)
{
  return BK_MAX(1, bk_iround(2.0f * bk_scale_fac));
}
static int space_x(void)
{
  return bk_iround(0.2f * u());
}

static int state_of(const BKNode *n)
{
  int st = 0;
  if (!NODE_ENABLED(n)) {
    st |= ST_DISABLED;
  }
  else if (bk_hovered(n)) {
    st |= ST_HOVER;
  }
  if (!(n->flags & NF_EMBOSS)) {
    st |= ST_NO_EMBOSS;
  }
  if (n->flags & NF_SELECTED) {
    st = (st & ~ST_HOVER) | ST_SELECT; /* part of a multi-drag */
  }
  /* aligned column: only the bottom button draws the emboss */
  if (n->parent && (n->parent->flags & NF_ALIGN) && !(n->corners & (CNR_BL | CNR_BR)) &&
      (n->parent->cls == &bk_cls_column))
  {
    st |= ST_ALIGN_DOWN;
  }
  return st;
}

static void format_number(const BKNode *n, double v, char *buf, int size)
{
  if (n->flags & NF_INTEGER) {
    snprintf(buf, (size_t)size, "%d", (int)floor(v + 0.5));
  }
  else {
    snprintf(buf, (size_t)size, "%.*f", n->precision, v);
  }
}

static void number_text(const BKNode *n, char *buf, int size)
{
  format_number(n, n->value, buf, size);
  if (n->suffix && n->suffix[0] && !(n->flags & NF_INTEGER)) {
    /* values with a unit read like Blender's BKE_unit_value_as_string: up to
     * five decimals, trailing zeros stripped ("0 m", "1.25 m", "90Â°") */
    snprintf(buf, (size_t)size, "%.*f", BK_MAX(n->precision, 5), n->value);
    if (strchr(buf, '.')) {
      size_t l = strlen(buf);
      while (l > 0 && buf[l - 1] == '0') {
        buf[--l] = 0;
      }
      if (l > 0 && buf[l - 1] == '.') {
        buf[--l] = 0;
      }
      if (strcmp(buf, "-0") == 0) {
        strcpy(buf, "0");
      }
    }
  }
  if (n->suffix) {
    strncat(buf, n->suffix, (size_t)(size - (int)strlen(buf) - 1));
  }
}

static const char *item_text(const BKNode *n, int i)
{
  return (i >= 0 && i < n->items.n) ? n->items.v[i] : "";
}

static int item_icon(const BKNode *n, int i)
{
  return (n->item_icons && i >= 0 && i < n->item_icons_s.n) ? n->item_icons[i] : 0;
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Column / Row / Grid / Box
 * \{ */

static int stack_gap(const BKNode *n, bool row)
{
  if (n->flags & NF_ALIGN) {
    return 0;
  }
  if (n->spacing >= 0) {
    return n->spacing;
  }
  if (n->cls == &bk_cls_split) {
    return bk_iround(0.4f * u()); /* Layout::split: style->columnspace */
  }
  return row ? space_x() : space_y();
}

static int join_corners(const BKNode *n, int i, int count, int first, int last)
{
  if (!(n->flags & NF_ALIGN) || count == 1) {
    return n->corners;
  }
  int c = 0;
  if (i == 0) {
    c |= first;
  }
  if (i == count - 1) {
    c |= last;
  }
  return c & n->corners;
}

static void column_measure(BKNode *n, int *rw, int *rh)
{
  const int gap = stack_gap(n, false), p = n->padding;
  int w = 0, h = 0, count = 0;
  for (int i = 0; i < n->nkids; i++) {
    BKNode *c = n->kids[i];
    if (!NODE_VISIBLE(c)) {
      continue;
    }
    int cw, ch;
    bk_natural(c, &cw, &ch);
    w = BK_MAX(w, cw);
    h += ch + (count ? gap : 0);
    count++;
  }
  *rw = w + 2 * p;
  *rh = h + 2 * p;
}

static void column_arrange(BKNode *n)
{
  const int gap = stack_gap(n, false), p = n->padding;
  int count = 0;
  for (int i = 0; i < n->nkids; i++) {
    count += NODE_VISIBLE(n->kids[i]);
  }
  /* spare height goes to the children marked `fill` */
  int used = 0, fills = 0;
  for (int i = 0; i < n->nkids; i++) {
    BKNode *c = n->kids[i];
    if (NODE_VISIBLE(c)) {
      int cw, ch;
      bk_natural(c, &cw, &ch);
      used += ch;
      fills += c->fill != 0;
    }
  }
  int spare = fills ? BK_MAX(0, n->h - 2 * p - used - gap * BK_MAX(0, count - 1)) : 0;
  int y = n->y + p, k = 0;
  for (int i = 0; i < n->nkids; i++) {
    BKNode *c = n->kids[i];
    if (!NODE_VISIBLE(c)) {
      continue;
    }
    c->corners = join_corners(n, k, count, CNR_TL | CNR_TR, CNR_BL | CNR_BR);
    int cw, ch;
    bk_natural(c, &cw, &ch);
    if (c->fill && fills) {
      const int extra = spare / fills;
      ch += extra;
      spare -= extra;
      fills--;
    }
    bk_arrange(c, n->x + p, y, n->w - 2 * p, ch);
    y += ch + gap;
    k++;
  }
}

static int vector_key(BKNode *n, BKEvent *e)
{
  /* Ctrl+Alt+C / Ctrl+Alt+V over a vector copies/pastes all components. */
  if (!(n->flags & NF_VECTOR) || !(EV_CTRL(e) && EV_ALT(e))) {
    return 0;
  }
  BKContext *ctx = bk_ctx(n);
  if (bk_keysym_is(e->keysym, "c")) {
    char buf[512] = "[", num[64];
    for (int i = 0; i < n->nkids; i++) {
      format_number(n->kids[i], n->kids[i]->value, num, sizeof(num));
      strcat(buf, num);
      strcat(buf, i + 1 < n->nkids ? ", " : "]");
    }
    bk_clipboard_set(ctx, buf);
    return 1;
  }
  if (bk_keysym_is(e->keysym, "v")) {
    char *s = bk_clipboard_get(ctx);
    if (!s) {
      return 1;
    }
    double vals[16];
    int count = 0;
    char *tok = s;
    while (*tok && count < 16) {
      while (*tok && (*tok == '[' || *tok == '(' || *tok == ' ' || *tok == ',')) {
        tok++;
      }
      if (!*tok || *tok == ']' || *tok == ')') {
        break;
      }
      char *end;
      vals[count] = strtod(tok, &end);
      if (end == tok) {
        count = -1;
        break;
      }
      count++;
      tok = end;
    }
    if (count == n->nkids) {
      for (int i = 0; i < count; i++) {
        bk_set_value(n->kids[i], vals[i]);
      }
    }
    free(s);
    return 1;
  }
  return 0;
}

const BKClass bk_cls_column = {
    .name = "column", .measure = column_measure, .arrange = column_arrange, .key = vector_key};

static void row_measure(BKNode *n, int *rw, int *rh)
{
  const int gap = stack_gap(n, true), p = n->padding;
  int w = 0, h = 0, count = 0;
  for (int i = 0; i < n->nkids; i++) {
    BKNode *c = n->kids[i];
    if (!NODE_VISIBLE(c)) {
      continue;
    }
    int cw, ch;
    bk_natural(c, &cw, &ch);
    w += cw + (count ? gap : 0);
    h = BK_MAX(h, ch);
    count++;
  }
  *rw = w + 2 * p;
  *rh = h + 2 * p;
}

static void row_arrange(BKNode *n)
{
  const int gap = stack_gap(n, true), p = n->padding;
  int count = 0, fixed = 0;
  float total = 0.0f;
  for (int i = 0; i < n->nkids; i++) {
    BKNode *c = n->kids[i];
    if (!NODE_VISIBLE(c)) {
      continue;
    }
    count++;
    if (c->weight <= 0.0f) { /* weight 0: natural width (icon buttons) */
      int cw, ch;
      bk_natural(c, &cw, &ch);
      fixed += cw;
    }
    else {
      total += c->weight;
    }
  }
  if (!count) {
    return;
  }
  const float avail = BK_MAX(0.0f, (float)(n->w - 2 * p - gap * (count - 1) - fixed));
  if (total <= 0.0f) {
    total = 1.0f;
  }
  float x = (float)(n->x + p);
  int k = 0;
  for (int i = 0; i < n->nkids; i++) {
    BKNode *c = n->kids[i];
    if (!NODE_VISIBLE(c)) {
      continue;
    }
    c->corners = join_corners(n, k, count, CNR_TL | CNR_BL, CNR_TR | CNR_BR);
    float cw;
    if (c->weight <= 0.0f) {
      int nw, nh;
      bk_natural(c, &nw, &nh);
      cw = (float)nw;
    }
    else {
      cw = avail * c->weight / total;
    }
    const int x0 = bk_iround(x), x1 = bk_iround(x + cw);
    bk_arrange(c, x0, n->y + p, x1 - x0, n->h - 2 * p);
    x += cw + gap;
    k++;
  }
}

const BKClass bk_cls_row = {.name = "row", .measure = row_measure, .arrange = row_arrange};

/* A property split: a row whose columns are `columnspace` apart. */
const BKClass bk_cls_split = {.name = "split", .measure = row_measure, .arrange = row_arrange};

static void grid_measure(BKNode *n, int *rw, int *rh)
{
  const int cols = BK_MAX(1, n->columns), gap = (n->flags & NF_ALIGN) ? 0 : space_x();
  int cw = 0, h = 0, rowh = 0, k = 0, rows = 0;
  for (int i = 0; i < n->nkids; i++) {
    BKNode *c = n->kids[i];
    if (!NODE_VISIBLE(c)) {
      continue;
    }
    int w, hh;
    bk_natural(c, &w, &hh);
    cw = BK_MAX(cw, w);
    rowh = BK_MAX(rowh, hh);
    if (++k % cols == 0) {
      h += rowh;
      rowh = 0;
      rows++;
    }
  }
  if (k % cols) {
    h += rowh;
    rows++;
  }
  *rw = cw * cols + gap * (cols - 1);
  *rh = h + space_y() * BK_MAX(0, rows - 1);
}

static void grid_arrange(BKNode *n)
{
  const int cols = BK_MAX(1, n->columns), gap = (n->flags & NF_ALIGN) ? 0 : space_x();
  const float cw = (n->w - gap * (cols - 1)) / (float)cols;
  int y = n->y, k = 0, rowh = 0;
  for (int i = 0; i < n->nkids; i++) {
    BKNode *c = n->kids[i];
    if (!NODE_VISIBLE(c)) {
      continue;
    }
    int w, h;
    bk_natural(c, &w, &h);
    const int col = k % cols;
    const int x0 = bk_iround(n->x + col * (cw + gap)), x1 = bk_iround(n->x + col * (cw + gap) + cw);
    bk_arrange(c, x0, y, x1 - x0, h);
    rowh = BK_MAX(rowh, h);
    if (col == cols - 1) {
      y += rowh + space_y();
      rowh = 0;
    }
    k++;
  }
}

static void grid_init(BKNode *n)
{
  n->columns = 2;
}

const BKClass bk_cls_grid = {
    .name = "grid", .init = grid_init, .measure = grid_measure, .arrange = grid_arrange};

static void box_init(BKNode *n)
{
  n->padding = bk_iround(0.3f * u());
}

static void box_draw(BKNode *n)
{
  BKRect r = bk_node_rect(n);
  BKWCol w = bk_theme.box;
  bk_widget_box(&w, &r, n->corners, NULL);
}

const BKClass bk_cls_box = {.name = "box",
                            .init = box_init,
                            .measure = column_measure,
                            .arrange = column_arrange,
                            .draw = box_draw};

/** \} */

/* -------------------------------------------------------------------- */
/** \name Panel (header + collapsible body, sub-panels, drag-collapse, reorder)
 * \{ */

#define PANEL_BODY(n) ((BKNode *)(n)->ext)

/* A panel's body, as area.cc lays a panel out (ED_region_panels_layout_ex):
 * its own content inset by style->panelspace on every side, then its child
 * panels at the full width, one under the other. */
static bool is_panel(const BKNode *c)
{
  return c->cls == &bk_cls_panel;
}

static void panel_body_layout(BKNode *n, bool place, int *r_w, int *r_h)
{
  const int p = n->padding, gap = stack_gap(n, false);
  int y = 0, w = 0;
  bool in_content = false; /* inside a run of content (not child panels) */
  for (int i = 0; i < n->nkids; i++) {
    BKNode *c = n->kids[i];
    if (!NODE_VISIBLE(c)) {
      continue;
    }
    int cw, ch;
    bk_natural(c, &cw, &ch);
    if (is_panel(c)) {
      if (in_content) {
        y += p; /* the content's bottom margin */
        in_content = false;
      }
      if (place) {
        bk_arrange(c, n->x, n->y + y, n->w, ch);
      }
      w = BK_MAX(w, cw);
      y += ch;
    }
    else {
      y += in_content ? gap : p; /* the top margin, or the gap between rows */
      in_content = true;
      if (place) {
        bk_arrange(c, n->x + p, n->y + y, n->w - 2 * p, ch);
      }
      w = BK_MAX(w, cw + 2 * p);
      y += ch;
    }
  }
  if (in_content) {
    y += p;
  }
  *r_w = w;
  *r_h = y;
}

static void panel_body_measure(BKNode *n, int *rw, int *rh)
{
  panel_body_layout(n, false, rw, rh);
}

static void panel_body_arrange(BKNode *n)
{
  int w, h;
  panel_body_layout(n, true, &w, &h);
}

static const BKClass bk_cls_panel_body = {
    .name = "column", .measure = panel_body_measure, .arrange = panel_body_arrange};

static void panel_init(BKNode *n)
{
  BKNode *body = bk_node_new_cls(&bk_cls_panel_body);
  body->padding = bk_iround(0.4f * u()); /* style->panelspace: 8 px */
  n->ext = body;
  BK_node_add(n, body, -1);
  n->value = 1; /* open */
}

static bool panel_is_sub(const BKNode *n)
{
  const BKNode *p = n->parent;
  return p && p->parent && p->parent->cls == &bk_cls_panel && PANEL_BODY(p->parent) == p;
}

static int panel_header_h(const BKNode *n)
{
  /* PNL_HEADER = UI_UNIT_Y * 1.25, sub-panels included */
  (void)n;
  return bk_iround(1.25f * u());
}

static int panel_grip_w(const BKNode *n)
{
  /* the drag widget's area: from xmax - 1.15 units */
  return panel_is_sub(n) ? 0 : bk_iround(1.15f * u());
}

static void panel_measure(BKNode *n, int *rw, int *rh)
{
  int bw = 0, bh = 0;
  bk_natural(PANEL_BODY(n), &bw, &bh);
  int hw = (int)bk_text_width(n->text, -1, false) + 3 * u();
  for (int i = 0; i < n->nkids; i++) {
    if (n->kids[i] != PANEL_BODY(n)) {
      int w, h;
      bk_natural(n->kids[i], &w, &h);
      hw += w;
    }
  }
  *rw = BK_MAX(bw, hw);
  float open = (n->flags & NF_COLLAPSED) ? 0.0f : 1.0f;
  if (bk_anim_running(n)) {
    const float t = n->anim; /* smoothstep, like Blender's panel animation */
    open = t * t * (3.0f - 2.0f * t);
  }
  *rh = panel_header_h(n) + (int)(bh * open + 0.5f);
}

/* The header's widgets (draw_header): a row from panel_label_offset, 1.0 unit
 * in (+0.7 for sub-panels). Returns its width: Blender's panel->labelofs. */
static float panel_header_widgets_w(BKNode *n)
{
  float w = 0.0f;
  int count = 0;
  for (int i = 0; i < n->nkids; i++) {
    if (n->kids[i] != PANEL_BODY(n)) {
      int cw, ch;
      bk_natural(n->kids[i], &cw, &ch);
      w += (float)cw + (count++ ? 0.4f * u() : 0.0f); /* buttonspacex between items */
    }
  }
  return w;
}

static float panel_title_x(BKNode *n)
{
  /* panel_draw_aligned_widgets: xmin (+0.7 units for sub-panels) + labelofs + 1.1 units */
  return n->x + (panel_is_sub(n) ? 0.7f : 0.0f) * u() + 1.1f * u() + panel_header_widgets_w(n);
}

static void panel_arrange(BKNode *n)
{
  const int hh = panel_header_h(n);
  float hx = n->x + (panel_is_sub(n) ? 0.7f : 0.0f) * u() + 1.0f * u();
  for (int i = 0; i < n->nkids; i++) {
    BKNode *c = n->kids[i];
    if (c == PANEL_BODY(n)) {
      continue;
    }
    int w, h;
    bk_natural(c, &w, &h);
    bk_arrange(c, (int)hx, n->y + (hh - u()) / 2, w, u());
    hx += w + 0.4f * u();
  }
  BKNode *body = PANEL_BODY(n);
  if ((n->flags & NF_COLLAPSED) && !bk_anim_running(n)) {
    body->flags &= ~NF_VISIBLE;
  }
  else {
    body->flags |= NF_VISIBLE;
  }
  int bw, bh;
  bk_natural(body, &bw, &bh);
  bk_arrange(body, n->x, n->y + hh, n->w, bh); /* clipped while animating */
  n->value = !(n->flags & NF_COLLAPSED);
}

static BKNode *panel_hit(BKNode *n, int x, int y)
{
  if (y < n->y + panel_header_h(n)) {
    for (int i = n->nkids - 1; i >= 0; i--) {
      if (n->kids[i] != PANEL_BODY(n)) {
        BKNode *f = bk_hit(n->kids[i], x, y);
        if (f) {
          return f;
        }
      }
    }
    return n;
  }
  return bk_hit(PANEL_BODY(n), x, y);
}

static bool panel_on_grip(const BKNode *n, int x, int y)
{
  return panel_grip_w(n) && x >= n->x + n->w - panel_grip_w(n) && y < n->y + panel_header_h(n);
}

static int panel_cursor_at(BKNode *n, int x, int y)
{
  return panel_on_grip(n, x, y) ? BK_CURSOR_MOVE : BK_CURSOR_DEFAULT;
}

static void panel_set_open(BKNode *n, bool open)
{
  if (open == !(n->flags & NF_COLLAPSED)) {
    return;
  }
  if (!bk_anim_running(n)) {
    n->anim = open ? 0.0f : 1.0f; /* animate from the current state */
  }
  if (open) {
    n->flags &= ~NF_COLLAPSED;
  }
  else {
    n->flags |= NF_COLLAPSED;
  }
  n->value = open;
  bk_anim_start(n);
  bk_relayout(n);
  bk_emit(n, BK_EV_CHANGE, NULL, open, 0);
}

static int panel_press(BKNode *n, BKEvent *e)
{
  n->dragmode = panel_on_grip(n, e->x, e->y) ? 1 : 0;
  if (n->dragmode) {
    return 1;
  }
  if (EV_CTRL(e) && n->parent) { /* Ctrl+click: collapse all others */
    for (int i = 0; i < n->parent->nkids; i++) {
      BKNode *s = n->parent->kids[i];
      if (s->cls == &bk_cls_panel) {
        panel_set_open(s, s == n);
      }
    }
    return 1;
  }
  panel_set_open(n, n->flags & NF_COLLAPSED);
  bk_start_drag_toggle(bk_ctx(n), DRAG_GROUP_PANEL, (int)n->value);
  return 1;
}

static int panel_drag(BKNode *n, BKEvent *e)
{
  BKNode *p = n->parent;
  if (n->dragmode != 1 || !p) {
    return 0;
  }
  /* reorder among sibling panels by pointer position */
  int self = -1, last_panel = -1, before = -1;
  for (int i = 0; i < p->nkids; i++) {
    BKNode *s = p->kids[i];
    if (s == n) {
      self = i;
      continue;
    }
    if (s->cls != &bk_cls_panel) {
      continue;
    }
    last_panel = i;
    if (before < 0 && e->y < s->y + s->h / 2) {
      before = i;
    }
  }
  if (self < 0 || last_panel < 0) {
    return 0;
  }
  int target = before >= 0 ? before : last_panel + 1;
  if (target > self) {
    target--;
  }
  if (target == self) {
    return 0;
  }
  memmove(p->kids + self, p->kids + self + 1, sizeof(BKNode *) * (size_t)(p->nkids - self - 1));
  memmove(p->kids + target + 1, p->kids + target,
          sizeof(BKNode *) * (size_t)(p->nkids - 1 - target));
  p->kids[target] = n;
  bk_relayout(n);
  return 1;
}

static int panel_release(BKNode *n, BKEvent *e)
{
  n->dragmode = 0;
  return 1;
}

static int panel_key(BKNode *n, BKEvent *e)
{
  if (bk_keysym_is(e->keysym, "Return") || bk_keysym_is(e->keysym, "KP_Enter") ||
      bk_keysym_is(e->keysym, "space") || (bk_keysym_is(e->keysym, "a") && !EV_CTRL(e)))
  {
    panel_set_open(n, n->flags & NF_COLLAPSED); /* Blender: A over a header toggles it */
    return 1;
  }
  return 0;
}

/* Ported from Blender's interface_panel.cc: panel_draw_aligned_backdrop,
 * panel_draw_aligned_widgets and panel_draw_border (y flipped: BUI's y grows
 * downwards). */
static void panel_draw(BKNode *n)
{
  const int hh = panel_header_h(n);
  const float uu = (float)u();
  const bool sub = panel_is_sub(n);
  const bool open = !(n->flags & NF_COLLAPSED);
  const float rad = bk_theme.panel_roundness * 0.5f * uu;
  float c[4];

  /* backdrop: the body (open), then the header for top-level panels */
  if (open) {
    const float pad = sub ? 0.1f * uu : 0.0f; /* sub-panels: inset by 0.1 units */
    bk_rgba(sub ? bk_theme.panel_sub_back : bk_theme.panel_back, c);
    bk_roundbox(n->x + pad, n->y + hh, n->w - 2.0f * pad, n->h - hh - pad, rad,
                CNR_BL | CNR_BR, c);
  }
  if (!sub) {
    bk_rgba(bk_theme.panel_header, c);
    bk_roundbox(n->x, n->y, n->w, hh, rad, open ? (CNR_TL | CNR_TR) : CNR_ALL, c);
  }

  /* header widgets: collapse icon, title, drag grip */
  const float wx = n->x + (sub ? 0.7f * uu : 0.0f);
  float alpha = 0.8f;
  if (n->w < 4.0f * uu) { /* dim as its space shrinks to nothing */
    alpha *= BK_MAX((n->w - uu) / (3.0f * uu), 0.0f);
  }
  const float isz = (float)bk_icon_size();
  /* icon_draw_ex at (xmin + h * 0.2, ymin + h * 0.17 closed / 0.14 open), y up */
  const float icy = n->y + hh - hh * (open ? 0.14f : 0.17f) - isz * 0.5f;
  bk_icon(bk_icons_lookup(open ? "DOWNARROW_HLT" : "RIGHTARROW"), wx + hh * 0.2f + isz * 0.5f, icy,
          bk_theme.text, alpha, 0);
  float tx = panel_title_x(n);
  if (n->icon) {
    bk_icon(n->icon, tx + isz * 0.5f, n->y + hh * 0.5f, NULL, 1.0f, 0);
    tx += isz + 0.25f * uu;
  }
  /* title: paneltitle style (soft shadow) for panels, widget style for sub-panels;
   * its rect reaches 2 px below the header (title_rect.ymin - 2) */
  BKRect tr = {(int)tx - bk_pad(), n->y, n->x + n->w - (sub ? 0 : (int)uu), n->y + hh + 2};
  extern int bk_text_shadow;
  const int shadow = bk_text_shadow;
  bk_text_shadow = sub ? 1 : 3;
  bk_text_in(&tr, n->text, bk_theme.text, BK_ALIGN_LEFT, false);
  bk_text_shadow = shadow;
  if (!sub) {
    /* the grip: x = xmax - 1.15 units, alpha 0.5, the title color */
    float ga = 0.5f;
    if (n->w < 5.0f * uu) {
      ga *= BK_MAX((n->w - uu) / (4.0f * uu), 0.0f);
    }
    bk_icon(bk_icons_lookup("GRIP"), n->x + n->w - 1.15f * uu + isz * 0.5f, n->y + hh * 0.5f,
            bk_theme.text, ga, 0);
    /* panel_draw_border: the outline around the panel (the header when closed) */
    bk_rgba(bk_theme.panel_outline, c);
    if (c[3] > 0.0f) {
      const int bh = open ? n->h : hh;
      BuiWidgetBase wb = {{(float)n->x, (float)n->y, (float)n->w, (float)bh}, rad, rad - 1, CNR_ALL};
      memcpy(wb.outline, c, sizeof(c));
      wb.pixelsize = 1.0f;
      BUI_raster_widget_base(bk_draw.r, &wb);
    }
  }
}

static void panel_draw_tree(BKNode *n)
{
  if (!NODE_VISIBLE(n)) {
    return;
  }
  BKRect r = bk_rect(n->x - 3, n->y - 3, n->w + 6, n->h + 6);
  if (!bk_clip_visible(&r)) {
    return;
  }
  panel_draw(n);
  bk_clip_push(n->x, n->y, n->w, n->h); /* the body slides under the panel edge */
  for (int i = 0; i < n->nkids; i++) {
    bk_draw_tree(n->kids[i]);
  }
  bk_clip_pop();
}

const BKClass bk_cls_panel = {.name = "panel",
                              .draw_tree = panel_draw_tree,
                              .flags = CLS_INTERACTIVE,
                              .drag_group = DRAG_GROUP_PANEL,
                              .init = panel_init,
                              .measure = panel_measure,
                              .arrange = panel_arrange,
                              .draw = panel_draw,
                              .hit = panel_hit,
                              .press = panel_press,
                              .drag = panel_drag,
                              .release = panel_release,
                              .key = panel_key,
                              .cursor_at = panel_cursor_at};

/** \} */

/* -------------------------------------------------------------------- */
/** \name Tabs (top) and SideTabs (properties editor navigation bar)
 * \{ */

static BKNode *current_page(const BKNode *n)
{
  return (n->current >= 0 && n->current < n->nkids) ? n->kids[n->current] : NULL;
}

static void tabs_select(BKNode *n, int i)
{
  if (i >= 0 && i < n->nkids && i != n->current) {
    n->current = i;
    bk_relayout(n);
    bk_emit(n, BK_EV_CHANGE, NULL, i, 0);
  }
}

static int tab_h(void)
{
  return bk_iround(1.25f * u());
}

static void tab_geom(BKNode *n, int i, int *x, int *w)
{
  int xx = n->x;
  for (int k = 0; k < n->items.n; k++) {
    const int tw = (int)bk_text_width(n->items.v[k], -1, false) + 2 * u();
    if (k == i) {
      *x = xx;
      *w = tw;
      return;
    }
    xx += tw + 2;
  }
  *x = xx;
  *w = 0;
}

static void tabs_measure(BKNode *n, int *rw, int *rh)
{
  int pw = 0, ph = 0, tabs_w = 0;
  for (int i = 0; i < n->nkids; i++) { /* the largest page: switching tabs keeps the size */
    int w, h;
    bk_natural(n->kids[i], &w, &h);
    pw = BK_MAX(pw, w);
    ph = BK_MAX(ph, h);
  }
  for (int k = 0; k < n->items.n; k++) {
    tabs_w += (int)bk_text_width(n->items.v[k], -1, false) + 2 * u() + 2;
  }
  *rw = BK_MAX(tabs_w, pw);
  *rh = tab_h() + ph;
}

static void tabs_arrange(BKNode *n)
{
  const int th = tab_h();
  for (int i = 0; i < n->nkids; i++) {
    BKNode *c = n->kids[i];
    if (i == n->current) {
      c->flags |= NF_VISIBLE;
      bk_arrange(c, n->x, n->y + th, n->w, BK_MAX(0, n->h - th));
    }
    else {
      c->flags &= ~NF_VISIBLE;
    }
  }
}

static int tabs_tab_at(BKNode *n, int x, int y)
{
  if (y >= n->y + tab_h()) {
    return -1;
  }
  for (int i = 0; i < n->items.n; i++) {
    int tx, tw;
    tab_geom(n, i, &tx, &tw);
    if (x >= tx && x < tx + tw) {
      return i;
    }
  }
  return -1;
}

static BKNode *tabs_hit(BKNode *n, int x, int y)
{
  if (y < n->y + tab_h()) {
    return n;
  }
  BKNode *page = current_page(n);
  return page ? bk_hit(page, x, y) : NULL;
}

static int tabs_press(BKNode *n, BKEvent *e)
{
  tabs_select(n, tabs_tab_at(n, e->x, e->y));
  return 1;
}

static int tabs_move(BKNode *n, BKEvent *e)
{
  const int hot = tabs_tab_at(n, e->x, e->y);
  if (hot != n->hot) {
    n->hot = hot;
    bk_invalidate(n);
    return 1;
  }
  return 0;
}

static int tabs_leave(BKNode *n)
{
  n->hot = -1;
  return 1;
}

static int tabs_wheel(BKNode *n, BKEvent *e)
{
  if (EV_CTRL(e)) {
    tabs_select(n, n->current - (e->delta > 0 ? 1 : -1));
    return 1;
  }
  return 0;
}

static void tabs_draw(BKNode *n)
{
  const int th = tab_h();
  const float rad = bk_widget_radius(&bk_theme.tab);
  float c[4];
  for (int i = 0; i < n->items.n; i++) {
    int tx, tw;
    tab_geom(n, i, &tx, &tw);
    const bool active = i == n->current;
    if (active) {
      bk_rgba(bk_theme.panel_back, c);
    }
    else {
      BKWCol w = bk_theme.tab;
      if (i == n->hot) {
        bk_widget_active_color(&w);
      }
      bk_rgba(w.inner, c);
    }
    bk_roundbox(tx, n->y + 2, tw, th - 2 + (active ? 2 : 0), rad, CNR_TL | CNR_TR, c);
    BKRect tr = bk_rect(tx, n->y + 2, tw, th - 2);
    bk_text_in(&tr, n->items.v[i], active ? bk_theme.tab.text_sel : bk_theme.tab.text,
               BK_ALIGN_CENTER, false);
  }
  bk_rgba(bk_theme.panel_back, c);
  bk_roundbox(n->x, n->y + th, n->w, n->h - th, rad, CNR_ALL & ~CNR_TL, c);
}

const BKClass bk_cls_tabs = {.name = "tabs",
                             .flags = CLS_INTERACTIVE,
                             .measure = tabs_measure,
                             .arrange = tabs_arrange,
                             .draw = tabs_draw,
                             .hit = tabs_hit,
                             .press = tabs_press,
                             .move = tabs_move,
                             .leave = tabs_leave,
                             .wheel = tabs_wheel};

/* The Properties editor's navigation bar (space_properties.py,
 * PROPERTIES_PT_navigation_bar): icon-only tab buttons scaled 1.4, in aligned
 * groups, on the region's tab_back color. */
static int side_bar_w(void)
{
  return bk_iround(1.4f * u());
}
static int side_tab_h(void)
{
  return bk_iround(1.4f * u());
}
static int side_gap(void)
{
  return bk_iround(0.5f * u()); /* between groups, and above the first */
}
static bool side_group_start(const BKNode *n, int i)
{
  return i > 0 && i < n->shortcuts.n && n->shortcuts.v[i][0] == '1';
}
static int side_tab_y(const BKNode *n, int i)
{
  int y = n->y + side_gap();
  for (int k = 1; k <= i; k++) {
    y += side_tab_h() + (side_group_start(n, k) ? side_gap() : 0);
  }
  return y;
}

static void sidetabs_measure(BKNode *n, int *rw, int *rh)
{
  BKNode *page = current_page(n);
  int pw = 0, ph = 0;
  if (page) {
    bk_natural(page, &pw, &ph);
  }
  *rw = side_bar_w() + pw;
  *rh = BK_MAX(ph, n->nkids > 0 ? side_tab_y(n, n->nkids - 1) - n->y + side_tab_h() + side_gap() : 0);
}

static void sidetabs_arrange(BKNode *n)
{
  const int bw = side_bar_w();
  for (int i = 0; i < n->nkids; i++) {
    BKNode *c = n->kids[i];
    if (i == n->current) {
      c->flags |= NF_VISIBLE;
      bk_arrange(c, n->x + bw, n->y, n->w - bw, n->h);
    }
    else {
      c->flags &= ~NF_VISIBLE;
    }
  }
}

static int sidetabs_tab_at(BKNode *n, int x, int y)
{
  if (x >= n->x + side_bar_w()) {
    return -1;
  }
  for (int i = 0; i < n->nkids; i++) {
    const int ty = side_tab_y(n, i);
    if (y >= ty && y < ty + side_tab_h()) {
      return i;
    }
  }
  return -1;
}

static BKNode *sidetabs_hit(BKNode *n, int x, int y)
{
  if (x < n->x + side_bar_w()) {
    return n;
  }
  BKNode *page = current_page(n);
  return page ? bk_hit(page, x, y) : NULL;
}

static int sidetabs_press(BKNode *n, BKEvent *e)
{
  tabs_select(n, sidetabs_tab_at(n, e->x, e->y));
  return 1;
}

static int sidetabs_move(BKNode *n, BKEvent *e)
{
  const int hot = sidetabs_tab_at(n, e->x, e->y);
  if (hot != n->hot) {
    n->hot = hot;
    /* the tooltip shows the hovered tab's name */
    bk_set_text(&n->desc, hot >= 0 ? item_text(n, hot) : "");
    bk_invalidate(n);
    return 1;
  }
  return 0;
}

static int sidetabs_wheel(BKNode *n, BKEvent *e)
{
  if (EV_CTRL(e) || e->x < n->x + side_bar_w()) {
    tabs_select(n, n->current - (e->delta > 0 ? 1 : -1));
    return 1;
  }
  return 0;
}

static void sidetabs_draw(BKNode *n)
{
  /* widget_tab (interface_widgets.cc) per tab with wcol_tab: inner for the
   * group backdrop, inner_sel for the active tab, hover lightened +15 */
  const int bw = side_bar_w(), th = side_tab_h();
  const float rad = bk_widget_radius(&bk_theme.tab);
  float c[4];
  bk_fill(n->x, n->y, bw, n->h, bk_theme.tabs_bg);
  const int tx = n->x + 1, tw = bw - 1;
  for (int i = 0; i < n->nkids; i++) {
    const int ty = side_tab_y(n, i);
    const bool first = i == 0 || side_group_start(n, i);
    const bool last = i == n->nkids - 1 || side_group_start(n, i + 1);
    const int corners = (first ? CNR_TL | CNR_TR : 0) | (last ? CNR_BL | CNR_BR : 0);
    bk_rgba(bk_theme.tab.inner, c);
    bk_roundbox(tx, ty, tw, th - 1, rad, corners, c);
    if (last) { /* the widget emboss under the group */
      bk_rgba(bk_theme.emboss, c);
      bk_roundbox(tx, ty + th - 1, tw, 1, 0.0f, 0, c);
    }
    const bool active = i == n->current, hot = i == n->hot;
    if (active || hot) {
      bk_rgba(active ? bk_theme.tab.inner_sel : bk_theme.tab.inner, c);
      if (hot && !active) { /* widget_active_color: inner + 15 */
        for (int k = 0; k < 3; k++) {
          c[k] = BK_MIN(1.0f, c[k] + 15.0f / 255.0f);
        }
      }
      bk_roundbox(tx, ty, tw, th - 1, rad, CNR_ALL, c);
    }
    bk_icon(item_icon(n, i), tx + tw * 0.5f, ty + (th - 1) * 0.5f, NULL, 1.0f, 0);
  }
}

const BKClass bk_cls_sidetabs = {.name = "sidetabs",
                                 .flags = CLS_INTERACTIVE,
                                 .measure = sidetabs_measure,
                                 .arrange = sidetabs_arrange,
                                 .draw = sidetabs_draw,
                                 .hit = sidetabs_hit,
                                 .press = sidetabs_press,
                                 .move = sidetabs_move,
                                 .leave = tabs_leave,
                                 .wheel = sidetabs_wheel};

/** \} */

/* -------------------------------------------------------------------- */
/** \name ScrollView
 * \{ */

static int scroll_bar_w(void)
{
  /* V2D_SCROLL_WIDTH: 0.45 units + 2 px */
  return bk_iround(0.45f * u() + 2.0f * bk_px()); /* UI_view2d.hh */
}

static int scroll_content_h(BKNode *n)
{
  if (!n->nkids) {
    return 0;
  }
  int w, h;
  bk_natural(n->kids[0], &w, &h);
  return h;
}

static int scroll_max(BKNode *n)
{
  return BK_MAX(0, scroll_content_h(n) - n->h);
}

static void scroll_measure(BKNode *n, int *rw, int *rh)
{
  int cw = 0, ch = 0;
  if (n->nkids) {
    bk_natural(n->kids[0], &cw, &ch);
  }
  *rw = cw + scroll_bar_w();
  *rh = n->height_units > 0 ? (int)(n->height_units * u()) : n->rows * u();
}

static void scroll_arrange(BKNode *n)
{
  n->scroll = BK_CLAMP(n->scroll, 0, scroll_max(n));
  if (n->nkids) {
    bk_arrange(n->kids[0], n->x, n->y - n->scroll, n->w - scroll_bar_w(),
               BK_MAX(n->h, scroll_content_h(n)));
  }
}

static BKNode *scroll_hit(BKNode *n, int x, int y)
{
  if (x >= n->x + n->w - scroll_bar_w()) {
    return n;
  }
  return n->nkids ? bk_hit(n->kids[0], x, y) : NULL;
}

static void scroll_thumb(BKNode *n, float *ty, float *th)
{
  const int ch = BK_MAX(1, scroll_content_h(n)), ms = scroll_max(n);
  *th = BK_MAX((float)u(), n->h * BK_MIN(1.0f, (float)n->h / ch));
  *ty = n->y + (n->h - *th) * (ms ? (float)n->scroll / ms : 0.0f);
}

static void scroll_set(BKNode *n, int v)
{
  v = BK_CLAMP(v, 0, scroll_max(n));
  if (v != n->scroll) {
    n->scroll = v;
    bk_relayout(n);
  }
}

static int scroll_press(BKNode *n, BKEvent *e)
{
  float ty, th;
  scroll_thumb(n, &ty, &th);
  if (!(e->y >= ty && e->y < ty + th)) { /* click the track: jump */
    const float frac = (e->y - n->y - th * 0.5f) / BK_MAX(1.0f, n->h - th);
    scroll_set(n, (int)(BK_CLAMP(frac, 0.0f, 1.0f) * scroll_max(n)));
  }
  n->drag_start = n->scroll;
  return 1;
}

static int scroll_drag(BKNode *n, BKEvent *e)
{
  float ty, th;
  scroll_thumb(n, &ty, &th);
  const float span = BK_MAX(1.0f, n->h - th);
  scroll_set(n, (int)(n->drag_start + e->dy * scroll_max(n) / span));
  return 1;
}

static int scroll_wheel(BKNode *n, BKEvent *e)
{
  if (EV_CTRL(e)) {
    return 0;
  }
  const int old = n->scroll;
  scroll_set(n, n->scroll - e->delta * u() * 2);
  return n->scroll != old;
}

static void scroll_draw_tree(BKNode *n)
{
  BKRect r = bk_node_rect(n);
  if (!bk_clip_visible(&r)) {
    return;
  }
  bk_clip_push(n->x, n->y, n->w, n->h);
  if (n->nkids) {
    bk_draw_tree(n->kids[0]);
  }
  bk_clip_pop();
  if (scroll_max(n) > 0) {
    float ty, th;
    scroll_thumb(n, &ty, &th);
    const int bw = scroll_bar_w();
    BKWCol w = bk_theme.scroll;
    BKRect track = bk_rect(n->x + n->w - bw * 3 / 4, n->y, bw / 2, n->h);
    BKRect slider = bk_rect(track.xmin, (int)ty + 1, bw / 2, (int)th - 2);
    if (bk_hovered(n)) {
      bk_widget_active_color(&w);
    }
    bk_widget_scroll(&w, &track, &slider, bk_pressed(n));
  }
}

const BKClass bk_cls_scroll = {.name = "scroll",
                               .flags = CLS_INTERACTIVE,
                               .measure = scroll_measure,
                               .arrange = scroll_arrange,
                               .draw_tree = scroll_draw_tree,
                               .hit = scroll_hit,
                               .press = scroll_press,
                               .drag = scroll_drag,
                               .wheel = scroll_wheel};

/* -------------------------------------------------------------------- */
/** \name Scrollbar
 *
 * A stand-alone Blender scroller (View2D style, drawn by bk_widget_scroll)
 * for Tk's Scrollbar: it shows the [first, last] fractions it is given and
 * asks its client to move with BK_EV_SCROLL (moveto / units), never
 * scrolling anything itself (Tk's scrollbar protocol).
 * \{ */

static bool sb_vertical(const BKNode *n)
{
  return (n->flags & NF_VERTICAL) != 0;
}

static void sb_measure(BKNode *n, int *rw, int *rh)
{
  const int thick = BK_MAX(8, bk_iround(0.5f * u()));
  *rw = sb_vertical(n) ? thick : 2 * u();
  *rh = sb_vertical(n) ? 2 * u() : thick;
}

/* Track and thumb rects; returns the track length along the axis. */
static int sb_rects(const BKNode *n, BKRect *track, BKRect *thumb)
{
  const bool v = sb_vertical(n);
  const int thick = v ? n->w : n->h, inset = BK_MAX(1, thick / 5);
  *track = v ? bk_rect(n->x + inset, n->y + 1, n->w - 2 * inset, n->h - 2) :
               bk_rect(n->x + 1, n->y + inset, n->w - 2, n->h - 2 * inset);
  const int len = v ? bk_rh(track) : bk_rw(track);
  const int min_thumb = BK_MIN(len, bk_iround(0.6f * u()));
  int a = (int)(n->value * len), b = (int)(n->factor * len);
  if (b - a < min_thumb) { /* keep a grabbable thumb, centered on the view */
    const int mid = (a + b) / 2;
    a = BK_CLAMP(mid - min_thumb / 2, 0, len - min_thumb);
    b = a + min_thumb;
  }
  *thumb = v ? bk_rect(track->xmin, track->ymin + a, bk_rw(track), b - a) :
               bk_rect(track->xmin + a, track->ymin, b - a, bk_rh(track));
  return BK_MAX(1, len);
}

static void sb_moveto(BKNode *n, double first)
{
  const double span = n->factor - n->value;
  bk_emit(n, BK_EV_SCROLL, NULL, 0, BK_CLAMP(first, 0.0, 1.0 - span));
}

static int sb_press(BKNode *n, BKEvent *e)
{
  BKRect track, thumb;
  const int len = sb_rects(n, &track, &thumb);
  const int pos = sb_vertical(n) ? e->y : e->x;
  const int t0 = sb_vertical(n) ? thumb.ymin : thumb.xmin;
  const int t1 = sb_vertical(n) ? thumb.ymax : thumb.xmax;
  n->drag_start = n->value;
  if (pos < t0 || pos >= t1) { /* Blender: the thumb jumps under the pointer */
    const int base = sb_vertical(n) ? track.ymin : track.xmin;
    const double span = n->factor - n->value;
    n->drag_start = BK_CLAMP((double)(pos - base) / len - span * 0.5, 0.0, 1.0 - span);
    sb_moveto(n, n->drag_start);
  }
  return 1;
}

static int sb_drag(BKNode *n, BKEvent *e)
{
  BKRect track, thumb;
  const int len = sb_rects(n, &track, &thumb);
  sb_moveto(n, n->drag_start + (double)(sb_vertical(n) ? e->dy : e->dx) / len);
  return 1;
}

static int sb_wheel(BKNode *n, BKEvent *e)
{
  bk_emit(n, BK_EV_SCROLL, NULL, 1, -e->delta);
  return 1;
}

static void sb_draw(BKNode *n)
{
  BKRect track, thumb;
  sb_rects(n, &track, &thumb);
  BKWCol w = bk_theme.scroll;
  if (bk_hovered(n) || bk_pressed(n)) {
    bk_widget_active_color(&w);
  }
  const bool full = n->value <= 0.0 && n->factor >= 1.0;
  BKRect none = bk_rect(0, 0, 0, 0);
  bk_widget_scroll(&w, &track, full ? &none : &thumb, bk_pressed(n));
}

static int sb_set_num(BKNode *n, const char *key, double v)
{
  if (strcmp(key, "first") == 0 || strcmp(key, "last") == 0) {
    double *dst = key[0] == 'f' ? &n->value : &n->factor;
    v = BK_CLAMP(v, 0.0, 1.0);
    if (*dst != v) {
      *dst = v;
      bk_invalidate(n);
    }
    return 1;
  }
  return 0;
}

static int sb_get_num(BKNode *n, const char *key, double *r_v)
{
  if (strcmp(key, "first") == 0 || strcmp(key, "last") == 0) {
    *r_v = key[0] == 'f' ? n->value : n->factor;
    return 1;
  }
  return 0;
}

static void sb_init(BKNode *n)
{
  n->flags |= NF_VERTICAL; /* Tk's default orient */
  n->value = 0.0;
  n->factor = 1.0;
}

const BKClass bk_cls_scrollbar = {.name = "scrollbar",
                                  .flags = CLS_INTERACTIVE,
                                  .init = sb_init,
                                  .measure = sb_measure,
                                  .draw = sb_draw,
                                  .press = sb_press,
                                  .drag = sb_drag,
                                  .wheel = sb_wheel,
                                  .set_num = sb_set_num,
                                  .get_num = sb_get_num};

/** \} */

/** \} */

/* -------------------------------------------------------------------- */
/** \name Label, Separator, Progress
 * \{ */

static void label_measure(BKNode *n, int *rw, int *rh)
{
  const BKImage *img = (const BKImage *)n->image;
  if (img && !(n->text && n->text[0])) {
    *rw = img->w;
    *rh = img->h;
    return;
  }
  const char *s = n->text ? n->text : "";
  int lines = 1, widest = 0;
  for (const char *line = s;; lines++) { /* widest line of a multi-line label */
    const char *end = strchr(line, '\n');
    const int len = end ? (int)(end - line) : (int)strlen(line);
    char buf[512];
    memcpy(buf, line, (size_t)BK_MIN(len, 511));
    buf[BK_MIN(len, 511)] = 0;
    widest = BK_MAX(widest, (int)bk_content_width(buf, n->icon, n->flags & NF_BOLD));
    if (!end) {
      break;
    }
    line = end + 1;
  }
  *rw = widest + 2 * bk_pad();
  *rh = lines > 1 ? BK_MAX(u(), (int)(lines * bk_line_height()) + bk_pad()) : u();
}

static void label_draw(BKNode *n)
{
  uchar c[4];
  memcpy(c, n->has_color_text ? n->color_text : bk_theme.text, 4);
  if (!NODE_ENABLED(n)) {
    c[3] /= 2;
  }
  BKRect r = bk_node_rect(n);
  const BKImage *img = (const BKImage *)n->image;
  if (img) { /* Tk Label(image=) / QLabel.setPixmap */
    bk_image_draw(img, r.xmin + (bk_rw(&r) - img->w) / 2, r.ymin + (bk_rh(&r) - img->h) / 2,
                  img->w, img->h, NODE_ENABLED(n) ? 1.0f : 0.5f);
    if (!(n->text && n->text[0])) {
      return;
    }
  }
  if (n->icon) {
    bk_text_icon(&r, n->text, n->icon, c, n->align, NODE_ENABLED(n));
  }
  else {
    /* BUT_NO_TEXT_PADDING: labels without icons sit one pixel in (widget_draw_text) */
    bk_text_in_pad(&r, n->text, c, n->align, n->flags & NF_BOLD, (int)bk_px());
  }
}

const BKClass bk_cls_label = {.name = "label", .measure = label_measure, .draw = label_draw};

static void separator_init(BKNode *n)
{
  n->factor = 0.5;
}

static void separator_measure(BKNode *n, int *rw, int *rh)
{
  *rw = 0;
  *rh = BK_MAX(1, (int)(n->factor * u()));
}

static void separator_draw(BKNode *n)
{
  if (n->flags & NF_LINE) {
    float c[4];
    bk_rgba_alpha(bk_theme.text, 0.15f, c);
    bk_fill_f(n->x, n->y + n->h / 2, n->w, 1, c);
  }
}

const BKClass bk_cls_separator = {.name = "separator",
                                  .init = separator_init,
                                  .measure = separator_measure,
                                  .draw = separator_draw};

static void progress_measure(BKNode *n, int *rw, int *rh)
{
  *rw = 6 * u();
  *rh = u();
}

static void progress_draw(BKNode *n)
{
  BKRect r = bk_node_rect(n);
  BKWCol w;
  bk_widget_state(&w, &bk_theme.progress, NODE_ENABLED(n) ? 0 : ST_DISABLED);
  bk_widget_progress(&w, &r, n->corners, (float)n->factor);
  if (n->text) {
    bk_text_in(&r, n->text, w.text, BK_ALIGN_CENTER, false);
  }
}

const BKClass bk_cls_progress = {
    .name = "progress", .measure = progress_measure, .draw = progress_draw};

/** \} */

/* -------------------------------------------------------------------- */
/** \name Button (operator), Toggle, Checkbox, Decorator
 * \{ */

static void button_measure(BKNode *n, int *rw, int *rh)
{
  const BKImage *img = (const BKImage *)n->image;
  if (img) { /* Tk Button(image=): the image replaces the text */
    *rw = img->w + bk_pad();
    *rh = BK_MAX(u(), img->h + bk_pad() / 2);
    return;
  }
  const char *s = n->cls == &bk_cls_button ? n->text : n->label;
  if (!s || !s[0]) {
    *rw = u();
  }
  else {
    *rw = BK_MAX((int)bk_content_width(s, n->icon, false) + 2 * u(), 3 * u());
  }
  *rh = u();
}

static int button_press(BKNode *n, BKEvent *e)
{
  bk_invalidate(n); /* pressed look */
  return 1;
}

static void button_click(BKNode *n)
{
  if (NODE_ENABLED(n)) {
    if (n->on_click) {
      n->on_click(n, n->on_click_data);
    }
    bk_emit(n, BK_EV_CLICK, NULL, 0, 0);
  }
}

static int button_release(BKNode *n, BKEvent *e)
{
  if (e->inside) {
    button_click(n);
  }
  return 1;
}

static int button_key(BKNode *n, BKEvent *e)
{
  if (bk_keysym_is(e->keysym, "Return") || bk_keysym_is(e->keysym, "KP_Enter")) {
    button_click(n);
    return 1;
  }
  return 0;
}

static void draw_flat_or_embossed(BKNode *n, BKWCol *w, BKRect *r, int st)
{
  /* EmbossType::None draws no backdrop (interface_widgets.cc: wt->draw is
   * skipped); hover shows in the icon's alpha instead */
  if (n->flags & NF_EMBOSS) {
    bk_widget_but(w, r, st, n->corners);
  }
}

static void button_draw(BKNode *n)
{
  BKRect r = bk_node_rect(n);
  int st = state_of(n);
  if (bk_pressed(n)) {
    st = (st & ~ST_HOVER) | ST_SELECT;
  }
  BKWCol w;
  bk_widget_state(&w, &bk_theme.regular, st);
  draw_flat_or_embossed(n, &w, &r, st);
  const BKImage *img = (const BKImage *)n->image;
  if (img) {
    bk_image_draw(img, r.xmin + (bk_rw(&r) - img->w) / 2, r.ymin + (bk_rh(&r) - img->h) / 2,
                  img->w, img->h, NODE_ENABLED(n) ? 1.0f : 0.5f);
    return;
  }
  bk_text_icon(&r, n->text, n->icon, (n->flags & NF_EMBOSS) ? w.text : bk_theme.text,
               BK_ALIGN_CENTER, NODE_ENABLED(n));
}

const BKClass bk_cls_button = {.name = "button",
                               .flags = CLS_INTERACTIVE,
                               .measure = button_measure,
                               .draw = button_draw,
                               .press = button_press,
                               .release = button_release,
                               .key = button_key};

/* -- booleans -- */

static int bool_to_text(BKNode *n, char *buf, int size)
{
  snprintf(buf, (size_t)size, "%s", n->value ? "True" : "False");
  return 1;
}

static int bool_from_text(BKNode *n, const char *s)
{
  if (bk_keysym_is(s, "1") || bk_keysym_is(s, "true") || bk_keysym_is(s, "yes") ||
      bk_keysym_is(s, "on"))
  {
    bk_set_value(n, 1);
    return 1;
  }
  if (bk_keysym_is(s, "0") || bk_keysym_is(s, "false") || bk_keysym_is(s, "no") ||
      bk_keysym_is(s, "off"))
  {
    bk_set_value(n, 0);
    return 1;
  }
  return 0;
}

static void bool_toggle(BKNode *n)
{
  bk_set_value(n, n->value ? 0 : 1);
}

static void bool_step(BKNode *n, int dir)
{
  bool_toggle(n);
}

static void bool_reset(BKNode *n, bool zero)
{
  bk_set_value(n, zero ? 0 : n->defval);
}

static int bool_press(BKNode *n, BKEvent *e)
{
  bool_toggle(n);
  bk_start_drag_toggle(bk_ctx(n), DRAG_GROUP_TOGGLE, n->value != 0);
  return 1;
}

static void checkbox_measure(BKNode *n, int *rw, int *rh)
{
  if (!n->label || !n->label[0]) {
    *rw = bk_iround(0.7f * u()); /* the box alone */
  }
  else {
    *rw = u() + (int)bk_text_width(n->label, -1, false) + bk_pad();
  }
  *rh = u();
}

static void checkbox_draw(BKNode *n)
{
  BKRect r = bk_node_rect(n);
  int st = state_of(n);
  if (n->value) {
    st |= ST_SELECT;
  }
  BKWCol w;
  bk_widget_state(&w, &bk_theme.option, st);
  bk_widget_optionbut(&w, &r, st);
  uchar tc[4];
  memcpy(tc, w.text, 4); /* widget_state: text_sel when checked */
  if (!NODE_ENABLED(n)) {
    tc[3] /= 2;
  }
  /* widget_optionbut set the text space; BUT_NO_TEXT_PADDING: nothing more */
  bk_text_in_pad(&r, n->label, tc, BK_ALIGN_LEFT, false, 0);
}

#define BOOL_CLASS_COMMON \
  .flags = CLS_INTERACTIVE | CLS_VALUE, .drag_group = DRAG_GROUP_TOGGLE, .press = bool_press, \
  .to_text = bool_to_text, .from_text = bool_from_text, .step = bool_step, \
  .reset = bool_reset, .activate = bool_toggle

const BKClass bk_cls_checkbox = {.name = "checkbox",
                                 BOOL_CLASS_COMMON,
                                 .measure = checkbox_measure,
                                 .draw = checkbox_draw};

static void toggle_draw(BKNode *n)
{
  BKRect r = bk_node_rect(n);
  int st = state_of(n);
  const bool on = n->value != 0;
  if (on && (n->flags & NF_EMBOSS)) {
    st |= ST_SELECT;
  }
  BKWCol w;
  bk_widget_state(&w, &bk_theme.toggle, st);
  draw_flat_or_embossed(n, &w, &r, st);
  const int icon = (on && n->icon_on) ? n->icon_on : n->icon;
  uchar tc[4];
  memcpy(tc, (n->flags & NF_EMBOSS) ? w.text : bk_theme.text, 4);
  /* widget_draw_icon: toggles not selected and not hovered draw at alpha 0.75 */
  const bool dim = !on && !(st & ST_HOVER) && !n->icon_on;
  if (dim) {
    tc[3] = (uchar)(tc[3] * 0.75f);
  }
  if (icon && (!n->label || !n->label[0])) {
    bk_icon(icon, r.xmin + bk_rw(&r) * 0.5f, r.ymin + bk_rh(&r) * 0.5f, dim ? tc : NULL,
            NODE_ENABLED(n) ? 1.0f : 0.5f, 0);
  }
  else {
    bk_text_icon(&r, n->label, icon, tc, BK_ALIGN_CENTER, NODE_ENABLED(n));
  }
}

const BKClass bk_cls_toggle = {
    .name = "toggle", BOOL_CLASS_COMMON, .measure = button_measure, .draw = toggle_draw};

static void decorator_measure(BKNode *n, int *rw, int *rh)
{
  *rw = *rh = u();
}

static void decorator_draw(BKNode *n)
{
  /* Blender's property decorator: a dot, or a keyframe diamond when keyed */
  const char *name = n->value ? "DECORATE_KEYFRAME" : "DECORATE";
  uchar c[4];
  memcpy(c, bk_theme.text, 4);
  c[3] = (uchar)(255 * (bk_hovered(n) ? 0.6f : 0.4f));
  bk_icon(bk_icons_lookup(name), n->x + n->w * 0.5f, n->y + n->h * 0.5f, n->value ? NULL : c,
          NODE_ENABLED(n) ? 1.0f : 0.5f, 0);
}

static int decorator_context(BKNode *n, BKMenuSpec *items, int max)
{
  return 0;
}

const BKClass bk_cls_decorator = {.name = "decorator",
                                  BOOL_CLASS_COMMON,
                                  .measure = decorator_measure,
                                  .draw = decorator_draw,
                                  .context_items = decorator_context};

/** \} */

/* -------------------------------------------------------------------- */
/** \name Numbers: NumberField and NumberSlider
 * \{ */

static int number_to_text(BKNode *n, char *buf, int size)
{
  format_number(n, n->value, buf, size);
  return 1;
}

static int number_from_text(BKNode *n, const char *s)
{
  char tmp[256];
  snprintf(tmp, sizeof(tmp), "%s", s);
  if (n->suffix && n->suffix[0]) {
    const size_t ls = strlen(tmp), lf = strlen(n->suffix);
    if (ls >= lf && strcmp(tmp + ls - lf, n->suffix) == 0) {
      tmp[ls - lf] = 0;
    }
  }
  double v;
  if (!BK_eval_number(tmp, &v)) {
    return 0;
  }
  bk_set_value(n, v);
  return 1;
}

static void number_step(BKNode *n, int dir)
{
  /* Ctrl+wheel: number fields step by their click step; sliders by a fixed
   * 0.01 (floats) or 1 (ints), ui_do_but_SLI click == 2 */
  double step = n->step;
  if (n->cls == &bk_cls_slider) {
    step = (n->flags & NF_INTEGER) ? 1.0 : 0.01;
  }
  bk_set_value(n, n->value + dir * step);
}

static void number_reset(BKNode *n, bool zero)
{
  bk_set_value(n, zero ? 0.0 : n->defval);
}

static void number_negate(BKNode *n)
{
  bk_set_value(n, -n->value);
}

static void number_begin_text(BKNode *n)
{
  char buf[128];
  format_number(n, n->value, buf, sizeof(buf));
  bk_begin_edit(n, buf, true);
}

static void number_finish(BKNode *n, bool confirm)
{
  if (confirm && n->edit) {
    number_from_text(n, n->edit->text);
  }
}

static int number_cursor_at(BKNode *n, int x, int y)
{
  return n->edit ? BK_CURSOR_TEXT : BK_CURSOR_EW;
}

/* Snapping while dragged with Ctrl (Ctrl+Shift: finer), ported from
 * interface_handlers.cc: numedit_apply_snapf / numedit_apply_snap for number
 * fields, numedit_but_SLI for sliders. */
static bool is_degrees(const BKNode *n)
{
  return n->suffix && strstr(n->suffix, "\xc2\xb0") != NULL;
}

static double snap_float(double v, double range, bool small)
{
  const double f = small ? 0.1 : 1.0;
  if (range < 2.10) {
    return round(v * 10.0 / f) * 0.1 * f;
  }
  if (range < 21.0) {
    return round(v / f) * f;
  }
  return round(v * 0.1 / f) * 10.0 * f;
}

static double snap(const BKNode *n, double v, int mods)
{
  if (!(mods & BK_CTRL) || v == n->vmin || v == n->vmax) {
    return v;
  }
  const bool small = (mods & BK_SHIFT) != 0;
  if (n->flags & NF_INTEGER) {
    const long t = (long)v, m = small ? 100 : 10;
    return (double)(m * (t / m));
  }
  double range = n->vmax - n->vmin;
  if (n->cls == &bk_cls_slider) {
    /* numedit_but_SLI: Shift makes the snap finer by ten */
    return small ? (range < 2.10 ? round(v * 100.0) * 0.01 :
                                   (range < 21.0 ? round(v * 10.0) * 0.1 : round(v))) :
                   snap_float(v, range, false);
  }
  /* too coarse for unbounded floats (location, scale), but allowed for degrees */
  if (range >= 21.0 && !is_degrees(n)) {
    range = 20.0;
  }
  return snap_float(v, range, small);
}

/* numedit_but_NUM, the cursor-warp mapping: floats move one click step per
 * pixel (Shift: a tenth), integers 1, 1/2 or 1/16 per pixel by their range.
 * Past a limit the start point moves, so dragging back responds at once. */
static double number_drag_value(BKNode *n, double mx, int mods)
{
  const double vmin = n->vmin, vmax = n->vmax, range = vmax - vmin;
  if (n->flags & NF_INTEGER) {
    const double fac = range > 256 ? 1.0 : (range > 32 ? 0.5 : 1.0 / 16.0);
    double t = (double)(long)(n->drag_start + (mx - n->drag_x0) * fac);
    t = snap(n, t, mods);
    if (t < vmin) {
      n->drag_x0 -= (vmin - t) / fac;
      t = vmin;
    }
    else if (t > vmax) {
      n->drag_x0 += (t - vmax) / fac;
      t = vmax;
    }
    return t;
  }
  const double fac = (n->step > 0 ? n->step : 0.01) * ((mods & BK_SHIFT) ? 0.1 : 1.0);
  double t = snap(n, n->drag_start + (mx - n->drag_x0) * fac, mods);
  if (t < vmin) {
    n->drag_x0 -= (vmin - t) / fac;
    t = vmin;
  }
  else if (t > vmax) {
    n->drag_x0 -= (vmax - t) / fac;
    t = vmax;
  }
  return t;
}

/* numedit_but_SLI: the value follows the pointer over the slider's width,
 * relative to where the drag began (Shift: 1/20 the speed). */
static double slider_drag_value(BKNode *n, double mx, int mods)
{
  const double w = BK_MAX(1, n->w);
  const double fac = (mods & BK_SHIFT) ? 0.05 : 1.0; /* mouse_scale_warp */
  const double mx_fl = n->drag_x0 * (1.0 - fac) + mx * fac;
  const double f = BK_CLAMP((mx_fl - n->drag_x0) / w + n->drag_f0, 0.0, 1.0);
  BKContext *ctx = bk_ctx(n);
  if (ctx) { /* USE_CONT_MOUSE_CORRECT: the pointer comes back at the value */
    ctx->ungrab_x = BK_CLAMP(n->x + (int)(f * w), n->x, n->x + n->w - 1);
    ctx->ungrab_y = n->y + n->h / 2;
    ctx->has_ungrab = true;
  }
  double t = n->vmin + f * (n->vmax - n->vmin);
  if (n->flags & NF_INTEGER) {
    t = round(t);
  }
  return snap(n, t, mods);
}

static double scrub_value(BKNode *n, BKEvent *e)
{
  /* a Shift press or release restarts from here, so the value doesn't jump */
  if ((e->mods & BK_SHIFT) != (n->drag_mods & BK_SHIFT)) {
    n->drag_mods = e->mods;
    if (n->cls == &bk_cls_slider) {
      const double span = n->vmax - n->vmin;
      n->drag_f0 = span != 0 ? (float)((n->value - n->vmin) / span) : 0.0f;
    }
    else {
      n->drag_start = n->value;
    }
    n->drag_x0 = e->x;
  }
  return n->cls == &bk_cls_slider ? slider_drag_value(n, e->x, e->mods) :
                                    number_drag_value(n, e->x, e->mods);
}

static void drag_origin(BKNode *n, BKEvent *e)
{
  /* but_dragedit_update_mval: the motion within the drag threshold is ignored */
  const double span = n->vmax - n->vmin;
  const int th = BK_MAX(3, (int)(0.15f * bk_unit())); /* the core's drag threshold */
  n->drag_x0 = (e->x - e->dx) + BK_CLAMP(e->dx, -th, th); /* the press, past the threshold */
  n->drag_f0 = span != 0 ? (float)((n->value - n->vmin) / span) : 0.0f;
  n->drag_start = n->value;
  n->drag_mods = e->mods;
}

static int number_press(BKNode *n, BKEvent *e)
{
  if (n->edit) {
    bk_te_click(n->edit, (float)(e->lx - bk_pad()), EV_SHIFT(e));
    bk_invalidate(n);
    return 1;
  }
  if (EV_CTRL(e)) { /* Blender: Ctrl+click types a value */
    number_begin_text(n);
    n->dragmode = -1;
    return 1;
  }
  n->drag_start = n->drag_orig = n->value;
  n->dragmode = 0;
  for (int i = 0; i < n->nlinked; i++) {
    n->linked_start[i] = n->linked[i]->value;
  }
  return 1;
}

static void number_cancel(BKNode *n)
{
  BKContext *ctx = bk_ctx(n);
  if (n->dragmode >= 2 && ctx) {
    for (int i = 0; i < ctx->nmulti; i++) {
      bk_set_value(ctx->multi[i], ctx->multi_start[i]);
      ctx->multi[i]->flags &= ~NF_SELECTED;
      bk_invalidate(ctx->multi[i]);
    }
    ctx->nmulti = 0;
  }
  else if (n->dragmode >= 0) {
    bk_set_value(n, n->drag_orig);
    for (int i = 0; i < n->nlinked; i++) {
      bk_set_value(n->linked[i], n->linked_start[i]);
    }
  }
  n->dragmode = -1; /* ignore the rest of this drag */
}

static bool is_number(const BKNode *n)
{
  return n->cls == &bk_cls_number || n->cls == &bk_cls_slider;
}

/* Blender's multi-value drag: press a number field, drag vertically over
 * other number fields to collect them, then drag horizontally to change
 * them all by the same amount. */
static int multi_drag(BKNode *n, BKEvent *e)
{
  BKContext *ctx = bk_ctx(n);
  if (!ctx) {
    return 0;
  }
  /* DRAG_MULTINUM_THRESHOLD_DRAG_X: UI_UNIT_Y / 4 */
  const int threshold = BK_MAX(3, (int)(0.25f * u()));
  if (n->dragmode == 2) { /* collecting */
    BKNode *under = bk_hit_at(ctx, n->x + n->w / 2, e->y);
    if (under && under != n && is_number(under) && NODE_ENABLED(under) &&
        !(under->flags & NF_SELECTED) && ctx->nmulti < BK_MAX_MULTI)
    {
      under->flags |= NF_SELECTED;
      ctx->multi_start[ctx->nmulti] = under->value;
      ctx->multi[ctx->nmulti++] = under;
      bk_invalidate(under);
    }
    if (abs(e->x - ctx->multi_x0) > threshold && ctx->nmulti > 1) {
      n->dragmode = 3; /* now edit them all */
      ctx->multi_x0 = e->x;
    }
    return 1;
  }
  /* editing: each field moves by its own amount for the same motion */
  for (int i = 0; i < ctx->nmulti; i++) {
    BKNode *m = ctx->multi[i];
    m->drag_start = ctx->multi_start[i];
    m->drag_x0 = ctx->multi_x0;
    const double span = m->vmax - m->vmin;
    m->drag_f0 = span != 0 ? (float)((ctx->multi_start[i] - m->vmin) / span) : 0.0f;
    m->drag_mods = e->mods;
    bk_set_value(m, m->cls == &bk_cls_slider ? slider_drag_value(m, e->x, e->mods) :
                                               number_drag_value(m, e->x, e->mods));
  }
  ctx->has_ungrab = false;
  return 1;
}

static int number_dbl(BKNode *n, BKEvent *e)
{
  if (n->edit) {
    bk_te_select_word(n->edit, bk_te_index_at(n->edit, (float)(e->lx - bk_pad())));
    bk_invalidate(n);
    return 1;
  }
  return number_press(n, e);
}

static int number_drag(BKNode *n, BKEvent *e)
{
  if (n->edit) {
    bk_te_drag(n->edit, (float)(e->lx - bk_pad()));
    bk_invalidate(n);
    return 1;
  }
  if (n->dragmode < 0) {
    return 1; /* cancelled */
  }
  if (n->dragmode == 0) { /* the gesture decides: vertical = multi-drag */
    /* ui_numedit_but_NUM multi-drag init: decided once the drag is longer than
     * DRAG_MULTINUM_THRESHOLD_DRAG_Y (UI_UNIT_Y / 4), vertical when
     * |dot(dir, y)| > DRAG_MULTINUM_THRESHOLD_VERTICAL (0.75) */
    const float len2 = (float)e->dx * e->dx + (float)e->dy * e->dy;
    const float margin = 0.25f * u();
    if (len2 <= margin * margin) {
      return 1;
    }
    drag_origin(n, e);
    BKContext *ctx = bk_ctx(n);
    if (ctx && (float)e->dy * e->dy > 0.75f * 0.75f * len2) {
      n->dragmode = 2;
      n->flags |= NF_SELECTED;
      ctx->nmulti = 1;
      ctx->multi[0] = n;
      ctx->multi_start[0] = n->drag_start;
      ctx->multi_x0 = e->x;
      bk_invalidate(n);
      return 1;
    }
    n->dragmode = 1;
  }
  if (n->dragmode >= 2) {
    return multi_drag(n, e);
  }
  bk_set_value(n, scrub_value(n, e));
  if (EV_ALT(e)) { /* Blender: Alt edits all linked fields by the same amount */
    const double delta = n->value - n->drag_orig;
    for (int i = 0; i < n->nlinked; i++) {
      bk_set_value(n->linked[i], n->linked_start[i] + delta);
    }
  }
  return 1;
}

static int arrow_w(const BKNode *n)
{
  return BK_MIN(n->w / 3, (int)(n->h * 0.7f)); /* handle_width */
}

static int number_release(BKNode *n, BKEvent *e)
{
  BKContext *ctx = bk_ctx(n);
  if (ctx && n->dragmode >= 2) {
    for (int i = 0; i < ctx->nmulti; i++) {
      ctx->multi[i]->flags &= ~NF_SELECTED;
      bk_invalidate(ctx->multi[i]);
    }
    ctx->nmulti = 0;
  }
  const int mode = n->dragmode;
  n->dragmode = 0;
  if (mode < 0 || n->edit) {
    return 1;
  }
  if (e->dragged && (mode >= 2 || n->value != n->drag_orig)) {
    return 1;
  }
  if (ctx) {
    ctx->has_ungrab = false; /* a click: the pointer goes back where it was */
  }
  if (!e->dragged && !e->inside) {
    return 1;
  }
  if (n->cls == &bk_cls_number) {
    const int aw = arrow_w(n);
    if (e->lx < aw) {
      number_step(n, -1);
      return 1;
    }
    if (e->lx >= n->w - aw) {
      number_step(n, 1);
      return 1;
    }
  }
  number_begin_text(n);
  return 1;
}

static int number_key(BKNode *n, BKEvent *e)
{
  if (n->edit) {
    return bk_edit_key(n, e);
  }
  return bk_value_key(n, e);
}

static int number_move(BKNode *n, BKEvent *e)
{
  if (n->cls != &bk_cls_number) {
    return 0;
  }
  const int aw = arrow_w(n);
  const int zone = e->lx < aw ? 1 : (e->lx >= n->w - aw ? 2 : 0);
  if (zone != n->hot) {
    n->hot = zone;
    bk_invalidate(n);
    return 1;
  }
  return 0;
}

static int number_leave(BKNode *n)
{
  n->hot = -1;
  return 1;
}

static void number_measure(BKNode *n, int *rw, int *rh)
{
  char val[128];
  number_text(n, val, sizeof(val));
  float tw = bk_text_width(val, -1, false);
  if (n->label && n->label[0]) {
    tw += bk_text_width(n->label, -1, false) + bk_text_width(": ", 2, false);
  }
  *rw = BK_MAX((int)tw + 2 * u(), 5 * u());
  *rh = u();
}

static void number_texts(BKNode *n, const BKRect *r, const uchar color[4])
{
  char val[128];
  number_text(n, val, sizeof(val));
  BKRect tr = *r; /* narrowed by the widget (arrow zones), plus the text margin */
  if (n->label && n->label[0]) {
    const float vw = bk_text_width(val, -1, false);
    BKRect lr = tr;
    lr.xmax = (int)(tr.xmax - vw - bk_pad());
    bk_text_in(&lr, n->label, color, BK_ALIGN_LEFT, false);
    bk_text_in(&tr, val, color, BK_ALIGN_RIGHT, false);
  }
  else {
    bk_text_in(&tr, val, color, BK_ALIGN_CENTER, false);
  }
}

static void draw_editing(BKNode *n)
{
  BKRect r = bk_node_rect(n);
  BKWCol w;
  bk_widget_state(&w, &bk_theme.textfield, ST_SELECT | ST_TEXT_INPUT);
  bk_widget_textbut(&w, &r, ST_SELECT, n->corners);
  bk_te_draw(n->edit, &r, w.text);
}

static void number_draw(BKNode *n)
{
  if (n->edit) {
    draw_editing(n);
    return;
  }
  BKRect r = bk_node_rect(n);
  int st = state_of(n);
  if (st & ST_HOVER) {
    st |= n->hot == 1 ? ST_HOVER_LEFT : (n->hot == 2 ? ST_HOVER_RIGHT : 0);
  }
  BKWCol w;
  bk_widget_state(&w, &bk_theme.num, st);
  bk_widget_numbut(&w, &r, st, n->corners);
  number_texts(n, &r, w.text);
}

#define NUMBER_CLASS_COMMON \
  .flags = CLS_INTERACTIVE | CLS_VALUE, .cursor = BK_CURSOR_EW, \
  .drag_cursor = BK_CURSOR_NONE, .measure = number_measure, .press = number_press, \
  .dbl = number_dbl, .drag = number_drag, .release = number_release, .key = number_key, \
  .move = number_move, .leave = number_leave, .cursor_at = number_cursor_at, \
  .cancel = number_cancel, \
  .finish_edit = number_finish, .to_text = number_to_text, .from_text = number_from_text, \
  .step = number_step, .reset = number_reset, .negate = number_negate, \
  .activate = number_begin_text

static void number_init(BKNode *n)
{
  n->vmin = -1e30;
  n->vmax = 1e30;
}

const BKClass bk_cls_number = {
    .name = "number", NUMBER_CLASS_COMMON, .init = number_init, .draw = number_draw};

static void slider_init(BKNode *n)
{
  n->vmin = 0.0;
  n->vmax = 1.0;
  n->flags |= NF_HAS_MIN | NF_HAS_MAX;
}

static void slider_draw(BKNode *n)
{
  if (n->edit) {
    draw_editing(n);
    return;
  }
  BKRect r = bk_node_rect(n);
  const int st = state_of(n);
  BKWCol w;
  bk_widget_state(&w, &bk_theme.numslider, st);
  const double span = (n->vmax - n->vmin) != 0 ? (n->vmax - n->vmin) : 1.0;
  bk_widget_numslider(&w, &r, st, n->corners, (float)((n->value - n->vmin) / span));
  number_texts(n, &r, w.text);
}

const BKClass bk_cls_slider = {
    .name = "slider", NUMBER_CLASS_COMMON, .init = slider_init, .draw = slider_draw};

/** \} */

/* -------------------------------------------------------------------- */
/** \name Text fields: TextField, SearchField, HotkeyField
 * \{ */

static int text_inset(const BKNode *n)
{
  return n->icon ? bk_icon_size() + (int)(0.3f * u()) : 0;
}

static int text_to_text(BKNode *n, char *buf, int size)
{
  snprintf(buf, (size_t)size, "%s", n->text ? n->text : "");
  return 1;
}

static int text_from_text(BKNode *n, const char *s)
{
  if (!bk_streq(n->text, s)) {
    bk_undo_record(n, 1);
    bk_set_text(&n->text, s);
    bk_changed(n);
  }
  return 1;
}

static void text_reset(BKNode *n, bool zero)
{
  text_from_text(n, "");
}

static void text_activate(BKNode *n)
{
  bk_begin_edit(n, n->text ? n->text : "", true);
}

static void text_finish(BKNode *n, bool confirm)
{
  if (confirm && n->edit) {
    text_from_text(n, n->edit->text);
  }
}

static float text_x(BKNode *n, BKEvent *e)
{
  return (float)(e->lx - bk_pad() - text_inset(n));
}

static int text_press(BKNode *n, BKEvent *e)
{
  if (!n->edit) {
    bk_begin_edit(n, n->text ? n->text : "", false);
  }
  bk_te_click(n->edit, text_x(n, e), EV_SHIFT(e));
  bk_invalidate(n);
  return 1;
}

static int text_dbl(BKNode *n, BKEvent *e)
{
  if (!n->edit) {
    return text_press(n, e);
  }
  bk_te_select_word(n->edit, bk_te_index_at(n->edit, text_x(n, e)));
  bk_invalidate(n);
  return 1;
}

static int text_drag(BKNode *n, BKEvent *e)
{
  if (n->edit) {
    bk_te_drag(n->edit, text_x(n, e));
    bk_invalidate(n);
  }
  return 1;
}

static int text_key(BKNode *n, BKEvent *e)
{
  if (!n->edit) {
    return bk_value_key(n, e);
  }
  BKTextEdit *te = n->edit;
  char *before = bk_strdup(te->text);
  BKContext *ctx = bk_ctx(n);
  const int res = bk_te_key(te, ctx, e->text, e->keysym, e->mods);
  if ((n->flags & NF_LIVE) && res == TE_CHANGED && strcmp(before, te->text) != 0) {
    text_from_text(n, te->text);
  }
  free(before);
  if (res == TE_NEXT || res == TE_PREV) {
    if (ctx) {
      ctx->focus = NULL;
      text_finish(n, true);
      bk_te_free(n->edit);
      n->edit = NULL;
      bk_edit_next(n, res == TE_NEXT ? 1 : -1);
    }
    bk_invalidate(n);
    return 1;
  }
  if (res == TE_CONFIRM && (n->flags & NF_EVENTS)) {
    bk_emit(n, BK_EV_SUBMIT, NULL, 0, 0);
  }
  if (res == TE_CONFIRM || res == TE_CANCEL) {
    /* end the edit and release focus */
    if (ctx) {
      ctx->focus = NULL;
      if (res == TE_CONFIRM) {
        text_finish(n, true);
      }
      else if (n->flags & NF_LIVE) {
        /* live fields already applied the text */
      }
      bk_te_free(n->edit);
      n->edit = NULL;
    }
  }
  bk_invalidate(n);
  return 1;
}

static int text_cursor_at(BKNode *n, int x, int y)
{
  return BK_CURSOR_TEXT;
}

static void text_measure(BKNode *n, int *rw, int *rh)
{
  *rw = 7 * u();
  *rh = u();
}

static void text_draw(BKNode *n)
{
  BKRect r = bk_node_rect(n);
  const int inset = text_inset(n);
  const bool editing = n->edit != NULL;
  int st = state_of(n);
  if (editing) {
    st = ST_SELECT | ST_TEXT_INPUT;
  }
  BKWCol w;
  bk_widget_state(&w, &bk_theme.textfield, st);
  bk_widget_textbut(&w, &r, st, n->corners);
  if (n->icon) {
    uchar ic[4];
    memcpy(ic, w.text, 4);
    ic[3] = (uchar)(ic[3] * 0.7f);
    bk_icon(n->icon, r.xmin + 0.2f * u() + bk_icon_size() * 0.5f, r.ymin + bk_rh(&r) * 0.5f, ic,
            1.0f, 0);
  }
  BKRect tr = r;
  tr.xmin += inset;
  if (editing) {
    bk_te_draw(n->edit, &tr, w.text);
    return;
  }
  char buf[512];
  const char *shown = n->text ? n->text : "";
  uchar tc[4];
  memcpy(tc, w.text, 4);
  if (!shown[0]) {
    shown = n->placeholder ? n->placeholder : "";
    tc[3] = (uchar)(tc[3] * 0.45f);
  }
  char masked[256];
  if (n->mask[0] && n->text && n->text[0]) {
    bk_mask_text(n->text, (int)strlen(n->text), n->mask, masked, sizeof(masked));
    shown = masked;
  }
  if (n->label && n->label[0]) {
    snprintf(buf, sizeof(buf), "%s: %s", n->label, shown);
    shown = buf;
  }
  bk_text_in(&tr, shown, tc, BK_ALIGN_LEFT, false);
}

#define TEXT_CLASS_COMMON \
  .flags = CLS_INTERACTIVE | CLS_VALUE, .cursor = BK_CURSOR_TEXT, .measure = text_measure, \
  .draw = text_draw, .press = text_press, .dbl = text_dbl, .drag = text_drag, \
  .key = text_key, .cursor_at = text_cursor_at, .finish_edit = text_finish, \
  .to_text = text_to_text, .from_text = text_from_text, .reset = text_reset, \
  .activate = text_activate

const BKClass bk_cls_text = {.name = "text", TEXT_CLASS_COMMON};

static void search_init(BKNode *n)
{
  n->icon = bk_icons_lookup("SEARCH");
  n->flags |= NF_LIVE;
  bk_set_text(&n->placeholder, "Search");
}

const BKClass bk_cls_search = {.name = "search", TEXT_CLASS_COMMON, .init = search_init};

/* Blender-style key text, e.g. "Ctrl Shift A". */
static void key_label(BKEvent *e, char *buf, int size)
{
  static const struct {
    const char *ks, *name;
  } names[] = {{"space", "Spacebar"},  {"Return", "Return"}, {"Escape", "Esc"},
               {"BackSpace", "Backspace"}, {"Delete", "Delete"}, {"Tab", "Tab"},
               {"Prior", "Page Up"},  {"Next", "Page Down"}};
  buf[0] = 0;
  if (EV_CTRL(e)) {
    strcat(buf, "Ctrl ");
  }
  if (EV_ALT(e)) {
    strcat(buf, "Alt ");
  }
  if (EV_SHIFT(e)) {
    strcat(buf, "Shift ");
  }
  for (size_t i = 0; i < sizeof(names) / sizeof(names[0]); i++) {
    if (bk_keysym_is(e->keysym, names[i].ks)) {
      strncat(buf, names[i].name, (size_t)(size - (int)strlen(buf) - 1));
      return;
    }
  }
  char k[32];
  snprintf(k, sizeof(k), "%s", e->keysym);
  if (strlen(k) == 1) {
    k[0] = (char)toupper((uchar)k[0]);
  }
  for (char *c = k; *c; c++) {
    if (*c == '_') {
      *c = ' ';
    }
  }
  strncat(buf, k, (size_t)(size - (int)strlen(buf) - 1));
}

static bool is_modifier(const char *ks)
{
  static const char *mods[] = {"Shift_L",  "Shift_R", "Control_L", "Control_R", "Alt_L",
                               "Alt_R",    "Meta_L",  "Meta_R",    "Super_L",   "Super_R",
                               "Caps_Lock", "Win_L",  "Win_R"};
  for (size_t i = 0; i < sizeof(mods) / sizeof(mods[0]); i++) {
    if (bk_keysym_is(ks, mods[i])) {
      return true;
    }
  }
  return false;
}

static void hotkey_activate(BKNode *n)
{
  BKContext *ctx = bk_ctx(n);
  n->dragmode = 99; /* capturing */
  if (ctx) {
    if (ctx->focus && ctx->focus != n && ctx->focus->cls->finish_edit) {
      ctx->focus->cls->finish_edit(ctx->focus, true);
    }
    ctx->focus = n;
  }
  bk_invalidate(n);
}

static int hotkey_press(BKNode *n, BKEvent *e)
{
  if (n->dragmode != 99) {
    hotkey_activate(n);
  }
  return 1;
}

static void hotkey_finish(BKNode *n, bool confirm)
{
  n->dragmode = 0;
  bk_invalidate(n);
}

static int hotkey_key(BKNode *n, BKEvent *e)
{
  if (n->dragmode != 99) {
    return bk_value_key(n, e);
  }
  BKContext *ctx = bk_ctx(n);
  if (bk_keysym_is(e->keysym, "Escape")) {
    if (ctx) {
      ctx->focus = NULL;
    }
    hotkey_finish(n, false);
  }
  else if (!is_modifier(e->keysym)) {
    char buf[96];
    key_label(e, buf, sizeof(buf));
    if (ctx) {
      ctx->focus = NULL;
    }
    hotkey_finish(n, true);
    text_from_text(n, buf);
  }
  return 1;
}

static void hotkey_measure(BKNode *n, int *rw, int *rh)
{
  *rw = 6 * u();
  *rh = u();
}

static void hotkey_draw(BKNode *n)
{
  BKRect r = bk_node_rect(n);
  const bool cap = n->dragmode == 99;
  int st = state_of(n) | (cap ? ST_SELECT : 0);
  BKWCol w;
  bk_widget_state(&w, &bk_theme.regular, st);
  bk_widget_but(&w, &r, st, n->corners);
  char buf[256];
  const char *v = cap ? "Press a key" : ((n->text && n->text[0]) ? n->text : "None");
  if (n->label && n->label[0]) {
    snprintf(buf, sizeof(buf), "%s: %s", n->label, v);
    v = buf;
  }
  bk_text_in(&r, v, w.text, BK_ALIGN_CENTER, false);
}

const BKClass bk_cls_hotkey = {.name = "hotkey",
                               .flags = CLS_INTERACTIVE | CLS_VALUE,
                               .measure = hotkey_measure,
                               .draw = hotkey_draw,
                               .press = hotkey_press,
                               .key = hotkey_key,
                               .finish_edit = hotkey_finish,
                               .to_text = text_to_text,
                               .from_text = text_from_text,
                               .reset = text_reset,
                               .activate = hotkey_activate};

/** \} */

/* -------------------------------------------------------------------- */
/** \name Choices: Dropdown (enum menu) and EnumRow (expanded enum)
 * \{ */

static int choice_to_text(BKNode *n, char *buf, int size)
{
  snprintf(buf, (size_t)size, "%s", item_text(n, (int)n->value));
  return 1;
}

static int choice_from_text(BKNode *n, const char *s)
{
  for (int i = 0; i < n->items.n; i++) {
    if (strcmp(n->items.v[i], s) == 0) {
      bk_set_value(n, i);
      return 1;
    }
  }
  return 0;
}

static void choice_step(BKNode *n, int dir)
{
  if (n->items.n) {
    bk_set_value(n, BK_CLAMP((int)n->value - dir, 0, n->items.n - 1)); /* wheel up = previous */
  }
}

static void choice_reset(BKNode *n, bool zero)
{
  bk_set_value(n, n->defval);
}

static void choice_init(BKNode *n)
{
  n->flags |= NF_HAS_MIN | NF_INTEGER;
  n->vmin = 0;
}

static void dropdown_pick(void *data)
{
  /* data encodes node + index */
}

static void dropdown_activate(BKNode *n)
{
  BKContext *ctx = bk_ctx(n);
  if (!ctx || !n->items.n) {
    return;
  }
  BKMenuSpec *items = (BKMenuSpec *)calloc((size_t)n->items.n, sizeof(BKMenuSpec));
  for (int i = 0; i < n->items.n; i++) {
    items[i].text = n->items.v[i];
    items[i].icon = item_icon(n, i);
    items[i].checked = -1;
    items[i].enabled = 1;
    items[i].fn = NULL;
    items[i].data = (void *)(intptr_t)i; /* picked index (handled by the menu) */
  }
  BKNode *menu = bk_menu_new(items, n->items.n, n->label && n->label[0] ? n->label : NULL,
                             item_text(n, (int)n->value));
  free(items);
  bk_open_popup(ctx, menu, n, NULL);
  (void)dropdown_pick;
}

static int dropdown_press(BKNode *n, BKEvent *e)
{
  BKContext *ctx = bk_ctx(n);
  if (bk_owns_popup(ctx, n)) {
    BK_close_popups(ctx);
  }
  else {
    dropdown_activate(n);
  }
  return 1;
}

static void dropdown_measure(BKNode *n, int *rw, int *rh)
{
  if (n->flags & NF_ICON_ONLY) {
    *rw = (int)(1.6f * u());
    *rh = u();
    return;
  }
  float widest = 0;
  for (int i = 0; i < n->items.n; i++) {
    widest = BK_MAX(widest, bk_content_width(n->items.v[i], item_icon(n, i), false));
  }
  const float lw = (n->label && n->label[0]) ? bk_text_width(n->label, -1, false) +
                                                   bk_text_width(": ", 2, false) :
                                               0;
  *rw = (int)(widest + lw + 2.5f * u());
  *rh = u();
}

static void dropdown_draw(BKNode *n)
{
  BKRect r = bk_node_rect(n);
  int st = state_of(n);
  if (bk_owns_popup(bk_ctx(n), n)) {
    st |= ST_SELECT;
  }
  BKWCol w;
  bk_widget_state(&w, &bk_theme.menu, st);
  bk_widget_menubut(&w, &r, st, n->corners);
  const int i = (int)n->value;
  if (n->flags & NF_ICON_ONLY) {
    bk_icon(item_icon(n, i), n->x + 0.6f * u(), n->y + n->h * 0.5f, NULL,
            NODE_ENABLED(n) ? 1.0f : 0.5f, 0);
    return;
  }
  char buf[256];
  const char *s = item_text(n, i);
  if (n->label && n->label[0]) {
    snprintf(buf, sizeof(buf), "%s: %s", n->label, s);
    s = buf;
  }
  bk_text_icon(&r, s, item_icon(n, i), w.text, BK_ALIGN_LEFT, NODE_ENABLED(n));
}

const BKClass bk_cls_dropdown = {.name = "dropdown",
                                 .flags = CLS_INTERACTIVE | CLS_VALUE,
                                 .init = choice_init,
                                 .measure = dropdown_measure,
                                 .draw = dropdown_draw,
                                 .press = dropdown_press,
                                 .to_text = choice_to_text,
                                 .from_text = choice_from_text,
                                 .step = choice_step,
                                 .reset = choice_reset,
                                 .activate = dropdown_activate};

static int enum_label_w(const BKNode *n)
{
  return (n->label && n->label[0] && !(n->flags & NF_VERTICAL)) ?
             (int)bk_text_width(n->label, -1, false) + 2 * bk_pad() :
             0;
}

static int enum_label_h(const BKNode *n)
{
  return (n->label && n->label[0] && (n->flags & NF_VERTICAL)) ? u() : 0;
}

static int enum_seg_w(const BKNode *n, int i)
{
  if (n->flags & NF_ICON_ONLY) {
    return u();
  }
  return (int)bk_content_width(n->items.v[i], item_icon(n, i), false) + 2 * bk_pad();
}

static void enum_measure(BKNode *n, int *rw, int *rh)
{
  if (n->flags & NF_VERTICAL) {
    int widest = n->label ? (int)bk_text_width(n->label, -1, false) : 0;
    for (int i = 0; i < n->items.n; i++) {
      widest = BK_MAX(widest, enum_seg_w(n, i));
    }
    *rw = widest;
    *rh = enum_label_h(n) + n->items.n * u();
    return;
  }
  int w = enum_label_w(n);
  for (int i = 0; i < n->items.n; i++) {
    w += enum_seg_w(n, i);
  }
  *rw = w;
  *rh = u();
}

static BKRect enum_segment(const BKNode *n, int i)
{
  if (n->flags & NF_VERTICAL) {
    return bk_rect(n->x, n->y + enum_label_h(n) + i * u(), n->w, u());
  }
  const int x0 = n->x + enum_label_w(n);
  const float sw = (n->x + n->w - x0) / (float)BK_MAX(1, n->items.n);
  const int a = bk_iround(x0 + i * sw), b = bk_iround(x0 + (i + 1) * sw);
  return bk_rect(a, n->y, b - a, n->h);
}

static int enum_seg_at(const BKNode *n, int x, int y)
{
  for (int i = 0; i < n->items.n; i++) {
    BKRect s = enum_segment(n, i);
    if (x >= s.xmin && x < s.xmax && y >= s.ymin && y < s.ymax) {
      return i;
    }
  }
  return -1;
}

static int enum_move(BKNode *n, BKEvent *e)
{
  const int hot = enum_seg_at(n, e->x, e->y);
  if (hot != n->hot) {
    n->hot = hot;
    bk_invalidate(n);
    return 1;
  }
  return 0;
}

static int enum_press(BKNode *n, BKEvent *e)
{
  const int i = enum_seg_at(n, e->x, e->y);
  if (i >= 0) {
    bk_set_value(n, i);
  }
  return 1;
}

static void enum_activate(BKNode *n)
{
  choice_step(n, -1);
}

static void enum_draw(BKNode *n)
{
  if (n->label && n->label[0]) {
    BKRect lr = (n->flags & NF_VERTICAL) ? bk_rect(n->x, n->y, n->w, u()) :
                                           bk_rect(n->x, n->y, enum_label_w(n), n->h);
    uchar c[4];
    memcpy(c, bk_theme.text, 4);
    if (!NODE_ENABLED(n)) {
      c[3] /= 2;
    }
    bk_text_in(&lr, n->label, c, BK_ALIGN_LEFT, false);
  }
  const bool vertical = n->flags & NF_VERTICAL;
  const int first_c = vertical ? (CNR_TL | CNR_TR) : (n->corners & (CNR_TL | CNR_BL));
  const int last_c = vertical ? (CNR_BL | CNR_BR) : (n->corners & (CNR_TR | CNR_BR));
  const int last = n->items.n - 1;
  for (int i = 0; i < n->items.n; i++) {
    BKRect s = enum_segment(n, i);
    const int corners = (i == 0 ? first_c : 0) | (i == last ? last_c : 0);
    int st = NODE_ENABLED(n) ? 0 : ST_DISABLED;
    if ((int)n->value == i) {
      st |= ST_SELECT;
    }
    else if (bk_hovered(n) && i == n->hot) {
      st |= ST_HOVER;
    }
    if (vertical && i != last) {
      st |= ST_ALIGN_DOWN;
    }
    BKWCol w;
    bk_widget_state(&w, &bk_theme.radio, st);
    bk_widget_but(&w, &s, st, corners); /* widget_radiobut */
    bk_text_icon(&s, (n->flags & NF_ICON_ONLY) ? "" : n->items.v[i], item_icon(n, i), w.text,
                 vertical ? BK_ALIGN_LEFT : BK_ALIGN_CENTER, NODE_ENABLED(n));
  }
}

const BKClass bk_cls_enum = {.name = "enum",
                             .flags = CLS_INTERACTIVE | CLS_VALUE,
                             .init = choice_init,
                             .measure = enum_measure,
                             .draw = enum_draw,
                             .press = enum_press,
                             .move = enum_move,
                             .leave = number_leave,
                             .to_text = choice_to_text,
                             .from_text = choice_from_text,
                             .step = choice_step,
                             .reset = choice_reset,
                             .activate = enum_activate};

/** \} */

/* -------------------------------------------------------------------- */
/** \name ColorSwatch
 * \{ */

static int swatch_to_text(BKNode *n, char *buf, int size)
{
  snprintf(buf, (size_t)size, "#%02x%02x%02x%02x", bk_iround(n->color[0] * 255),
           bk_iround(n->color[1] * 255), bk_iround(n->color[2] * 255),
           bk_iround(n->color[3] * 255));
  return 1;
}

int bk_parse_hex(const char *s, float out[4], const float alpha)
{
  while (*s == ' ' || *s == '#') {
    s++;
  }
  const size_t len = strlen(s);
  if (len != 6 && len != 8) {
    return 0;
  }
  unsigned v[4] = {0, 0, 0, (unsigned)bk_iround(alpha * 255)};
  for (size_t i = 0; i < len / 2; i++) {
    char h[3] = {s[2 * i], s[2 * i + 1], 0};
    char *end;
    v[i] = (unsigned)strtoul(h, &end, 16);
    if (*end) {
      return 0;
    }
  }
  for (int k = 0; k < 4; k++) {
    out[k] = v[k] / 255.0f;
  }
  return 1;
}

static int swatch_from_text(BKNode *n, const char *s)
{
  float c[4];
  if (!bk_parse_hex(s, c, n->color[3])) {
    return 0;
  }
  BK_set_color(n, "set_value", c);
  return 1;
}

static void swatch_reset(BKNode *n, bool zero)
{
  const float black[4] = {0, 0, 0, 1};
  BK_set_color(n, "set_value", black);
}

static void swatch_activate(BKNode *n)
{
  BKContext *ctx = bk_ctx(n);
  if (ctx) {
    bk_open_popup(ctx, bk_picker_new(n), n, NULL);
  }
}

static int swatch_press(BKNode *n, BKEvent *e)
{
  BKContext *ctx = bk_ctx(n);
  if (bk_owns_popup(ctx, n)) {
    BK_close_popups(ctx);
  }
  else {
    swatch_activate(n);
  }
  return 1;
}

static void swatch_init(BKNode *n)
{
  n->color[0] = n->color[1] = n->color[2] = n->color[3] = 1.0f;
  n->flags |= NF_ALPHA;
}

static void swatch_measure(BKNode *n, int *rw, int *rh)
{
  const int lw = (n->label && n->label[0]) ? (int)bk_text_width(n->label, -1, false) + 2 * bk_pad() :
                                             0;
  *rw = lw + 3 * u();
  *rh = u();
}

static void swatch_draw(BKNode *n)
{
  int lw = 0;
  if (n->label && n->label[0]) {
    lw = (int)bk_text_width(n->label, -1, false) + 2 * bk_pad();
    BKRect lr = bk_rect(n->x, n->y, lw, n->h);
    bk_text_in(&lr, n->label, bk_theme.text, BK_ALIGN_LEFT, false);
  }
  BKRect r = bk_rect(n->x + lw, n->y, n->w - lw, n->h);
  const int st = state_of(n);
  BKWCol w;
  bk_widget_state(&w, &bk_theme.regular, st);
  /* Blender: opaque color on the left half, color with alpha on the right */
  const float opaque[4] = {n->color[0], n->color[1], n->color[2], 1.0f};
  bk_widget_swatch(&w, &r, st, n->corners, opaque);
  if (n->color[3] < 1.0f) {
    bk_clip_push(r.xmin + bk_rw(&r) / 2, r.ymin - 2, bk_rw(&r), bk_rh(&r) + 4);
    bk_widget_swatch(&w, &r, st, n->corners, n->color);
    bk_clip_pop();
  }
}

const BKClass bk_cls_swatch = {.name = "swatch",
                               .flags = CLS_INTERACTIVE | CLS_VALUE,
                               .init = swatch_init,
                               .measure = swatch_measure,
                               .draw = swatch_draw,
                               .press = swatch_press,
                               .to_text = swatch_to_text,
                               .from_text = swatch_from_text,
                               .reset = swatch_reset,
                               .activate = swatch_activate};

/** \} */

/* -------------------------------------------------------------------- */
/** \name Popover button, MenuButton, MenuBar
 * \{ */

static void popover_measure(BKNode *n, int *rw, int *rh)
{
  *rw = BK_MAX((int)bk_content_width(n->text, n->icon, false) + 2 * u(), 2 * u());
  *rh = u();
}

BKNode *bk_popover_panel_new(BKNode *owner, BKNode *body);

static int popover_press(BKNode *n, BKEvent *e)
{
  BKContext *ctx = bk_ctx(n);
  if (!ctx) {
    return 0;
  }
  if (bk_owns_popup(ctx, n)) {
    BK_close_popups(ctx);
  }
  else if (n->nkids) {
    bk_open_popup(ctx, bk_popover_panel_new(n, n->kids[0]), n, NULL);
  }
  return 1;
}

static BKNode *popover_hit(BKNode *n, int x, int y)
{
  return n; /* the body lives in the popup while open */
}

static void popover_draw_tree(BKNode *n)
{
  BKRect r = bk_node_rect(n);
  if (!NODE_VISIBLE(n) || !bk_clip_visible(&r)) {
    return;
  }
  int st = state_of(n);
  if (bk_owns_popup(bk_ctx(n), n)) {
    st |= ST_SELECT;
  }
  BKWCol w;
  bk_widget_state(&w, &bk_theme.menu, st);
  bk_widget_menubut(&w, &r, st, n->corners);
  bk_text_icon(&r, n->text, n->icon, w.text, BK_ALIGN_LEFT, NODE_ENABLED(n));
}

static void popover_init(BKNode *n)
{
  BKNode *body = bk_node_new_cls(&bk_cls_column);
  body->spacing = bk_iround(0.2f * u());
  BK_node_add(n, body, -1);
}

const BKClass bk_cls_popover = {.name = "popover",
                                .flags = CLS_INTERACTIVE,
                                .init = popover_init,
                                .measure = popover_measure,
                                .draw_tree = popover_draw_tree,
                                .hit = popover_hit,
                                .press = popover_press};

typedef struct MenuButtonData {
  BKMenuSpec *items;
  int n;
} MenuButtonData;

void bk_menuspec_free(BKMenuSpec *items, int n);
BKMenuSpec *bk_menuspec_copy(const BKMenuSpec *items, int n);

void BK_menubutton_set_items(BKNode *n, const BKMenuSpec *items, int count)
{
  MenuButtonData *d = (MenuButtonData *)n->ext;
  if (!d) {
    d = (MenuButtonData *)calloc(1, sizeof(MenuButtonData));
    n->ext = d;
  }
  bk_menuspec_free(d->items, d->n);
  d->items = bk_menuspec_copy(items, count);
  d->n = count;
}

static void menubutton_free(BKNode *n)
{
  MenuButtonData *d = (MenuButtonData *)n->ext;
  if (d) {
    bk_menuspec_free(d->items, d->n);
    free(d);
  }
}

static void menubutton_open(BKNode *n)
{
  BKContext *ctx = bk_ctx(n);
  MenuButtonData *d = (MenuButtonData *)n->ext;
  if (ctx && d) {
    bk_open_popup(ctx, bk_menu_new(d->items, d->n, NULL, NULL), n, NULL);
  }
}

static int menubutton_press(BKNode *n, BKEvent *e)
{
  BKContext *ctx = bk_ctx(n);
  if (bk_owns_popup(ctx, n)) {
    BK_close_popups(ctx);
  }
  else {
    menubutton_open(n);
  }
  return 1;
}

static int menubutton_move(BKNode *n, BKEvent *e)
{
  /* Blender: while a menu of this bar is open, hovering switches menus. */
  BKContext *ctx = bk_ctx(n);
  if (!ctx || !ctx->npopups || bk_owns_popup(ctx, n)) {
    return 0;
  }
  BKNode *owner = ctx->popups[0]->parent;
  if (owner && owner->cls == &bk_cls_menubutton && owner->parent == n->parent) {
    menubutton_open(n);
    return 1;
  }
  return 0;
}

static int menubutton_key(BKNode *n, BKEvent *e)
{
  if (bk_keysym_is(e->keysym, "Return") || bk_keysym_is(e->keysym, "space")) {
    menubutton_open(n);
    return 1;
  }
  return 0;
}

static void menubutton_measure(BKNode *n, int *rw, int *rh)
{
  *rw = (int)(bk_content_width(n->text, n->icon, false) + 2 * bk_pad() + 0.4f * u());
  *rh = u();
}

static void menubutton_draw(BKNode *n)
{
  BKRect r = bk_node_rect(n);
  const bool open = bk_owns_popup(bk_ctx(n), n);
  int st = (open || bk_hovered(n)) ? ST_HOVER : 0;
  BKWCol w = bk_theme.pulldown;
  bk_widget_pulldownbut(&w, &r, st, CNR_ALL);
  bk_text_icon(&r, n->text, n->icon, open || (st & ST_HOVER) ? w.text_sel : w.text,
               BK_ALIGN_CENTER, NODE_ENABLED(n));
}

const BKClass bk_cls_menubutton = {.name = "menubutton",
                                   .flags = CLS_INTERACTIVE,
                                   .free_ext = menubutton_free,
                                   .measure = menubutton_measure,
                                   .draw = menubutton_draw,
                                   .press = menubutton_press,
                                   .move = menubutton_move,
                                   .key = menubutton_key};

static void menubar_measure(BKNode *n, int *rw, int *rh)
{
  const int p = bk_iround(0.2f * u());
  int w = 0;
  for (int i = 0; i < n->nkids; i++) {
    int cw, ch;
    bk_natural(n->kids[i], &cw, &ch);
    w += cw;
  }
  *rw = w + 2 * p;
  *rh = u() + 2 * p;
}

static void menubar_arrange(BKNode *n)
{
  /* menu buttons on the left, other components (weight 0) on the right */
  const int p = bk_iround(0.2f * u());
  int xl = n->x + p, xr = n->x + n->w - p;
  for (int i = 0; i < n->nkids; i++) {
    BKNode *c = n->kids[i];
    int cw, ch;
    bk_natural(c, &cw, &ch);
    if (c->cls == &bk_cls_menubutton) {
      bk_arrange(c, xl, n->y + p, cw, u());
      xl += cw;
    }
  }
  for (int i = n->nkids - 1; i >= 0; i--) {
    BKNode *c = n->kids[i];
    if (c->cls == &bk_cls_menubutton) {
      continue;
    }
    int cw, ch;
    bk_natural(c, &cw, &ch);
    xr -= cw;
    bk_arrange(c, xr, n->y + p, cw, u());
    xr -= bk_iround(0.2f * u());
  }
}

static void menubar_draw(BKNode *n)
{
  bk_fill(n->x, n->y, n->w, n->h, bk_theme.header_bg);
}

const BKClass bk_cls_menubar = {.name = "menubar",
                                .measure = menubar_measure,
                                .arrange = menubar_arrange,
                                .draw = menubar_draw};

/** \} */
