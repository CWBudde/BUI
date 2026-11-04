/* Kit core: nodes, properties, layout cache, input context, damage-driven
 * rendering, text editing and number evaluation.
 *
 * The interaction rules follow Blender's interface_handlers.cc: drag
 * threshold, double-click, drag-toggle, continuous grab while scrubbing
 * (pointer hidden and warped at the window edge, restored on release),
 * value shortcuts while hovering, tooltips after a delay.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <ctype.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#ifdef _WIN32
#  define WIN32_LEAN_AND_MEAN
#  include <windows.h>
#endif

#include "kit_intern.h"

/* Blender's UTF-8 cursor stepping (blenlib, linked from bui_core). */
typedef enum { STRCUR_JUMP_NONE, STRCUR_JUMP_DELIM, STRCUR_JUMP_ALL } eStrCursorJumpType;
typedef enum { STRCUR_DIR_PREV, STRCUR_DIR_NEXT } eStrCursorJumpDirection;
void BLI_str_cursor_step_utf8(const char *str, int str_maxlen, int *pos,
                              eStrCursorJumpDirection direction, eStrCursorJumpType jump,
                              bool use_init_step);
void BLI_str_cursor_step_bounds_utf8(const char *str, int str_maxlen, int pos, int *r_start,
                                     int *r_end);

/* -------------------------------------------------------------------- */
/** \name Utilities
 * \{ */

static void (*g_retain)(void *) = NULL;
static void (*g_release)(void *) = NULL;

void BK_set_ref_hooks(void (*retain)(void *ref), void (*release)(void *ref))
{
  g_retain = retain;
  g_release = release;
}
void bk_retain(void *ref)
{
  if (ref && g_retain) {
    g_retain(ref);
  }
}
void bk_release(void *ref)
{
  if (ref && g_release) {
    g_release(ref);
  }
}

char *bk_strdup(const char *s)
{
  if (!s) {
    return NULL;
  }
  const size_t n = strlen(s) + 1;
  char *d = (char *)malloc(n);
  if (d) {
    memcpy(d, s, n);
  }
  return d;
}

void bk_set_text(char **dst, const char *s)
{
  char *d = bk_strdup(s ? s : "");
  free(*dst);
  *dst = d;
}

void bk_strs_free(BKStrs *s)
{
  for (int i = 0; i < s->n; i++) {
    free(s->v[i]);
  }
  free(s->v);
  s->v = NULL;
  s->n = 0;
}

void bk_strs_set(BKStrs *s, const char *const *v, int n)
{
  bk_strs_free(s);
  if (n <= 0) {
    return;
  }
  s->v = (char **)calloc((size_t)n, sizeof(char *));
  for (int i = 0; i < n; i++) {
    s->v[i] = bk_strdup(v[i] ? v[i] : "");
  }
  s->n = n;
}

bool bk_streq(const char *a, const char *b)
{
  return a && b && strcmp(a, b) == 0;
}

bool bk_keysym_is(const char *keysym, const char *name)
{
  if (!keysym) {
    return false;
  }
  for (; *keysym && *name; keysym++, name++) {
    if (tolower((uchar)*keysym) != tolower((uchar)*name)) {
      return false;
    }
  }
  return *keysym == 0 && *name == 0;
}

