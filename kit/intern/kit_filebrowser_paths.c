/* File browser paths: the BLI_path_utils subset the browser needs. Platform rules (separators,
 * drive prefixes, the working folder) come from kit_filebrowser_posix.c / _win32.c.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <stdio.h>
#include <string.h>
#include <sys/stat.h>

#include "kit_filebrowser_intern.h"

/* BLI_path_slash_ensure */
void fb_path_slash_ensure(char *p, size_t size)
{
  const size_t len = strlen(p);
  if ((len == 0 || !fb_os_is_sep(p[len - 1])) && len + 1 < size) {
    p[len] = fb_os_sep();
    p[len + 1] = '\0';
  }
}

void fb_path_strip_slash(char *p)
{
  size_t len = strlen(p);
  while (len > 1 && fb_os_is_sep(p[len - 1]) && !fb_os_path_is_root(p)) {
    p[--len] = '\0';
  }
}

/* BLI_path_parent_dir: "/a/b/" -> "/a/" */
bool fb_path_parent(char *p)
{
  if (fb_os_path_is_root(p)) {
    return false;
  }
  size_t len = strlen(p);
  while (len > 0 && fb_os_is_sep(p[len - 1])) {
    len--;
  }
  while (len > 0 && !fb_os_is_sep(p[len - 1])) {
    len--;
  }
  if (len == 0) {
    return false;
  }
  p[len] = '\0';
  return true;
}

void fb_path_join(char *dst, size_t size, const char *dir, const char *file)
{
  const size_t len = strlen(dir);
  const bool sep = len > 0 && !fb_os_is_sep(dir[len - 1]);
  snprintf(dst, size, "%s%s%s", dir, sep ? (char[2]){fb_os_sep(), '\0'} : "", file);
}

bool fb_path_is_dir(const char *p)
{
  struct stat st;
  return stat(p, &st) == 0 && (st.st_mode & S_IFMT) == S_IFDIR;
}

bool fb_path_exists(const char *p)
{
  struct stat st;
  return stat(p, &st) == 0;
}

/* The last component, ignoring a trailing separator. */
const char *fb_path_basename(const char *p, char *buf, size_t size)
{
  size_t len = strlen(p);
  while (len > 1 && fb_os_is_sep(p[len - 1])) {
    len--;
  }
  size_t s = len;
  while (s > 0 && !fb_os_is_sep(p[s - 1])) {
    s--;
  }
  snprintf(buf, size, "%.*s", (int)(len - s), p + s);
  return buf;
}

/* "~" and relative paths become absolute. */
static void path_absolute(char *p, size_t size)
{
  char tmp[FB_PATH];
  const char *home = fb_os_home();
  if (p[0] == '~' && (p[1] == '\0' || fb_os_is_sep(p[1])) && home) {
    snprintf(tmp, sizeof(tmp), "%s%s", home, p + 1);
    snprintf(p, size, "%s", tmp);
  }
  char cwd[FB_PATH];
  if (!fb_os_path_is_absolute(p) && fb_os_getcwd(cwd, sizeof(cwd))) {
    fb_path_join(tmp, sizeof(tmp), cwd, p);
    snprintf(p, size, "%s", tmp);
  }
}

/* Collapse separators, "." and "..", keeping the drive prefix; the root keeps its separator. */
static void path_collapse(char *p, size_t size)
{
  char out[FB_PATH];
  const size_t root = fb_os_root_len(p);
  size_t o = root, i = root;
  memcpy(out, p, root);
  while (p[i] && o + 2 < sizeof(out)) {
    while (fb_os_is_sep(p[i])) {
      i++;
    }
    size_t j = i;
    while (p[j] && !fb_os_is_sep(p[j])) {
      j++;
    }
    const size_t n = j - i;
    if (n == 2 && p[i] == '.' && p[i + 1] == '.') {
      while (o > root && out[o - 1] != fb_os_sep()) {
        o--;
      }
      o = o > root + 1 ? o - 1 : root; /* drop the separator before the removed name */
    }
    else if (n > 0 && !(n == 1 && p[i] == '.') && o + n + 2 < sizeof(out)) {
      out[o++] = fb_os_sep();
      memcpy(out + o, p + i, n);
      o += n;
    }
    i = j;
  }
  if (o == root) {
    out[o++] = fb_os_sep();
  }
  out[o] = '\0';
  snprintf(p, size, "%s", out);
}

/* BLI_path_normalize, made absolute. */
void fb_path_normalize(char *p, size_t size)
{
  path_absolute(p, size);
  path_collapse(p, size);
}

void fb_path_resolve(const char *dir, const char *typed, char *out, size_t size)
{
  if (fb_os_path_is_absolute(typed) || typed[0] == '~') {
    snprintf(out, size, "%s", typed);
  }
  else {
    fb_path_join(out, size, dir, typed);
  }
  fb_path_normalize(out, size);
}
