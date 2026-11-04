/* File list data: entries, their icons and detail strings, sorting and filtering
 * (filelist/filelist.cc, filelist_sort.cc, filelist_filter.cc, filelist_readjob_common.cc).
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "kit_filebrowser_intern.h"

/* blenlib (bui_core) */
int BLI_strcasecmp_natural(const char *s1, const char *s2);
void BLI_str_format_byte_unit(char dst[15], long long int bytes, bool base_10);

/* -------------------------------------------------------------------- */
/** \name File types (ED_path_extension_type, ED_file_extension_icon)
 * \{ */

static const char *const g_ext_script[] = {".py", NULL};
static const char *const g_ext_text[] = {
    ".txt", ".glsl", ".osl", ".data", ".pov", ".ini", ".mcr", ".inc", ".fountain", ".toml",
    ".md",  ".json", ".c",   ".h",    ".cc",  ".hh",  ".cpp", ".rs",  ".js",       ".ts",
    ".css", ".html", ".xml", ".yaml", ".yml", ".csv", ".log", ".cfg", NULL};
static const char *const g_ext_font[] = {".ttf", ".pfb", ".otf", ".woff", ".woff2", NULL};
static const char *const g_ext_btx[] = {".btx", NULL};
static const char *const g_ext_volume[] = {".vdb", NULL};
static const char *const g_ext_archive[] = {".zip", ".tar", ".gz", ".xz", ".bz2", ".7z", ".zst",
                                            NULL};
static const char *const g_ext_3d[] = {".abc", ".usd", ".usda", ".usdc", ".usdz", ".obj", ".mtl",
                                       ".3ds", ".fbx", ".glb",  ".gltf", ".svg",  ".pdf", ".ply",
                                       ".stl", NULL};
/* imb_ext_image, imb_ext_movie, imb_ext_audio */
static const char *const g_ext_image[] = {
    ".png", ".tga", ".bmp", ".jpg", ".jpeg", ".sgi", ".rgb", ".rgba", ".tif", ".tiff", ".tx",
    ".jp2", ".j2c", ".hdr", ".dds", ".dpx",  ".cin", ".exr", ".psd",  ".pdd", ".psb",  ".webp",
    ".avif", ".gif", NULL};
static const char *const g_ext_movie[] = {
    ".avi", ".flc", ".mov", ".movie", ".mp4", ".m4v",  ".m2v",  ".m2t",  ".m2ts", ".mts",
    ".ts",  ".mv",  ".avs", ".wmv",   ".ogv", ".ogg",  ".r3d",  ".dv",   ".mpeg", ".mpg",
    ".mpg2", ".vob", ".mkv", ".flv",  ".divx", ".xvid", ".mxf", ".webm", NULL};
static const char *const g_ext_audio[] = {".wav", ".mp2", ".mp3",  ".ac3", ".aac", ".flac", ".wma",
                                          ".eac3", ".aif", ".aiff", ".m4a", ".mka", ".opus", NULL};

/* In ED_path_extension_type's order: the first list that matches wins. */
static const struct {
  const char *const *exts;
  const char *icon;
} g_file_types[] = {
    {g_ext_script, "FILE_SCRIPT"}, {g_ext_text, "FILE_TEXT"},     {g_ext_font, "FILE_FONT"},
    {g_ext_btx, "FILE_BLANK"},     {g_ext_volume, "FILE_VOLUME"}, {g_ext_archive, "FILE_ARCHIVE"},
    {g_ext_3d, "FILE_3D"},         {g_ext_image, "FILE_IMAGE"},   {g_ext_movie, "FILE_MOVIE"},
    {g_ext_audio, "FILE_SOUND"},
};

static bool ascii_ieq(const char *a, const char *b)
{
  for (; *a && *b; a++, b++) {
    const int ca = (*a >= 'A' && *a <= 'Z') ? *a + 32 : *a;
    const int cb = (*b >= 'A' && *b <= 'Z') ? *b + 32 : *b;
    if (ca != cb) {
      return false;
    }
  }
  return *a == *b;
}

static bool ext_in(const char *dot, const char *const *exts)
{
  for (; *exts; exts++) {
    if (ascii_ieq(dot, *exts)) {
      return true;
    }
  }
  return false;
}

