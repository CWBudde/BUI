/* File browser internals: shared types, limits and layout factors.
 *
 * The browser is ported from Blender's editors/space_file; each file names the upstream
 * source it follows:
 *   kit_filebrowser.c        the composite node, keymap, properties, blocking dialog
 *   kit_fileops.c            navigation, history, execute, new folder, rename (file_ops.cc)
 *   kit_filelist.c           entries, icons, sort, filter (filelist/)
 *   kit_filelist_view.c      the list view (file_draw.cc, filesel.cc)
 *   kit_fsmenu.c             Volumes / System / Bookmarks / Recent (fsmenu.cc)
 *   kit_filebrowser_paths.c  path helpers (BLI_path_utils subset)
 *   kit_filebrowser_posix.c, kit_filebrowser_win32.c   platform parts
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */
#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <time.h>

#include "kit_intern.h"

/* -------------------------------------------------------------------- */
/* Limits and layout (UI units unless noted) */

#define FB_PATH 4096
#define FB_HISTORY 64        /* folder_history.cc depth */
#define FB_RECENT_MAX 10     /* U.recent_dirs */
#define FB_DIALOG_W 50       /* blocking dialog window, units */
#define FB_DIALOG_H 30
#define FB_NATURAL_W 30      /* the node's natural size */
#define FB_NATURAL_H 18
#define FB_SIDE_W 10          /* tools region width from FB_SIDE_AREA_WIDE units of area, */
#define FB_SIDE_W_NARROW 7    /* narrow from FB_SIDE_AREA_NARROW, hidden below */
#define FB_SIDE_AREA_WIDE 24
#define FB_SIDE_AREA_NARROW 14
#define FB_BAR_SCALE 1.3f    /* scale_y of the directory-path and execute rows */
#define FB_MARGIN 0.4f       /* region margin */
#define FB_SPACE 0.25f       /* style->buttonspacex */
#define FB_BUTTON_MIN_W 4    /* Cancel / execute */
#define FB_FILTER_MIN_W 5
#define FB_SEARCH_MIN_W 4
#define FB_SEARCH_MAX_W 8
#define FB_PLACES_PAD 0.2f   /* template_list box padding */
#define FB_WHEEL_ROWS 2      /* rows scrolled per wheel step */

/* -------------------------------------------------------------------- */
/* Types */

typedef enum FBMode { FB_MODE_OPEN = 0, FB_MODE_SAVE, FB_MODE_DIRECTORY } FBMode;
typedef enum FBState { FB_STATE_CANCELLED = -1, FB_STATE_BROWSING = 0, FB_STATE_CHOSEN = 1 } FBState;
typedef enum FBKind { FB_KIND_FILE = 0, FB_KIND_DIR, FB_KIND_PARENT } FBKind;
typedef enum FBSort { FB_SORT_ALPHA = 0, FB_SORT_TIME, FB_SORT_SIZE } FBSort;
typedef enum FBColumn { FB_COL_NAME = 0, FB_COL_DATE, FB_COL_SIZE, FB_COL_NUM } FBColumn;
/* FSMenuCategory, in the tools region's panel order */
typedef enum FBCategory {
  FB_CAT_VOLUMES = 0,
  FB_CAT_SYSTEM,
  FB_CAT_BOOKMARKS,
  FB_CAT_RECENT,
  FB_CAT_NUM
} FBCategory;

/* FileDirEntry, reduced to a plain directory listing. */
typedef struct FBEntry {
  char *name;
  FBKind kind;
  bool hidden, link, sel;
  long long size;
  time_t mtime;
  /* draw_data: filled the first time the row is drawn (fb_entry_details) */
  bool has_details;
  int icon;
  char date[40];
  char size_str[16];
} FBEntry;

/* FSMenuEntry */
typedef struct FBPlace {
  char *path;
  char *label;
  int icon;
} FBPlace;

typedef struct FBFilter {
  char *label;
  char **pats;
  int npats;
} FBFilter;

typedef struct FBData {
  BKNode *self;
  FBMode mode;
  bool multiple;
  FBState state;
  char dir[FB_PATH];
  char *exec_label;
  char error[256];

  /* listing (FileList) */
  FBEntry *all;
  int nall, capall;
  int *vis; /* visible entries: filtered and sorted */
  int nvis;
  int active; /* params->active_file (visible index), -1 none */
  int anchor;
  FBSort sort;
  bool invert;
  char search[256];
  FBFilter *filters; /* the selected one is filter_dd's value */
  int nfilters;
  bool scroller_hot; /* pointer over the list's scroller */
  struct tm today, yesterday; /* for "Today" / "Yesterday" dates, from the last read */

  /* folder_history.cc */
  char *back[FB_HISTORY], *fwd[FB_HISTORY];
  int nback, nfwd;

  /* fsmenu */
  FBPlace *places[FB_CAT_NUM];
  int nplaces[FB_CAT_NUM];
  char bookmarks_path[FB_PATH];

  BKStrs result;

  /* parts; b_hidden's value is the show-hidden setting */
  BKNode *b_prev, *b_next, *b_parent, *b_refresh, *b_newdir, *path, *search_f, *b_hidden;
  BKNode *side, *panels[FB_CAT_NUM], *lists[FB_CAT_NUM];
  BKNode *list;
  BKNode *fname, *filter_dd, *b_cancel, *b_exec;

  /* region rects, screen space */
  BKRect r_side, r_header, r_exec;
} FBData;

