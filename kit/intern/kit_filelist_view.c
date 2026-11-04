/* File list view: the main region in vertical (list) display (file_draw.cc file_draw_list,
 * filesel.cc ED_fileselect_init_layout FILE_VERTICALDISPLAY) and its selection (file_ops.cc
 * file_select, file_walk_select).
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <math.h>
#include <stdio.h>
#include <string.h>

#include "kit_filebrowser_intern.h"

static int u(void)
{
  return bk_unit();
}

/* -------------------------------------------------------------------- */
/** \name Layout
 * \{ */

/* The attribute columns: one table for the header and the rows. */
static const struct {
  const char *name;
  FBSort sort;
  int align;
} g_columns[FB_COL_NUM] = {
    {"Name", FB_SORT_ALPHA, BK_ALIGN_LEFT},
    {"Date Modified", FB_SORT_TIME, BK_ALIGN_LEFT},
    {"Size", FB_SORT_SIZE, BK_ALIGN_RIGHT},
};

typedef struct FBLayout {
  int border_x, border_y, pad_top, tile_w, tile_h, header_h, item_h, prv_w, col_pad;
  int col_w[FB_COL_NUM];
} FBLayout;

/* ED_fileselect_init_layout (FILE_VERTICALDISPLAY) + file_attribute_columns_widths */
static FBLayout list_layout(const BKNode *n)
{
  FBLayout l;
  l.border_x = (int)(0.4f * u());
  l.border_y = (int)(0.05f * u());
  l.pad_top = 2 * l.border_y;
  l.tile_h = bk_iround(ceilf(bk_font_size() * 1.25f) * 1.4f); /* fontstyle_height_max * 1.4 */
  l.tile_w = n->w - 2 * l.border_x;
  l.header_h = (int)(l.tile_h * 1.2f) + 2 * l.border_y; /* attribute_column_header_h */
  l.item_h = l.tile_h + 2 * l.border_y;
  l.prv_w = (int)(u() * 0.8f);   /* ICON_DEFAULT_WIDTH_SCALE */
  l.col_pad = (int)(0.5f * u()); /* ATTRIBUTE_COLUMN_PADDING */
  l.col_w[FB_COL_DATE] = (int)(bk_text_width("30 Sep 2099 03:59", -1, false) + 2 * l.col_pad +
                               0.5f * u());
  l.col_w[FB_COL_SIZE] = (int)(bk_text_width("098.7 MiB", -1, false) + 2 * l.col_pad);
  l.col_w[FB_COL_NAME] = l.tile_w - l.col_w[FB_COL_DATE] - l.col_w[FB_COL_SIZE];
  return l;
}

static int column_x(const BKNode *n, const FBLayout *l, int c)
{
  int x = n->x;
  for (int k = 0; k < c; k++) {
    x += l->col_w[k];
  }
  return x;
}

static int column_at(const BKNode *n, const FBLayout *l, int x)
{
  for (int c = 0; c < FB_COL_NUM; c++) {
    if (x >= column_x(n, l, c) && x < column_x(n, l, c + 1)) {
      return c;
    }
  }
  return -1;
}

static int content_h(const BKNode *n, const FBLayout *l)
{
  return FB(n)->nvis * l->item_h + 2 * l->pad_top;
}

static int view_h(const BKNode *n, const FBLayout *l)
{
  return BK_MAX(0, n->h - l->header_h);
}

static int max_scroll(const BKNode *n, const FBLayout *l)
{
  return BK_MAX(0, content_h(n, l) - view_h(n, l));
}

static BKRect view_rect(const BKNode *n, const FBLayout *l)
{
  return bk_rect(n->x, n->y + l->header_h, n->w, view_h(n, l));
}

/* tile_draw_rect_get (screen space, y down) */
static BKRect tile_rect(const BKNode *n, const FBLayout *l, int i)
{
  const int y = n->y + l->header_h + l->pad_top + l->border_y + i * l->item_h - n->scroll;
  return bk_rect(n->x + l->border_x, y, l->tile_w, l->tile_h);
}

static int row_at(const BKNode *n, const FBLayout *l, int y)
{
  const int rel = y - (n->y + l->header_h + l->pad_top) + n->scroll;
  const int i = (y >= n->y + l->header_h && rel >= 0) ? rel / l->item_h : -1;
  return i < FB(n)->nvis ? i : -1;
}

static bool has_scroller(const BKNode *n, const FBLayout *l)
{
  return content_h(n, l) > view_h(n, l);
}

