/* Data views: UI list, tree view (outliner) and grid view (preview tiles).
 *
 * List and tree share one row-view core (rows, scrolling, hover, selection,
 * keyboard navigation, rename, scrollbar); each only describes its rows.
 * Rows draw with Blender's widget_list_itembut.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "kit_intern.h"

static int u(void)
{
  return bk_unit();
}

/* -------------------------------------------------------------------- */
/** \name Row view core
 * \{ */

typedef struct RowOps {
  int (*count)(BKNode *n);
  const char *(*text)(BKNode *n, int row);
  int (*selected)(BKNode *n);
  void (*select)(BKNode *n, int row);
  void (*rename)(BKNode *n, int row, const char *text);
  int (*press_row)(BKNode *n, int row, BKEvent *e); /* 1 = consumed (arrows, toggles) */
  void (*draw_row)(BKNode *n, int row, const BKRect *r, const uchar color[4]);
  /* multi-selection (selmode != 0) */
  bool (*marked)(BKNode *n, int row);
  void (*mark)(BKNode *n, int row, bool on);
  void (*clear_marks)(BKNode *n);
} RowOps;

static const RowOps *row_ops(const BKNode *n);

static int rv_pad(void)
{
  return bk_iround(0.15f * u());
}

static int rv_header_h(const BKNode *n)
{
  return n->shortcuts.n ? u() : 0; /* column headers ("headers" property) */
}

static int rv_top(const BKNode *n)
{
  return rv_pad() + rv_header_h(n);
}

static int rv_visible(const BKNode *n)
{
  return BK_MAX(1, (n->h - rv_pad() - rv_top(n)) / u());
}

static int rv_max_scroll(BKNode *n)
{
  return BK_MAX(0, row_ops(n)->count(n) - rv_visible(n));
}

static int rv_row_at(BKNode *n, int y)
{
  if (y < n->y + rv_top(n)) {
    return -1;
  }
  const int i = (y - n->y - rv_top(n)) / u() + n->scroll;
  return (i >= 0 && i < row_ops(n)->count(n)) ? i : -1;
}

static int rv_row_y(const BKNode *n, int row)
{
  return n->y + rv_top(n) + (row - n->scroll) * u();
}

static int rv_bar_w(BKNode *n)
{
  if (n->flags & NF_NOSCROLLBAR) {
    return 0;
  }
  return row_ops(n)->count(n) > rv_visible(n) ? bk_iround(0.3f * u()) : 0;
}

static void rv_view(BKNode *n, int axis, int *first, int *vis, int *total)
{
  if (axis == 0) { /* rows */
    *first = n->scroll;
    *vis = rv_visible(n);
    *total = row_ops(n)->count(n);
  }
}

static void rv_set_view(BKNode *n, int axis, int first)
{
  if (axis == 0) {
    n->scroll = first;
  }
}

static void rv_scroll_into_view(BKNode *n)
{
  const int i = row_ops(n)->selected(n), vis = rv_visible(n);
  if (i < 0) {
    return;
  }
  if (i < n->scroll) {
    n->scroll = i;
  }
  else if (i >= n->scroll + vis) {
    n->scroll = i - vis + 1;
  }
  bk_invalidate(n);
}

static void rv_measure(BKNode *n, int *rw, int *rh)
{
  *rw = 8 * u();
  *rh = n->rows * u() + 2 * rv_pad() + rv_header_h(n);
}

/* Column x positions for multi-column views: column 0 gets the larger share. */
static void rv_columns(const BKNode *n, int ncols, int x0, int w, int *xs)
{
  if (ncols <= 1) {
    xs[0] = x0;
    xs[1] = x0 + w;
    return;
  }
  const int first = ncols == 2 ? w / 2 : (int)(w * 0.4f);
  xs[0] = x0;
  xs[1] = x0 + first;
  for (int k = 2; k <= ncols; k++) {
    xs[k] = xs[1] + (w - first) * (k - 1) / (ncols - 1);
  }
}

static void rv_draw_headers(BKNode *n, int x0, int w)
{
  const int hh = rv_header_h(n);
  if (!hh) {
    return;
  }
  int xs[17];
  const int ncols = BK_MIN(16, n->shortcuts.n);
  rv_columns(n, ncols, x0, w, xs);
  float line[4];
  bk_rgba_alpha(bk_theme.text, 0.12f, line);
  const int y = n->y + rv_pad();
  bk_fill_f(x0, y + hh - 1, w, 1, line);
  uchar tc[4];
  memcpy(tc, bk_theme.text, 4);
  tc[3] = 170;
  for (int k = 0; k < ncols; k++) {
    BKRect r = bk_rect(xs[k], y, xs[k + 1] - xs[k], hh);
    bk_text_in(&r, n->shortcuts.v[k], tc, BK_ALIGN_LEFT, false);
    if (k) {
      bk_fill_f(xs[k], y + 3, 1, hh - 6, line);
    }
  }
}

static int rv_move(BKNode *n, BKEvent *e)
{
  const int hot = rv_row_at(n, e->y);
  if (hot != n->hot) {
    n->hot = hot;
    bk_invalidate(n);
    return 1;
  }
  return 0;
}

static int rv_leave(BKNode *n)
{
  n->hot = -1;
  return 1;
}

static bool rv_selected_row(BKNode *n, int row, int sel)
{
  return n->selmode ? row_ops(n)->marked(n, row) : row == sel;
}

