/* File browser: Blender's File Browser (editors/space_file) as a kit composite.
 *
 * A "filebrowser" node lays its parts out like Blender's file browser area:
 * - the tools region on the left: Volumes / System / Bookmarks / Recent (kit_fsmenu.c),
 * - the directory-path region at the top: back, forward, parent, refresh, new folder, the path
 *   field, search and the hidden-files toggle (bl_ui FILEBROWSER_PT_directory_path),
 * - the main region: the file list (kit_filelist_view.c),
 * - the execute region at the bottom: file name, file-type filter, Cancel and the execute
 *   button (file_panels.cc file_panel_execution_buttons_draw).
 * This file builds the node, lays it out, maps keys and properties, and runs the blocking
 * dialog; kit_filebrowser_intern.h lists the other parts. Built without BUI_FILEBROWSER
 * (features.json), only BK_file_dialog remains: it reports no browser (-1).
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "kit_filebrowser_intern.h"

#ifdef BUI_FILEBROWSER

static int u(void)
{
  return bk_unit();
}

/* -------------------------------------------------------------------- */
/** \name Keymap (the file browser keymap; also reached from the list)
 * \{ */

typedef void (*FBAction)(FBData *f, int mods);

static void act_cancel(FBData *f, int mods)
{
  (void)mods;
  fb_finish(f, FB_STATE_CANCELLED);
}

static void act_parent(FBData *f, int mods)
{
  (void)mods;
  fb_go_parent(f);
}

static void act_back(FBData *f, int mods)
{
  (void)mods;
  fb_go_back(f);
}

static void act_forward(FBData *f, int mods)
{
  (void)mods;
  fb_go_forward(f);
}

static void act_hidden(FBData *f, int mods)
{
  (void)mods;
  f->b_hidden->value = !fb_show_hidden(f);
  bk_invalidate(f->b_hidden);
  fb_filter_sort(f);
}

static void act_bookmark(FBData *f, int mods)
{
  (void)mods;
  fb_bookmark_dir(f);
}

static void act_refresh(FBData *f, int mods)
{
  (void)mods;
  fb_refresh(f);
}

static void act_new_folder(FBData *f, int mods)
{
  (void)mods;
  fb_new_folder(f);
}

static void act_rename(FBData *f, int mods)
{
  (void)mods;
  fb_rename_begin(f, f->active);
}

static void act_activate(FBData *f, int mods)
{
  (void)mods;
  fb_activate(f);
}

static int active_or_first(const FBData *f)
{
  return f->active < 0 ? 0 : f->active;
}

static void act_up(FBData *f, int mods)
{
  fb_walk(f, f->active < 0 ? 0 : f->active - 1, mods);
}

static void act_down(FBData *f, int mods)
{
  fb_walk(f, f->active < 0 ? 0 : f->active + 1, mods);
}

static void act_home(FBData *f, int mods)
{
  fb_walk(f, 0, mods);
}

static void act_end(FBData *f, int mods)
{
  fb_walk(f, f->nvis - 1, mods);
}

static int page_rows(const FBData *f)
{
  return BK_MAX(1, fb_list_page_rows(f->list) - 1);
}

static void act_page_up(FBData *f, int mods)
{
  fb_walk(f, active_or_first(f) - page_rows(f), mods);
}

static void act_page_down(FBData *f, int mods)
{
  fb_walk(f, active_or_first(f) + page_rows(f), mods);
}

/* file.select_all: every file (folders too when choosing one), or none. */
static void select_all(FBData *f, bool on)
{
  for (int i = 0; f->multiple && i < f->nvis; i++) {
    FBEntry *v = fb_vis_entry(f, i);
    v->sel = on && v->kind != FB_KIND_PARENT &&
             (f->mode == FB_MODE_DIRECTORY || v->kind == FB_KIND_FILE);
  }
  fb_update_exec(f);
  bk_invalidate(f->list);
}

static void act_select_all(FBData *f, int mods)
{
  (void)mods;
  select_all(f, true);
}

static void act_deselect_all(FBData *f, int mods)
{
  (void)mods;
  select_all(f, false);
}

