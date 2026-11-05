/* File browser platform parts for Windows: directory reading, system folders and drives
 * (fsmenu_system_win32.cc), home, time and sleep. Compiles to nothing elsewhere
 * (kit_filebrowser_posix.c).
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#ifdef _WIN32

#  ifndef WIN32_LEAN_AND_MEAN
#    define WIN32_LEAN_AND_MEAN
#  endif
#  include <direct.h>
#  include <stdio.h>
#  include <stdlib.h>
#  include <string.h>
#  include <windows.h>

#  include "kit_filebrowser_intern.h"

char fb_os_sep(void)
{
  return '\\';
}

bool fb_os_is_sep(char c)
{
  return c == '/' || c == '\\';
}

size_t fb_os_root_len(const char *p)
{
  return (p[0] && p[1] == ':') ? 2 : 0;
}

bool fb_os_path_is_absolute(const char *p)
{
  return fb_os_root_len(p) == 2 || (fb_os_is_sep(p[0]) && fb_os_is_sep(p[1]));
}

bool fb_os_path_is_root(const char *p)
{
  return strlen(p) <= 3 && fb_os_root_len(p) == 2;
}

bool fb_os_getcwd(char *buf, size_t size)
{
  return _getcwd(buf, (int)size) != NULL;
}

bool fb_os_mkdir(const char *p)
{
  return _mkdir(p) == 0;
}

const char *fb_os_home(void)
{
  const char *h = getenv("HOME");
  if (!h || !h[0]) {
    h = getenv("USERPROFILE");
  }
  return (h && h[0]) ? h : NULL;
}

void fb_os_localtime(time_t t, struct tm *r)
{
  localtime_s(r, &t);
}

void fb_os_sleep_ms(int ms)
{
  Sleep((DWORD)ms);
}

void fb_os_bookmarks_path(char *buf, size_t size)
{
  const char *base = getenv("APPDATA");
  if (base) {
    snprintf(buf, size, "%s\\bui\\bookmarks.txt", base);
  }
}

static time_t filetime_to_time(FILETIME ft)
{
  const unsigned long long t = ((unsigned long long)ft.dwHighDateTime << 32) | ft.dwLowDateTime;
  return (time_t)((t - 116444736000000000ULL) / 10000000ULL); /* 100 ns since 1601 */
}

bool fb_os_read_dir(FBData *f)
{
  char pattern[FB_PATH];
  wchar_t wpat[FB_PATH];
  fb_path_join(pattern, sizeof(pattern), f->dir, "*");
  MultiByteToWideChar(CP_UTF8, 0, pattern, -1, wpat, FB_PATH);
  WIN32_FIND_DATAW fd;
  HANDLE h = FindFirstFileW(wpat, &fd);
  if (h == INVALID_HANDLE_VALUE) {
    return false;
  }
  do {
    char name[1024];
    WideCharToMultiByte(CP_UTF8, 0, fd.cFileName, -1, name, sizeof(name), NULL, NULL);
    if (strcmp(name, ".") == 0 || strcmp(name, "..") == 0) {
      continue;
    }
    FBEntry *e = fb_entry_add(f, name);
    e->kind = (fd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) ? FB_KIND_DIR : FB_KIND_FILE;
    e->hidden = (fd.dwFileAttributes & FILE_ATTRIBUTE_HIDDEN) != 0 || name[0] == '.';
    e->link = (fd.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT) != 0;
    e->size = ((long long)fd.nFileSizeHigh << 32) | fd.nFileSizeLow;
    e->mtime = filetime_to_time(fd.ftLastWriteTime);
  } while (FindNextFileW(h, &fd));
  FindClose(h);
  return true;
}

static const struct {
  const char *name, *icon;
} g_user_dirs[] = {
    {"Desktop", "DESKTOP"},     {"Documents", "DOCUMENTS"},   {"Downloads", "IMPORT"},
    {"Music", "FILE_SOUND"},    {"Pictures", "FILE_IMAGE"},   {"Videos", "FILE_MOVIE"},
};

static const char *drive_icon(UINT type)
{
  switch (type) {
    case DRIVE_REMOTE:
      return "NETWORK_DRIVE";
    case DRIVE_REMOVABLE:
      return "EXTERNAL_DRIVE";
    default:
      return "DISK_DRIVE";
  }
}

/* fsmenu_read_system */
void fb_os_read_system(FBData *f)
{
  const char *home = fb_os_home();
  if (home) {
    fb_place_add(f, FB_CAT_SYSTEM, home, "Home", "HOME", false);
    for (size_t i = 0; i < sizeof(g_user_dirs) / sizeof(g_user_dirs[0]); i++) {
      char p[FB_PATH];
      fb_path_join(p, sizeof(p), home, g_user_dirs[i].name);
      if (fb_path_is_dir(p)) {
        fb_place_add(f, FB_CAT_SYSTEM, p, g_user_dirs[i].name, g_user_dirs[i].icon, false);
      }
    }
  }
  char drives[512];
  const DWORD n = GetLogicalDriveStringsA(sizeof(drives), drives);
  for (const char *d = drives; n && *d; d += strlen(d) + 1) {
    fb_place_add(f, FB_CAT_VOLUMES, d, d, drive_icon(GetDriveTypeA(d)), false);
  }
}

#endif /* _WIN32 */
