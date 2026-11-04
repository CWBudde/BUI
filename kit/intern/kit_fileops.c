/* File browser operations: navigation and history (file_ops.cc, folder_history.cc), new folder,
 * rename, and executing the browser (FILE_OT_execute, FILE_OT_cancel).
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "kit_filebrowser_intern.h"

/* -------------------------------------------------------------------- */
/** \name Navigation (ED_file_change_dir, folder_history.cc)
 * \{ */

static void history_push(char **stack, int *n, const char *dir)
{
  if (*n == FB_HISTORY) {
    free(stack[0]);
    memmove(stack, stack + 1, sizeof(char *) * (FB_HISTORY - 1));
    (*n)--;
  }
  stack[(*n)++] = bk_strdup(dir);
}

static void history_clear(char **stack, int *n)
{
  while (*n > 0) {
    free(stack[--(*n)]);
  }
}

void fb_history_free(FBData *f)
{
  history_clear(f->back, &f->nback);
  history_clear(f->fwd, &f->nfwd);
}

static void update_parts(FBData *f)
{
  bk_set_text(&f->path->text, f->dir);
  bk_invalidate(f->path);
  BK_set_num(f->b_prev, "enabled", f->nback > 0);
  BK_set_num(f->b_next, "enabled", f->nfwd > 0);
  BK_set_num(f->b_parent, "enabled", !fb_os_path_is_root(f->dir));
  bk_invalidate(f->side);
  fb_update_exec(f);
}

void fb_navigate(FBData *f, const char *dir_in, bool history)
{
  char dir[FB_PATH];
  snprintf(dir, sizeof(dir), "%s", dir_in && dir_in[0] ? dir_in : ".");
  fb_path_normalize(dir, sizeof(dir));
  fb_path_slash_ensure(dir, sizeof(dir));
  if (!fb_path_is_dir(dir)) { /* file_directory_enter_handle: keep the current folder */
    bk_set_text(&f->path->text, f->dir);
    bk_invalidate(f->path);
    return;
  }
  if (history && f->dir[0] && strcmp(dir, f->dir) != 0) {
    history_push(f->back, &f->nback, f->dir);
    history_clear(f->fwd, &f->nfwd);
  }
  snprintf(f->dir, sizeof(f->dir), "%s", dir);
  fb_read_dir(f);
  f->active = f->anchor = -1;
  fb_filter_sort(f);
  f->list->scroll = 0;
  f->list->hot = -1;
  update_parts(f);
  bk_emit(f->self, BK_EV_CHANGE, NULL, 0, 0);
}

/* file.previous / file.next: pop one stack, push the current folder on the other. */
static void history_step(FBData *f, char **from, int *nfrom, char **to, int *nto)
{
  if (*nfrom == 0) {
    return;
  }
  history_push(to, nto, f->dir);
  char *d = from[--(*nfrom)];
  fb_navigate(f, d, false);
  free(d);
}

void fb_go_back(FBData *f)
{
  history_step(f, f->back, &f->nback, f->fwd, &f->nfwd);
}

void fb_go_forward(FBData *f)
{
  history_step(f, f->fwd, &f->nfwd, f->back, &f->nback);
}

void fb_go_parent(FBData *f)
{
  char p[FB_PATH];
  snprintf(p, sizeof(p), "%s", f->dir);
  if (fb_path_parent(p)) {
    fb_navigate(f, p, true);
  }
}

