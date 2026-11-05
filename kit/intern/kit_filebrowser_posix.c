/* File browser platform parts for Linux, BSD and macOS: directory reading, system folders
 * (fsmenu_system_unix.cc / fsmenu_system_macos.mm), home, time and sleep.
 * Compiles to nothing on Windows (kit_filebrowser_win32.c).
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#ifndef _WIN32

#  include <dirent.h>
#  include <stdio.h>
#  include <stdlib.h>
#  include <string.h>
#  include <sys/stat.h>
#  include <unistd.h>
#  ifdef __linux__
#    include <mntent.h>
#  endif

#  include "kit_filebrowser_intern.h"

char fb_os_sep(void)
{
  return '/';
}

bool fb_os_is_sep(char c)
{
  return c == '/';
}

size_t fb_os_root_len(const char *p)
{
  (void)p;
  return 0;
}

bool fb_os_path_is_absolute(const char *p)
{
  return p[0] == '/';
}

bool fb_os_path_is_root(const char *p)
{
  return p[0] == '/' && p[1] == '\0';
}

bool fb_os_getcwd(char *buf, size_t size)
{
  return getcwd(buf, size) != NULL;
}

bool fb_os_mkdir(const char *p)
{
  return mkdir(p, 0777) == 0;
}

const char *fb_os_home(void)
{
  const char *h = getenv("HOME");
  return (h && h[0]) ? h : NULL;
}

void fb_os_localtime(time_t t, struct tm *r)
{
  localtime_r(&t, r);
}

void fb_os_sleep_ms(int ms)
{
  usleep((useconds_t)ms * 1000);
}

void fb_os_bookmarks_path(char *buf, size_t size)
{
  const char *xch = getenv("XDG_CONFIG_HOME");
  if (xch && xch[0]) {
    snprintf(buf, size, "%s/bui/bookmarks.txt", xch);
  }
  else if (fb_os_home()) {
    snprintf(buf, size, "%s/.config/bui/bookmarks.txt", fb_os_home());
  }
}

/* filelist_readjob_common.cc: stat follows links, lstat tells them apart. */
bool fb_os_read_dir(FBData *f)
{
  DIR *d = opendir(f->dir);
  if (!d) {
    return false;
  }
  struct dirent *de;
  while ((de = readdir(d)) != NULL) {
    if (strcmp(de->d_name, ".") == 0 || strcmp(de->d_name, "..") == 0) {
      continue;
    }
    char full[FB_PATH];
    fb_path_join(full, sizeof(full), f->dir, de->d_name);
    struct stat st, lst;
    const bool ok = stat(full, &st) == 0, lok = lstat(full, &lst) == 0;
    const struct stat *use = ok ? &st : &lst; /* a broken link reports itself */
    FBEntry *e = fb_entry_add(f, de->d_name);
    e->hidden = de->d_name[0] == '.';
    e->link = lok && S_ISLNK(lst.st_mode);
    e->kind = (ok && S_ISDIR(st.st_mode)) ? FB_KIND_DIR : FB_KIND_FILE;
    e->size = (ok || lok) ? (long long)use->st_size : 0;
    e->mtime = (ok || lok) ? use->st_mtime : 0;
  }
  closedir(d);
  return true;
}

/* The six user folders, by their names in $HOME (and XDG keys on Linux). */
static const struct {
  const char *xdg_key, *name, *icon;
} g_user_dirs[] = {
    {"XDG_DESKTOP_DIR", "Desktop", "DESKTOP"},
    {"XDG_DOCUMENTS_DIR", "Documents", "DOCUMENTS"},
    {"XDG_DOWNLOAD_DIR", "Downloads", "IMPORT"},
#  ifdef __APPLE__
    {NULL, "Movies", "FILE_MOVIE"},
#  else
    {"XDG_VIDEOS_DIR", "Videos", "FILE_MOVIE"},
#  endif
    {"XDG_PICTURES_DIR", "Pictures", "FILE_IMAGE"},
    {"XDG_MUSIC_DIR", "Music", "FILE_SOUND"},
};