/* Tk's selectmode: browse/single (0), multiple (1: click toggles), extended
 * (2: click selects one, Ctrl toggles, Shift extends from the active row). */
static void rv_click_select(BKNode *n, int row, BKEvent *e)
{
  const RowOps *ops = row_ops(n);
  if (!n->selmode) {
    ops->select(n, row);
    return;
  }
  const int active = ops->selected(n);
  if (n->selmode == 3) { /* single, shown with marks (can be cleared) */
    ops->clear_marks(n);
    ops->mark(n, row, true);
  }
  else if (n->selmode == 1 || EV_CTRL(e)) {
    ops->mark(n, row, !ops->marked(n, row));
  }
  else if (EV_SHIFT(e) && active >= 0) {
    ops->clear_marks(n);
    for (int r = BK_MIN(active, row); r <= BK_MAX(active, row); r++) {
      ops->mark(n, r, true);
    }
  }
  else {
    ops->clear_marks(n);
    ops->mark(n, row, true);
  }
  if (!EV_SHIFT(e)) {
    ops->select(n, row); /* the active row (Shift keeps the anchor) */
  }
  bk_invalidate(n);
  bk_emit(n, BK_EV_CHANGE, NULL, row, 0);
}

static int rv_press(BKNode *n, BKEvent *e)
{
  if (n->edit) {
    return 1;
  }
  if (rv_header_h(n) && e->y < n->y + rv_top(n)) { /* column heading: Tk's heading command */
    int xs[17];
    const int ncols = BK_MIN(16, n->shortcuts.n);
    rv_columns(n, ncols, n->x + rv_pad(), n->w - 2 * rv_pad() - rv_bar_w(n), xs);
    for (int k = 0; k < ncols; k++) {
      if (e->x >= xs[k] && e->x < xs[k + 1]) {
        bk_emit(n, BK_EV_CLICK, NULL, k, 0);
        break;
      }
    }
    return 1;
  }
  const RowOps *ops = row_ops(n);
  const int i = rv_row_at(n, e->y);
  if (i >= 0 && !(ops->press_row && ops->press_row(n, i, e))) {
    rv_click_select(n, i, e);
  }
  return 1;
}

static void rv_begin_rename(BKNode *n, int row)
{
  n->rename_row = row;
  bk_begin_edit(n, row_ops(n)->text(n, row), true);
}

static int rv_dbl(BKNode *n, BKEvent *e)
{
  const RowOps *ops = row_ops(n);
  const int i = rv_row_at(n, e->y);
  if (i < 0 || (ops->press_row && ops->press_row(n, i, e))) {
    return 1;
  }
  ops->select(n, i);
  rv_begin_rename(n, i);
  return 1;
}

static void rv_step(BKNode *n, int dir)
{
  const RowOps *ops = row_ops(n);
  const int count = ops->count(n);
  if (!count) {
    return;
  }
  const int i = ops->selected(n);
  ops->select(n, BK_CLAMP((i >= 0 ? i : 0) - dir, 0, count - 1));
  if (n->selmode >= 2) { /* extended / single: the keyboard moves the selection */
    ops->clear_marks(n);
    ops->mark(n, ops->selected(n), true);
    bk_emit(n, BK_EV_CHANGE, NULL, ops->selected(n), 0);
  }
  rv_scroll_into_view(n);
}

static int rv_wheel(BKNode *n, BKEvent *e)
{
  if (EV_CTRL(e)) {
    rv_step(n, e->delta > 0 ? 1 : -1);
    return 1;
  }
  const int ns = BK_CLAMP(n->scroll - e->delta, 0, rv_max_scroll(n));
  if (ns != n->scroll) {
    n->scroll = ns;
    bk_invalidate(n);
    return 1;
  }
  return 0;
}

static int rv_key(BKNode *n, BKEvent *e)
{
  if (n->edit) {
    return bk_edit_key(n, e);
  }
  const RowOps *ops = row_ops(n);
  if (bk_keysym_is(e->keysym, "Up") || bk_keysym_is(e->keysym, "Down")) {
    rv_step(n, bk_keysym_is(e->keysym, "Up") ? 1 : -1);
    return 1;
  }
  if ((bk_keysym_is(e->keysym, "Home") || bk_keysym_is(e->keysym, "End")) && ops->count(n)) {
    ops->select(n, bk_keysym_is(e->keysym, "Home") ? 0 : ops->count(n) - 1);
    rv_scroll_into_view(n);
    return 1;
  }
  if (bk_keysym_is(e->keysym, "F2") && ops->selected(n) >= 0) {
    rv_begin_rename(n, ops->selected(n));
    return 1;
  }
  return bk_value_key(n, e);
}

static void rv_finish(BKNode *n, bool confirm)
{
  const RowOps *ops = row_ops(n);
  const int i = n->rename_row;
  if (confirm && n->edit && n->edit->text[0] && i >= 0 && i < ops->count(n)) {
    ops->rename(n, i, n->edit->text);
    bk_emit(n, BK_EV_RENAME, NULL, i, 0);
  }
}

