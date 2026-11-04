/* File browser places: Volumes, System, Bookmarks and Recent (fsmenu.cc), read from and written
 * to a bookmarks.txt in fsmenu's format, and the tools-region list that shows them
 * (template_list over FILEBROWSER_UL_dir).
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "kit_filebrowser_intern.h"

int BLI_strcasecmp_natural(const char *s1, const char *s2); /* blenlib (bui_core) */

static int u(void)
{
  return bk_unit();
}

/* -------------------------------------------------------------------- */
/** \name Places (fsmenu_insert_entry, fsmenu_remove_entry)
 * \{ */

static int place_find(const FBData *f, FBCategory cat, const char *path)
{
  for (int i = 0; i < f->nplaces[cat]; i++) {
    if (strcmp(f->places[cat][i].path, path) == 0) {
      return i;
    }
  }
  return -1;
}

static void place_insert(FBData *f, FBCategory cat, int at, const char *path, const char *label,
                         const char *icon)
{
  char base[256];
  f->places[cat] = (FBPlace *)realloc(f->places[cat],
                                      sizeof(FBPlace) * (size_t)(f->nplaces[cat] + 1));
  memmove(&f->places[cat][at + 1], &f->places[cat][at],
          sizeof(FBPlace) * (size_t)(f->nplaces[cat] - at));
  FBPlace *pl = &f->places[cat][at];
  pl->path = bk_strdup(path);
  pl->label = bk_strdup(label                    ? label :
                        fb_os_path_is_root(path) ? path :
                                                   fb_path_basename(path, base, sizeof(base)));
  pl->icon = bk_icons_lookup(icon);
  f->nplaces[cat]++;
}

static void place_remove(FBData *f, FBCategory cat, int i)
{
  free(f->places[cat][i].path);
  free(f->places[cat][i].label);
  memmove(&f->places[cat][i], &f->places[cat][i + 1],
          sizeof(FBPlace) * (size_t)(f->nplaces[cat] - i - 1));
  f->nplaces[cat]--;
}

static void places_clear(FBData *f, FBCategory cat)
{
  while (f->nplaces[cat] > 0) {
    place_remove(f, cat, f->nplaces[cat] - 1);
  }
}

/* FS_INSERT_LAST or FS_INSERT_SORTED; a path that is listed already is not added again. */
void fb_place_add(FBData *f, FBCategory cat, const char *path, const char *label,
                  const char *icon, bool sorted)
{
  char p[FB_PATH];
  snprintf(p, sizeof(p), "%s", path);
  fb_path_slash_ensure(p, sizeof(p));
  if (place_find(f, cat, p) >= 0) {
    return;
  }
  int at = f->nplaces[cat];
  while (sorted && at > 0 && BLI_strcasecmp_natural(f->places[cat][at - 1].path, p) > 0) {
    at--;
  }
  place_insert(f, cat, at, p, label, icon);
}