void fb_refresh(FBData *f)
{
  fb_read_dir(f);
  fb_filter_sort(f);
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name New folder and rename
 * \{ */

static void select_by_name(FBData *f, const char *name)
{
  for (int k = 0; k < f->nvis; k++) {
    if (strcmp(fb_vis_entry(f, k)->name, name) == 0) {
      fb_click_select(f, k, 0);
      fb_list_see(f->list, k);
      return;
    }
  }
}

/* file.directory_new: "untitled", "untitled.001", ... then rename it. */
void fb_new_folder(FBData *f)
{
  char name[64] = "untitled", p[FB_PATH];
  fb_path_join(p, sizeof(p), f->dir, name);
  for (int i = 1; fb_path_exists(p) && i < 1000; i++) {
    snprintf(name, sizeof(name), "untitled.%03d", i);
    fb_path_join(p, sizeof(p), f->dir, name);
  }
  if (!fb_os_mkdir(p)) {
    snprintf(f->error, sizeof(f->error), "Could not create new folder");
    bk_invalidate(f->list);
    return;
  }
  fb_refresh(f);
  select_by_name(f, name);
  fb_rename_begin(f, f->active);
}

/* renamebutton_cb: refuse separators and names that exist, then rename. */
bool fb_rename(FBData *f, int i, const char *name_in)
{
  const FBEntry *e = fb_vis_entry(f, i);
  char from[FB_PATH], to[FB_PATH], name[1024];
  snprintf(name, sizeof(name), "%s", name_in);
  if (!e || strcmp(name, e->name) == 0) {
    return false;
  }
  fb_path_join(from, sizeof(from), f->dir, e->name);
  fb_path_join(to, sizeof(to), f->dir, name);
  const bool has_sep = strchr(name, '/') || strchr(name, fb_os_sep());
  if (has_sep || fb_path_exists(to) || rename(from, to) != 0) {
    snprintf(f->error, sizeof(f->error), "Could not rename \"%s\"", e->name);
    bk_invalidate(f->list);
    return false;
  }
  fb_refresh(f);
  select_by_name(f, name);
  return true;
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Execute and cancel
 * \{ */

/* The execute button: "Overwrite" when saving over a file (file_draw_check_exists). */
void fb_update_exec(FBData *f)
{
  static const char *const labels[] = {"Open", "Save", "Accept"};
  const char *typed = f->fname->text ? f->fname->text : "";
  char p[FB_PATH];
  fb_path_join(p, sizeof(p), f->dir, typed);
  const bool overwrite = f->mode == FB_MODE_SAVE && typed[0] && fb_path_exists(p) &&
                         !fb_path_is_dir(p);
  const char *label = overwrite ? "Overwrite" : (f->exec_label ? f->exec_label : labels[f->mode]);
  if (!bk_streq(f->b_exec->text, label)) {
    bk_set_text(&f->b_exec->text, label);
    bk_relayout(f->b_exec);
  }
  bk_invalidate(f->b_exec);
}

void fb_finish(FBData *f, FBState state)
{
  f->state = state;
  if (state == FB_STATE_CHOSEN) {
    fb_add_recent(f, f->dir);
    fb_write_bookmarks(f);
  }
  bk_emit(f->self, BK_EV_SUBMIT, NULL, state, 0);
}

static void choose(FBData *f, const char *const *paths, int n)
{
  bk_strs_set(&f->result, paths, n);
  fb_finish(f, FB_STATE_CHOSEN);
}

/* A single selected folder is the answer, else the current one. */
static void execute_directory(FBData *f)
{
  char p[FB_PATH];
  int last = -1;
  if (fb_count_selected(f, false, &last) == 1) {
    fb_path_join(p, sizeof(p), f->dir, fb_vis_entry(f, last)->name);
  }
  else {
    snprintf(p, sizeof(p), "%s", f->dir);
  }
  fb_path_strip_slash(p);
  choose(f, (const char *[]){p}, 1);
}

/* Several selected files ("multiple"); false when there are fewer than two. */
static bool execute_selection(FBData *f)
{
  const int n = fb_count_selected(f, true, NULL);
  if (f->mode != FB_MODE_OPEN || !f->multiple || n < 2) {
    return false;
  }
  char **v = (char **)calloc((size_t)n, sizeof(char *));
  int k = 0;
  for (int i = 0; i < f->nvis; i++) {
    const FBEntry *e = fb_vis_entry(f, i);
    if (e->sel && e->kind == FB_KIND_FILE) {
      char q[FB_PATH];
      fb_path_join(q, sizeof(q), f->dir, e->name);
      v[k++] = bk_strdup(q);
    }
  }
  choose(f, (const char *const *)v, k);
  for (int i = 0; i < k; i++) {
    free(v[i]);
  }
  free(v);
  return true;
}

/* file_exec / FILE_OT_execute */
void fb_execute(FBData *f)
{
  bk_strs_free(&f->result);
  if (f->mode == FB_MODE_DIRECTORY) {
    execute_directory(f);
    return;
  }
  const char *typed = f->fname->text ? f->fname->text : "";
  char p[FB_PATH];
  fb_path_resolve(f->dir, typed, p, sizeof(p));
  if (typed[0] && fb_path_is_dir(p)) { /* a typed folder: go there (file_filename_enter_handle) */
    bk_set_text(&f->fname->text, "");
    fb_navigate(f, p, true);
    return;
  }
  if (execute_selection(f)) {
    return;
  }
  if (!typed[0]) { /* no name: a selected folder opens, else there is nothing to do yet */
    const FBEntry *a = fb_vis_entry(f, f->active);
    if (a && a->sel && a->kind != FB_KIND_FILE) {
      fb_open_entry(f, f->active);
    }
    return;
  }
  if (f->mode == FB_MODE_OPEN && !fb_path_exists(p)) {
    snprintf(f->error, sizeof(f->error), "File not found: %s", typed);
    bk_invalidate(f->list);
    return;
  }
  choose(f, (const char *[]){p}, 1);
}

/* Double click: folders open, files execute. */
void fb_open_entry(FBData *f, int i)
{
  const FBEntry *e = fb_vis_entry(f, i);
  char p[FB_PATH];
  if (!e || e->kind == FB_KIND_FILE) {
    fb_execute(f);
    return;
  }
  snprintf(p, sizeof(p), "%s", f->dir);
  if (e->kind == FB_KIND_PARENT) {
    fb_path_parent(p);
  }
  else {
    fb_path_join(p, sizeof(p), f->dir, e->name);
  }
  fb_navigate(f, p, true);
}

/* Return: a selected folder opens (outside folder selection), else execute. */
void fb_activate(FBData *f)
{
  const FBEntry *a = fb_vis_entry(f, f->active);
  if (a && a->sel && a->kind != FB_KIND_FILE && f->mode != FB_MODE_DIRECTORY) {
    fb_open_entry(f, f->active);
  }
  else {
    fb_execute(f);
  }
}

/** \} */