static void rv_draw(BKNode *n)
{
  const RowOps *ops = row_ops(n);
  BKRect r = bk_node_rect(n);
  BKWCol box = bk_theme.box;
  if (!NODE_ENABLED(n)) {
    box.inner[3] /= 2;
  }
  BKRect br = r;
  bk_widget_box(&box, &br, n->corners, NULL);
  const int p = rv_pad(), rh = u(), sb = rv_bar_w(n), count = ops->count(n);
  n->scroll = BK_CLAMP(n->scroll, 0, rv_max_scroll(n));
  const int sel = ops->selected(n);
  bk_clip_push(n->x + 1, n->y + 1, n->w - 2, n->h - 2);
  rv_draw_headers(n, n->x + p, n->w - 2 * p - sb);
  for (int i = n->scroll; i < count; i++) {
    const int y = rv_row_y(n, i);
    if (y >= n->y + n->h) {
      break;
    }
    BKRect row = bk_rect(n->x + p, y, n->w - 2 * p - sb, rh);
    int st = (rv_selected_row(n, i, sel) ? ST_SELECT : 0) |
             (i == n->hot && bk_hovered(n) ? ST_HOVER : 0);
    BKWCol w;
    bk_widget_state(&w, &bk_theme.list_item, st & ST_SELECT);
    BKRect wr = row;
    bk_widget_list_itembut(&w, &wr, st);
    if (n->edit && i == n->rename_row) {
      bk_te_draw(n->edit, &row, bk_theme.list_item.text);
    }
    else {
      uchar tc[4];
      memcpy(tc, (st & ST_SELECT) ? bk_theme.list_item.text_sel : bk_theme.list_item.text, 4);
      if (!NODE_ENABLED(n)) {
        tc[3] /= 2;
      }
      ops->draw_row(n, i, &row, tc);
    }
  }
  bk_clip_pop();
  if (sb) { /* thin Blender scrollbar */
    const int top = rv_top(n);
    const int track_h = n->h - p - top, vis = rv_visible(n);
    const float th = BK_MAX(u() * 0.6f, (float)track_h * vis / count);
    const float ty = n->y + top + (track_h - th) * n->scroll / (float)BK_MAX(1, count - vis);
    float c[4];
    bk_rgba(bk_theme.scroll.item, c);
    bk_roundbox(n->x + n->w - sb - p * 0.5f, ty, sb * 0.6f, th, sb * 0.3f, CNR_ALL, c);
  }
}

static int rv_cursor(BKNode *n, int x, int y)
{
  return n->edit ? BK_CURSOR_TEXT : BK_CURSOR_DEFAULT;
}

#define ROWVIEW_CLASS_COMMON \
  .flags = CLS_INTERACTIVE | CLS_VALUE, .measure = rv_measure, .draw = rv_draw, \
  .press = rv_press, .dbl = rv_dbl, .move = rv_move, .leave = rv_leave, .key = rv_key, \
  .wheel = rv_wheel, .finish_edit = rv_finish, .cursor_at = rv_cursor, .step = rv_step, \
  .view = rv_view, .set_view = rv_set_view

/** \} */

/* -------------------------------------------------------------------- */
/** \name ListView
 * \{ */

static int list_count(BKNode *n)
{
  return n->items.n;
}
static const char *list_text(BKNode *n, int row)
{
  return (row >= 0 && row < n->items.n) ? n->items.v[row] : "";
}
static int list_selected(BKNode *n)
{
  return n->items.n ? (int)n->value : -1;
}
static void list_select(BKNode *n, int row)
{
  bk_set_value(n, row);
}
static void list_rename(BKNode *n, int row, const char *text)
{
  free(n->items.v[row]);
  n->items.v[row] = bk_strdup(text);
}
static void list_draw_row(BKNode *n, int row, const BKRect *r, const uchar color[4])
{
  const int icon = (n->item_icons && row < n->item_icons_s.n) ? n->item_icons[row] : 0;
  bk_text_icon(r, n->items.v[row], icon, color, BK_ALIGN_LEFT, NODE_ENABLED(n));
}

static bool list_marked(BKNode *n, int row)
{
  return row >= 0 && row < n->nmarks && n->marks[row];
}

static void list_mark(BKNode *n, int row, bool on)
{
  if (row < 0) {
    return;
  }
  if (row >= n->nmarks) {
    const int count = BK_MAX(row + 1, n->items.n);
    n->marks = (unsigned char *)realloc(n->marks, (size_t)count);
    memset(n->marks + n->nmarks, 0, (size_t)(count - n->nmarks));
    n->nmarks = count;
  }
  n->marks[row] = on;
  bk_invalidate(n);
}

static void list_clear_marks(BKNode *n)
{
  if (n->marks) {
    memset(n->marks, 0, (size_t)n->nmarks);
  }
  bk_invalidate(n);
}

static int list_get_num(BKNode *n, const char *key, double *r_v)
{
  if (strncmp(key, "mark", 4) == 0 && key[4] >= '0' && key[4] <= '9') { /* "mark12" */
    *r_v = list_marked(n, atoi(key + 4));
    return 1;
  }
  return 0;
}

static int list_set_num(BKNode *n, const char *key, double v)
{
  if (strncmp(key, "mark", 4) == 0 && key[4] >= '0' && key[4] <= '9') {
    list_mark(n, atoi(key + 4), v != 0);
    return 1;
  }
  if (strcmp(key, "clearmarks") == 0) {
    list_clear_marks(n);
    return 1;
  }
  return 0;
}

static const RowOps g_list_ops = {list_count,  list_text,       list_selected,
                                  list_select, list_rename,     NULL,
                                  list_draw_row, list_marked,   list_mark,
                                  list_clear_marks};