/* Matched on Ctrl and Alt exactly; Shift passes through (it extends walks). */
static const struct {
  const char *keysym;
  int mods;
  FBAction fn;
} g_keymap[] = {
    {"Escape", 0, act_cancel},          {"BackSpace", 0, act_parent},
    {"Up", BK_ALT, act_parent},         {"Left", BK_ALT, act_back},
    {"Right", BK_ALT, act_forward},     {"h", BK_CTRL, act_hidden},
    {"b", BK_CTRL, act_bookmark},       {"r", 0, act_refresh},
    {"i", 0, act_new_folder},           {"F2", 0, act_rename},
    {"Return", BK_CTRL, act_rename},    {"Return", 0, act_activate},
    {"KP_Enter", 0, act_activate},      {"Up", 0, act_up},
    {"Down", 0, act_down},              {"Home", 0, act_home},
    {"End", 0, act_end},                {"Prior", 0, act_page_up},
    {"Next", 0, act_page_down},         {"a", 0, act_select_all},
    {"a", BK_ALT, act_deselect_all},
};

int fb_keymap(FBData *f, const BKEvent *e)
{
  const int mods = e->mods & (BK_CTRL | BK_ALT);
  for (size_t i = 0; i < sizeof(g_keymap) / sizeof(g_keymap[0]); i++) {
    if (mods == g_keymap[i].mods && bk_keysym_is(e->keysym, g_keymap[i].keysym)) {
      g_keymap[i].fn(f, e->mods);
      return 1;
    }
  }
  return 0;
}