/* fsmenu_xdg_user_dirs_parse: the KEY="value" line of user-dirs.dirs, "$HOME/" expanded. */
static bool xdg_lookup(const char *home, const char *key, char *out, size_t size)
{
  char cfg[FB_PATH], line[FB_PATH + 128];
  const char *xch = getenv("XDG_CONFIG_HOME");
  if (xch && xch[0]) {
    fb_path_join(cfg, sizeof(cfg), xch, "user-dirs.dirs");
  }
  else {
    snprintf(cfg, sizeof(cfg), "%s/.config/user-dirs.dirs", home);
  }
  FILE *fp = key ? fopen(cfg, "r") : NULL;
  bool found = false;
  const size_t klen = key ? strlen(key) : 0;
  while (fp && !found && fgets(line, sizeof(line), fp)) {
    if (strncmp(line, key, klen) != 0 || line[klen] != '=') {
      continue;
    }
    char *v = line + klen + 1;
    v[strcspn(v, "\r\n")] = '\0';
    const size_t vl = strlen(v);
    if (vl < 2 || v[0] != '"' || v[vl - 1] != '"') {
      continue;
    }
    v[vl - 1] = '\0';
    v++;
    if (strncmp(v, "$HOME/", 6) == 0) {
      snprintf(out, size, "%s/%s", home, v + 6);
    }
    else {
      snprintf(out, size, "%s", v);
    }
    found = true;
  }
  if (fp) {
    fclose(fp);
  }
  return found;
}

static void read_user_dirs(FBData *f, const char *home)
{
  fb_place_add(f, FB_CAT_SYSTEM, home, "Home", "HOME", false);
  for (size_t i = 0; i < sizeof(g_user_dirs) / sizeof(g_user_dirs[0]); i++) {
    char p[FB_PATH];
    if (!xdg_lookup(home, g_user_dirs[i].xdg_key, p, sizeof(p))) {
      fb_path_join(p, sizeof(p), home, g_user_dirs[i].name);
    }
    if (fb_path_is_dir(p)) {
      fb_place_add(f, FB_CAT_SYSTEM, p, g_user_dirs[i].name, g_user_dirs[i].icon, false);
    }
  }
}

/* Same as STRPREFIX, but only at a separator or the end. */
static bool dir_prefix(const char *a, const char *b)
{
  const size_t n = strlen(b);
  return strncmp(a, b, n) == 0 && (a[n] == '\0' || a[n] == '/');
}

static bool read_volumes(FBData *f)
{
  bool found = false;
#  ifdef __linux__
  /* Mount points of /dev devices, without /boot, /efi and snap loop devices. */
  FILE *fp = setmntent("/proc/mounts", "r");
  struct mntent *mnt;
  while (fp && (mnt = getmntent(fp)) != NULL) {
    if (dir_prefix(mnt->mnt_dir, "/boot") || dir_prefix(mnt->mnt_dir, "/efi") ||
        !dir_prefix(mnt->mnt_fsname, "/dev") || strncmp(mnt->mnt_fsname, "/dev/loop", 9) == 0)
    {
      continue;
    }
    fb_place_add(f, FB_CAT_VOLUMES, mnt->mnt_dir, NULL, "DISK_DRIVE", true);
    found = true;
  }
  if (fp) {
    endmntent(fp);
  }
#  elif defined(__APPLE__)
  DIR *d = opendir("/Volumes");
  struct dirent *de;
  while (d && (de = readdir(d)) != NULL) {
    char p[FB_PATH];
    fb_path_join(p, sizeof(p), "/Volumes", de->d_name);
    if (de->d_name[0] != '.') {
      fb_place_add(f, FB_CAT_VOLUMES, p, de->d_name, "DISK_DRIVE", true);
    }
  }
  if (d) {
    closedir(d);
  }
#  else
  (void)dir_prefix;
#  endif
  return found;
}

/* fsmenu_read_system */
void fb_os_read_system(FBData *f)
{
  if (fb_os_home()) {
    read_user_dirs(f, fb_os_home());
  }
  if (!read_volumes(f)) {
    fb_place_add(f, FB_CAT_VOLUMES, "/", NULL, "DISK_DRIVE", true);
  }
}

#endif /* !_WIN32 */