static int list_to_text(BKNode *n, char *buf, int size)
{
  snprintf(buf, (size_t)size, "%s", list_text(n, (int)n->value));
  return 1;
}

static void list_init(BKNode *n)
{
  n->flags |= NF_HAS_MIN | NF_INTEGER;
  n->vmin = 0;
}

const BKClass bk_cls_list = {.name = "list",
                             ROWVIEW_CLASS_COMMON,
                             .init = list_init,
                             .to_text = list_to_text,
                             .set_num = list_set_num,
                             .get_num = list_get_num};

/** \} */

/* -------------------------------------------------------------------- */
/** \name TreeView
 * \{ */

struct BKTreeItem {
  BKStrs values; /* extra columns */
  char *label;
  int icon;
  bool expanded;
  bool marked; /* multi-selection */
  unsigned toggles; /* bit per toggle column */
  BKTreeItem *parent;
  BKTreeItem **kids;
  int nkids, cap;
  void *ref;
};

typedef struct TreeData {
  BKTreeItem **roots;
  int nroots;
  BKTreeItem *selected;
  int columns[8]; /* toggle columns: icon pairs (off, on) */
  int ncolumns;
  /* flattened rows, rebuilt on demand */
  BKTreeItem **rows;
  int *depth;
  int nrows, caprows;
  bool flat; /* no item has children: rows line up with the headers (a table) */
  int toggle_col, toggle_value; /* drag along a toggle column */
} TreeData;

#define TREE(n) ((TreeData *)(n)->ext)

BKTreeItem *BK_tree_item_new(const char *label, int icon)
{
  BKTreeItem *it = (BKTreeItem *)calloc(1, sizeof(BKTreeItem));
  it->label = bk_strdup(label ? label : "");
  it->icon = icon;
  it->expanded = true;
  return it;
}

void BK_tree_item_free(BKTreeItem *it)
{
  if (!it) {
    return;
  }
  for (int i = 0; i < it->nkids; i++) {
    it->kids[i]->parent = NULL;
    if (it->kids[i]->ref) {
      bk_release(it->kids[i]->ref);
    }
    else {
      BK_tree_item_free(it->kids[i]);
    }
  }
  free(it->kids);
  free(it->label);
  bk_strs_free(&it->values);
  free(it);
}

void BK_tree_item_set_values(BKTreeItem *it, const char *const *values, int n)
{
  bk_strs_set(&it->values, values, n);
}

void BK_tree_item_set_ref(BKTreeItem *it, void *ref)
{
  it->ref = ref;
}
void *BK_tree_item_ref(const BKTreeItem *it)
{
  return it->ref;
}

void BK_tree_item_clear(BKTreeItem *parent)
{
  for (int i = 0; i < parent->nkids; i++) {
    BKTreeItem *kid = parent->kids[i];
    kid->parent = NULL;
    if (kid->ref) {
      bk_release(kid->ref);
    }
  }
  parent->nkids = 0;
}

void BK_tree_item_add(BKTreeItem *parent, BKTreeItem *child)
{
  if (parent->nkids == parent->cap) {
    parent->cap = parent->cap ? parent->cap * 2 : 4;
    parent->kids = (BKTreeItem **)realloc(parent->kids, sizeof(BKTreeItem *) * (size_t)parent->cap);
  }
  parent->kids[parent->nkids++] = child;
  child->parent = parent;
  bk_retain(child->ref);
}

void BK_tree_item_set(BKTreeItem *it, const char *key, double v)
{
  if (strcmp(key, "expanded") == 0) {
    it->expanded = v != 0;
  }
  else if (strcmp(key, "marked") == 0) {
    it->marked = v != 0;
  }
  else if (strcmp(key, "icon") == 0) {
    it->icon = (int)v;
  }
  else if (strncmp(key, "toggle", 6) == 0) {
    const int bit = atoi(key + 6);
    if (v != 0) {
      it->toggles |= 1u << bit;
    }
    else {
      it->toggles &= ~(1u << bit);
    }
  }
}

double BK_tree_item_get(const BKTreeItem *it, const char *key)
{
  if (strcmp(key, "expanded") == 0) {
    return it->expanded;
  }
  if (strcmp(key, "marked") == 0) {
    return it->marked;
  }
  if (strcmp(key, "icon") == 0) {
    return it->icon;
  }
  if (strncmp(key, "toggle", 6) == 0) {
    return (it->toggles >> atoi(key + 6)) & 1u;
  }
  return 0;
}

void BK_tree_item_set_label(BKTreeItem *it, const char *label)
{
  bk_set_text(&it->label, label);
}
const char *BK_tree_item_label(const BKTreeItem *it)
{
  return it->label;
}
int BK_tree_item_child_count(const BKTreeItem *it)
{
  return it->nkids;
}
BKTreeItem *BK_tree_item_child(const BKTreeItem *it, int i)
{
  return (i >= 0 && i < it->nkids) ? it->kids[i] : NULL;
}

static void tree_add_rows(TreeData *t, BKTreeItem *it, int depth)
{
  if (t->nrows == t->caprows) {
    t->caprows = t->caprows ? t->caprows * 2 : 32;
    t->rows = (BKTreeItem **)realloc(t->rows, sizeof(BKTreeItem *) * (size_t)t->caprows);
    t->depth = (int *)realloc(t->depth, sizeof(int) * (size_t)t->caprows);
  }
  t->rows[t->nrows] = it;
  t->depth[t->nrows] = depth;
  t->nrows++;
  if (it->expanded) {
    for (int i = 0; i < it->nkids; i++) {
      tree_add_rows(t, it->kids[i], depth + 1);
    }
  }
}