double bk_time(void)
{
#ifdef _WIN32
  static LARGE_INTEGER freq;
  LARGE_INTEGER now;
  if (!freq.QuadPart) {
    QueryPerformanceFrequency(&freq);
  }
  QueryPerformanceCounter(&now);
  return (double)now.QuadPart / (double)freq.QuadPart;
#else
  struct timespec ts;
  clock_gettime(CLOCK_MONOTONIC, &ts);
  return ts.tv_sec + ts.tv_nsec * 1e-9;
#endif
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Nodes
 * \{ */

static int g_gen = 1; /* layout generation for the natural-size cache */

static const BKClass *const g_classes[] = {
    &bk_cls_column,   &bk_cls_row,       &bk_cls_split,       &bk_cls_grid,       &bk_cls_box,
    &bk_cls_panel,    &bk_cls_tabs,      &bk_cls_sidetabs,   &bk_cls_sidebar, &bk_cls_scroll, &bk_cls_scrollbar,
    &bk_cls_label,    &bk_cls_separator, &bk_cls_progress,   &bk_cls_button,
    &bk_cls_toggle,   &bk_cls_checkbox,  &bk_cls_decorator,  &bk_cls_text,
    &bk_cls_search,   &bk_cls_hotkey,    &bk_cls_number,     &bk_cls_slider,
    &bk_cls_dropdown, &bk_cls_enum,      &bk_cls_swatch,     &bk_cls_popover,
    &bk_cls_menubutton, &bk_cls_menubar, &bk_cls_list,       &bk_cls_tree,
    &bk_cls_gridview, &bk_cls_ramp,      &bk_cls_curve,      &bk_cls_normal,
    &bk_cls_histogram, &bk_cls_frame,     &bk_cls_radio,      &bk_cls_canvas,
    &bk_cls_textarea,  &bk_cls_image,
};

const BKClass *bk_class_find(const char *name)
{
  for (size_t i = 0; i < sizeof(g_classes) / sizeof(g_classes[0]); i++) {
    if (strcmp(g_classes[i]->name, name) == 0) {
      return g_classes[i];
    }
  }
  return NULL;
}

BKNode *bk_node_new_cls(const BKClass *cls)
{
  BKNode *n = (BKNode *)calloc(1, sizeof(BKNode));
  n->cls = cls;
  n->flags = NF_VISIBLE | NF_ENABLED | NF_EMBOSS;
  n->weight = 1.0f;
  n->corners = CNR_ALL;
  n->precision = 3;
  n->step = 0.1;
  n->vmin = 0.0;
  n->vmax = 1.0;
  n->hot = -1;
  n->rename_row = -1;
  n->spacing = -1;
  n->rows = 5;
  n->split = 0.4f;
  n->color[3] = 1.0f;
  n->nat_gen = 0;
  if (cls->init) {
    cls->init(n);
  }
  return n;
}

BKNode *BK_node_new(const char *type)
{
  const BKClass *cls = bk_class_find(type);
  return cls ? bk_node_new_cls(cls) : NULL;
}

const char *BK_node_type(const BKNode *n)
{
  return n->cls->name;
}

static void node_detach(BKNode *child)
{
  BKNode *p = child->parent;
  if (!p) {
    return;
  }
  BKContext *ctx = bk_ctx(child);
  if (ctx) {
    bk_forget(ctx, child);
    bk_relayout(p);
  }
  for (int i = 0; i < p->nkids; i++) {
    if (p->kids[i] == child) {
      memmove(p->kids + i, p->kids + i + 1, sizeof(BKNode *) * (size_t)(p->nkids - i - 1));
      p->nkids--;
      break;
    }
  }
  child->parent = NULL;
}

void BK_node_free(BKNode *n)
{
  if (!n) {
    return;
  }
  if (n->parent) {
    node_detach(n);
  }
  if (n->ctx_root) {
    n->ctx_root->root = NULL;
  }
  for (int i = 0; i < n->nkids; i++) {
    BKNode *c = n->kids[i];
    c->parent = NULL;
    if (c->ref) {
      bk_release(c->ref); /* host owns it */
    }
    else {
      BK_node_free(c); /* C-owned (built by a composite) */
    }
  }
  free(n->kids);
  if (n->cls->free_ext) {
    n->cls->free_ext(n);
  }
  free(n->text);
  free(n->label);
  free(n->desc);
  free(n->placeholder);
  free(n->suffix);
  free(n->category);
  bk_strs_free(&n->items);
  free(n->marks);
  bk_strs_free(&n->item_icons_s);
  bk_strs_free(&n->shortcuts);
  free(n->item_icons);
  free(n->linked);
  free(n->linked_start);
  bk_te_free(n->edit);
  free(n->geom);
  bk_image_free((BKImage *)n->image);
  free(n);
}

void BK_node_set_ref(BKNode *n, void *ref)
{
  n->ref = ref;
}
void *BK_node_ref(const BKNode *n)
{
  return n->ref;
}

void bk_node_add(BKNode *parent, BKNode *child)
{
  BK_node_add(parent, child, -1);
}

int BK_node_add(BKNode *parent, BKNode *child, int index)
{
  if (!parent || !child || child == parent) {
    return 0;
  }
  if (child->parent) {
    void *ref = child->ref;
    bk_retain(ref); /* keep alive across the move */
    BK_node_remove(child->parent, child);
    BK_node_add(parent, child, index);
    bk_release(ref);
    return 1;
  }
  if (parent->nkids == parent->capkids) {
    parent->capkids = parent->capkids ? parent->capkids * 2 : 4;
    parent->kids = (BKNode **)realloc(parent->kids, sizeof(BKNode *) * (size_t)parent->capkids);
  }
  if (index < 0 || index > parent->nkids) {
    index = parent->nkids;
  }
  memmove(parent->kids + index + 1, parent->kids + index,
          sizeof(BKNode *) * (size_t)(parent->nkids - index));
  parent->kids[index] = child;
  parent->nkids++;
  child->parent = parent;
  bk_retain(child->ref);
  bk_relayout(parent);
  return 1;
}

void BK_node_remove(BKNode *parent, BKNode *child)
{
  if (!child || child->parent != parent) {
    return;
  }
  node_detach(child);
  bk_release(child->ref);
}

int BK_node_child_count(const BKNode *n)
{
  return n->nkids;
}
BKNode *BK_node_child(const BKNode *n, int i)
{
  return (i >= 0 && i < n->nkids) ? n->kids[i] : NULL;
}
BKNode *BK_node_parent(const BKNode *n)
{
  return n->parent;
}
void BK_node_rect(const BKNode *n, int r[4])
{
  r[0] = n->x;
  r[1] = n->y;
  r[2] = n->w;
  r[3] = n->h;
}
void BK_node_natural(BKNode *n, int r[2])
{
  bk_natural(n, &r[0], &r[1]);
}
void BK_node_invalidate(BKNode *n)
{
  bk_invalidate(n);
}
void BK_node_relayout(BKNode *n)
{
  bk_relayout(n);
}

BKContext *bk_ctx(const BKNode *n)
{
  while (n) {
    if (n->ctx_root) {
      return n->ctx_root;
    }
    n = n->parent;
  }
  return NULL;
}

BKRect bk_node_rect(const BKNode *n)
{
  return bk_rect(n->x, n->y, n->w, n->h);
}

void bk_invalidate(BKNode *n)
{
  BKContext *ctx = bk_ctx(n);
  if (!ctx) {
    return;
  }
  int m = 3;
  for (const BKNode *p = n; p; p = p->parent) {
    if (p->cls->flags & CLS_POPUP) {
      m = (int)(bk_theme.menu_shadow_width * bk_scale_fac) + 6;
      break;
    }
  }
  bk_ctx_dirty_rect(ctx, bk_rect(n->x - m, n->y - m, n->w + 2 * m, n->h + 2 * m));
}

void bk_relayout(BKNode *n)
{
  g_gen++;
  BKContext *ctx = bk_ctx(n);
  if (ctx) {
    ctx->layout_dirty = true;
    bk_ctx_request(ctx);
  }
}

void bk_natural(BKNode *n, int *r_w, int *r_h)
{
  if (n->nat_gen != g_gen) {
    int w = 4 * bk_unit(), h = bk_unit();
    if (n->cls->measure) {
      const float saved = bk_text_scale;
      bk_text_scale = n->font_scale > 0.0f ? n->font_scale : 1.0f;
      n->cls->measure(n, &w, &h);
      if (bk_text_scale > 1.0f) { /* taller text needs a taller widget */
        h = BK_MAX(h, (int)(h * bk_text_scale + 0.5f));
      }
      bk_text_scale = saved;
    }
    if (n->geom) { /* explicit requested size (Tk width=/height=, Qt fixed size) */
      if (n->geom->reqw > 0) {
        w = n->geom->reqw;
      }
      if (n->geom->reqh > 0) {
        h = n->geom->reqh;
      }
    }
    n->nat_w = w;
    n->nat_h = h;
    n->nat_gen = g_gen;
  }
  *r_w = n->nat_w;
  *r_h = n->nat_h;
}

int bk_configure_pending;

void bk_arrange(BKNode *n, int x, int y, int w, int h)
{
  if ((n->flags & NF_EVENTS) && (n->x != x || n->y != y || n->w != w || n->h != h)) {
    n->flags |= NF_CONFIGURED; /* reported after layout (Tk's <Configure>) */
    bk_configure_pending = 1;
  }
  n->x = x;
  n->y = y;
  n->w = w;
  n->h = h;
  if (n->cls->arrange) {
    n->cls->arrange(n);
  }
}

void bk_draw_tree(BKNode *n)
{
  if (!NODE_VISIBLE(n)) {
    return;
  }
  if (n->cls->draw_tree) {
    n->cls->draw_tree(n);
    return;
  }
  /* Skip whole subtrees outside the redrawn region (children stay inside
   * their parent's rect; emboss/outlines spill by a pixel or two). */
  BKRect r = bk_rect(n->x - 3, n->y - 3, n->w + 6, n->h + 6);
  if (!bk_clip_visible(&r)) {
    return;
  }
  if (n->cls->draw) {
    const float saved = bk_text_scale;
    bk_text_scale = n->font_scale > 0.0f ? n->font_scale : 1.0f;
    n->cls->draw(n);
    bk_text_scale = saved;
  }
  for (int i = 0; i < n->nkids; i++) {
    bk_draw_tree(n->kids[i]);
  }
}

bool bk_contains(const BKNode *n, int x, int y)
{
  return x >= n->x && x < n->x + n->w && y >= n->y && y < n->y + n->h;
}

BKNode *bk_hit(BKNode *n, int x, int y)
{
  if (!NODE_VISIBLE(n) || !bk_contains(n, x, y)) {
    return NULL;
  }
  if (n->cls->hit) {
    return n->cls->hit(n, x, y);
  }
  for (int i = n->nkids - 1; i >= 0; i--) {
    BKNode *f = bk_hit(n->kids[i], x, y);
    if (f) {
      return f;
    }
  }
  return ((n->cls->flags & CLS_INTERACTIVE) || (n->flags & NF_EVENTS)) ? n : NULL;
}

bool bk_hovered(const BKNode *n)
{
  const BKContext *ctx = bk_ctx(n);
  return ctx && ctx->hover == n && NODE_ENABLED(n);
}

bool bk_pressed(const BKNode *n)
{
  const BKContext *ctx = bk_ctx(n);
  return ctx && ctx->grab == n && ctx->hover == n;
}

void bk_emit(BKNode *n, int kind, void *ref, double a, double b)
{
  BKContext *ctx = bk_ctx(n);
  if (ctx && ctx->host.emit) {
    ctx->host.emit(ctx->host.data, n, kind, ref, a, b);
  }
}

void bk_changed(BKNode *n)
{
  bk_invalidate(n);
  if (n->on_change) {
    n->on_change(n, n->on_change_data);
  }
  bk_emit(n, BK_EV_CHANGE, NULL, n->value, 0);
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Values & shortcuts
 * \{ */

double bk_clamp_value(const BKNode *n, double v)
{
  if (n->flags & NF_HAS_MIN && v < n->vmin) {
    v = n->vmin;
  }
  if (n->flags & NF_HAS_MAX && v > n->vmax) {
    v = n->vmax;
  }
  if (n->flags & NF_INTEGER) {
    v = floor(v + 0.5);
  }
  return v;
}

void bk_set_value_silent(BKNode *n, double v)
{
  v = bk_clamp_value(n, v);
  if (v != n->value) {
    n->value = v;
    bk_invalidate(n);
  }
}

void bk_set_value(BKNode *n, double v)
{
  v = bk_clamp_value(n, v);
  if (v != n->value) {
    bk_undo_record(n, 0);
    n->value = v;
    bk_changed(n);
  }
}

char *bk_clipboard_get(BKContext *ctx);
void bk_clipboard_set(BKContext *ctx, const char *s);

static void vc_copy(void *data)
{
  BKNode *n = (BKNode *)data;
  char buf[256];
  if (n->cls->to_text && n->cls->to_text(n, buf, sizeof(buf))) {
    bk_clipboard_set(bk_ctx(n), buf);
  }
}

static void vc_paste(void *data)
{
  BKNode *n = (BKNode *)data;
  char *s = bk_clipboard_get(bk_ctx(n));
  if (s) {
    /* trim */
    char *b = s;
    while (*b && isspace((uchar)*b)) {
      b++;
    }
    size_t l = strlen(b);
    while (l && isspace((uchar)b[l - 1])) {
      b[--l] = 0;
    }
    if (n->cls->from_text) {
      n->cls->from_text(n, b);
    }
    free(s);
  }
}

static void vc_reset(void *data)
{
  BKNode *n = (BKNode *)data;
  if (n->cls->reset) {
    n->cls->reset(n, false);
  }
}

int bk_value_context_items(BKNode *n, BKMenuSpec *items, int max)
{
  if (max < 4) {
    return 0;
  }
  memset(items, 0, sizeof(BKMenuSpec) * 4);
  items[0] = (BKMenuSpec){"Copy Value", bk_icons_lookup("COPYDOWN"), "Ctrl C", -1, 1};
  items[0].fn = vc_copy;
  items[0].data = n;
  items[1] = (BKMenuSpec){"Paste Value", bk_icons_lookup("PASTEDOWN"), "Ctrl V", -1, 1};
  items[1].fn = vc_paste;
  items[1].data = n;
  items[2] = (BKMenuSpec){"-", 0, NULL, -1, 1};
  items[3] = (BKMenuSpec){"Reset to Default Value", bk_icons_lookup("LOOP_BACK"), "Backspace",
                          -1, 1};
  items[3].fn = vc_reset;
  items[3].data = n;
  return 4;
}

int bk_value_key(BKNode *n, BKEvent *e)
{
  const BKClass *c = n->cls;
  if (EV_CTRL(e) && EV_ALT(e)) {
    return 0; /* Ctrl+Alt+C/V: whole vector, handled by the parent */
  }
  if (EV_CTRL(e) && bk_keysym_is(e->keysym, "c")) {
    vc_copy(n);
    return 1;
  }
  if (EV_CTRL(e) && bk_keysym_is(e->keysym, "v")) {
    vc_paste(n);
    return 1;
  }
  if (bk_keysym_is(e->keysym, "BackSpace")) {
    if (c->reset) {
      c->reset(n, true);
    }
    return 1;
  }
  if (bk_keysym_is(e->keysym, "minus") || bk_keysym_is(e->keysym, "KP_Subtract")) {
    if (c->negate) {
      c->negate(n);
      return 1;
    }
    return 0;
  }
  if (bk_keysym_is(e->keysym, "Return") || bk_keysym_is(e->keysym, "KP_Enter")) {
    if (c->activate) {
      c->activate(n);
    }
    return 1;
  }
  return 0;
}

int bk_value_wheel(BKNode *n, BKEvent *e)
{
  if (EV_CTRL(e) && n->cls->step) {
    n->cls->step(n, e->delta > 0 ? 1 : -1);
    return 1;
  }
  return 0;
}

void bk_begin_edit(BKNode *n, const char *text, bool select_all)
{
  BKContext *ctx = bk_ctx(n);
  if (ctx && ctx->focus && ctx->focus != n) {
    BKNode *f = ctx->focus;
    ctx->focus = NULL;
    if (f->cls->finish_edit) {
      f->cls->finish_edit(f, true);
    }
  }
  bk_te_free(n->edit);
  n->edit = bk_te_new(text, select_all);
  memcpy(n->edit->mask, n->mask, sizeof(n->mask));
  if (ctx) {
    ctx->focus = n;
    ctx->cursor = -1; /* re-evaluate (text cursor) */
  }
  bk_invalidate(n);
}

static void end_edit(BKContext *ctx, bool confirm)
{
  BKNode *f = ctx->focus;
  ctx->focus = NULL;
  if (f) {
    if (f->cls->finish_edit) {
      f->cls->finish_edit(f, confirm);
    }
    bk_te_free(f->edit);
    f->edit = NULL;
    bk_invalidate(f);
  }
}

int bk_edit_key(BKNode *n, BKEvent *e)
{
  BKContext *ctx = bk_ctx(n);
  const int res = bk_te_key(n->edit, ctx, e->text, e->keysym, e->mods);
  if ((res == TE_CONFIRM || res == TE_CANCEL) && ctx) {
    end_edit(ctx, res == TE_CONFIRM);
  }
  else if ((res == TE_NEXT || res == TE_PREV) && ctx) {
    end_edit(ctx, true);
    bk_edit_next(n, res == TE_NEXT ? 1 : -1);
  }
  bk_invalidate(n);
  return 1;
}

void BK_link_fields(BKNode *const *fields, int n)
{
  for (int i = 0; i < n; i++) {
    BKNode *f = fields[i];
    free(f->linked);
    free(f->linked_start);
    f->linked = (BKNode **)calloc((size_t)BK_MAX(1, n - 1), sizeof(BKNode *));
    f->linked_start = (double *)calloc((size_t)BK_MAX(1, n - 1), sizeof(double));
    f->nlinked = 0;
    for (int j = 0; j < n; j++) {
      if (j != i) {
        f->linked[f->nlinked++] = fields[j];
      }
    }
  }
  if (n > 0 && fields[0]->parent) {
    fields[0]->parent->flags |= NF_VECTOR;
  }
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Properties
 * \{ */

static int str_align(const char *s)
{
  if (bk_streq(s, "right")) {
    return BK_ALIGN_RIGHT;
  }
  if (bk_streq(s, "center")) {
    return BK_ALIGN_CENTER;
  }
  return BK_ALIGN_LEFT;
}

static const struct {
  const char *key;
  unsigned flag;
  bool relayout;
} g_flag_props[] = {
    {"visible", NF_VISIBLE, true},     {"enabled", NF_ENABLED, false},
    {"align", NF_ALIGN, true},         {"emboss", NF_EMBOSS, false},
    {"vertical", NF_VERTICAL, true},   {"icon_only", NF_ICON_ONLY, true},
    {"collapsed", NF_COLLAPSED, true}, {"integer", NF_INTEGER, false},
    {"live", NF_LIVE, false},          {"alpha", NF_ALPHA, false},
    {"square", NF_SQUARE, false},      {"directory", NF_DIRECTORY, false},
    {"line", NF_LINE, false},          {"bold", NF_BOLD, true},
    {"events", NF_EVENTS, false},      {"nopropagate", NF_NOPROPAGATE, true},
    {"viewevents", NF_VIEWEVENTS, false}, {"noscrollbar", NF_NOSCROLLBAR, true},
    {"opaque", NF_OPAQUE, false},
};

static int g_view_watchers; /* > 0 once any node asked for view events */

/* Scrolled views (Tk's xview/yview): the class reports first/visible/total
 * in its own units; fractions, moveto, scrolling by units and change
 * notification are shared here. */
static void view_of(BKNode *n, int axis, int *first, int *vis, int *total)
{
  *first = 0;
  *vis = *total = 1;
  if (n->cls->view) {
    n->cls->view(n, axis, first, vis, total);
  }
  *total = BK_MAX(1, *total);
  *vis = BK_CLAMP(*vis, 1, *total);
}

void bk_view_fractions(BKNode *n, int axis, float *r_first, float *r_last)
{
  int first, vis, total;
  view_of(n, axis, &first, &vis, &total);
  *r_first = BK_CLAMP((float)first / total, 0.0f, 1.0f);
  *r_last = BK_CLAMP((float)(first + vis) / total, 0.0f, 1.0f);
}

static void view_set_first(BKNode *n, int axis, int first)
{
  int cur, vis, total;
  view_of(n, axis, &cur, &vis, &total);
  first = BK_CLAMP(first, 0, total - vis);
  if (n->cls->set_view && first != cur) {
    n->cls->set_view(n, axis, first);
    bk_invalidate(n);
  }
}

static int view_set_num(BKNode *n, const char *key, double v)
{
  const int axis = key[0] == 'x';
  int first, vis, total;
  if (strcmp(key + 1, "view") == 0) { /* moveto fraction */
    view_of(n, axis, &first, &vis, &total);
    view_set_first(n, axis, (int)floor(v * total + 0.5));
    return 1;
  }
  if (strcmp(key + 1, "scroll") == 0) { /* by units */
    view_of(n, axis, &first, &vis, &total);
    view_set_first(n, axis, first + (int)v);
    return 1;
  }
  return 0;
}

static int view_get_num(BKNode *n, const char *key, double *r_v)
{
  const int axis = key[0] == 'x';
  if (strcmp(key + 1, "first") == 0 || strcmp(key + 1, "last") == 0) {
    float a, b;
    bk_view_fractions(n, axis, &a, &b);
    *r_v = key[1] == 'f' ? a : b;
    return 1;
  }
  if (strcmp(key + 1, "total") == 0 || strcmp(key + 1, "visible") == 0) {
    int first, vis, total;
    view_of(n, axis, &first, &vis, &total);
    *r_v = key[1] == 't' ? total : vis;
    return 1;
  }
  return 0;
}

typedef struct Notify {
  BKNode *node;
  int kind;
} Notify;

static void notify_collect(BKNode *n, Notify *out, int *count, int max)
{
  if (*count >= max) {
    return;
  }
  if (n->flags & NF_CONFIGURED) {
    n->flags &= ~NF_CONFIGURED;
    out[(*count)++] = (Notify){n, BK_EV_CONFIGURE};
  }
  if ((n->flags & NF_VIEWEVENTS) && *count < max) {
    float f[4];
    bk_view_fractions(n, 0, &f[0], &f[1]);
    bk_view_fractions(n, 1, &f[2], &f[3]);
    if (memcmp(f, n->view_seen, sizeof(f)) != 0) {
      memcpy(n->view_seen, f, sizeof(f));
      out[(*count)++] = (Notify){n, BK_EV_VIEW};
    }
  }
  for (int i = 0; i < n->nkids; i++) {
    notify_collect(n->kids[i], out, count, max);
  }
}

/* Tell the host what layout and input changed: a widget's geometry (Tk's
 * <Configure>) and scrolled views (Tk's x/yscrollcommand). One shared pass
 * after input and after layout. */
void bk_views_check(BKContext *ctx)
{
  if ((!g_view_watchers && !bk_configure_pending) || !ctx->root) {
    return;
  }
  bk_configure_pending = 0;
  Notify changed[128];
  int count = 0;
  notify_collect(ctx->root, changed, &count, 128);
  for (int i = 0; i < count; i++) { /* after the walk: callbacks may edit the tree */
    BKNode *n = changed[i].node;
    bk_emit(n, changed[i].kind, NULL, changed[i].kind == BK_EV_CONFIGURE ? n->w : 0,
            changed[i].kind == BK_EV_CONFIGURE ? n->h : 0);
  }
}

int BK_set_num(BKNode *n, const char *key, double v)
{
  if (n->cls->set_num && n->cls->set_num(n, key, v)) {
    return 1;
  }
  if ((key[0] == 'x' || key[0] == 'y') && view_set_num(n, key, v)) {
    return 1;
  }
  if (strcmp(key, "viewevents") == 0 && v != 0) {
    g_view_watchers++;
  }
  for (size_t i = 0; i < sizeof(g_flag_props) / sizeof(g_flag_props[0]); i++) {
    if (strcmp(key, g_flag_props[i].key) == 0) {
      if (v != 0) {
        n->flags |= g_flag_props[i].flag;
      }
      else {
        n->flags &= ~g_flag_props[i].flag;
      }
      if (g_flag_props[i].relayout) {
        bk_relayout(n);
      }
      bk_invalidate(n);
      if (strcmp(key, "enabled") == 0 || strcmp(key, "visible") == 0) {
        BKContext *ctx = bk_ctx(n);
        if (ctx && !(n->flags & g_flag_props[i].flag)) {
          bk_forget(ctx, n);
        }
      }
      return 1;
    }
  }
  if (strcmp(key, "value") == 0) {
    if (n->cls == &bk_cls_tabs || n->cls == &bk_cls_sidetabs || n->cls == &bk_cls_sidebar) {
      n->current = (int)v;
      bk_relayout(n);
    }
    else {
      bk_set_value_silent(n, v);
    }
    return 1;
  }
  if (strcmp(key, "set_value") == 0) { /* notifying setter */
    bk_set_value(n, v);
    return 1;
  }
  if (strcmp(key, "default") == 0) {
    n->defval = v;
    return 1;
  }
  if (strcmp(key, "min") == 0) {
    n->vmin = v;
    n->flags |= NF_HAS_MIN;
    return 1;
  }
  if (strcmp(key, "max") == 0) {
    n->vmax = v;
    n->flags |= NF_HAS_MAX;
    return 1;
  }
  if (strcmp(key, "step") == 0) {
    n->step = v;
    return 1;
  }
  if (strcmp(key, "precision") == 0) {
    n->precision = (int)v;
    bk_invalidate(n);
    return 1;
  }
  if (strcmp(key, "weight") == 0) {
    n->weight = (float)v;
    bk_relayout(n);
    return 1;
  }
  if (strcmp(key, "fill") == 0) {
    n->fill = v != 0.0;
    bk_relayout(n);
    return 1;
  }
  if (strcmp(key, "factor") == 0) {
    n->factor = v;
    bk_relayout(n);
    bk_invalidate(n);
    return 1;
  }
  if (strcmp(key, "spacing") == 0) {
    n->spacing = (int)v;
    bk_relayout(n);
    return 1;
  }
  if (strcmp(key, "padding") == 0) {
    n->padding = (int)v;
    bk_relayout(n);
    return 1;
  }
  if (strcmp(key, "columns") == 0) {
    n->columns = (int)v;
    bk_relayout(n);
    return 1;
  }
  if (strcmp(key, "rows") == 0) {
    n->rows = (int)v;
    bk_relayout(n);
    return 1;
  }
  if (strcmp(key, "tile") == 0) {
    n->tile = (float)v;
    bk_relayout(n);
    return 1;
  }
  if (strcmp(key, "height") == 0) {
    n->height_units = (float)v;
    bk_relayout(n);
    return 1;
  }
  if (strcmp(key, "split") == 0) {
    n->split = (float)v;
    bk_relayout(n);
    return 1;
  }
  if (strcmp(key, "scroll") == 0) {
    n->scroll = (int)v;
    bk_relayout(n);
    return 1;
  }
  if (strcmp(key, "scroll_x") == 0) {
    n->scroll_x = (int)v;
    bk_relayout(n);
    return 1;
  }
  if (strcmp(key, "selmode") == 0) {
    n->selmode = (int)v;
    bk_invalidate(n);
    return 1;
  }
  if (strcmp(key, "font_scale") == 0) {
    n->font_scale = (float)v;
    bk_relayout(n);
    return 1;
  }
  if (strcmp(key, "cursor") == 0) { /* < 0: back to the class cursor */
    n->cursor_override = v < 0 ? 0 : (int)v + 1;
    return 1;
  }
  if (strcmp(key, "current") == 0) {
    n->current = (int)v;
    bk_relayout(n);
    return 1;
  }
  if (strcmp(key, "icon") == 0) {
    n->icon = (int)v;
    bk_relayout(n);
    return 1;
  }
  return 0;
}

double BK_get_num(BKNode *n, const char *key)
{
  double cv;
  if (n->cls->get_num && n->cls->get_num(n, key, &cv)) {
    return cv;
  }
  for (size_t i = 0; i < sizeof(g_flag_props) / sizeof(g_flag_props[0]); i++) {
    if (strcmp(key, g_flag_props[i].key) == 0) {
      return (n->flags & g_flag_props[i].flag) ? 1.0 : 0.0;
    }
  }
  if (strcmp(key, "value") == 0) {
    if (n->cls == &bk_cls_tabs || n->cls == &bk_cls_sidetabs || n->cls == &bk_cls_sidebar) {
      return n->current;
    }
    return n->value;
  }
  if (strcmp(key, "default") == 0) {
    return n->defval;
  }
  if (strcmp(key, "min") == 0) {
    return n->vmin;
  }
  if (strcmp(key, "max") == 0) {
    return n->vmax;
  }
  if (strcmp(key, "step") == 0) {
    return n->step;
  }
  if (strcmp(key, "factor") == 0) {
    return n->factor;
  }
  if (strcmp(key, "weight") == 0) {
    return n->weight;
  }
  if (strcmp(key, "fill") == 0) {
    return n->fill;
  }
  if (strcmp(key, "scroll") == 0) {
    return n->scroll;
  }
  if (strcmp(key, "scroll_x") == 0) {
    return n->scroll_x;
  }
  if ((key[0] == 'x' || key[0] == 'y') && view_get_num(n, key, &cv)) {
    return cv;
  }
  if (strcmp(key, "current") == 0) {
    return n->current;
  }
  if (strcmp(key, "tile") == 0) {
    return n->tile;
  }
  if (strcmp(key, "height") == 0) {
    return n->height_units;
  }
  if (strcmp(key, "hot") == 0) {
    return n->hot;
  }
  if (strcmp(key, "editing") == 0) {
    return n->edit != NULL || n->dragmode == 99;
  }
  if (strcmp(key, "corners") == 0) {
    return n->corners;
  }
  if (strcmp(key, "rename_row") == 0) {
    return n->rename_row;
  }
  return 0.0;
}

/* A field being edited shows a programmatic text change (Tk's entry.delete /
 * insert, validatecommand rejecting a keystroke). */
static void edit_follow_text(BKNode *n)
{
  BKTextEdit *te = n->edit;
  const char *s = n->text ? n->text : "";
  if (!te || strcmp(te->text, s) == 0) {
    return;
  }
  const int caret = te->caret;
  BKTextEdit *fresh = bk_te_new(s, false);
  memcpy(fresh->mask, te->mask, sizeof(te->mask));
  fresh->caret = BK_MIN(caret, fresh->len);
  fresh->anchor = -1;
  bk_te_free(te);
  n->edit = fresh;
}

int BK_set_str(BKNode *n, const char *key, const char *v)
{
  if (strcmp(key, "text") == 0 || strcmp(key, "title") == 0) {
    bk_set_text(&n->text, v);
    bk_relayout(n);
    bk_invalidate(n);
    return 1;
  }
  if (strcmp(key, "value") == 0 && n->cls == &bk_cls_textarea) {
    return bk_textarea_set_text(n, v);
  }
  if (strcmp(key, "value") == 0) { /* string-valued nodes */
    bk_set_text(&n->text, v);
    edit_follow_text(n);
    bk_invalidate(n);
    return 1;
  }
  if (strcmp(key, "set_value") == 0) {
    if (!bk_streq(n->text, v)) {
      bk_undo_record(n, 1);
      bk_set_text(&n->text, v);
      edit_follow_text(n);
      bk_changed(n);
    }
    return 1;
  }
  if (strcmp(key, "label") == 0) {
    bk_set_text(&n->label, v);
    bk_relayout(n);
    bk_invalidate(n);
    return 1;
  }
  if (strcmp(key, "description") == 0) {
    bk_set_text(&n->desc, v);
    return 1;
  }
  if (strcmp(key, "placeholder") == 0) {
    bk_set_text(&n->placeholder, v);
    bk_invalidate(n);
    return 1;
  }
  if (strcmp(key, "mask") == 0) { /* one UTF-8 character, "" = plain */
    memset(n->mask, 0, sizeof(n->mask));
    strncpy(n->mask, v ? v : "", sizeof(n->mask) - 1);
    if (n->edit) {
      memcpy(n->edit->mask, n->mask, sizeof(n->mask));
    }
    bk_invalidate(n);
    return 1;
  }
  if (strcmp(key, "suffix") == 0) {
    bk_set_text(&n->suffix, v);
    bk_invalidate(n);
    return 1;
  }
  if (strcmp(key, "category") == 0) {
    bk_set_text(&n->category, v);
    bk_relayout(n);
    bk_invalidate(n);
    return 1;
  }
  if (strcmp(key, "icon") == 0) {
    n->icon = bk_icons_lookup(v);
    bk_relayout(n);
    bk_invalidate(n);
    return 1;
  }
  if (strcmp(key, "icon_on") == 0) {
    n->icon_on = bk_icons_lookup(v);
    bk_invalidate(n);
    return 1;
  }
  if (strcmp(key, "text_align") == 0) {
    n->align = str_align(v);
    bk_invalidate(n);
    return 1;
  }
  return 0;
}

const char *BK_get_str(BKNode *n, const char *key)
{
  if (strcmp(key, "value") == 0 && n->cls == &bk_cls_textarea) {
    return bk_textarea_text(n);
  }
  if (strcmp(key, "text") == 0 || strcmp(key, "value") == 0 || strcmp(key, "title") == 0) {
    return n->text ? n->text : "";
  }
  if (strcmp(key, "label") == 0) {
    return n->label ? n->label : "";
  }
  if (strcmp(key, "description") == 0) {
    return n->desc ? n->desc : "";
  }
  if (strcmp(key, "edit") == 0) {
    return n->edit ? n->edit->text : "";
  }
  if (strcmp(key, "type") == 0) {
    return n->cls->name;
  }
  return "";
}

int BK_set_color(BKNode *n, const char *key, const float c[4])
{
  if (strcmp(key, "color") == 0 || strcmp(key, "value") == 0) {
    for (int k = 0; k < 4; k++) { /* normals keep their sign */
      n->color[k] = n->cls == &bk_cls_normal ? c[k] : BK_CLAMP(c[k], 0.0f, 1.0f);
    }
    bk_invalidate(n);
    return 1;
  }
  if (strcmp(key, "bg") == 0) { /* frames / canvases */
    memcpy(n->color, c, sizeof(float[4]));
    n->flags = c[3] > 0 ? (n->flags | NF_HAS_BG) : (n->flags & ~NF_HAS_BG);
    bk_invalidate(n);
    return 1;
  }
  if (strcmp(key, "set_value") == 0) {
    float v[4];
    for (int k = 0; k < 4; k++) {
      v[k] = BK_CLAMP(c[k], 0.0f, 1.0f);
    }
    if (memcmp(v, n->color, sizeof(v)) != 0) {
      bk_undo_record(n, 2);
      memcpy(n->color, v, sizeof(v));
      bk_changed(n);
    }
    return 1;
  }
  if (strcmp(key, "text_color") == 0) {
    for (int k = 0; k < 4; k++) {
      n->color_text[k] = (uchar)(BK_CLAMP(c[k], 0.0f, 1.0f) * 255.0f + 0.5f);
    }
    n->has_color_text = true;
    bk_invalidate(n);
    return 1;
  }
  return 0;
}

int BK_get_color(BKNode *n, const char *key, float r[4])
{
  if (strcmp(key, "color") == 0 || strcmp(key, "value") == 0) {
    memcpy(r, n->color, sizeof(float[4]));
    return 1;
  }
  return 0;
}

int BK_set_strs(BKNode *n, const char *key, const char *const *v, int count)
{
  if (strcmp(key, "headers") == 0) { /* tree/table column headers */
    bk_strs_set(&n->shortcuts, v, count);
    bk_relayout(n);
    bk_invalidate(n);
    return 1;
  }
  if (strcmp(key, "items") == 0) {
    bk_strs_set(&n->items, v, count);
    if (n->value >= count) {
      n->value = count > 0 ? count - 1 : 0;
    }
    bk_relayout(n);
    bk_invalidate(n);
    return 1;
  }
  if (strcmp(key, "groups") == 0) { /* side tabs: "1" where a tab group starts */
    bk_strs_set(&n->shortcuts, v, count);
    bk_relayout(n);
    bk_invalidate(n);
    return 1;
  }
  if (strcmp(key, "icons") == 0) {
    bk_strs_set(&n->item_icons_s, v, count);
    free(n->item_icons);
    n->item_icons = (int *)calloc((size_t)BK_MAX(1, count), sizeof(int));
    for (int i = 0; i < count; i++) {
      n->item_icons[i] = bk_icons_lookup(v[i]);
    }
    if (n->cls == &bk_cls_tree) {
      bk_tree_columns_from_icons(n); /* (off, on) icon pairs per toggle column */
    }
    bk_relayout(n);
    bk_invalidate(n);
    return 1;
  }
  return 0;
}

int BK_strs_count(BKNode *n, const char *key)
{
  return strcmp(key, "items") == 0 ? n->items.n : 0;
}

const char *BK_strs_get(BKNode *n, const char *key, int i)
{
  if (strcmp(key, "items") == 0 && i >= 0 && i < n->items.n) {
    return n->items.v[i];
  }
  return "";
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Context: damage, popups, services
 * \{ */

BKContext *BK_context_new(BKNode *root)
{
  BKContext *ctx = (BKContext *)calloc(1, sizeof(BKContext));
  ctx->root = root;
  ctx->cursor = BK_CURSOR_DEFAULT;
  ctx->layout_dirty = true;
  ctx->full_dirty = true;
  if (root) {
    root->ctx_root = ctx;
  }
  return ctx;
}

void BK_context_free(BKContext *ctx)
{
  if (!ctx) {
    return;
  }
  BK_close_popups(ctx);
  for (int i = 0; i < ctx->nkeys; i++) {
    bk_release(ctx->keys[i].ref);
  }
  if (ctx->root) {
    ctx->root->ctx_root = NULL;
  }
  if (ctx->header) {
    ctx->header->ctx_root = NULL;
  }
  free(ctx);
}

void BK_context_set_host(BKContext *ctx, const BKHost *host)
{
  ctx->host = *host;
}

void BK_context_set_header(BKContext *ctx, BKNode *header)
{
  if (ctx->header) {
    ctx->header->ctx_root = NULL;
  }
  ctx->header = header;
  if (header) {
    header->ctx_root = ctx;
  }
  ctx->layout_dirty = true;
}

BKNode *BK_context_root(BKContext *ctx)
{
  return ctx->root;
}

void bk_ctx_request(BKContext *ctx)
{
  if (ctx->host.request) {
    ctx->host.request(ctx->host.data);
  }
}

static bool rect_overlap(const BKRect *a, const BKRect *b)
{
  return a->xmin < b->xmax && b->xmin < a->xmax && a->ymin < b->ymax && b->ymin < a->ymax;
}

static void rect_union(BKRect *a, const BKRect *b)
{
  a->xmin = BK_MIN(a->xmin, b->xmin);
  a->ymin = BK_MIN(a->ymin, b->ymin);
  a->xmax = BK_MAX(a->xmax, b->xmax);
  a->ymax = BK_MAX(a->ymax, b->ymax);
}

void bk_ctx_dirty_rect(BKContext *ctx, BKRect r)
{
  if (r.xmax <= r.xmin || r.ymax <= r.ymin) {
    return;
  }
  bk_ctx_request(ctx);
  if (ctx->full_dirty) {
    return;
  }
  for (int i = 0; i < ctx->ndirty; i++) {
    if (rect_overlap(&ctx->dirty[i], &r)) {
      rect_union(&ctx->dirty[i], &r);
      return;
    }
  }
  if (ctx->ndirty == BK_MAX_DIRTY) {
    for (int i = 1; i < ctx->ndirty; i++) {
      rect_union(&ctx->dirty[0], &ctx->dirty[i]);
    }
    rect_union(&ctx->dirty[0], &r);
    ctx->ndirty = 1;
    return;
  }
  ctx->dirty[ctx->ndirty++] = r;
}

static bool is_within(const BKNode *n, const BKNode *subtree)
{
  for (; n; n = n->parent) {
    if (n == subtree) {
      return true;
    }
  }
  return false;
}

static void undo_forget(BKContext *ctx, BKNode *subtree);

void bk_forget(BKContext *ctx, BKNode *subtree)
{
  undo_forget(ctx, subtree);
  for (int i = ctx->nanim - 1; i >= 0; i--) {
    if (is_within(ctx->anim[i], subtree)) {
      ctx->anim[i] = ctx->anim[--ctx->nanim];
    }
  }
  for (int i = 0; i < ctx->nmulti; i++) {
    if (is_within(ctx->multi[i], subtree)) {
      ctx->nmulti = 0;
      break;
    }
  }
  if (ctx->hover && is_within(ctx->hover, subtree)) {
    ctx->hover = NULL;
  }
  if (ctx->grab && is_within(ctx->grab, subtree)) {
    ctx->grab = NULL;
  }
  if (ctx->focus && is_within(ctx->focus, subtree)) {
    BKNode *f = ctx->focus;
    ctx->focus = NULL;
    bk_te_free(f->edit);
    f->edit = NULL;
  }
  if (ctx->modal && is_within(ctx->modal, subtree)) {
    ctx->modal = NULL;
  }
  if (ctx->tip_node && is_within(ctx->tip_node, subtree)) {
    ctx->tip_node = NULL;
    ctx->tip_shown = 0;
  }
  if (ctx->last_click_node && is_within(ctx->last_click_node, subtree)) {
    ctx->last_click_node = NULL;
  }
  /* popups owned by the subtree */
  for (int i = 0; i < ctx->npopups; i++) {
    if (ctx->popups[i]->parent && is_within(ctx->popups[i]->parent, subtree)) {
      bk_close_popups_after(ctx, i > 0 ? ctx->popups[i - 1] : NULL);
      break;
    }
  }
}

int bk_popup_index(BKContext *ctx, const BKNode *popup)
{
  for (int i = 0; i < ctx->npopups; i++) {
    if (ctx->popups[i] == popup) {
      return i;
    }
  }
  return -1;
}

bool bk_owns_popup(BKContext *ctx, const BKNode *owner)
{
  if (!ctx) {
    return false;
  }
  for (int i = 0; i < ctx->npopups; i++) {
    if (ctx->popups[i]->parent == owner) {
      return true;
    }
  }
  return false;
}

static void free_popup(BKNode *p)
{
  if (p->cls == &bk_cls_menu || p->cls == &bk_cls_pie) {
    bk_menu_on_close(p);
  }
  p->parent = NULL;
  BK_node_free(p);
}

void bk_close_popups_after(BKContext *ctx, BKNode *after)
{
  const int keep = after ? bk_popup_index(ctx, after) + 1 : 0;
  while (ctx->npopups > keep) {
    BKNode *p = ctx->popups[--ctx->npopups];
    bk_invalidate(p);
    /* clear references into the popup before freeing it */
    if (ctx->hover && is_within(ctx->hover, p)) {
      ctx->hover = NULL;
    }
    if (ctx->grab && is_within(ctx->grab, p)) {
      ctx->grab = NULL;
    }
    if (ctx->focus && is_within(ctx->focus, p)) {
      BKNode *f = ctx->focus;
      ctx->focus = NULL;
      bk_te_free(f->edit);
      f->edit = NULL;
    }
    if (ctx->tip_node && is_within(ctx->tip_node, p)) {
      ctx->tip_node = NULL;
      ctx->tip_shown = 0;
    }
    if (ctx->last_click_node && is_within(ctx->last_click_node, p)) {
      ctx->last_click_node = NULL;
    }
    if (ctx->modal && is_within(ctx->modal, p)) {
      ctx->modal = NULL;
    }
    /* The popover panel's body belongs to its owner, not the popup. */
    if (p->cls == &bk_cls_popover_panel && p->nkids) {
      BKNode *body = p->kids[0];
      p->nkids = 0;
      body->parent = p->parent;
    }
    BKNode *owner = p->parent;
    free_popup(p);
    if (owner) {
      bk_invalidate(owner);
    }
  }
  ctx->layout_dirty = ctx->layout_dirty; /* no relayout needed */
  bk_ctx_request(ctx);
}

void BK_close_popups(BKContext *ctx)
{
  bk_close_popups_after(ctx, NULL);
}

void bk_open_popup(BKContext *ctx, BKNode *popup, BKNode *owner, BKNode *parent_popup)
{
  if (!parent_popup) { /* opened from inside a popup: nest under it */
    for (const BKNode *nd = owner; nd && !parent_popup; nd = nd->parent) {
      if (bk_popup_index(ctx, nd) >= 0) {
        parent_popup = (BKNode *)nd;
      }
    }
  }
  if (parent_popup && bk_popup_index(ctx, parent_popup) >= 0) {
    bk_close_popups_after(ctx, parent_popup);
  }
  else {
    BK_close_popups(ctx);
  }
  if (ctx->npopups == BK_MAX_POPUPS) {
    BK_node_free(popup);
    return;
  }
  popup->parent = owner ? owner : ctx->root;
  ctx->popups[ctx->npopups++] = popup;
  ctx->tip_serial++;
  if (ctx->tip_shown) {
    ctx->tip_shown = 0;
    bk_ctx_dirty_rect(ctx, ctx->tip_rect);
  }
  if (ctx->win_w > 0) {
    bk_popup_place(popup, ctx->win_w, ctx->win_h);
  }
  bk_invalidate(popup);
  if (owner) {
    bk_invalidate(owner);
  }
}

int BK_popup_count(BKContext *ctx)
{
  return ctx->npopups;
}
BKNode *BK_popup(BKContext *ctx, int i)
{
  return (i >= 0 && i < ctx->npopups) ? ctx->popups[i] : NULL;
}
BKNode *BK_popup_owner(BKContext *ctx, int i)
{
  return (i >= 0 && i < ctx->npopups) ? ctx->popups[i]->parent : NULL;
}

void bk_start_drag_toggle(BKContext *ctx, int group, int value)
{
  ctx->dt_group = group;
  ctx->dt_value = value;
  ctx->dt_active = 1;
}

static void update_cursor(BKContext *ctx);

void bk_begin_modal(BKContext *ctx, BKNode *tool)
{
  if (!tool->parent) {
    tool->parent = ctx->root;
  }
  ctx->modal = tool;
  update_cursor(ctx);
  bk_ctx_request(ctx);
}

void bk_end_modal(BKContext *ctx)
{
  BKNode *m = ctx->modal;
  ctx->modal = NULL;
  if (m) {
    m->parent = NULL;
    BK_node_free(m);
  }
  update_cursor(ctx);
  bk_ctx_request(ctx);
}

static char *g_clipboard = NULL; /* fallback when the host has none */

char *bk_clipboard_get(BKContext *ctx)
{
  if (ctx && ctx->host.clipboard_get) {
    char *s = ctx->host.clipboard_get(ctx->host.data);
    if (s) {
      return s;
    }
  }
  return bk_strdup(g_clipboard ? g_clipboard : "");
}

void bk_clipboard_set(BKContext *ctx, const char *s)
{
  bk_set_text(&g_clipboard, s);
  if (ctx && ctx->host.clipboard_set) {
    ctx->host.clipboard_set(ctx->host.data, s);
  }
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Rendering
 * \{ */

static void layout(BKContext *ctx)
{
  g_gen++;
  const int w = ctx->win_w, h = ctx->win_h;
  int hw = 0, hh = 0;
  if (ctx->header) {
    bk_natural(ctx->header, &hw, &hh);
  }
  ctx->header_h = hh;
  int cw = 0, ch = 0;
  if (ctx->root) {
    bk_natural(ctx->root, &cw, &ch);
  }
  ctx->content_h = ch;
  const int view_h = h - hh;
  ctx->scroll = BK_CLAMP(ctx->scroll, 0, BK_MAX(0, ch - view_h));
  if (ctx->root) {
    bk_arrange(ctx->root, 0, hh - ctx->scroll, w, BK_MAX(ch, view_h));
  }
  if (ctx->header) {
    bk_arrange(ctx->header, 0, 0, w, hh);
  }
  for (int i = 0; i < ctx->npopups; i++) {
    bk_popup_place(ctx->popups[i], w, h);
  }
  ctx->layout_dirty = false;
}

static void tooltip_rect(BKContext *ctx, BKRect *r_rect)
{
  const BKNode *n = ctx->tip_node;
  const char *desc = n && n->desc ? n->desc : "";
  const int u = bk_unit(), p = bk_pad();
  int lines = 1;
  float w = 0.0f;
  const char *s = desc;
  while (1) {
    const char *nl = strchr(s, '\n');
    const int len = nl ? (int)(nl - s) : (int)strlen(s);
    w = BK_MAX(w, bk_text_width(s, len, false));
    if (!nl) {
      break;
    }
    lines++;
    s = nl + 1;
  }
  const int tw = (int)w + 2 * p, th = lines * u + p / 2;
  int tx = BK_MAX(0, BK_MIN(ctx->tip_x - tw / 4, ctx->win_w - tw));
  int ty = ctx->tip_y + (int)(0.9f * u);
  if (ty + th > ctx->win_h) {
    ty = BK_MAX(0, ctx->tip_y - th - (int)(0.4f * u));
  }
  *r_rect = bk_rect(tx, ty, tw, th);
}

static void draw_tooltip(BKContext *ctx)
{
  if (!ctx->tip_shown || !ctx->tip_node || !ctx->tip_node->desc) {
    return;
  }
  const BKRect r = ctx->tip_rect;
  bk_widget_tooltip_back(&r);
  const int u = bk_unit(), p = bk_pad();
  const char *s = ctx->tip_node->desc;
  int line = 0;
  while (1) {
    const char *nl = strchr(s, '\n');
    const int len = nl ? (int)(nl - s) : (int)strlen(s);
    BKRect lr = bk_rect(r.xmin, r.ymin + p / 4 + line * u, bk_rw(&r), u);
    uchar c[4];
    memcpy(c, bk_theme.tooltip.text, 4);
    if (line > 0) {
      c[3] = (uchar)(c[3] * 0.65f); /* details are dimmer than the title */
    }
    char buf[256];
    const int bl = BK_MIN(len, 255);
    memcpy(buf, s, (size_t)bl);
    buf[bl] = 0;
    bk_text_in(&lr, buf, c, BK_ALIGN_LEFT, false);
    if (!nl) {
      break;
    }
    s = nl + 1;
    line++;
  }
}

static void draw_region(BKContext *ctx, const BKRect *rect)
{
  const int w = ctx->win_w, h = ctx->win_h;
  bk_clip_reset(w, h);
  bk_clip_push(rect->xmin, rect->ymin, bk_rw(rect), bk_rh(rect));
  if (!(ctx->root && (ctx->root->flags & NF_OPAQUE))) {
    bk_fill(rect->xmin, rect->ymin, bk_rw(rect), bk_rh(rect), bk_theme.region_bg);
  }
  const int top = ctx->header_h, view_h = h - top;
  bk_clip_push(0, top, w, view_h);
  if (ctx->root) {
    bk_draw_tree(ctx->root);
  }
  bk_clip_pop();
  if (ctx->header) {
    bk_draw_tree(ctx->header);
  }
  if (ctx->content_h > view_h) { /* region scrollbar */
    const int sb = BK_MAX(2, (int)(0.15f * bk_unit()));
    const float th = BK_MAX((float)bk_unit(), (float)view_h * view_h / ctx->content_h);
    const float ty = top + (view_h - th) * ctx->scroll / (float)(ctx->content_h - view_h);
    float c[4];
    bk_rgba(bk_theme.scroll.item, c);
    bk_roundbox(w - sb - 2, ty, sb, th, sb * 0.5f, CNR_ALL, c);
  }
  for (int i = 0; i < ctx->npopups; i++) {
    bk_draw_tree(ctx->popups[i]);
  }
  draw_tooltip(ctx);
  bk_clip_reset(w, h);
}

int BK_needs_render(BKContext *ctx)
{
  return ctx->layout_dirty || ctx->full_dirty || ctx->ndirty > 0;
}

static void anim_step(BKContext *ctx);

int BK_render(BKContext *ctx, BuiRaster *r, int w, int h, int full)
{
  bk_draw.r = r;
  ctx->raster = r;
  anim_step(ctx);
  if (w != ctx->win_w || h != ctx->win_h || BUI_raster_width(r) != w ||
      BUI_raster_height(r) != h)
  {
    ctx->win_w = w;
    ctx->win_h = h;
    ctx->layout_dirty = true;
  }
  if (full || ctx->layout_dirty) {
    ctx->full_dirty = true;
  }
  if (ctx->layout_dirty) {
    layout(ctx);
  }
  bk_views_check(ctx);
  if (ctx->layout_dirty) { /* a scroll command changed the layout */
    layout(ctx);
  }
  if (!ctx->full_dirty && ctx->ndirty == 0) {
    return 0;
  }
  BUI_raster_begin_frame(r, w, h);
  int regions;
  if (ctx->full_dirty) {
    const BKRect all = bk_rect(0, 0, w, h);
    draw_region(ctx, &all);
    BUI_raster_end_frame(r);
    regions = 1;
  }
  else {
    BKRect bbox = ctx->dirty[0];
    for (int i = 0; i < ctx->ndirty; i++) {
      BKRect d = ctx->dirty[i];
      d.xmin = BK_MAX(d.xmin, 0);
      d.ymin = BK_MAX(d.ymin, 0);
      d.xmax = BK_MIN(d.xmax, w);
      d.ymax = BK_MIN(d.ymax, h);
      if (d.xmax > d.xmin && d.ymax > d.ymin) {
        draw_region(ctx, &d);
      }
      rect_union(&bbox, &ctx->dirty[i]);
    }
    BUI_raster_end_frame_region(r, bbox.xmin, bbox.ymin, bk_rw(&bbox), bk_rh(&bbox));
    regions = ctx->ndirty;
  }
  ctx->full_dirty = false;
  ctx->ndirty = 0;
  return regions;
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Input
 * \{ */

#define DOUBLE_CLICK_S 0.35
#define DOUBLE_CLICK_PX 4
#define TOOLTIP_DELAY_MS 600

static BKNode *hit_all(BKContext *ctx, int x, int y)
{
  for (int i = ctx->npopups - 1; i >= 0; i--) {
    if (bk_contains(ctx->popups[i], x, y)) {
      return bk_hit(ctx->popups[i], x, y);
    }
  }
  if (ctx->header && bk_contains(ctx->header, x, y)) {
    return bk_hit(ctx->header, x, y);
  }
  if (y < ctx->header_h) {
    return NULL;
  }
  return ctx->root ? bk_hit(ctx->root, x, y) : NULL;
}

static BKEvent make_event(BKContext *ctx, BKNode *target, int x, int y, int mods)
{
  BKEvent e;
  memset(&e, 0, sizeof(e));
  e.x = x;
  e.y = y;
  e.mods = mods;
  if (target) {
    e.lx = x - target->x;
    e.ly = y - target->y;
    e.inside = bk_contains(target, x, y);
  }
  e.dx = x - ctx->press_x;
  e.dy = y - ctx->press_y;
  e.dragged = ctx->dragging;
  e.text = "";
  e.keysym = "";
  return e;
}

static int hide_tooltip(BKContext *ctx)
{
  ctx->tip_serial++;
  if (ctx->tip_shown) {
    ctx->tip_shown = 0;
    bk_ctx_dirty_rect(ctx, bk_rect(ctx->tip_rect.xmin - 12, ctx->tip_rect.ymin - 12,
                                   bk_rw(&ctx->tip_rect) + 24, bk_rh(&ctx->tip_rect) + 24));
    return 1;
  }
  return 0;
}

static void arm_tooltip(BKContext *ctx, BKNode *target, int x, int y)
{
  ctx->tip_serial++;
  if (!target || !target->desc || !target->desc[0] || !ctx->host.schedule) {
    return;
  }
  ctx->tip_node = target;
  ctx->tip_x = x;
  ctx->tip_y = y;
  ctx->host.schedule(ctx->host.data, TOOLTIP_DELAY_MS, ctx->tip_serial);
}

void BK_timer(BKContext *ctx, int timer)
{
  if (timer == ctx->tip_serial && ctx->tip_node && ctx->hover == ctx->tip_node && !ctx->grab &&
      !ctx->tip_shown)
  {
    ctx->tip_shown = 1;
    tooltip_rect(ctx, &ctx->tip_rect);
    bk_ctx_dirty_rect(ctx, bk_rect(ctx->tip_rect.xmin - 12, ctx->tip_rect.ymin - 12,
                                   bk_rw(&ctx->tip_rect) + 24, bk_rh(&ctx->tip_rect) + 24));
  }
}

static void set_hover(BKContext *ctx, BKNode *n)
{
  if (n == ctx->hover) {
    return;
  }
  BKNode *old = ctx->hover;
  ctx->hover = n;
  if (old) {
    if (old->cls->leave) {
      old->cls->leave(old);
    }
    bk_invalidate(old);
    bk_event(old, BK_EV_LEAVE, 0, NULL);
  }
  if (n) {
    bk_invalidate(n);
    bk_event(n, BK_EV_ENTER, 0, NULL);
  }
}

static void update_cursor(BKContext *ctx)
{
  int name;
  if (ctx->modal) {
    name = ctx->modal->cls->cursor ? ctx->modal->cls->cursor : BK_CURSOR_CROSSHAIR;
  }
  else if (ctx->grab && ctx->dragging && ctx->grab->cls->drag_cursor) {
    name = ctx->grab->cls->drag_cursor;
  }
  else {
    BKNode *n = ctx->grab ? ctx->grab : ctx->hover;
    name = BK_CURSOR_DEFAULT;
    if (n && n->cursor_override) {
      name = n->cursor_override - 1;
    }
    else if (n && NODE_ENABLED(n)) {
      name = n->cls->cursor_at ? n->cls->cursor_at(n, ctx->pointer_x, ctx->pointer_y) :
                                 n->cls->cursor;
    }
  }
  if (name != ctx->cursor) {
    ctx->cursor = name;
    if (ctx->host.set_cursor) {
      ctx->host.set_cursor(ctx->host.data, name);
    }
  }
}

static bool continuous(BKContext *ctx)
{
  return ctx->host.warp && ctx->grab && ctx->dragging &&
         ctx->grab->cls->drag_cursor == BK_CURSOR_NONE;
}

static void wrap_pointer(BKContext *ctx, int x, int y)
{
  const int e = 12, w = ctx->win_w, h = ctx->win_h;
  if (w <= 2 * e || h <= 2 * e) {
    return;
  }
  if (x > e && x < w - e && y > e && y < h - e) {
    return;
  }
  int px = ctx->press_x - ctx->off_x, py = ctx->press_y - ctx->off_y;
  px = BK_CLAMP(px, e + 1, w - e - 1);
  py = BK_CLAMP(py, e + 1, h - e - 1);
  ctx->off_x += x - px;
  ctx->off_y += y - py;
  ctx->skip_x = px;
  ctx->skip_y = py;
  ctx->has_skip = true;
  ctx->host.warp(ctx->host.data, px, py);
}

void BK_set_pointer(BKContext *ctx, int x, int y)
{
  ctx->pointer_x = x;
  ctx->pointer_y = y;
}

static int in_BK_motion(BKContext *ctx, int x, int y, int mods)
{
  if (ctx->has_skip) {
    ctx->has_skip = false;
    if (x == ctx->skip_x && y == ctx->skip_y) {
      return 0; /* the warp we caused */
    }
  }
  ctx->pointer_x = x;
  ctx->pointer_y = y;
  int changed = hide_tooltip(ctx);
  if (ctx->modal) {
    BKEvent e = make_event(ctx, ctx->modal, x, y, mods);
    if (ctx->modal->cls->move) {
      changed |= ctx->modal->cls->move(ctx->modal, &e);
    }
    update_cursor(ctx);
    return changed;
  }
  if (ctx->grab) {
    const int vx = x + ctx->off_x, vy = y + ctx->off_y;
    const int threshold = BK_MAX(3, (int)(0.15f * bk_unit()));
    if (!ctx->dragging &&
        (abs(vx - ctx->press_x) > threshold || abs(vy - ctx->press_y) > threshold))
    {
      ctx->dragging = true;
      update_cursor(ctx);
    }
    if (ctx->dragging) {
      BKEvent e = make_event(ctx, ctx->grab, vx, vy, mods);
      if (ctx->grab->cls->drag) {
        changed |= ctx->grab->cls->drag(ctx->grab, &e);
      }
      if (ctx->grab) {
        bk_event(ctx->grab, BK_EV_MOTION, mods, &e);
      }
    }
    if (!ctx->grab) { /* the drag handler may have closed things */
      return 1;
    }
    BKNode *under = hit_all(ctx, x, y);
    if (ctx->dt_active && under && under->cls->drag_group == ctx->dt_group &&
        NODE_ENABLED(under) && under != ctx->grab)
    {
      if ((int)(under->value != 0) != ctx->dt_value) { /* Blender: drag across toggles */
        if (under->cls->drag_group == DRAG_GROUP_PANEL) {
          if (!bk_anim_running(under)) {
            under->anim = (under->flags & NF_COLLAPSED) ? 0.0f : 1.0f;
          }
          under->flags = ctx->dt_value ? (under->flags & ~NF_COLLAPSED) :
                                         (under->flags | NF_COLLAPSED);
          under->value = ctx->dt_value;
          bk_anim_start(under);
          bk_relayout(under);
        }
        else {
          bk_set_value(under, ctx->dt_value);
        }
        changed = 1;
      }
    }
    if (under != ctx->hover) {
      set_hover(ctx, under);
      changed = 1;
    }
    if (continuous(ctx)) {
      wrap_pointer(ctx, x, y);
    }
    return changed;
  }
  BKNode *under = hit_all(ctx, x, y);
  if (under != ctx->hover) {
    set_hover(ctx, under);
    changed = 1;
  }
  if (under) {
    BKEvent e = make_event(ctx, under, x, y, mods);
    if (under->cls->move) {
      changed |= under->cls->move(under, &e);
    }
    bk_event(under, BK_EV_MOTION, mods, &e);
  }
  arm_tooltip(ctx, under, x, y);
  update_cursor(ctx);
  return changed;
}

static int in_BK_press(BKContext *ctx, int x, int y, int mods)
{
  hide_tooltip(ctx);
  ctx->pointer_x = x;
  ctx->pointer_y = y;
  ctx->press_x = x;
  ctx->press_y = y;
  ctx->dragging = false;
  ctx->has_ungrab = false;
  ctx->off_x = ctx->off_y = 0;
  if (ctx->modal) {
    BKEvent e = make_event(ctx, ctx->modal, x, y, mods);
    if (ctx->modal->cls->press) {
      ctx->modal->cls->press(ctx->modal, &e);
    }
    update_cursor(ctx);
    return 1;
  }
  if (ctx->npopups) {
    bool inside = false;
    for (int i = 0; i < ctx->npopups; i++) {
      inside |= bk_contains(ctx->popups[i], x, y);
    }
    if (!inside) {
      BK_close_popups(ctx); /* Blender: an outside click only closes menus */
      ctx->hover = NULL;
      update_cursor(ctx);
      return 1;
    }
  }
  BKNode *target = hit_all(ctx, x, y);
  if (ctx->focus && target != ctx->focus) {
    end_edit(ctx, true);
  }
  set_hover(ctx, target);
  if (!target || !NODE_ENABLED(target)) {
    ctx->grab = NULL; /* disabled components get no drags either */
    return 1;
  }
  ctx->grab = target;
  ctx->interaction++;
  bk_invalidate(target);
  BKEvent e = make_event(ctx, target, x, y, mods);
  const double now = bk_time();
  if (ctx->last_click_node == target && now - ctx->last_click_t < DOUBLE_CLICK_S &&
      abs(x - ctx->last_click_x) <= DOUBLE_CLICK_PX && abs(y - ctx->last_click_y) <= DOUBLE_CLICK_PX)
  {
    ctx->last_click_node = NULL;
    bk_event(target, BK_EV_DOUBLE, mods, &e);
    if (target->cls->dbl) {
      target->cls->dbl(target, &e);
    }
    else if (target->cls->press) {
      target->cls->press(target, &e);
    }
  }
  else {
    ctx->last_click_t = now;
    ctx->last_click_node = target;
    ctx->last_click_x = x;
    ctx->last_click_y = y;
    bk_event(target, BK_EV_PRESS, mods, &e);
    if (target->cls->press) {
      target->cls->press(target, &e);
    }
  }
  update_cursor(ctx);
  return 1;
}

static int in_BK_release(BKContext *ctx, int x, int y, int mods)
{
  if (ctx->modal) {
    BKEvent e = make_event(ctx, ctx->modal, x, y, mods);
    if (ctx->modal->cls->release) {
      ctx->modal->cls->release(ctx->modal, &e);
    }
    return 1;
  }
  const bool was_continuous = continuous(ctx);
  const int vx = x + ctx->off_x, vy = y + ctx->off_y;
  BKNode *grab = ctx->grab;
  ctx->dt_active = 0;
  if (grab && NODE_ENABLED(grab)) {
    BKEvent e = make_event(ctx, grab, vx, vy, mods);
    if (grab->cls->release) {
      grab->cls->release(grab, &e);
    }
    if (ctx->grab == grab) {
      bk_event(grab, BK_EV_RELEASE, mods, &e);
    }
  }
  /* The handler may have closed (freed) popups; that clears ctx->grab when
   * it pointed into them, so re-read it. */
  grab = ctx->grab;
  ctx->grab = NULL;
  if (grab) {
    bk_invalidate(grab);
  }
  if (ctx->dragging && bk_popup_index(ctx, grab) < 0) {
    /* Blender: press a menu button, drag onto an item, release to pick. */
    for (int i = ctx->npopups - 1; i >= 0; i--) {
      BKNode *p = ctx->popups[i];
      if (bk_contains(p, x, y) && p != grab) {
        BKEvent e = make_event(ctx, p, x, y, mods);
        if (p->cls->release) {
          p->cls->release(p, &e);
        }
        break;
      }
    }
  }
  ctx->dragging = false;
  if (was_continuous) {
    /* Blender puts the pointer back where the drag began, or where the
     * widget asks (USE_CONT_MOUSE_CORRECT: a slider's value). */
    const int ux = ctx->has_ungrab ? ctx->ungrab_x : ctx->press_x;
    const int uy = ctx->has_ungrab ? ctx->ungrab_y : ctx->press_y;
    ctx->has_ungrab = false;
    ctx->skip_x = ux;
    ctx->skip_y = uy;
    ctx->has_skip = true;
    ctx->host.warp(ctx->host.data, ux, uy);
    x = ux;
    y = uy;
  }
  ctx->off_x = ctx->off_y = 0;
  ctx->pointer_x = x;
  ctx->pointer_y = y;
  set_hover(ctx, hit_all(ctx, x, y));
  update_cursor(ctx);
  return 1;
}

static void raw_event(BKNode *n, int kind, int mods, const BKEvent *e, int button);

static int in_BK_context_menu(BKContext *ctx, int x, int y, int mods)
{
  hide_tooltip(ctx);
  if (ctx->modal) {
    bk_end_modal(ctx);
    return 1;
  }
  if (ctx->focus) {
    end_edit(ctx, true);
  }
  BKNode *under = hit_all(ctx, x, y);
  if (under && (under->flags & NF_EVENTS)) { /* <Button-3> bindings see it first */
    BKEvent e = make_event(ctx, under, x, y, mods);
    raw_event(under, BK_EV_PRESS, mods, &e, 3);
  }
  bool in_popup = false;
  for (int i = 0; i < ctx->npopups; i++) {
    in_popup |= bk_contains(ctx->popups[i], x, y);
  }
  BKNode *target = hit_all(ctx, x, y);
  if (!in_popup) {
    BK_close_popups(ctx);
  }
  for (BKNode *n = target; n; n = n->parent) {
    if (!NODE_ENABLED(n) || bk_popup_index(ctx, n) >= 0) {
      continue;
    }
    BKMenuSpec items[16];
    int count = 0;
    if (n->cls->context_items) {
      count = n->cls->context_items(n, items, 16);
    }
    else if (n->cls->flags & CLS_VALUE) {
      count = bk_value_context_items(n, items, 16);
    }
    if (count > 0) {
      if (in_popup) {
        BK_close_popups(ctx);
      }
      BK_open_menu(ctx, n, items, count, n->label && n->label[0] ? n->label : NULL, x, y);
      return 1;
    }
  }
  return 1;
}

static int key_mods_match(int mods, const BKKeyBinding *k)
{
  return (mods & (BK_SHIFT | BK_CTRL | BK_ALT)) == k->mods;
}

/* Blender: Ctrl+Z / Ctrl+Shift+Z (and Ctrl+Y) undo/redo UI edits */
static int undo_key(BKContext *ctx, const char *keysym, int mods)
{
  if ((mods & BK_CTRL) && bk_keysym_is(keysym, "z")) {
    return (mods & BK_SHIFT) ? BK_redo(ctx) : BK_undo(ctx);
  }
  if ((mods & BK_CTRL) && bk_keysym_is(keysym, "y")) {
    return BK_redo(ctx);
  }
  return 0;
}

static int in_BK_key(BKContext *ctx, const char *text, const char *keysym, int mods)
{
  hide_tooltip(ctx);
  BKEvent e = make_event(ctx, NULL, ctx->pointer_x, ctx->pointer_y, mods);
  e.text = text ? text : "";
  e.keysym = keysym ? keysym : "";
  ctx->interaction++;
  for (BKNode *n = ctx->focus ? ctx->focus : ctx->hover; n; n = n->parent) {
    if (n->flags & NF_EVENTS) { /* Tk/Qt key bindings see the key first */
      bk_event(n, BK_EV_KEY, mods, &e);
      break;
    }
  }
  if (ctx->modal) {
    if (bk_keysym_is(e.keysym, "Escape")) {
      bk_end_modal(ctx);
    }
    return 1;
  }
  if (ctx->grab && ctx->dragging && ctx->grab->cls->cancel && bk_keysym_is(e.keysym, "Escape")) {
    ctx->grab->cls->cancel(ctx->grab); /* Blender: Escape cancels a drag */
    ctx->has_ungrab = false;
    return 1;
  }
  if (ctx->focus) {
    BKNode *f = ctx->focus;
    if (f->cls->key && f->cls->key(f, &e)) {
      return 1;
    }
    return undo_key(ctx, e.keysym, mods);
  }
  if (ctx->npopups) {
    BKNode *top = ctx->popups[ctx->npopups - 1];
    if (top->cls->key && top->cls->key(top, &e)) {
      return 1;
    }
    if (bk_keysym_is(e.keysym, "Escape")) {
      bk_close_popups_after(ctx, ctx->npopups > 1 ? ctx->popups[ctx->npopups - 2] : NULL);
      return 1;
    }
    return 0;
  }
  for (BKNode *n = ctx->hover; n; n = n->parent) { /* bubble up to a handler */
    if (!NODE_ENABLED(n)) {
      continue;
    }
    if (n->cls->key) {
      e.lx = e.x - n->x;
      e.ly = e.y - n->y;
      if (n->cls->key(n, &e)) {
        return 1;
      }
    }
    else if ((n->cls->flags & CLS_VALUE) && bk_value_key(n, &e)) {
      return 1;
    }
  }
  for (int i = 0; i < ctx->nkeys; i++) {
    if (key_mods_match(mods, &ctx->keys[i]) && bk_keysym_is(e.keysym, ctx->keys[i].keysym)) {
      if (ctx->host.emit) {
        ctx->host.emit(ctx->host.data, NULL, BK_EV_CALL, ctx->keys[i].ref, 0, 0);
      }
      return 1;
    }
  }
  return undo_key(ctx, e.keysym, mods);
}

static BKNode *deepest(BKNode *n, int x, int y)
{
  if (!NODE_VISIBLE(n) || !bk_contains(n, x, y)) {
    return NULL;
  }
  for (int i = n->nkids - 1; i >= 0; i--) {
    BKNode *f = deepest(n->kids[i], x, y);
    if (f) {
      return f;
    }
  }
  return n;
}

static int in_BK_wheel(BKContext *ctx, int x, int y, int delta, int mods)
{
  hide_tooltip(ctx);
  BKNode *target = hit_all(ctx, x, y);
  if (!target && ctx->root && y >= ctx->header_h) {
    target = deepest(ctx->root, x, y); /* non-interactive area: let containers scroll */
  }
  for (BKNode *n = target; n; n = n->parent) {
    if (!NODE_ENABLED(n)) {
      continue;
    }
    BKEvent e = make_event(ctx, n, x, y, mods);
    e.delta = delta;
    if (n == target) {
      bk_event(n, BK_EV_WHEEL, mods, &e);
    }
    if (n->cls->wheel ? n->cls->wheel(n, &e) :
                        ((n->cls->flags & CLS_VALUE) && bk_value_wheel(n, &e)))
    {
      return 1;
    }
  }
  /* scroll the region */
  const int view_h = ctx->win_h - ctx->header_h;
  if (ctx->content_h > view_h && !ctx->npopups) {
    const int ns = BK_CLAMP(ctx->scroll - delta * 2 * bk_unit(), 0, ctx->content_h - view_h);
    if (ns != ctx->scroll) {
      ctx->scroll = ns;
      ctx->layout_dirty = true;
      bk_ctx_request(ctx);
      return 1;
    }
  }
  return 0;
}

void BK_bind(BKContext *ctx, const char *combo, void *ref)
{
  if (ctx->nkeys == BK_MAX_KEYS) {
    return;
  }
  BKKeyBinding *k = &ctx->keys[ctx->nkeys++];
  memset(k, 0, sizeof(*k));
  const char *s = combo;
  while (1) {
    const char *plus = strchr(s, '+');
    if (!plus) {
      break;
    }
    const int len = (int)(plus - s);
    if (len == 4 && strncmp(s, "ctrl", 4) == 0) {
      k->mods |= BK_CTRL;
    }
    else if (len == 5 && strncmp(s, "shift", 5) == 0) {
      k->mods |= BK_SHIFT;
    }
    else if (len == 3 && strncmp(s, "alt", 3) == 0) {
      k->mods |= BK_ALT;
    }
    s = plus + 1;
  }
  strncpy(k->keysym, s, sizeof(k->keysym) - 1);
  k->ref = ref;
  bk_retain(ref);
}

int BK_cursor(BKContext *ctx)
{
  return ctx->cursor;
}
BKNode *BK_hover(BKContext *ctx)
{
  return ctx->hover;
}
BKNode *BK_focus(BKContext *ctx)
{
  return ctx->focus;
}

void BK_focus_node(BKContext *ctx, BKNode *n)
{
  if (!n) {
    if (ctx->focus) {
      end_edit(ctx, true);
    }
    return;
  }
  if (n == ctx->focus || !NODE_ENABLED(n)) {
    return;
  }
  if (n->cls == &bk_cls_textarea) {
    if (ctx->focus) {
      end_edit(ctx, true);
    }
    ctx->focus = n;
    bk_invalidate(n);
  }
  else if ((n->cls == &bk_cls_text || n->cls == &bk_cls_search || n->cls == &bk_cls_number) &&
           n->cls->activate)
  {
    n->cls->activate(n);
  }
}

char *BK_clipboard_get(BKContext *ctx)
{
  return bk_clipboard_get(ctx);
}

void BK_clipboard_set(BKContext *ctx, const char *text)
{
  bk_clipboard_set(ctx, text);
}
int BK_modal_active(BKContext *ctx)
{
  return ctx->modal != NULL;
}
int BK_tooltip_visible(BKContext *ctx)
{
  return ctx->tip_shown;
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Text editing (Blender's text button: ui_textedit_*)
 * \{ */

BKTextEdit *bk_te_new(const char *text, bool select_all)
{
  BKTextEdit *te = (BKTextEdit *)calloc(1, sizeof(BKTextEdit));
  te->len = text ? (int)strlen(text) : 0;
  te->cap = te->len + 64;
  te->text = (char *)malloc((size_t)te->cap);
  memcpy(te->text, text ? text : "", (size_t)te->len + 1);
  te->caret = te->len;
  te->anchor = select_all ? 0 : -1;
  return te;
}

void bk_te_free(BKTextEdit *te)
{
  if (te) {
    free(te->text);
    free(te);
  }
}

static bool te_selection(const BKTextEdit *te, int *a, int *b)
{
  if (te->anchor < 0 || te->anchor == te->caret) {
    return false;
  }
  *a = BK_MIN(te->anchor, te->caret);
  *b = BK_MAX(te->anchor, te->caret);
  return true;
}

static void te_delete(BKTextEdit *te, int a, int b)
{
  memmove(te->text + a, te->text + b, (size_t)(te->len - b + 1));
  te->len -= b - a;
  te->caret = a;
  te->anchor = -1;
}

static bool te_delete_selection(BKTextEdit *te)
{
  int a, b;
  if (!te_selection(te, &a, &b)) {
    return false;
  }
  te_delete(te, a, b);
  return true;
}

static void te_insert(BKTextEdit *te, const char *s)
{
  te_delete_selection(te);
  int n = (int)strlen(s);
  if (te->len + n + 1 > te->cap) {
    te->cap = te->len + n + 64;
    te->text = (char *)realloc(te->text, (size_t)te->cap);
  }
  memmove(te->text + te->caret + n, te->text + te->caret, (size_t)(te->len - te->caret + 1));
  memcpy(te->text + te->caret, s, (size_t)n);
  /* no line breaks in single-line fields */
  for (int i = 0; i < n; i++) {
    if (te->text[te->caret + i] == '\n' || te->text[te->caret + i] == '\r') {
      te->text[te->caret + i] = ' ';
    }
  }
  te->len += n;
  te->caret += n;
  te->anchor = -1;
}

static void te_move(BKTextEdit *te, int to, bool extend)
{
  if (extend) {
    if (te->anchor < 0) {
      te->anchor = te->caret;
    }
  }
  else {
    te->anchor = -1;
  }
  te->caret = BK_CLAMP(to, 0, te->len);
}

static int te_step(BKTextEdit *te, int pos, bool next, bool word)
{
  BLI_str_cursor_step_utf8(te->text, te->len, &pos, next ? STRCUR_DIR_NEXT : STRCUR_DIR_PREV,
                           word ? STRCUR_JUMP_DELIM : STRCUR_JUMP_NONE, true);
  return pos;
}

int bk_te_key(BKTextEdit *te, BKContext *ctx, const char *text, const char *ks, int mods)
{
  const bool ctrl = mods & BK_CTRL, shift = mods & BK_SHIFT;
  if (bk_keysym_is(ks, "ISO_Left_Tab")) { /* X11's Shift+Tab */
    return TE_PREV;
  }
  if (bk_keysym_is(ks, "Tab")) {
    return (mods & BK_SHIFT) ? TE_PREV : TE_NEXT; /* Blender: Tab edits the next field */
  }
  if (bk_keysym_is(ks, "Return") || bk_keysym_is(ks, "KP_Enter")) {
    return TE_CONFIRM;
  }
  if (bk_keysym_is(ks, "Escape")) {
    return TE_CANCEL;
  }
  int a, b;
  const bool has_sel = te_selection(te, &a, &b);
  if (ctrl && (bk_keysym_is(ks, "a") || bk_keysym_is(ks, "c") || bk_keysym_is(ks, "x") ||
               bk_keysym_is(ks, "v")))
  {
    if (bk_keysym_is(ks, "a")) {
      te->anchor = 0;
      te->caret = te->len;
    }
    else if ((bk_keysym_is(ks, "c") || bk_keysym_is(ks, "x")) && has_sel) {
      char *sel = (char *)malloc((size_t)(b - a + 1));
      memcpy(sel, te->text + a, (size_t)(b - a));
      sel[b - a] = 0;
      bk_clipboard_set(ctx, sel);
      free(sel);
      if (bk_keysym_is(ks, "x")) {
        te_delete_selection(te);
      }
    }
    else if (bk_keysym_is(ks, "v")) {
      char *clip = bk_clipboard_get(ctx);
      if (clip) {
        te_insert(te, clip);
        free(clip);
      }
    }
    return TE_CHANGED;
  }
  if (bk_keysym_is(ks, "Left")) {
    if (has_sel && !shift) {
      te_move(te, a, false);
    }
    else {
      te_move(te, te_step(te, te->caret, false, ctrl), shift);
    }
    return TE_CHANGED;
  }
  if (bk_keysym_is(ks, "Right")) {
    if (has_sel && !shift) {
      te_move(te, b, false);
    }
    else {
      te_move(te, te_step(te, te->caret, true, ctrl), shift);
    }
    return TE_CHANGED;
  }
  if (bk_keysym_is(ks, "Home") || bk_keysym_is(ks, "Up")) {
    te_move(te, 0, shift);
    return TE_CHANGED;
  }
  if (bk_keysym_is(ks, "End") || bk_keysym_is(ks, "Down")) {
    te_move(te, te->len, shift);
    return TE_CHANGED;
  }
  if (bk_keysym_is(ks, "BackSpace")) {
    if (!te_delete_selection(te) && te->caret > 0) {
      te_delete(te, te_step(te, te->caret, false, ctrl), te->caret);
    }
    return TE_CHANGED;
  }
  if (bk_keysym_is(ks, "Delete")) {
    if (!te_delete_selection(te) && te->caret < te->len) {
      const int c = te->caret;
      te_delete(te, c, te_step(te, c, true, ctrl));
    }
    return TE_CHANGED;
  }
  if (text && (uchar)text[0] >= 0x20 && text[0] != 0x7f && !ctrl) {
    te_insert(te, text);
    return TE_CHANGED;
  }
  return TE_NONE;
}

int bk_te_index_at(BKTextEdit *te, float px)
{
  px += te->scroll;
  float prev = 0.0f;
  int i = 0;
  while (i < te->len) {
    int nx = i;
    BLI_str_cursor_step_utf8(te->text, te->len, &nx, STRCUR_DIR_NEXT, STRCUR_JUMP_NONE, true);
    const float w = bk_masked_width(te->text, nx, te->mask, false);
    if (px < (prev + w) * 0.5f) {
      return i;
    }
    prev = w;
    i = nx;
  }
  return te->len;
}

void bk_te_click(BKTextEdit *te, float px, bool extend)
{
  te_move(te, bk_te_index_at(te, px), extend);
}

void bk_te_drag(BKTextEdit *te, float px)
{
  if (te->anchor < 0) {
    te->anchor = te->caret;
  }
  te->caret = bk_te_index_at(te, px);
}

void bk_te_select_word(BKTextEdit *te, int index)
{
  int a, b;
  BLI_str_cursor_step_bounds_utf8(te->text, te->len, BK_CLAMP(index, 0, te->len), &a, &b);
  if (a == b) {
    te->anchor = 0;
    te->caret = te->len;
  }
  else {
    te->anchor = a;
    te->caret = b;
  }
}

void bk_te_draw(BKTextEdit *te, const BKRect *rect, const uchar color[4])
{
  const int p = bk_pad();
  const float inner = BK_MAX(1.0f, (float)(bk_rw(rect) - 2 * p));
  const float caret_px = bk_masked_width(te->text, te->caret, te->mask, false);
  if (caret_px - te->scroll > inner) {
    te->scroll = caret_px - inner;
  }
  else if (caret_px - te->scroll < 0) {
    te->scroll = caret_px;
  }
  const float tx = rect->xmin + p - te->scroll;
  const int h = bk_rh(rect);
  bk_clip_push(rect->xmin + 1, rect->ymin, bk_rw(rect) - 2, h);
  int a, b;
  if (te_selection(te, &a, &b)) {
    const float sx = tx + bk_masked_width(te->text, a, te->mask, false);
    const float sw = bk_masked_width(te->text, b, te->mask, false) -
                     bk_masked_width(te->text, a, te->mask, false);
    const float m = 0.15f * h;
    bk_fill(sx, rect->ymin + m, sw, h - 2 * m, bk_theme.selection);
  }
  if (te->mask[0]) {
    char buf[1024];
    bk_text(tx, bk_text_baseline(rect), buf, bk_mask_text(te->text, te->len, te->mask, buf,
                                                           sizeof(buf)),
            color, false);
  }
  else {
    bk_text(tx, bk_text_baseline(rect), te->text, te->len, color, false);
  }
  const float m = 0.2f * h;
  bk_fill(floorf(tx + caret_px), rect->ymin + m, BK_MAX(1.0f, floorf(0.1f * bk_unit() + 0.5f)),
          h - 2 * m, bk_theme.cursor);
  bk_clip_pop();
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Number expressions ("2*pi", "1/3+2^2")
 * \{ */

typedef struct Parser {
  const char *s;
  bool err;
} Parser;

static void skip_ws(Parser *p)
{
  while (*p->s == ' ' || *p->s == '\t') {
    p->s++;
  }
}

static double parse_expr(Parser *p);

static double parse_atom(Parser *p)
{
  skip_ws(p);
  if (*p->s == '(') {
    p->s++;
    const double v = parse_expr(p);
    skip_ws(p);
    if (*p->s == ')') {
      p->s++;
    }
    else {
      p->err = true;
    }
    return v;
  }
  if (isalpha((uchar)*p->s)) {
    char name[16];
    int n = 0;
    while (isalnum((uchar)*p->s) && n < 15) {
      name[n++] = *p->s++;
    }
    name[n] = 0;
    if (strcmp(name, "pi") == 0) {
      return 3.14159265358979323846;
    }
    if (strcmp(name, "tau") == 0) {
      return 6.28318530717958647692;
    }
    if (strcmp(name, "e") == 0) {
      return 2.71828182845904523536;
    }
    /* functions */
    skip_ws(p);
    if (*p->s == '(') {
      const double a = parse_atom(p);
      if (strcmp(name, "sqrt") == 0) {
        return sqrt(a);
      }
      if (strcmp(name, "sin") == 0) {
        return sin(a);
      }
      if (strcmp(name, "cos") == 0) {
        return cos(a);
      }
      if (strcmp(name, "tan") == 0) {
        return tan(a);
      }
      if (strcmp(name, "abs") == 0) {
        return fabs(a);
      }
      if (strcmp(name, "radians") == 0) {
        return a * 3.14159265358979323846 / 180.0;
      }
      if (strcmp(name, "degrees") == 0) {
        return a * 180.0 / 3.14159265358979323846;
      }
    }
    p->err = true;
    return 0.0;
  }
  char *end;
  const double v = strtod(p->s, &end);
  if (end == p->s) {
    p->err = true;
    return 0.0;
  }
  p->s = end;
  return v;
}

static double parse_unary(Parser *p);

static double parse_power(Parser *p)
{
  const double base = parse_atom(p);
  skip_ws(p);
  if ((p->s[0] == '*' && p->s[1] == '*') || p->s[0] == '^') {
    p->s += p->s[0] == '^' ? 1 : 2;
    return pow(base, parse_unary(p)); /* right associative */
  }
  return base;
}

static double parse_unary(Parser *p)
{
  skip_ws(p);
  if (*p->s == '-') {
    p->s++;
    return -parse_unary(p);
  }
  if (*p->s == '+') {
    p->s++;
    return parse_unary(p);
  }
  return parse_power(p);
}

static double parse_term(Parser *p)
{
  double v = parse_unary(p);
  while (!p->err) {
    skip_ws(p);
    const char c = *p->s;
    if (c == '*' && p->s[1] != '*') {
      p->s++;
      v *= parse_unary(p);
    }
    else if (c == '/' && p->s[1] == '/') {
      p->s += 2;
      const double d = parse_unary(p);
      if (d == 0.0) {
        p->err = true;
        return 0.0;
      }
      v = floor(v / d);
    }
    else if (c == '/') {
      p->s++;
      const double d = parse_unary(p);
      if (d == 0.0) {
        p->err = true;
        return 0.0;
      }
      v /= d;
    }
    else if (c == '%') {
      p->s++;
      const double d = parse_unary(p);
      if (d == 0.0) {
        p->err = true;
        return 0.0;
      }
      v = fmod(v, d);
    }
    else {
      break;
    }
  }
  return v;
}

static double parse_expr(Parser *p)
{
  double v = parse_term(p);
  while (!p->err) {
    skip_ws(p);
    if (*p->s == '+') {
      p->s++;
      v += parse_term(p);
    }
    else if (*p->s == '-') {
      p->s++;
      v -= parse_term(p);
    }
    else {
      break;
    }
  }
  return v;
}

int BK_eval_number(const char *expr, double *r_value)
{
  Parser p = {expr, false};
  const double v = parse_expr(&p);
  skip_ws(&p);
  if (p.err || *p.s != 0 || !isfinite(v)) {
    return 0;
  }
  *r_value = v;
  return 1;
}

/** \} */

/* Public input entry points: mark the input scope (undo grouping) and report
 * the view changes the event caused (scrollbars follow). */
#define INPUT_ENTRY(call) \
  ctx->in_input++; \
  const int r = call; \
  ctx->in_input--; \
  bk_views_check(ctx); \
  return r

int BK_press(BKContext *ctx, int x, int y, int mods)
{
  INPUT_ENTRY(in_BK_press(ctx, x, y, mods));
}

int BK_release(BKContext *ctx, int x, int y, int mods)
{
  INPUT_ENTRY(in_BK_release(ctx, x, y, mods));
}

int BK_motion(BKContext *ctx, int x, int y, int mods)
{
  INPUT_ENTRY(in_BK_motion(ctx, x, y, mods));
}

int BK_wheel(BKContext *ctx, int x, int y, int delta, int mods)
{
  INPUT_ENTRY(in_BK_wheel(ctx, x, y, delta, mods));
}

int BK_key(BKContext *ctx, const char *text, const char *keysym, int mods)
{
  INPUT_ENTRY(in_BK_key(ctx, text, keysym, mods));
}

int BK_context_menu(BKContext *ctx, int x, int y, int mods)
{
  INPUT_ENTRY(in_BK_context_menu(ctx, x, y, mods));
}

int BK_key_release(BKContext *ctx, const char *keysym, int mods)
{
  BKEvent e = make_event(ctx, NULL, ctx->pointer_x, ctx->pointer_y, mods);
  e.text = "";
  e.keysym = keysym ? keysym : "";
  for (BKNode *n = ctx->focus ? ctx->focus : ctx->hover; n; n = n->parent) {
    if (n->flags & NF_EVENTS) {
      char buf[96];
      snprintf(buf, sizeof(buf), "\t%s", e.keysym);
      bk_emit(n, BK_EV_KEYRELEASE, buf, mods, 0);
      return 1;
    }
  }
  return 0;
}

int BK_button(BKContext *ctx, int x, int y, int button, int release, int mods)
{
  BKNode *under = hit_all(ctx, x, y);
  if (!under || !(under->flags & NF_EVENTS)) {
    return 0;
  }
  BKEvent e = make_event(ctx, under, x, y, mods);
  raw_event(under, release ? BK_EV_RELEASE : BK_EV_PRESS, mods, &e, button);
  return 1;
}

/* -------------------------------------------------------------------- */
/** \name Raw events, undo, animation, Tab order
 * \{ */

/* Raw events for host bindings (Tk's bind). Pointer events carry
 * "state button" in `ref` so hosts can match <Control-Button-3> etc. */
static void raw_event(BKNode *n, int kind, int mods, const BKEvent *e, int button)
{
  if (!(n->flags & NF_EVENTS)) {
    return;
  }
  char buf[128];
  if (kind == BK_EV_KEY && e) {
    snprintf(buf, sizeof(buf), "%s\t%s", e->text ? e->text : "", e->keysym ? e->keysym : "");
    bk_emit(n, kind, buf, mods, 0);
    return;
  }
  snprintf(buf, sizeof(buf), "%d %d", mods, button);
  if (kind == BK_EV_WHEEL && e) {
    bk_emit(n, kind, buf, e->delta, 0);
    return;
  }
  bk_emit(n, kind, buf, e ? e->x - n->x : 0, e ? e->y - n->y : 0);
}

void bk_event(BKNode *n, int kind, int mods, const BKEvent *e)
{
  raw_event(n, kind, mods, e, 1);
}

static const char *node_text(BKNode *n)
{
  return n->cls == &bk_cls_textarea ? bk_textarea_text(n) : (n->text ? n->text : "");
}

static void undo_entry_free(BKUndo *u)
{
  free(u->t[0]);
  free(u->t[1]);
  memset(u, 0, sizeof(*u));
}

void bk_undo_record(BKNode *n, int kind)
{
  BKContext *ctx = bk_ctx(n);
  if (!ctx || !ctx->in_input || ctx->in_undo) {
    return;
  }
  for (const BKNode *p = n; p; p = p->parent) {
    if (p->cls->flags & CLS_POPUP) {
      return; /* popup internals (picker sliders): the owner records */
    }
  }
  if (ctx->undo_pos > 0) {
    const BKUndo *top = &ctx->undo[ctx->undo_pos - 1];
    const int span = kind == 1 ? 24 : 0; /* a typing burst is one step */
    if (top->node == n && ctx->interaction - top->interaction <= span) {
      return; /* same interaction: keep the first "before" */
    }
  }
  for (int i = ctx->undo_pos; i < ctx->nundo; i++) {
    undo_entry_free(&ctx->undo[i]); /* drop the redo tail */
  }
  ctx->nundo = ctx->undo_pos;
  if (ctx->nundo == BK_MAX_UNDO) {
    undo_entry_free(&ctx->undo[0]);
    memmove(ctx->undo, ctx->undo + 1, sizeof(BKUndo) * (BK_MAX_UNDO - 1));
    memset(&ctx->undo[BK_MAX_UNDO - 1], 0, sizeof(BKUndo));
    ctx->nundo--;
  }
  BKUndo *u = &ctx->undo[ctx->nundo++];
  memset(u, 0, sizeof(*u));
  u->node = n;
  u->kind = kind;
  u->interaction = ctx->interaction;
  if (kind == 0) {
    u->v[0] = n->value;
  }
  else if (kind == 1) {
    u->t[0] = bk_strdup(node_text(n));
  }
  else {
    memcpy(u->c[0], n->color, sizeof(float[4]));
  }
  ctx->undo_pos = ctx->nundo;
}

static void undo_capture(BKUndo *u, int slot)
{
  BKNode *n = u->node;
  if (u->kind == 0) {
    u->v[slot] = n->value;
  }
  else if (u->kind == 1) {
    free(u->t[slot]);
    u->t[slot] = bk_strdup(node_text(n));
  }
  else {
    memcpy(u->c[slot], n->color, sizeof(float[4]));
  }
}

static void undo_apply(BKContext *ctx, BKUndo *u, int slot)
{
  BKNode *n = u->node;
  ctx->in_undo = 1;
  if (u->kind == 0) {
    bk_set_value(n, u->v[slot]);
  }
  else if (u->kind == 1) {
    if (n->cls == &bk_cls_textarea) {
      bk_textarea_set_text(n, u->t[slot]);
      bk_emit(n, BK_EV_CHANGE, NULL, 0, 0);
    }
    else {
      BK_set_str(n, "set_value", u->t[slot] ? u->t[slot] : "");
    }
  }
  else {
    BK_set_color(n, "set_value", u->c[slot]);
  }
  ctx->in_undo = 0;
}

int BK_undo(BKContext *ctx)
{
  if (ctx->undo_pos == 0) {
    return 0;
  }
  const int group = ctx->undo[ctx->undo_pos - 1].interaction;
  while (ctx->undo_pos > 0 && ctx->undo[ctx->undo_pos - 1].interaction == group) {
    BKUndo *u = &ctx->undo[--ctx->undo_pos];
    undo_capture(u, 1);
    undo_apply(ctx, u, 0);
  }
  return 1;
}

int BK_redo(BKContext *ctx)
{
  if (ctx->undo_pos >= ctx->nundo) {
    return 0;
  }
  const int group = ctx->undo[ctx->undo_pos].interaction;
  while (ctx->undo_pos < ctx->nundo && ctx->undo[ctx->undo_pos].interaction == group) {
    undo_apply(ctx, &ctx->undo[ctx->undo_pos++], 1);
  }
  return 1;
}

int BK_undo_depth(BKContext *ctx, int *r_redo)
{
  if (r_redo) {
    *r_redo = ctx->nundo - ctx->undo_pos;
  }
  return ctx->undo_pos;
}

static void undo_forget(BKContext *ctx, BKNode *subtree)
{
  int w = 0;
  const int pos = ctx->undo_pos;
  int new_pos = pos;
  for (int i = 0; i < ctx->nundo; i++) {
    if (is_within(ctx->undo[i].node, subtree)) {
      undo_entry_free(&ctx->undo[i]);
      if (i < pos) {
        new_pos--;
      }
      continue;
    }
    if (w != i) {
      ctx->undo[w] = ctx->undo[i];
      memset(&ctx->undo[i], 0, sizeof(BKUndo));
    }
    w++;
  }
  ctx->nundo = w;
  ctx->undo_pos = BK_MIN(new_pos, w);
}

static int g_animations = 1;
#define ANIM_SECONDS 0.12

void BK_set_animations(int enabled)
{
  g_animations = enabled != 0;
}

int BK_animating(BKContext *ctx)
{
  return ctx->nanim > 0;
}

void bk_anim_start(BKNode *n)
{
  BKContext *ctx = bk_ctx(n);
  if (!ctx || !g_animations) {
    n->anim = (n->flags & NF_COLLAPSED) ? 0.0f : 1.0f;
    return;
  }
  for (int i = 0; i < ctx->nanim; i++) {
    if (ctx->anim[i] == n) {
      return; /* already running: it just reverses direction */
    }
  }
  if (ctx->nanim < BK_MAX_ANIM) {
    if (!ctx->nanim) {
      ctx->anim_t = bk_time();
    }
    ctx->anim[ctx->nanim++] = n;
    bk_ctx_request(ctx);
  }
}

static void anim_step(BKContext *ctx)
{
  if (!ctx->nanim) {
    return;
  }
  const double now = bk_time();
  const float step = (float)((now - ctx->anim_t) / ANIM_SECONDS);
  ctx->anim_t = now;
  for (int i = ctx->nanim - 1; i >= 0; i--) {
    BKNode *n = ctx->anim[i];
    const float target = (n->flags & NF_COLLAPSED) ? 0.0f : 1.0f;
    if (n->anim < target) {
      n->anim = BK_MIN(target, n->anim + step);
    }
    else {
      n->anim = BK_MAX(target, n->anim - step);
    }
    if (n->anim == target) {
      ctx->anim[i] = ctx->anim[--ctx->nanim];
    }
  }
  g_gen++;
  ctx->layout_dirty = true;
}

/* Tab / Shift+Tab: the next editable field in tree order (Blender's
 * ui_textedit_next_but), wrapping around. */
static bool is_editable(const BKNode *n)
{
  return n->cls == &bk_cls_number || n->cls == &bk_cls_slider || n->cls == &bk_cls_text ||
         n->cls == &bk_cls_search;
}

static void collect_editable(BKNode *n, BKNode **out, int *count, int max)
{
  if (!NODE_VISIBLE(n) || *count >= max) {
    return;
  }
  if (n->cls == &bk_cls_panel && (n->flags & NF_COLLAPSED)) {
    return;
  }
  if (is_editable(n) && NODE_ENABLED(n)) {
    out[(*count)++] = n;
  }
  for (int i = 0; i < n->nkids; i++) {
    collect_editable(n->kids[i], out, count, max);
  }
}

BKNode *bk_next_editable(BKNode *from, int dir)
{
  BKNode *top = from;
  while (top->parent) {
    top = top->parent;
  }
  BKNode *list[512];
  int count = 0, at = -1;
  collect_editable(top, list, &count, 512);
  for (int i = 0; i < count; i++) {
    if (list[i] == from) {
      at = i;
    }
  }
  if (at < 0 || count < 2) {
    return NULL;
  }
  return list[(at + dir + count) % count];
}

BKNode *bk_hit_at(BKContext *ctx, int x, int y)
{
  return hit_all(ctx, x, y);
}

/** \} */

bool bk_anim_running(const BKNode *n)
{
  const BKContext *ctx = bk_ctx(n);
  if (!ctx) {
    return false;
  }
  for (int i = 0; i < ctx->nanim; i++) {
    if (ctx->anim[i] == n) {
      return true;
    }
  }
  return false;
}

void bk_edit_next(BKNode *from, int dir)
{
  BKNode *next = bk_next_editable(from, dir);
  if (next && next->cls->activate) {
    next->cls->activate(next);
  }
}