static void scroller(const BKNode *n, const FBLayout *l, BKRect *r_track, BKRect *r_thumb)
{
  const bool wide = FB(n)->scroller_hot || n->dragmode == 1;
  const BKRect area = view_rect(n, l);
  bk_scroller_rects(&area, n->scroll, content_h(n, l), wide ? 255 : 0, r_track, r_thumb);
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Drawing
 * \{ */

/* file_draw_string with text_clip_middle_ex: long names lose their middle. */
static void draw_name(const BKRect *rect, const char *s, const uchar col[4])
{
  static const char ell[] = "\xe2\x80\xa6"; /* BLI_STR_UTF8_HORIZONTAL_ELLIPSIS */
  const int len = (int)strlen(s);
  char buf[FB_PATH];
  snprintf(buf, sizeof(buf), "%s", s);
  for (int keep = len - 1; keep >= 0 && bk_text_width(buf, -1, false) > bk_rw(rect); keep--) {
    int l = keep / 2, rs = len - (keep - keep / 2);
    while (l > 0 && (s[l] & 0xC0) == 0x80) { /* stay on UTF-8 boundaries */
      l--;
    }
    while (rs < len && (s[rs] & 0xC0) == 0x80) {
      rs++;
    }
    snprintf(buf, sizeof(buf), "%.*s%s%s", l, s, ell, s + rs);
  }
  bk_text_in_pad(rect, buf, col, BK_ALIGN_LEFT, false, 0);
}

/* draw_tile_background */
static void draw_tile_background(const BKRect *r, bool selected, bool highlighted)
{
  const BKWCol *w = &bk_theme.list_item;
  float bg[4], outline[4];
  bk_rgba(selected ? w->inner_sel : w->inner, bg);
  bk_rgba(selected ? w->outline_sel : w->outline, outline);
  bg[3] = highlighted ? BK_MIN(bg[3] + 0.1f, 1.0f) : bg[3]; /* hover: at least 10% */
  const float rad = w->roundness * u();
  if (outline[3] > 0.0f) {
    bk_roundbox(r->xmin, r->ymin, bk_rw(r), bk_rh(r), rad, CNR_ALL, outline);
  }
  if (bg[3] > 0.0f) {
    const int in = outline[3] > 0.0f ? 1 : 0;
    bk_roundbox(r->xmin + in, r->ymin + in, bk_rw(r) - 2 * in, bk_rh(r) - 2 * in,
                BK_MAX(0.0f, rad - in), CNR_ALL, bg);
  }
}

/* draw_background: alternating rows, TH_BACK blended with TH_ROW_ALTERNATE. */
static void draw_background(const BKNode *n, const FBLayout *l, int first, int rows)
{
  uchar alt[4];
  bk_color_over(bk_theme.file_row_alternate, bk_theme.file_back, alt);
  bk_fill(n->x, n->y, n->w, n->h, bk_theme.file_back);
  for (int i = first | 1; i < first + rows; i += 2) {
    bk_fill(n->x, tile_rect(n, l, i).ymin - l->border_y, n->w, l->item_h, alt);
  }
}

static void draw_row(const BKNode *n, const FBLayout *l, int i)
{
  const FBEntry *e = fb_vis_entry(FB(n), i);
  const BKRect t = tile_rect(n, l, i);
  const bool editing = n->edit && i == n->rename_row;
  const int padx = (int)(0.1f * u());
  if (!editing) {
    draw_tile_background(&t, e->sel, bk_hovered(n) && i == n->hot);
  }
  bk_icon(e->icon, t.xmin + padx + l->prv_w * 0.5f, t.ymin + bk_rh(&t) * 0.5f, NULL,
          e->hidden ? 0.5f : 1.0f, 0); /* file_add_icon_but: hidden files at half alpha */
  BKRect tr = t; /* text_draw_rect_get */
  tr.xmin += l->prv_w + 2 * padx + 1;
  tr.xmax = t.xmin + l->col_w[FB_COL_NAME] - l->border_x;
  if (editing) {
    bk_te_draw(n->edit, &tr, bk_theme.textfield.text);
  }
  else {
    draw_name(&tr, e->name, e->sel ? bk_theme.list_item.text_sel : bk_theme.list_item.text);
  }
  /* draw_details_columns */
  const char *details[FB_COL_NUM] = {NULL, e->kind == FB_KIND_PARENT ? NULL : e->date,
                                     e->kind == FB_KIND_FILE ? e->size_str : NULL};
  for (int c = FB_COL_DATE; c < FB_COL_NUM; c++) {
    const BKRect cr = bk_rect(column_x(n, l, c) + l->col_pad, t.ymin,
                              l->col_w[c] - 2 * l->col_pad, bk_rh(&t));
    if (details[c]) {
      bk_text_in_pad(&cr, details[c], bk_theme.file_text, g_columns[c].align, false, 0);
    }
  }
}

/* The "No items" / filter / error message at UI_UNIT_X, UI_UNIT_Y into the first free tile. */
static void draw_hint(const BKNode *n, const FBLayout *l)
{
  const FBData *f = FB(n);
  const bool only_parent = f->nvis == 1 && fb_vis_entry(f, 0)->kind == FB_KIND_PARENT;
  if (f->nvis > 0 && !(only_parent && f->error[0])) {
    return;
  }
  const BKRect t = tile_rect(n, l, f->nvis);
  uchar c[4];
  memcpy(c, bk_theme.file_text, 4);
  c[3] = f->search[0] ? c[3] : c[3] / 2;
  const char *msg = f->error[0]  ? f->error :
                    f->search[0] ? "No results match the search filter" :
                                   "No items";
  bk_text(t.xmin + u(), t.ymin + u() * 0.7f, msg, -1, c, false);
}

/* draw_fixed_header_background + draw_columnheader_columns, drawn last. */
static void draw_column_header(const BKNode *n, const FBLayout *l)
{
  const FBData *f = FB(n);
  uchar back[4], div[4];
  float divc[4], tc[4];
  bk_color_shade(bk_theme.file_back, 11, back);
  bk_color_shade(bk_theme.file_back, -10, div);
  bk_rgba(div, divc);
  bk_rgba(bk_theme.file_text, tc);
  bk_fill(n->x, n->y, n->w, l->header_h, back);
  const float divider_pad = 0.2f * l->header_h;
  for (int c = 0; c < FB_COL_NUM; c++) {
    const int sx = column_x(n, l, c);
    if (f->sort == g_columns[c].sort) { /* active sort type triangle, 't' when inverted */
      bk_tria(sx + l->col_w[c] - 0.3f * u() - l->col_pad / 2.0f, n->y + l->header_h * 0.5f,
              f->invert ? 't' : 'v', tc, 0);
    }
    const BKRect hr = bk_rect(sx + l->col_pad, n->y + l->border_y, l->col_w[c] - 2 * l->col_pad,
                              l->header_h - l->border_y);
    bk_text_in_pad(&hr, g_columns[c].name, bk_theme.file_text, BK_ALIGN_LEFT, false, 0);
    if (c != FB_COL_NAME) {
      bk_line(sx - 1, n->y + divider_pad, sx - 1, n->y + l->header_h - divider_pad, divc, 1.0f);
    }
  }
  bk_line(n->x, n->y + 0.5f, n->x + n->w, n->y + 0.5f, divc, 1.0f);
  bk_line(n->x, n->y + l->header_h - 0.5f, n->x + n->w, n->y + l->header_h - 0.5f, divc, 1.0f);
}

static void list_draw(BKNode *n)
{
  const FBLayout l = list_layout(n);
  n->scroll = BK_CLAMP(n->scroll, 0, max_scroll(n, &l));
  const int first = BK_MAX(0, (n->scroll - l.pad_top) / l.item_h);
  const int rows = view_h(n, &l) / l.item_h + 2;
  const BKRect v = view_rect(n, &l);
  bk_clip_push(v.xmin, v.ymin, bk_rw(&v), bk_rh(&v)); /* the header draws its own back */
  draw_background(n, &l, first, rows);
  for (int i = first; i < FB(n)->nvis && i < first + rows; i++) {
    draw_row(n, &l, i);
  }
  draw_hint(n, &l);
  bk_clip_pop();
  draw_column_header(n, &l);
  if (has_scroller(n, &l)) {
    BKRect track, thumb;
    scroller(n, &l, &track, &thumb);
    bk_scroller_draw(&track, &thumb, FB(n)->scroller_hot || n->dragmode == 1 ? 255 : 0,
                     n->dragmode == 1);
  }
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Selection (file_select, file_walk_select)
 * \{ */

void fb_list_see(BKNode *n, int i)
{
  const FBLayout l = list_layout(n);
  const int y = l.pad_top + i * l.item_h, vh = view_h(n, &l);
  n->scroll = y < n->scroll ? y : (y + l.item_h > n->scroll + vh ? y + l.item_h - vh : n->scroll);
  n->scroll = BK_CLAMP(n->scroll, 0, max_scroll(n, &l));
  bk_invalidate(n);
}

/* The name field follows the last selected file (params->file). */
static void sync_filename(FBData *f)
{
  const FBEntry *e = fb_vis_entry(f, f->active);
  if (f->fname && e && e->sel && e->kind == FB_KIND_FILE && f->mode != FB_MODE_DIRECTORY) {
    bk_set_text(&f->fname->text, e->name);
    bk_invalidate(f->fname);
  }
  fb_update_exec(f);
}

/* Click selects, Ctrl toggles (extend), Shift fills from the anchor. */
void fb_click_select(FBData *f, int i, int mods)
{
  FBEntry *e = fb_vis_entry(f, i);
  if (!e) {
    return;
  }
  const bool multi = f->multiple && e->kind != FB_KIND_PARENT;
  if (multi && (mods & BK_SHIFT) && f->anchor >= 0) {
    if (!(mods & BK_CTRL)) {
      fb_select_none(f);
    }
    for (int k = BK_MIN(f->anchor, i); k <= BK_MAX(f->anchor, i); k++) {
      fb_vis_entry(f, k)->sel |= fb_vis_entry(f, k)->kind != FB_KIND_PARENT;
    }
  }
  else {
    const bool toggle = multi && (mods & BK_CTRL);
    const bool on = toggle ? !e->sel : true;
    if (!toggle) {
      fb_select_none(f);
    }
    e->sel = on;
    f->anchor = i;
  }
  f->active = i;
  sync_filename(f);
  bk_invalidate(f->list);
}

/* Arrows move the active file; Shift extends from the anchor. */
void fb_walk(FBData *f, int to, int mods)
{
  if (f->nvis == 0) {
    return;
  }
  to = BK_CLAMP(to, 0, f->nvis - 1);
  const bool extend = f->multiple && (mods & BK_SHIFT);
  if (extend && f->anchor < 0) {
    f->anchor = BK_MAX(f->active, 0);
  }
  fb_click_select(f, to, extend ? BK_SHIFT : 0); /* Shift keeps the anchor */
  fb_list_see(f->list, to);
}

void fb_rename_begin(FBData *f, int i)
{
  const FBEntry *e = fb_vis_entry(f, i);
  if (!e || e->kind == FB_KIND_PARENT) {
    return;
  }
  f->list->rename_row = i;
  bk_begin_edit(f->list, e->name, true);
  fb_list_see(f->list, i);
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Input
 * \{ */

static void end_rename(BKNode *n)
{
  BKContext *ctx = bk_ctx(n);
  if (n->edit && ctx && ctx->focus == n) {
    BK_focus_node(ctx, NULL); /* file_params_rename_end */
  }
}

/* A press on the scroller: drag the thumb, or jump to the click first. */
static int press_scroller(BKNode *n, const FBLayout *l, const BKEvent *e)
{
  BKRect track, thumb;
  FB(n)->scroller_hot = true;
  scroller(n, l, &track, &thumb);
  const int vh = BK_MAX(1, view_h(n, l));
  if (e->y < thumb.ymin || e->y >= thumb.ymax) {
    n->scroll = BK_CLAMP(
        (int)((float)(e->y - track.ymin) / vh * content_h(n, l) - vh * 0.5f), 0, max_scroll(n, l));
  }
  n->dragmode = 1;
  n->drag_orig = n->scroll;
  bk_invalidate(n);
  return 1;
}

/* file_column_sort_ui_context: a header click sorts by the column, again inverts. */
static int press_header(BKNode *n, const FBLayout *l, const BKEvent *e)
{
  FBData *f = FB(n);
  const int c = column_at(n, l, e->x);
  if (c >= 0) {
    f->invert = f->sort == g_columns[c].sort && !f->invert;
    f->sort = g_columns[c].sort;
    fb_filter_sort(f);
  }
  return 1;
}

static int list_press(BKNode *n, BKEvent *e)
{
  FBData *f = FB(n);
  const FBLayout l = list_layout(n);
  end_rename(n);
  n->dragmode = 0;
  if (f->scroller_hot && has_scroller(n, &l) && e->y >= n->y + l.header_h) {
    return press_scroller(n, &l, e);
  }
  if (e->y < n->y + l.header_h) {
    return press_header(n, &l, e);
  }
  const int i = row_at(n, &l, e->y);
  if (i >= 0) {
    fb_click_select(f, i, e->mods);
    return 1;
  }
  fb_select_none(f); /* empty space deselects */
  f->active = -1;
  fb_update_exec(f);
  bk_invalidate(n);
  return 1;
}

static int list_dbl(BKNode *n, BKEvent *e)
{
  FBData *f = FB(n);
  const FBLayout l = list_layout(n);
  const int i = row_at(n, &l, e->y);
  if (e->y < n->y + l.header_h || n->dragmode == 1 || i < 0) {
    return e->y < n->y + l.header_h ? list_press(n, e) : 1; /* a header click toggles again */
  }
  if (!fb_vis_entry(f, i)->sel) {
    fb_click_select(f, i, 0);
  }
  fb_open_entry(f, i);
  return 1;
}

static int list_drag(BKNode *n, BKEvent *e)
{
  const FBLayout l = list_layout(n);
  const int vh = BK_MAX(1, view_h(n, &l));
  if (n->dragmode != 1) {
    return 0;
  }
  n->scroll = BK_CLAMP((int)(n->drag_orig + (double)e->dy * content_h(n, &l) / vh), 0,
                       max_scroll(n, &l));
  bk_invalidate(n);
  return 1;
}

static int list_release(BKNode *n, BKEvent *e)
{
  (void)e;
  n->dragmode = 0;
  bk_invalidate(n);
  return 1;
}

static int list_move(BKNode *n, BKEvent *e)
{
  FBData *f = FB(n);
  const FBLayout l = list_layout(n);
  const int edge = n->x + n->w - (int)(0.45f * u() + 2 * bk_px()) - (int)(0.3f * u());
  const bool on_scroller = has_scroller(n, &l) && e->y >= n->y + l.header_h && e->x >= edge;
  const int hot = on_scroller ? -1 : row_at(n, &l, e->y);
  if (hot != n->hot || on_scroller != f->scroller_hot) {
    n->hot = hot;
    f->scroller_hot = on_scroller;
    bk_invalidate(n);
  }
  return 0;
}

static int list_leave(BKNode *n)
{
  n->hot = -1;
  FB(n)->scroller_hot = false;
  bk_invalidate(n);
  return 0;
}

static int list_wheel(BKNode *n, BKEvent *e)
{
  const FBLayout l = list_layout(n);
  const int ns = BK_CLAMP(n->scroll - e->delta * l.item_h * FB_WHEEL_ROWS, 0, max_scroll(n, &l));
  if (ns == n->scroll) {
    return 0;
  }
  n->scroll = ns;
  list_move(n, e);
  bk_invalidate(n);
  return 1;
}

/* Keys go to the rename field while editing, else to the browser's keymap. */
static int list_key(BKNode *n, BKEvent *e)
{
  return n->edit ? bk_edit_key(n, e) : fb_keymap(FB(n), e);
}

static void list_finish_edit(BKNode *n, bool confirm)
{
  const int i = n->rename_row;
  n->rename_row = -1;
  if (confirm && n->edit && n->edit->text[0]) {
    fb_rename(FB(n), i, n->edit->text);
  }
  bk_invalidate(n);
}

static int list_cursor_at(BKNode *n, int x, int y)
{
  (void)x;
  (void)y;
  return n->edit ? BK_CURSOR_TEXT : BK_CURSOR_DEFAULT;
}

static void list_measure(BKNode *n, int *rw, int *rh)
{
  (void)n;
  *rw = 16 * u();
  *rh = 8 * u();
}

int fb_list_page_rows(const BKNode *n)
{
  const FBLayout l = list_layout(n);
  return view_h(n, &l) / BK_MAX(1, l.item_h);
}

/* Tk-style yview in rows. */
static void list_view(BKNode *n, int axis, int *r_first, int *r_visible, int *r_total)
{
  const FBLayout l = list_layout(n);
  *r_first = axis == 0 ? n->scroll / BK_MAX(1, l.item_h) : 0;
  *r_visible = axis == 0 ? view_h(n, &l) / BK_MAX(1, l.item_h) : 1;
  *r_total = axis == 0 ? FB(n)->nvis : 1;
}

static void list_set_view(BKNode *n, int axis, int first)
{
  const FBLayout l = list_layout(n);
  if (axis == 0) {
    n->scroll = BK_CLAMP(first * l.item_h, 0, max_scroll(n, &l));
    bk_invalidate(n);
  }
}

const BKClass bk_cls_fb_list = {.name = "filelist",
                                .flags = CLS_INTERACTIVE,
                                .measure = list_measure,
                                .draw = list_draw,
                                .press = list_press,
                                .dbl = list_dbl,
                                .drag = list_drag,
                                .release = list_release,
                                .move = list_move,
                                .leave = list_leave,
                                .key = list_key,
                                .wheel = list_wheel,
                                .cursor_at = list_cursor_at,
                                .finish_edit = list_finish_edit,
                                .view = list_view,
                                .set_view = list_set_view};

/** \} */