static void tree_flatten(BKNode *n)
{
  TreeData *t = TREE(n);
  t->nrows = 0;
  t->flat = true;
  for (int i = 0; i < t->nroots; i++) {
    tree_add_rows(t, t->roots[i], 0);
    t->flat = t->flat && t->roots[i]->nkids == 0;
  }
}

static void tree_init(BKNode *n)
{
  n->ext = calloc(1, sizeof(TreeData));
  n->rows = 8;
}

static void tree_free(BKNode *n)
{
  TreeData *t = TREE(n);
  for (int i = 0; i < t->nroots; i++) {
    if (t->roots[i]->ref) {
      bk_release(t->roots[i]->ref);
    }
    else {
      BK_tree_item_free(t->roots[i]);
    }
  }
  free(t->roots);
  free(t->rows);
  free(t->depth);
  free(t);
}

void BK_tree_set_roots(BKNode *n, BKTreeItem *const *roots, int count)
{
  TreeData *t = TREE(n);
  for (int i = 0; i < count; i++) {
    bk_retain(roots[i]->ref);
  }
  for (int i = 0; i < t->nroots; i++) {
    if (t->roots[i]->ref) {
      bk_release(t->roots[i]->ref);
    }
  }
  free(t->roots);
  t->roots = (BKTreeItem **)malloc(sizeof(BKTreeItem *) * (size_t)BK_MAX(1, count));
  memcpy(t->roots, roots, sizeof(BKTreeItem *) * (size_t)count);
  t->nroots = count;
  t->selected = NULL;
  tree_flatten(n);
  bk_invalidate(n);
}

static int tree_count(BKNode *n)
{
  tree_flatten(n);
  return TREE(n)->nrows;
}

static const char *tree_text(BKNode *n, int row)
{
  tree_flatten(n);
  return (row >= 0 && row < TREE(n)->nrows) ? TREE(n)->rows[row]->label : "";
}

static int tree_selected(BKNode *n)
{
  TreeData *t = TREE(n);
  tree_flatten(n);
  for (int i = 0; i < t->nrows; i++) {
    if (t->rows[i] == t->selected) {
      return i;
    }
  }
  return -1;
}

static void tree_select(BKNode *n, int row)
{
  TreeData *t = TREE(n);
  tree_flatten(n);
  if (row >= 0 && row < t->nrows && t->rows[row] != t->selected) {
    t->selected = t->rows[row];
    bk_invalidate(n);
    bk_emit(n, BK_EV_CHANGE, t->selected->ref, row, 0);
  }
}

static void tree_rename(BKNode *n, int row, const char *text)
{
  bk_set_text(&TREE(n)->rows[row]->label, text);
}

static int tree_indent(void)
{
  return bk_iround(0.9f * u());
}

static int tree_col_x(BKNode *n, int k)
{
  TreeData *t = TREE(n);
  const int right = n->x + n->w - rv_pad() - rv_bar_w(n);
  return right - (t->ncolumns - k) * u();
}

static void tree_set_expanded(BKTreeItem *it, bool expanded, bool recursive)
{
  it->expanded = expanded;
  if (recursive) {
    for (int i = 0; i < it->nkids; i++) {
      tree_set_expanded(it->kids[i], expanded, true);
    }
  }
}

static void tree_set_toggle(BKNode *n, BKTreeItem *it, int col, int value)
{
  const unsigned bit = 1u << col;
  if (((it->toggles & bit) != 0) != (value != 0)) {
    it->toggles = value ? (it->toggles | bit) : (it->toggles & ~bit);
    bk_invalidate(n);
    bk_emit(n, BK_EV_TOGGLE, it->ref, col, value);
  }
}

/* User expand/collapse: tell the host (Tk's <<TreeviewOpen>> / <<TreeviewClose>>). */
static void tree_open(BKNode *n, BKTreeItem *it, bool expanded, bool recursive)
{
  const bool was = it->expanded;
  tree_set_expanded(it, expanded, recursive);
  if (was != expanded) {
    bk_emit(n, BK_EV_TOGGLE, it->ref, -1, expanded);
  }
}

static int tree_press_row(BKNode *n, int row, BKEvent *e)
{
  TreeData *t = TREE(n);
  BKTreeItem *it = t->rows[row];
  const int ind = tree_indent();
  const int ax = n->x + rv_pad() + t->depth[row] * ind;
  if (it->nkids && e->x >= ax && e->x < ax + ind) {
    tree_open(n, it, !it->expanded, EV_SHIFT(e)); /* Shift: recursively */
    bk_invalidate(n);
    return 1;
  }
  for (int k = 0; k < t->ncolumns; k++) {
    const int cx = tree_col_x(n, k);
    if (e->x >= cx && e->x < cx + u()) {
      const int value = !((it->toggles >> k) & 1u);
      tree_set_toggle(n, it, k, value);
      t->toggle_col = k + 1;
      t->toggle_value = value;
      return 1;
    }
  }
  return 0;
}