void fb_places_free(FBData *f)
{
  for (int c = 0; c < FB_CAT_NUM; c++) {
    places_clear(f, (FBCategory)c);
    free(f->places[c]);
    f->places[c] = NULL;
  }
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name bookmarks.txt (fsmenu_read_bookmarks, fsmenu_write_file)
 * \{ */

static const char *const g_sections[FB_CAT_NUM] = {NULL, NULL, "[Bookmarks]", "[Recent]"};
static const char *const g_icons[FB_CAT_NUM] = {NULL, NULL, "BOOKMARKS", "FILE_FOLDER"};

void fb_read_bookmarks(FBData *f)
{
  places_clear(f, FB_CAT_BOOKMARKS);
  places_clear(f, FB_CAT_RECENT);
  FILE *fp = f->bookmarks_path[0] ? fopen(f->bookmarks_path, "r") : NULL;
  char line[FB_PATH];
  int cat = -1;
  while (fp && fgets(line, sizeof(line), fp)) {
    line[strcspn(line, "\r\n")] = '\0';
    const bool bookmarks = strcmp(line, g_sections[FB_CAT_BOOKMARKS]) == 0;
    const bool recent = strcmp(line, g_sections[FB_CAT_RECENT]) == 0;
    cat = bookmarks ? FB_CAT_BOOKMARKS : (recent ? FB_CAT_RECENT : cat);
    /* "!#!name" lines are labels; paths that are gone are skipped */
    if (!bookmarks && !recent && cat >= 0 && line[0] && strncmp(line, "!#!", 3) != 0 &&
        fb_path_is_dir(line))
    {
      fb_place_add(f, (FBCategory)cat, line, NULL, g_icons[cat], false);
    }
  }
  if (fp) {
    fclose(fp);
  }
}

void fb_write_bookmarks(FBData *f)
{
  char dir[FB_PATH];
  snprintf(dir, sizeof(dir), "%s", f->bookmarks_path);
  if (!f->bookmarks_path[0] || !fb_path_parent(dir)) {
    return;
  }
  if (!fb_path_is_dir(dir)) {
    fb_os_mkdir(dir); /* the config folder, one level (BKE_appdir_folder_id_create) */
  }
  FILE *fp = fopen(f->bookmarks_path, "w");
  for (int c = FB_CAT_BOOKMARKS; fp && c <= FB_CAT_RECENT; c++) {
    fprintf(fp, "%s\n", g_sections[c]);
    for (int i = 0; i < f->nplaces[c]; i++) {
      fprintf(fp, "%s\n", f->places[c][i].path);
    }
  }
  if (fp) {
    fclose(fp);
  }
}

static void default_bookmarks_path(FBData *f)
{
  const char *env = getenv("BUI_BOOKMARKS");
  if (env && env[0]) {
    snprintf(f->bookmarks_path, sizeof(f->bookmarks_path), "%s", env);
  }
  else {
    fb_os_bookmarks_path(f->bookmarks_path, sizeof(f->bookmarks_path));
  }
}

/* FS_INSERT_SAVE into Recent: first, once, at most U.recent_dirs entries. */
void fb_add_recent(FBData *f, const char *dir)
{
  const int old = place_find(f, FB_CAT_RECENT, dir);
  if (old >= 0) {
    place_remove(f, FB_CAT_RECENT, old);
  }
  place_insert(f, FB_CAT_RECENT, 0, dir, NULL, g_icons[FB_CAT_RECENT]);
  while (f->nplaces[FB_CAT_RECENT] > FB_RECENT_MAX) {
    place_remove(f, FB_CAT_RECENT, f->nplaces[FB_CAT_RECENT] - 1);
  }
}

/* file.bookmark_add */
void fb_bookmark_dir(FBData *f)
{
  fb_place_add(f, FB_CAT_BOOKMARKS, f->dir, NULL, g_icons[FB_CAT_BOOKMARKS], false);
  fb_write_bookmarks(f);
  bk_relayout(f->self);
  bk_invalidate(f->side);
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name The tools-region list
 * \{ */

typedef struct FBPlacesView {
  FBData *f;
  FBCategory cat;
} FBPlacesView;

#define PV(n) ((FBPlacesView *)(n)->ext)

static int pad(void)
{
  return (int)(FB_PLACES_PAD * u());
}

static void pv_measure(BKNode *n, int *rw, int *rh)
{
  *rw = 6 * u();
  *rh = BK_MAX(1, PV(n)->f->nplaces[PV(n)->cat]) * u() + 2 * pad();
}

static int pv_row_at(BKNode *n, int y)
{
  const int i = (y - n->y - pad()) / u();
  return (y >= n->y + pad() && i < PV(n)->f->nplaces[PV(n)->cat]) ? i : -1;
}

static void pv_draw(BKNode *n)
{
  const FBData *f = PV(n)->f;
  const FBPlace *places = f->places[PV(n)->cat];
  BKRect r = bk_node_rect(n);
  BKWCol box = bk_theme.box;
  bk_widget_box(&box, &r, CNR_ALL, NULL);
  for (int i = 0; i < f->nplaces[PV(n)->cat]; i++) {
    BKRect row = bk_rect(n->x + pad(), n->y + pad() + i * u(), n->w - 2 * pad(), u());
    const bool active = strcmp(places[i].path, f->dir) == 0; /* the current folder */
    const int st = (active ? ST_SELECT : 0) | (i == n->hot && bk_hovered(n) ? ST_HOVER : 0);
    BKWCol w;
    bk_widget_state(&w, &bk_theme.list_item, st & ST_SELECT);
    bk_widget_list_itembut(&w, &(BKRect){row.xmin, row.ymin, row.xmax, row.ymax}, st);
    bk_text_icon(&row, places[i].label, places[i].icon,
                 active ? bk_theme.list_item.text_sel : bk_theme.list_item.text, BK_ALIGN_LEFT,
                 true);
  }
}

static int pv_move(BKNode *n, BKEvent *e)
{
  const int i = pv_row_at(n, e->y);
  if (i != n->hot) {
    n->hot = i;
    bk_set_text(&n->desc, i >= 0 ? PV(n)->f->places[PV(n)->cat][i].path : NULL); /* tooltip */
    bk_invalidate(n);
  }
  return 0;
}

static int pv_leave(BKNode *n)
{
  n->hot = -1;
  bk_invalidate(n);
  return 0;
}

static int pv_press(BKNode *n, BKEvent *e)
{
  char path[FB_PATH];
  const int i = pv_row_at(n, e->y);
  if (i >= 0) {
    snprintf(path, sizeof(path), "%s", PV(n)->f->places[PV(n)->cat][i].path);
    fb_navigate(PV(n)->f, path, true);
  }
  return 1;
}

/* file.bookmark_delete / file.reset_recent on the hovered row: Delete or X. */
static int pv_key(BKNode *n, BKEvent *e)
{
  FBPlacesView *pv = PV(n);
  const bool removable = pv->cat == FB_CAT_BOOKMARKS || pv->cat == FB_CAT_RECENT;
  if (!removable || n->hot < 0 ||
      !(bk_keysym_is(e->keysym, "Delete") || bk_keysym_is(e->keysym, "x")))
  {
    return 0; /* bubbles up to the browser's keymap */
  }
  place_remove(pv->f, pv->cat, n->hot);
  n->hot = -1;
  fb_write_bookmarks(pv->f);
  bk_relayout(n);
  bk_invalidate(n);
  return 1;
}

static void pv_free(BKNode *n)
{
  free(n->ext);
  n->ext = NULL;
}

static const BKClass bk_cls_fb_places = {.name = "fsmenu",
                                  .flags = CLS_INTERACTIVE,
                                  .free_ext = pv_free,
                                  .measure = pv_measure,
                                  .draw = pv_draw,
                                  .press = pv_press,
                                  .move = pv_move,
                                  .leave = pv_leave,
                                  .key = pv_key};

/* One list per category, in the panels' bodies. */
BKNode *fb_places_view_new(FBData *f, FBCategory cat, BKNode *parent)
{
  BKNode *n = bk_node_new_cls(&bk_cls_fb_places);
  FBPlacesView *pv = (FBPlacesView *)calloc(1, sizeof(FBPlacesView));
  pv->f = f;
  pv->cat = cat;
  n->ext = pv;
  n->hot = -1;
  bk_node_add(parent, n);
  return n;
}

/* fsmenu_read_system + fsmenu_read_bookmarks */
void fb_places_read(FBData *f)
{
  fb_os_read_system(f);
  default_bookmarks_path(f);
  fb_read_bookmarks(f);
}

/** \} */