static int fb_key(BKNode *n, BKEvent *e)
{
  return fb_keymap(FB(n), e);
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Parts and their callbacks
 * \{ */

static void on_prev(BKNode *b, void *data)
{
  (void)b;
  fb_go_back((FBData *)data);
}

static void on_next(BKNode *b, void *data)
{
  (void)b;
  fb_go_forward((FBData *)data);
}

static void on_parent(BKNode *b, void *data)
{
  (void)b;
  fb_go_parent((FBData *)data);
}

static void on_refresh(BKNode *b, void *data)
{
  (void)b;
  fb_refresh((FBData *)data);
}

static void on_newdir(BKNode *b, void *data)
{
  (void)b;
  fb_new_folder((FBData *)data);
}

static void on_path(BKNode *n, void *data)
{
  fb_navigate((FBData *)data, n->text, true); /* file_directory_enter_handle */
}

static void on_search(BKNode *n, void *data)
{
  FBData *f = (FBData *)data;
  snprintf(f->search, sizeof(f->search), "%s", n->text ? n->text : "");
  fb_filter_sort(f);
}

/* The hidden toggle and the filter dropdown hold their settings: re-filter on change. */
static void on_refilter(BKNode *n, void *data)
{
  (void)n;
  fb_filter_sort((FBData *)data);
}

/* file_filename_enter_handle: a folder name (or path) goes there. */
static void on_fname(BKNode *n, void *data)
{
  FBData *f = (FBData *)data;
  char p[FB_PATH];
  if (n->text && n->text[0]) {
    fb_path_resolve(f->dir, n->text, p, sizeof(p));
    if (fb_path_is_dir(p)) {
      bk_set_text(&n->text, "");
      fb_navigate(f, p, true);
      return;
    }
  }
  fb_update_exec(f);
}

static void on_cancel(BKNode *b, void *data)
{
  (void)b;
  fb_finish((FBData *)data, FB_STATE_CANCELLED); /* file.cancel */
}

static void on_exec(BKNode *b, void *data)
{
  (void)b;
  fb_execute((FBData *)data);
}

static BKNode *part(BKNode *parent, const BKClass *cls)
{
  BKNode *c = bk_node_new_cls(cls);
  bk_node_add(parent, c);
  return c;
}

static BKNode *button(FBData *f, const char *icon, const char *text, const char *tip,
                      void (*cb)(BKNode *, void *))
{
  BKNode *b = part(f->self, &bk_cls_button);
  b->icon = icon ? bk_icons_lookup(icon) : 0;
  bk_set_text(&b->text, text);
  bk_set_text(&b->desc, tip);
  b->on_click = cb;
  b->on_click_data = f;
  return b;
}

static BKNode *field(FBData *f, const BKClass *cls, const char *tip,
                     void (*cb)(BKNode *, void *))
{
  BKNode *t = part(f->self, cls);
  bk_set_text(&t->desc, tip);
  t->on_change = cb;
  t->on_change_data = f;
  return t;
}

static void build_header(FBData *f)
{
  f->b_prev = button(f, "BACK", NULL, "Move to previous folder", on_prev);
  f->b_next = button(f, "FORWARD", NULL, "Move to next folder", on_next);
  f->b_parent = button(f, "FILE_PARENT", NULL, "Move to parent directory", on_parent);
  f->b_refresh = button(f, "FILE_REFRESH", NULL, "Refresh the file list", on_refresh);
  f->b_newdir = button(f, "NEWFOLDER", NULL, "Create a new folder", on_newdir);
  f->path = field(f, &bk_cls_text, "Directory path", on_path);
  f->search_f = field(f, &bk_cls_search, NULL, on_search);
  bk_set_text(&f->search_f->placeholder, "Filter");
  f->search_f->flags |= NF_LIVE;
  f->b_hidden = field(f, &bk_cls_toggle, "Show hidden dot files (Ctrl+H)", on_refilter);
  f->b_hidden->icon = bk_icons_lookup("HIDE_ON");
  f->b_hidden->icon_on = bk_icons_lookup("HIDE_OFF");
}

static void build_side(FBData *f)
{
  static const char *const titles[FB_CAT_NUM] = {"Volumes", "System", "Bookmarks", "Recent"};
  f->side = part(f->self, &bk_cls_column);
  f->side->padding = (int)(FB_MARGIN * u());
  f->side->spacing = (int)(0.2f * u());
  for (int c = 0; c < FB_CAT_NUM; c++) {
    f->panels[c] = part(f->side, &bk_cls_panel);
    bk_set_text(&f->panels[c]->text, titles[c]);
    f->lists[c] = fb_places_view_new(f, (FBCategory)c, (BKNode *)f->panels[c]->ext); /* body */
  }
}

static void build_exec(FBData *f)
{
  f->fname = field(f, &bk_cls_text, "File name", on_fname);
  f->filter_dd = field(f, &bk_cls_dropdown, NULL, on_refilter);
  f->b_cancel = button(f, NULL, "Cancel", NULL, on_cancel);
  f->b_exec = button(f, NULL, NULL, NULL, on_exec);
  f->b_exec->flags |= NF_ACTIVE_DEFAULT; /* row.active_default_set(true) */
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Layout and drawing
 * \{ */

/* Show or hide a part from arrange (no relayout request, unlike the "visible" property). */
static void arrange_shown(BKNode *c, bool on, int x, int y, int w, int h)
{
  c->flags = on ? (c->flags | NF_VISIBLE) : (c->flags & ~NF_VISIBLE);
  bk_arrange(c, x, y, on ? w : 0, on ? h : 0);
}

static int natural_w(BKNode *c, int min)
{
  int w, h;
  bk_natural(c, &w, &h);
  return BK_MAX(w, min);
}

/* An aligned row (UI_block_align_begin): rounded only at the ends. Returns the end x. */
static int arrange_aligned(BKNode *const *row, int nrow, int x, int y, int size)
{
  for (int i = 0; i < nrow; i++) {
    row[i]->corners = nrow == 1        ? CNR_ALL :
                      i == 0           ? (CNR_TL | CNR_BL) :
                      i == nrow - 1    ? (CNR_TR | CNR_BR) :
                                         0;
    bk_arrange(row[i], x + i * size, y, size, size);
  }
  return x + nrow * size;
}

static int bar_h(void)
{
  return bk_iround(FB_BAR_SCALE * u()) + 2 * (int)(FB_MARGIN * u());
}

/* FILEBROWSER_PT_directory_path */
static void arrange_header(FBData *f, int x, int y, int w)
{
  const int sp = (int)(FB_SPACE * u()), margin = (int)(FB_MARGIN * u());
  const int bh = bk_iround(FB_BAR_SCALE * u()), by = y + margin;
  const int search_w = BK_MIN(FB_SEARCH_MAX_W * u(), BK_MAX(FB_SEARCH_MIN_W * u(), w / 5));
  int left = x + margin, right = x + w - margin - bh;
  f->r_header = bk_rect(x, y, w, bar_h());
  left = arrange_aligned((BKNode *[]){f->b_prev, f->b_next, f->b_parent, f->b_refresh}, 4,
                         left, by, bh) + sp;
  left = arrange_aligned(&f->b_newdir, 1, left, by, bh) + sp;
  arrange_aligned(&f->b_hidden, 1, right, by, bh);
  right -= sp + search_w;
  bk_arrange(f->search_f, right, by, search_w, bh);
  bk_arrange(f->path, left, by, BK_MAX(2 * u(), right - sp - left), bh);
}

/* file_panel_execution_buttons_draw: Cancel then execute (Windows: swapped). */
static void arrange_exec(FBData *f, int x, int y, int w)
{
  const int sp = (int)(FB_SPACE * u()), margin = (int)(FB_MARGIN * u());
  const int bh = bk_iround(FB_BAR_SCALE * u()), by = y + margin;
  const int exec_w = natural_w(f->b_exec, FB_BUTTON_MIN_W * u());
  const int cancel_w = natural_w(f->b_cancel, FB_BUTTON_MIN_W * u());
  const bool windows = fb_os_sep() == '\\';
  BKNode *first = windows ? f->b_cancel : f->b_exec, *second = windows ? f->b_exec : f->b_cancel;
  int right = x + w - margin - (windows ? cancel_w : exec_w);
  f->r_exec = bk_rect(x, y, w, bar_h());
  bk_arrange(first, right, by, windows ? cancel_w : exec_w, bh);
  right -= sp + (windows ? exec_w : cancel_w);
  bk_arrange(second, right, by, windows ? exec_w : cancel_w, bh);
  const int dw = BK_MIN(natural_w(f->filter_dd, FB_FILTER_MIN_W * u()),
                        BK_MAX(FB_FILTER_MIN_W * u(), w / 3));
  right -= f->nfilters > 0 ? sp + dw : 0;
  arrange_shown(f->filter_dd, f->nfilters > 0, right, by, dw, bh);
  arrange_shown(f->fname, f->mode != FB_MODE_DIRECTORY, x + margin, by,
                BK_MAX(2 * u(), right - sp - x - margin), bh);
}

static void fb_arrange(BKNode *n)
{
  FBData *f = FB(n);
  const int side_w = n->w >= FB_SIDE_AREA_WIDE * u()   ? FB_SIDE_W * u() :
                     n->w >= FB_SIDE_AREA_NARROW * u() ? FB_SIDE_W_NARROW * u() :
                                                         0;
  f->r_side = bk_rect(n->x, n->y, side_w, n->h);
  arrange_shown(f->side, side_w > 0, n->x, n->y, side_w, n->h);
  const int mx = n->x + side_w, mw = n->w - side_w;
  arrange_header(f, mx, n->y, mw);
  arrange_exec(f, mx, n->y + n->h - bar_h(), mw);
  bk_arrange(f->list, mx, n->y + bar_h(), mw, BK_MAX(0, n->h - 2 * bar_h()));
}

/* Region backgrounds: tools (regions.sidebars.back), header and execute (space_file.header over
 * the back color); the main region draws itself. */
static void fb_draw(BKNode *n)
{
  const FBData *f = FB(n);
  uchar header[4];
  bk_color_over(bk_theme.file_header, bk_theme.file_back, header);
  bk_fill(f->r_side.xmin, f->r_side.ymin, bk_rw(&f->r_side), bk_rh(&f->r_side),
          bk_theme.region_bg);
  bk_fill(f->r_header.xmin, f->r_header.ymin, bk_rw(&f->r_header), bk_rh(&f->r_header), header);
  bk_fill(f->r_exec.xmin, f->r_exec.ymin, bk_rw(&f->r_exec), bk_rh(&f->r_exec), header);
}

static void fb_measure(BKNode *n, int *rw, int *rh)
{
  (void)n;
  *rw = FB_NATURAL_W * u();
  *rh = FB_NATURAL_H * u();
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Properties (see BUI_kit.h)
 * \{ */

/* "file": a name, or a path whose folder becomes the directory (Tk's initialfile). */
static void set_file(FBData *f, const char *v)
{
  char t[FB_PATH];
  snprintf(t, sizeof(t), "%s", v ? v : "");
  char *name = t + strlen(t);
  while (name > t && !fb_os_is_sep(name[-1])) {
    name--;
  }
  if (name > t) {
    name[-1] = '\0';
    fb_navigate(f, t[0] ? t : (char[2]){fb_os_sep(), '\0'}, false);
  }
  bk_set_text(&f->fname->text, name);
  bk_invalidate(f->fname);
  fb_update_exec(f);
}

static int fb_set_str(BKNode *n, const char *key, const char *v)
{
  FBData *f = FB(n);
  if (strcmp(key, "dir") == 0 || strcmp(key, "directory") == 0) {
    fb_navigate(f, v, false);
  }
  else if (strcmp(key, "file") == 0 || strcmp(key, "filename") == 0) {
    set_file(f, v);
  }
  else if (strcmp(key, "button") == 0) {
    free(f->exec_label);
    f->exec_label = (v && v[0]) ? bk_strdup(v) : NULL;
    fb_update_exec(f);
  }
  else if (strcmp(key, "search") == 0) {
    bk_set_text(&f->search_f->text, v);
    on_search(f->search_f, f);
    bk_invalidate(f->search_f);
  }
  else if (strcmp(key, "bookmarks_file") == 0) {
    snprintf(f->bookmarks_path, sizeof(f->bookmarks_path), "%s", v ? v : "");
    fb_read_bookmarks(f);
  }
  else {
    return 0;
  }
  bk_relayout(n);
  return 1;
}

static const char *fb_get_str(BKNode *n, const char *key)
{
  FBData *f = FB(n);
  const bool dir = strcmp(key, "dir") == 0 || strcmp(key, "directory") == 0;
  const bool file = strcmp(key, "file") == 0 || strcmp(key, "filename") == 0;
  return dir                                   ? f->dir :
         file                                  ? (f->fname->text ? f->fname->text : "") :
         strcmp(key, "path") == 0              ? (f->result.n ? f->result.v[0] : "") :
         strcmp(key, "search") == 0            ? f->search :
         strcmp(key, "bookmarks_file") == 0    ? f->bookmarks_path :
                                                 NULL;
}

static int fb_set_strs(BKNode *n, const char *key, const char *const *v, int count)
{
  if (strcmp(key, "filters") != 0) {
    return 0;
  }
  fb_filters_set(FB(n), v, count);
  fb_filter_sort(FB(n));
  bk_relayout(n);
  return 1;
}

static const BKStrs *fb_get_strs(BKNode *n, const char *key)
{
  return strcmp(key, "result") == 0 ? &FB(n)->result : NULL;
}

/* Settings that re-filter the list after they change. */
static bool set_listing_num(FBData *f, const char *key, double v)
{
  if (strcmp(key, "hidden") == 0 || strcmp(key, "show_hidden") == 0) {
    f->b_hidden->value = v != 0;
    bk_invalidate(f->b_hidden);
  }
  else if (strcmp(key, "filter") == 0) {
    f->filter_dd->value = BK_CLAMP((int)v, 0, BK_MAX(0, f->nfilters - 1));
    bk_invalidate(f->filter_dd);
  }
  else if (strcmp(key, "sort") == 0) {
    f->sort = (FBSort)BK_CLAMP((int)v, FB_SORT_ALPHA, FB_SORT_SIZE);
  }
  else if (strcmp(key, "invert") == 0) {
    f->invert = v != 0;
  }
  else if (strcmp(key, "mode") == 0) {
    f->mode = (FBMode)BK_CLAMP((int)v, FB_MODE_OPEN, FB_MODE_DIRECTORY);
    fb_update_exec(f);
    bk_relayout(f->self);
  }
  else {
    return false;
  }
  fb_filter_sort(f);
  return true;
}

static int fb_set_num(BKNode *n, const char *key, double v)
{
  FBData *f = FB(n);
  if (set_listing_num(f, key, v)) {
    return 1;
  }
  if (strcmp(key, "multiple") == 0) {
    f->multiple = v != 0;
  }
  else if (strcmp(key, "state") == 0) {
    f->state = v > 0 ? FB_STATE_CHOSEN : (v < 0 ? FB_STATE_CANCELLED : FB_STATE_BROWSING);
  }
  else if (strcmp(key, "refresh") == 0) {
    fb_refresh(f);
  }
  else if (strcmp(key, "execute") == 0) {
    fb_execute(f);
  }
  else if (strcmp(key, "cancel") == 0) {
    fb_finish(f, FB_STATE_CANCELLED);
  }
  else {
    return 0;
  }
  return 1;
}

static int fb_get_num(BKNode *n, const char *key, double *r_v)
{
  const FBData *f = FB(n);
  static const char *const keys[] = {"mode",  "multiple", "hidden", "show_hidden",  "filter",
                                     "sort",  "invert",   "state",  "count",        "selected",
                                     "dialog_width", "dialog_height"};
  const double values[] = {f->mode,
                           f->multiple,
                           fb_show_hidden(f),
                           fb_show_hidden(f),
                           fb_filter_index(f),
                           f->sort,
                           f->invert,
                           f->state,
                           f->nvis, /* listed entries, ".." included */
                           fb_count_selected(f, false, NULL),
                           FB_DIALOG_W * u(),
                           FB_DIALOG_H * u()};
  for (size_t i = 0; i < sizeof(keys) / sizeof(keys[0]); i++) {
    if (strcmp(key, keys[i]) == 0) {
      *r_v = values[i];
      return 1;
    }
  }
  return 0;
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name The node
 * \{ */

static void fb_init(BKNode *n)
{
  FBData *f = (FBData *)calloc(1, sizeof(FBData));
  n->ext = f;
  f->self = n;
  f->active = f->anchor = -1;
  build_header(f);
  build_side(f);
  f->list = part(n, &bk_cls_fb_list);
  f->list->ext = f;
  f->list->hot = f->list->rename_row = -1;
  build_exec(f);
  fb_places_read(f);
  fb_navigate(f, ".", false);
}

static void fb_free(BKNode *n)
{
  FBData *f = FB(n);
  fb_entries_free(f);
  free(f->all);
  fb_places_free(f);
  fb_filters_free(f);
  fb_history_free(f);
  bk_strs_free(&f->result);
  free(f->exec_label);
  free(f);
  n->ext = NULL;
}

const BKClass bk_cls_filebrowser = {.name = "filebrowser",
                                    .flags = CLS_INTERACTIVE,
                                    .init = fb_init,
                                    .free_ext = fb_free,
                                    .measure = fb_measure,
                                    .arrange = fb_arrange,
                                    .draw = fb_draw,
                                    .key = fb_key,
                                    .set_num = fb_set_num,
                                    .get_num = fb_get_num,
                                    .set_str = fb_set_str,
                                    .get_str = fb_get_str,
                                    .set_strs = fb_set_strs,
                                    .get_strs = fb_get_strs};

/** \} */

/* -------------------------------------------------------------------- */
/** \name Blocking dialog in a native window (WM_event_add_fileselect, as a temp window)
 * \{ */

static BKNode *dialog_node(int mode, const char *dir, const char *file,
                           const char *const *filters, int nfilters, int multiple)
{
  BKNode *fb = BK_node_new("filebrowser");
  BK_set_num(fb, "mode", mode);
  BK_set_num(fb, "multiple", multiple);
  if (nfilters > 0) {
    BK_set_strs(fb, "filters", filters, nfilters);
  }
  if (dir && dir[0]) {
    BK_set_str(fb, "dir", dir);
  }
  if (file && file[0]) {
    BK_set_str(fb, "file", file);
  }
  return fb;
}

static int dialog_result(BKNode *fb, char ***r_paths)
{
  const int count = BK_get_num(fb, "state") > 0 ? BK_strs_count(fb, "result") : 0;
  *r_paths = count > 0 ? (char **)calloc((size_t)count + 1, sizeof(char *)) : NULL;
  for (int i = 0; i < count; i++) {
    (*r_paths)[i] = bk_strdup(BK_strs_get(fb, "result", i));
  }
  return count;
}

int BK_file_dialog(const char *title,
                   int mode,
                   const char *dir,
                   const char *file,
                   const char *const *filters,
                   int nfilters,
                   int multiple,
                   char ***r_paths)
{
  static const char *const titles[] = {"Open", "Save As", "Select Folder"};
  *r_paths = NULL;
  if (!BK_window_supported()) {
    return -1;
  }
  BKNode *fb = dialog_node(mode, dir, file, filters, nfilters, multiple);
  BKContext *ctx = BK_context_new(fb);
  BKWindow *win = BK_window_new(ctx, title && title[0] ? title : titles[FB(fb)->mode],
                                (int)BK_get_num(fb, "dialog_width"),
                                (int)BK_get_num(fb, "dialog_height"));
  int count = -1;
  if (win) {
    BK_window_show(win);
    while (BK_window_is_open(win) && FB(fb)->state == FB_STATE_BROWSING && BK_window_pump()) {
      if (!BK_needs_render(ctx)) {
        fb_os_sleep_ms(4);
      }
    }
    count = dialog_result(fb, r_paths);
    if (BK_window_is_open(win)) {
      BK_window_close(win);
    }
    BK_window_free(win);
  }
  BK_context_free(ctx);
  BK_node_free(fb);
  return count;
}

void BK_file_dialog_free(char **paths)
{
  for (char **p = paths; p && *p; p++) {
    free(*p);
  }
  free(paths);
}

/** \} */

#else /* !BUI_FILEBROWSER */

int BK_file_dialog(const char *title,
                   int mode,
                   const char *dir,
                   const char *file,
                   const char *const *filters,
                   int nfilters,
                   int multiple,
                   char ***r_paths)
{
  (void)title;
  (void)mode;
  (void)dir;
  (void)file;
  (void)filters;
  (void)nfilters;
  (void)multiple;
  *r_paths = NULL;
  return -1; /* no file browser in this build */
}

void BK_file_dialog_free(char **paths)
{
  (void)paths;
}

#endif /* BUI_FILEBROWSER */