static void tree_draw_row(BKNode *n, int row, const BKRect *r, const uchar color[4])
{
  TreeData *t = TREE(n);
  BKTreeItem *it = t->rows[row];
  const int depth = t->depth[row], ind = tree_indent();
  float line[4];
  bk_rgba_alpha(bk_theme.text, 0.08f, line);
  for (int d = 0; d < depth; d++) { /* faint hierarchy lines */
    bk_fill_f(r->xmin + d * ind + ind * 0.5f, r->ymin, 1, bk_rh(r), line);
  }
  const int ax = r->xmin + depth * ind;
  if (it->nkids) {
    float c[4];
    bk_rgba_alpha(color, 0.75f, c);
    bk_tria(ax + ind * 0.5f, r->ymin + bk_rh(r) * 0.5f, it->expanded ? 'v' : 'h', c,
            0.15f * u());
  }
  const int right = t->ncolumns ? tree_col_x(n, 0) : r->xmax;
  int xs[17];
  const int ncols = BK_MIN(16, BK_MAX(n->shortcuts.n, it->values.n + 1));
  rv_columns(n, ncols, r->xmin, right - r->xmin, xs);
  BKRect lr = {t->flat ? r->xmin : ax + ind - bk_pad() / 2, r->ymin, xs[1], r->ymax};
  bk_text_icon(&lr, it->label, it->icon, color, BK_ALIGN_LEFT, NODE_ENABLED(n));
  for (int k = 1; k < ncols && k - 1 < it->values.n; k++) {
    BKRect cr = bk_rect(xs[k], r->ymin, xs[k + 1] - xs[k], bk_rh(r));
    bk_text_in(&cr, it->values.v[k - 1], color, BK_ALIGN_LEFT, false);
  }
  for (int k = 0; k < t->ncolumns; k++) {
    const int on = (it->toggles >> k) & 1u;
    const int icon = t->columns[k * 2 + on];
    uchar ic[4];
    memcpy(ic, bk_theme.text, 4);
    ic[3] = (uchar)(on ? 115 : 217);
    bk_icon(icon, tree_col_x(n, k) + u() * 0.5f, r->ymin + bk_rh(r) * 0.5f, ic, 1.0f, 0);
  }
}

static bool tree_marked(BKNode *n, int row)
{
  tree_flatten(n);
  return row >= 0 && row < TREE(n)->nrows && TREE(n)->rows[row]->marked;
}

static void tree_mark(BKNode *n, int row, bool on)
{
  tree_flatten(n);
  if (row >= 0 && row < TREE(n)->nrows) {
    TREE(n)->rows[row]->marked = on;
    bk_invalidate(n);
  }
}

static void tree_unmark(BKTreeItem *it)
{
  it->marked = false;
  for (int i = 0; i < it->nkids; i++) {
    tree_unmark(it->kids[i]);
  }
}

static void tree_clear_marks(BKNode *n)
{
  for (int i = 0; i < TREE(n)->nroots; i++) {
    tree_unmark(TREE(n)->roots[i]);
  }
  bk_invalidate(n);
}

static const RowOps g_tree_ops = {tree_count,    tree_text,   tree_selected,   tree_select,
                                  tree_rename,   tree_press_row, tree_draw_row, tree_marked,
                                  tree_mark,     tree_clear_marks};

static int tree_drag(BKNode *n, BKEvent *e)
{
  TreeData *t = TREE(n);
  if (t->toggle_col) { /* drag down a toggle column (outliner) */
    const int row = rv_row_at(n, e->y);
    if (row >= 0) {
      tree_set_toggle(n, t->rows[row], t->toggle_col - 1, t->toggle_value);
    }
    return 1;
  }
  return 0;
}

static int tree_release(BKNode *n, BKEvent *e)
{
  TREE(n)->toggle_col = 0;
  return 1;
}

static int tree_key(BKNode *n, BKEvent *e)
{
  TreeData *t = TREE(n);
  if (!n->edit && t->selected &&
      (bk_keysym_is(e->keysym, "Left") || bk_keysym_is(e->keysym, "Right")))
  {
    BKTreeItem *it = t->selected;
    if (bk_keysym_is(e->keysym, "Right") && it->nkids) {
      tree_open(n, it, true, EV_SHIFT(e));
    }
    else if (bk_keysym_is(e->keysym, "Left")) {
      if (it->expanded && it->nkids) {
        tree_open(n, it, false, EV_SHIFT(e));
      }
      else if (it->parent) {
        t->selected = it->parent;
        bk_emit(n, BK_EV_CHANGE, it->parent->ref, tree_selected(n), 0);
        rv_scroll_into_view(n);
      }
    }
    bk_invalidate(n);
    return 1;
  }
  return rv_key(n, e);
}

static void tree_expand_all(void *data)
{
  BKNode *n = (BKNode *)data;
  if (TREE(n)->selected) {
    tree_open(n, TREE(n)->selected, true, true);
    bk_invalidate(n);
  }
}

static void tree_collapse_all(void *data)
{
  BKNode *n = (BKNode *)data;
  if (TREE(n)->selected) {
    tree_open(n, TREE(n)->selected, false, true);
    bk_invalidate(n);
  }
}

static void tree_rename_selected(void *data)
{
  BKNode *n = (BKNode *)data;
  const int row = tree_selected(n);
  if (row >= 0) {
    rv_begin_rename(n, row);
  }
}