/* The browser and its list both keep the FBData in ext. */
#define FB(n) ((FBData *)(n)->ext)

/* -------------------------------------------------------------------- */
/* kit_filebrowser_paths.c */

void fb_path_slash_ensure(char *p, size_t size);
bool fb_path_parent(char *p);
void fb_path_join(char *dst, size_t size, const char *dir, const char *file);
bool fb_path_is_dir(const char *p);
bool fb_path_exists(const char *p);
const char *fb_path_basename(const char *p, char *buf, size_t size);
void fb_path_normalize(char *p, size_t size);
/* A typed name or path: absolute (or "~") as is, else inside dir; normalized. */
void fb_path_resolve(const char *dir, const char *typed, char *out, size_t size);
/* "/a/b/" -> "/a/b" (roots keep their separator). */
void fb_path_strip_slash(char *p);

/* -------------------------------------------------------------------- */
/* kit_filebrowser_posix.c / kit_filebrowser_win32.c */

char fb_os_sep(void);
bool fb_os_is_sep(char c);
size_t fb_os_root_len(const char *p); /* drive prefix ("C:"), 0 on POSIX */
bool fb_os_path_is_absolute(const char *p);
bool fb_os_path_is_root(const char *p);
bool fb_os_getcwd(char *buf, size_t size);
bool fb_os_mkdir(const char *p);
const char *fb_os_home(void);
void fb_os_localtime(time_t t, struct tm *r);
void fb_os_sleep_ms(int ms);
void fb_os_bookmarks_path(char *buf, size_t size);
bool fb_os_read_dir(FBData *f);     /* adds entries; false when the folder can't be read */
void fb_os_read_system(FBData *f);  /* System and Volumes places */

/* -------------------------------------------------------------------- */
/* kit_filelist.c */

FBEntry *fb_entry_add(FBData *f, const char *name);
void fb_entry_details(const FBData *f, FBEntry *e); /* icon, date and size strings, cached */
void fb_entries_free(FBData *f);
void fb_read_dir(FBData *f);
void fb_filter_sort(FBData *f);
FBEntry *fb_vis_entry(const FBData *f, int i);
bool fb_show_hidden(const FBData *f);
int fb_filter_index(const FBData *f);
int fb_count_selected(const FBData *f, bool files_only, int *r_last);
void fb_select_none(FBData *f);
void fb_filters_set(FBData *f, const char *const *v, int count);
void fb_filters_free(FBData *f);

/* -------------------------------------------------------------------- */
/* kit_filelist_view.c */

extern const BKClass bk_cls_fb_list;
void fb_list_see(BKNode *list, int i);
int fb_list_page_rows(const BKNode *list);
void fb_click_select(FBData *f, int i, int mods);
void fb_walk(FBData *f, int to, int mods);
void fb_rename_begin(FBData *f, int i);

/* -------------------------------------------------------------------- */
/* kit_fsmenu.c */

BKNode *fb_places_view_new(FBData *f, FBCategory cat, BKNode *parent);
void fb_places_read(FBData *f); /* system places, then bookmarks.txt */
void fb_place_add(FBData *f, FBCategory cat, const char *path, const char *label,
                  const char *icon, bool sorted);
void fb_places_free(FBData *f);
void fb_read_bookmarks(FBData *f);
void fb_write_bookmarks(FBData *f);
void fb_add_recent(FBData *f, const char *dir);
void fb_bookmark_dir(FBData *f);

/* -------------------------------------------------------------------- */
/* kit_filebrowser.c */

int fb_keymap(FBData *f, const BKEvent *e); /* the browser's keymap; 1 when handled */

/* -------------------------------------------------------------------- */
/* kit_fileops.c */

void fb_navigate(FBData *f, const char *dir, bool history);
void fb_go_back(FBData *f);
void fb_go_forward(FBData *f);
void fb_go_parent(FBData *f);
void fb_refresh(FBData *f);
void fb_new_folder(FBData *f);
bool fb_rename(FBData *f, int i, const char *name);
void fb_open_entry(FBData *f, int i);
void fb_activate(FBData *f);
void fb_execute(FBData *f);
void fb_finish(FBData *f, FBState state);
void fb_update_exec(FBData *f);
void fb_history_free(FBData *f);