static const char *file_icon_name(const char *name)
{
  const char *dot = strrchr(name, '.');
  if (!dot) {
    return "FILE_BLANK";
  }
  if (ascii_ieq(dot, ".blend")) {
    return "FILE_BLEND";
  }
  if (strncmp(dot, ".blend", 6) == 0 && dot[6] >= '0' && dot[6] <= '9') {
    return "FILE_BACKUP"; /* file_is_blend_backup: ".blend1" ... */
  }
  for (size_t i = 0; i < sizeof(g_file_types) / sizeof(g_file_types[0]); i++) {
    if (ext_in(dot, g_file_types[i].exts)) {
      return g_file_types[i].icon;
    }
  }
  return "FILE_BLANK";
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Entries
 * \{ */

/* date_string::datetime with "Today" / "Yesterday" */
static void format_datetime(time_t t, char *buf, size_t size)
{
  struct tm tm, today, yesterday;
  fb_os_localtime(t, &tm);
  fb_os_localtime(time(NULL), &today);
  fb_os_localtime(time(NULL) - 86400, &yesterday);
  char hm[16];
  strftime(hm, sizeof(hm), "%H:%M", &tm);
  const bool is_today = tm.tm_year == today.tm_year && tm.tm_yday == today.tm_yday;
  const bool is_yesterday = tm.tm_year == yesterday.tm_year && tm.tm_yday == yesterday.tm_yday;
  if (is_today || is_yesterday) {
    snprintf(buf, size, "%s %s", is_today ? "Today" : "Yesterday", hm);
  }
  else {
    strftime(buf, size, "%d %b %Y %H:%M", &tm);
  }
}

FBEntry *fb_entry_add(FBData *f, const char *name)
{
  if (f->nall == f->capall) {
    f->capall = f->capall ? f->capall * 2 : 64;
    f->all = (FBEntry *)realloc(f->all, sizeof(FBEntry) * (size_t)f->capall);
  }
  FBEntry *e = &f->all[f->nall++];
  memset(e, 0, sizeof(*e));
  e->name = bk_strdup(name);
  return e;
}

/* filelist_geticon_file_type_ex + filelist_get_details_column_string */
void fb_entry_finish(FBEntry *e)
{
  static const char *const kind_icons[] = {NULL, "FILE_FOLDER", "FILE_PARENT"};
  const char *icon = e->kind == FB_KIND_FILE ? file_icon_name(e->name) :
                     (e->kind == FB_KIND_DIR && e->link) ? "FOLDER_REDIRECT" :
                                                            kind_icons[e->kind];
  e->icon = bk_icons_lookup(icon);
  if (e->kind != FB_KIND_PARENT) {
    format_datetime(e->mtime, e->date, sizeof(e->date));
  }
  if (e->kind == FB_KIND_FILE) {
    BLI_str_format_byte_unit(e->size_str, e->size, true);
  }
}

void fb_entries_free(FBData *f)
{
  for (int i = 0; i < f->nall; i++) {
    free(f->all[i].name);
  }
  f->nall = 0;
  free(f->vis);
  f->vis = NULL;
  f->nvis = 0;
}

void fb_read_dir(FBData *f)
{
  fb_entries_free(f);
  f->error[0] = '\0';
  if (!fb_os_path_is_root(f->dir)) {
    FBEntry *e = fb_entry_add(f, "..");
    e->kind = FB_KIND_PARENT;
    fb_entry_finish(e);
  }
  if (!fb_os_read_dir(f)) {
    snprintf(f->error, sizeof(f->error), "Cannot open directory");
  }
}

FBEntry *fb_vis_entry(const FBData *f, int i)
{
  return (i >= 0 && i < f->nvis) ? &f->all[f->vis[i]] : NULL;
}

bool fb_show_hidden(const FBData *f)
{
  return f->b_hidden && f->b_hidden->value != 0;
}

int fb_filter_index(const FBData *f)
{
  return f->filter_dd ? (int)f->filter_dd->value : 0;
}

/* Selected visible entries (files only, or any but ".."); r_last gets the last one. */
int fb_count_selected(const FBData *f, bool files_only, int *r_last)
{
  int count = 0;
  for (int i = 0; i < f->nvis; i++) {
    const FBEntry *e = fb_vis_entry(f, i);
    if (e->sel && (files_only ? e->kind == FB_KIND_FILE : e->kind != FB_KIND_PARENT)) {
      count++;
      if (r_last) {
        *r_last = i;
      }
    }
  }
  return count;
}

void fb_select_none(FBData *f)
{
  for (int i = 0; i < f->nall; i++) {
    f->all[i].sel = false;
  }
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Sorting (filelist_sort.cc)
 * \{ */

static int cmp_entries(const FBData *f, int ia, int ib)
{
  const FBEntry *a = &f->all[ia], *b = &f->all[ib];
  if (a->kind != b->kind) { /* compare_direntry_generic: "..", then folders, then files */
    return a->kind > b->kind ? -1 : 1;
  }
  int r;
  if (f->sort == FB_SORT_TIME && a->mtime != b->mtime) {
    r = a->mtime < b->mtime ? 1 : -1; /* compare_date: newest first */
  }
  else if (f->sort == FB_SORT_SIZE && a->size != b->size) {
    r = a->size < b->size ? 1 : -1; /* compare_size: largest first */
  }
  else {
    r = BLI_strcasecmp_natural(a->name, b->name); /* compare_tiebreaker */
  }
  return f->invert ? -r : r;
}

/* Stable merge sort of the visible indices, bottom-up (no recursion). */
static void sort_visible(FBData *f)
{
  int *tmp = (int *)malloc(sizeof(int) * (size_t)(f->nvis > 0 ? f->nvis : 1));
  for (int width = 1; width < f->nvis; width *= 2) {
    for (int lo = 0; lo < f->nvis; lo += 2 * width) {
      const int mid = BK_MIN(lo + width, f->nvis), hi = BK_MIN(lo + 2 * width, f->nvis);
      int a = lo, b = mid, o = lo;
      while (a < mid || b < hi) {
        const bool take_a = b >= hi || (a < mid && cmp_entries(f, f->vis[a], f->vis[b]) <= 0);
        tmp[o++] = take_a ? f->vis[a++] : f->vis[b++];
      }
    }
    memcpy(f->vis, tmp, sizeof(int) * (size_t)f->nvis);
  }
  free(tmp);
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Filtering (filelist_filter.cc)
 * \{ */

/* fnmatch() with FNM_CASEFOLD for '*' and '?' (portable). */
static bool glob_match(const char *p, const char *s)
{
  const char *star = NULL, *retry = NULL;
  while (*s) {
    const int cp = (*p >= 'A' && *p <= 'Z') ? *p + 32 : *p;
    const int cs = (*s >= 'A' && *s <= 'Z') ? *s + 32 : *s;
    if (*p == '*') {
      star = p++;
      retry = s;
    }
    else if (*p && (*p == '?' || cp == cs)) {
      p++;
      s++;
    }
    else if (star) { /* let the last '*' take one more character */
      p = star + 1;
      s = ++retry;
    }
    else {
      return false;
    }
  }
  while (*p == '*') {
    p++;
  }
  return *p == '\0';
}

static bool matches_filter(const FBData *f, const FBEntry *e)
{
  const int fi = fb_filter_index(f);
  if (e->kind != FB_KIND_FILE || f->nfilters == 0 || fi < 0 || fi >= f->nfilters) {
    return true;
  }
  const FBFilter *flt = &f->filters[fi];
  bool any = flt->npats == 0;
  for (int i = 0; i < flt->npats && !any; i++) {
    any = glob_match(flt->pats[i], e->name);
  }
  return any;
}

static bool entry_visible(const FBData *f, const FBEntry *e)
{
  char pat[300];
  snprintf(pat, sizeof(pat), "*%s*", f->search); /* filter_search is matched as "*search*" */
  return e->kind == FB_KIND_PARENT ||
         ((!e->hidden || fb_show_hidden(f)) &&               /* is_hidden_dot_filename */
          (f->mode != FB_MODE_DIRECTORY || e->kind == FB_KIND_DIR) &&
          (!f->search[0] || glob_match(pat, e->name)) && matches_filter(f, e));
}

void fb_filter_sort(FBData *f)
{
  free(f->vis);
  f->vis = (int *)malloc(sizeof(int) * (size_t)(f->nall > 0 ? f->nall : 1));
  f->nvis = 0;
  for (int i = 0; i < f->nall; i++) {
    const bool shown = entry_visible(f, &f->all[i]);
    f->all[i].sel &= shown;
    if (shown) {
      f->vis[f->nvis++] = i;
    }
  }
  sort_visible(f);
  f->active = BK_MIN(f->active, f->nvis - 1);
  if (f->list) {
    bk_invalidate(f->list);
  }
}

/* "Label\tpattern pattern ..." (no tab: the text is both); ".py" means "*.py". */
static void parse_filter(FBFilter *flt, const char *s)
{
  const char *tab = strchr(s, '\t');
  const size_t label_len = tab ? (size_t)(tab - s) : strlen(s);
  flt->label = (char *)calloc(label_len + 1, 1);
  memcpy(flt->label, s, label_len);
  for (const char *p = tab ? tab + 1 : s; *p;) {
    p += strspn(p, " ;,");
    const size_t n = strcspn(p, " ;,");
    if (n > 0) {
      flt->pats = (char **)realloc(flt->pats, sizeof(char *) * (size_t)(flt->npats + 1));
      char *pat = (char *)calloc(n + 2, 1);
      snprintf(pat, n + 2, "%s%.*s", p[0] == '.' ? "*" : "", (int)n, p);
      flt->pats[flt->npats++] = pat;
    }
    p += n;
  }
}

void fb_filters_free(FBData *f)
{
  for (int i = 0; i < f->nfilters; i++) {
    free(f->filters[i].label);
    for (int k = 0; k < f->filters[i].npats; k++) {
      free(f->filters[i].pats[k]);
    }
    free(f->filters[i].pats);
  }
  free(f->filters);
  f->filters = NULL;
  f->nfilters = 0;
}

void fb_filters_set(FBData *f, const char *const *v, int count)
{
  fb_filters_free(f);
  f->filters = count > 0 ? (FBFilter *)calloc((size_t)count, sizeof(FBFilter)) : NULL;
  const char **labels = (const char **)calloc((size_t)(count > 0 ? count : 1), sizeof(char *));
  for (int i = 0; i < count; i++) {
    parse_filter(&f->filters[i], v[i] ? v[i] : "");
    labels[i] = f->filters[i].label;
  }
  f->nfilters = BK_MAX(count, 0);
  BK_set_strs(f->filter_dd, "items", labels, f->nfilters);
  f->filter_dd->value = 0;
  free(labels);
}

/** \} */