static int tree_context(BKNode *n, BKMenuSpec *items, int max)
{
  if (!TREE(n)->selected || max < 4) {
    return 0;
  }
  memset(items, 0, sizeof(BKMenuSpec) * 4);
  items[0] = (BKMenuSpec){"Expand All", 0, NULL, -1, 1};
  items[0].fn = tree_expand_all;
  items[1] = (BKMenuSpec){"Collapse All", 0, NULL, -1, 1};
  items[1].fn = tree_collapse_all;
  items[2] = (BKMenuSpec){"-", 0, NULL, -1, 1};
  items[3] = (BKMenuSpec){"Rename", 0, "F2", -1, 1};
  items[3].fn = tree_rename_selected;
  for (int i = 0; i < 4; i++) {
    items[i].data = n;
  }
  return 4;
}

static int tree_to_text(BKNode *n, char *buf, int size)
{
  snprintf(buf, (size_t)size, "%s", TREE(n)->selected ? TREE(n)->selected->label : "");
  return 1;
}

static void tree_reset(BKNode *n, bool zero)
{
}

const BKClass bk_cls_tree = {.name = "tree",
                             ROWVIEW_CLASS_COMMON,
                             .init = tree_init,
                             .free_ext = tree_free,
                             .drag = tree_drag,
                             .release = tree_release,
                             .key = tree_key,
                             .context_items = tree_context,
                             .to_text = tree_to_text,
                             .reset = tree_reset};

BKTreeItem *BK_tree_selected(BKNode *n)
{
  return TREE(n)->selected;
}

void BK_tree_select(BKNode *n, BKTreeItem *it)
{
  TREE(n)->selected = it;
  bk_invalidate(n);
}

int BK_tree_row_count(BKNode *n)
{
  return tree_count(n);
}

BKTreeItem *BK_tree_row(BKNode *n, int row)
{
  tree_flatten(n);
  return (row >= 0 && row < TREE(n)->nrows) ? TREE(n)->rows[row] : NULL;
}

void BK_tree_row_rect(BKNode *n, int row, int r[4])
{
  r[0] = n->x + rv_pad();
  r[1] = rv_row_y(n, row);
  r[2] = n->w - 2 * rv_pad() - rv_bar_w(n);
  r[3] = u();
}

/* Toggle columns are configured through the "icons" strings property:
 * pairs of (off, on) icon names, e.g. HIDE_OFF, HIDE_ON. */
int bk_tree_columns_from_icons(BKNode *n)
{
  TreeData *t = TREE(n);
  t->ncolumns = BK_MIN(4, n->item_icons_s.n / 2);
  for (int i = 0; i < t->ncolumns * 2; i++) {
    t->columns[i] = n->item_icons[i];
  }
  return t->ncolumns;
}

/** \} */

static const RowOps *row_ops(const BKNode *n)
{
  return n->cls == &bk_cls_tree ? &g_tree_ops : &g_list_ops;
}

/* -------------------------------------------------------------------- */
/** \name GridView (preview tiles)
 * \{ */

typedef struct GridData {
  float (*colors)[4]; /* per tile preview color (alpha 0 = icon) */
  int ncolors;
} GridData;

static int tile_px(const BKNode *n)
{
  return bk_iround((n->tile > 0 ? n->tile : 4.0f) * u());
}

static int grid_cols(const BKNode *n)
{
  return BK_MAX(1, (n->w - bk_pad()) / (tile_px(n) + bk_pad()));
}

static BKRect grid_tile(const BKNode *n, int i)
{
  const int t = tile_px(n), p = bk_pad(), cols = grid_cols(n);
  const float gap = (n->w - cols * t) / (float)(cols + 1);
  const int r = i / cols, c = i % cols;
  return bk_rect(bk_iround(n->x + gap + c * (t + gap)), n->y + p + (r - n->scroll) * (t + p), t,
                 t);
}

static int grid_index_at(const BKNode *n, int x, int y)
{
  for (int i = 0; i < n->items.n; i++) {
    BKRect r = grid_tile(n, i);
    if (x >= r.xmin && x < r.xmax && y >= r.ymin && y < r.ymax) {
      return i;
    }
  }
  return -1;
}

static int grid_visible_rows(const BKNode *n)
{
  return BK_MAX(1, n->h / (tile_px(n) + bk_pad()));
}

static int grid_row_count(const BKNode *n)
{
  return (n->items.n + grid_cols(n) - 1) / grid_cols(n);
}

static void gridview_init(BKNode *n)
{
  n->ext = calloc(1, sizeof(GridData));
  n->tile = 4.0f;
  n->height_units = 2.0f; /* rows */
  n->flags |= NF_HAS_MIN | NF_INTEGER;
}

static void gridview_free(BKNode *n)
{
  GridData *g = (GridData *)n->ext;
  free(g->colors);
  free(g);
}

void BK_gridview_set_colors(BKNode *n, const float (*colors)[4], int count)
{
  GridData *g = (GridData *)n->ext;
  free(g->colors);
  g->colors = (float(*)[4])calloc((size_t)BK_MAX(1, count), sizeof(float[4]));
  memcpy(g->colors, colors, sizeof(float[4]) * (size_t)count);
  g->ncolors = count;
  bk_invalidate(n);
}

static void gridview_measure(BKNode *n, int *rw, int *rh)
{
  const int t = tile_px(n), p = bk_pad();
  *rw = t * 2 + 3 * p;
  *rh = (int)(n->height_units * (t + p) + p);
}

