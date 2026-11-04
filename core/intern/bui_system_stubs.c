/* Stub replacements for OS services used by the converted blenlib core.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#include <stdio.h>
#include <stdlib.h>

#include "BLI_system.h"

#ifdef _WIN32
#  define WIN32_LEAN_AND_MEAN
#  include <windows.h>
#else
#  include <time.h>
#  include <unistd.h> /* sysconf */
#endif

void BLI_system_backtrace(FILE *fp)
{
  /* Portable builds get no backtrace; hook a platform impl here later. */
  fprintf(fp, "(backtrace not available)\n");
}

void BLI_system_backtrace_with_os_info(FILE *fp, const void *os_info)
{
  (void)os_info;
  BLI_system_backtrace(fp);
}

void BLI_system_sleep(int duration_ms)
{
#ifdef _WIN32
  Sleep(duration_ms);
#else
  struct timespec req;
  req.tv_sec = duration_ms / 1000;
  req.tv_nsec = (duration_ms % 1000) * 1000000L;
  nanosleep(&req, NULL);
#endif
}

int BLI_system_memory_max_in_megabytes_int(void)
{
#ifdef _WIN32
  MEMORYSTATUSEX state;
  state.dwLength = sizeof(state);
  if (GlobalMemoryStatusEx(&state)) {
    return (int)(state.ullTotalPhys >> 20);
  }
  return 0;
#else
  long pages = sysconf(_SC_PHYS_PAGES);
  long page_size = sysconf(_SC_PAGE_SIZE);
  return (pages > 0 && page_size > 0) ? (int)((long long)pages * page_size >> 20) : 0;
#endif
}
