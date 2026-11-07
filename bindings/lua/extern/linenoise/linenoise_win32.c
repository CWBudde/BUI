/* The part of linenoise's API the bui-lua REPL uses, for Windows: linenoise itself needs
 * termios.h. This reads a line with fgets and keeps no history, which is all the REPL needs there
 * (the console's own line editing still works).
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "linenoise.h"

char *linenoise(const char *prompt)
{
  size_t cap = 256, len = 0;
  char *buf = (char *)malloc(cap);
  if (!buf) {
    return NULL;
  }
  fputs(prompt, stdout);
  fflush(stdout);
  for (;;) {
    if (!fgets(buf + len, (int)(cap - len), stdin)) {
      if (len == 0) { /* end of input */
        free(buf);
        return NULL;
      }
      break;
    }
    len += strlen(buf + len);
    if (len > 0 && buf[len - 1] == '\n') {
      break;
    }
    char *grown = (char *)realloc(buf, cap * 2); /* a longer line: keep reading */
    if (!grown) {
      free(buf);
      return NULL;
    }
    buf = grown;
    cap *= 2;
  }
  while (len > 0 && (buf[len - 1] == '\n' || buf[len - 1] == '\r')) {
    buf[--len] = '\0';
  }
  return buf;
}

int linenoiseHistoryAdd(const char *line)
{
  (void)line;
  return 0;
}

int linenoiseHistorySetMaxLen(int len)
{
  (void)len;
  return 0;
}

int linenoiseHistorySave(const char *filename)
{
  (void)filename;
  return 0;
}

int linenoiseHistoryLoad(const char *filename)
{
  (void)filename;
  return 0;
}