static int gridview_move(BKNode *n, BKEvent *e)
{
  const int hot = grid_index_at(n, e->x, e->y);
  if (hot != n->hot) {
    n->hot = hot;
    bk_invalidate(n);
    return 1;
  }
  return 0;
}

static int gridview_press(BKNode *n, BKEvent *e)
{
  const int i = grid_index_at(n, e->x, e->y);
  if (i >= 0) {
    bk_set_value(n, i);
  }
  return 1;
}

static void gridview_activate(BKNode *n)
{
  bk_emit(n, BK_EV_ACTIVATE, NULL, n->value, 0);
}

static int gridview_dbl(BKNode *n, BKEvent *e)
{
  const int i = grid_index_at(n, e->x, e->y);
  if (i >= 0) {
    bk_set_value(n, i);
    gridview_activate(n);
  }
  return 1;
}

static int gridview_wheel(BKNode *n, BKEvent *e)
{
  if (EV_CTRL(e)) {
    bk_set_value(n, BK_CLAMP((int)n->value - (e->delta > 0 ? 1 : -1), 0, n->items.n - 1));
    return 1;
  }
  const int ns = BK_CLAMP(n->scroll - e->delta, 0, BK_MAX(0, grid_row_count(n) - grid_visible_rows(n)));
  if (ns != n->scroll) {
    n->scroll = ns;
    bk_invalidate(n);
    return 1;
  }
  return 0;
}

static int gridview_key(BKNode *n, BKEvent *e)
{
  int move = 0;
  if (bk_keysym_is(e->keysym, "Left")) {
    move = -1;
  }
  else if (bk_keysym_is(e->keysym, "Right")) {
    move = 1;
  }
  else if (bk_keysym_is(e->keysym, "Up")) {
    move = -grid_cols(n);
  }
  else if (bk_keysym_is(e->keysym, "Down")) {
    move = grid_cols(n);
  }
  if (move && n->items.n) {
    bk_set_value(n, BK_CLAMP((int)n->value + move, 0, n->items.n - 1));
    const int r = (int)n->value / grid_cols(n), vis = grid_visible_rows(n);
    n->scroll = BK_MIN(BK_MAX(n->scroll, r - vis + 1), r);
    return 1;
  }
  return bk_value_key(n, e);
}

static void gridview_draw(BKNode *n)
{
  GridData *g = (GridData *)n->ext;
  BKRect r = bk_node_rect(n);
  BKWCol box = bk_theme.box;
  bk_widget_box(&box, &r, n->corners, NULL);
  bk_clip_push(n->x + 1, n->y + 1, n->w - 2, n->h - 2);
  const float uu = (float)u();
  for (int i = 0; i < n->items.n; i++) {
    BKRect t = grid_tile(n, i);
    if (t.ymax < n->y || t.ymin > n->y + n->h) {
      continue;
    }
    const bool sel = i == (int)n->value, hot = i == n->hot && bk_hovered(n);
    BKWCol w;
    bk_widget_state(&w, &bk_theme.list_item, sel ? ST_SELECT : 0);
    if (!sel) {
      /* preview tiles keep a subtle background (BUT_ICON_PREVIEW) */
      memcpy(w.inner, bk_theme.text, 3);
      w.inner[3] = hot ? 28 : 11;
    }
    BKRect tr = t;
    bk_widget_list_itembut(&w, &tr, 0);
    const float pad = 0.35f * uu;
    const float pw = bk_rw(&t) - 2 * pad, ph = bk_rh(&t) - 2 * pad - uu;
    if (i < g->ncolors && g->colors[i][3] > 0) {
      bk_roundbox(t.xmin + pad, t.ymin + pad, pw, ph, 0.2f * uu, CNR_ALL, g->colors[i]);
    }
    else {
      const int icon = (n->item_icons && i < n->item_icons_s.n) ? n->item_icons[i] : 0;
      bk_icon(icon, t.xmin + bk_rw(&t) * 0.5f, t.ymin + pad + ph * 0.5f, NULL, 1.0f,
              BK_MIN(pw, ph) * 0.6f);
    }
    BKRect lr = bk_rect(t.xmin, t.ymax - (int)(1.15f * uu), bk_rw(&t), (int)uu);
    bk_text_in(&lr, n->items.v[i], sel ? bk_theme.list_item.text_sel : bk_theme.list_item.text,
               BK_ALIGN_CENTER, false);
  }
  bk_clip_pop();
}

static int gridview_to_text(BKNode *n, char *buf, int size)
{
  snprintf(buf, (size_t)size, "%s",
           ((int)n->value >= 0 && (int)n->value < n->items.n) ? n->items.v[(int)n->value] : "");
  return 1;
}

static void gridview_step(BKNode *n, int dir)
{
  bk_set_value(n, BK_CLAMP((int)n->value - dir, 0, BK_MAX(0, n->items.n - 1)));
}

const BKClass bk_cls_gridview = {.name = "gridview",
                                 .flags = CLS_INTERACTIVE | CLS_VALUE,
                                 .init = gridview_init,
                                 .free_ext = gridview_free,
                                 .measure = gridview_measure,
                                 .draw = gridview_draw,
                                 .press = gridview_press,
                                 .dbl = gridview_dbl,
                                 .move = gridview_move,
                                 .leave = rv_leave,
                                 .key = gridview_key,
                                 .wheel = gridview_wheel,
                                 .to_text = gridview_to_text,
                                 .step = gridview_step,
                                 .activate = gridview_activate};

/** \} */
