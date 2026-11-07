/* Linux window host selection: Wayland or X11, once, on the first call.
 *
 * Like GHOST_ISystem::createSystem, Wayland is tried first and X11 is the fallback: a session
 * with WAYLAND_DISPLAY and a compositor that answers gets Wayland windows, anything else X11.
 * Two cases keep X11 on a Wayland session (XWayland), because that is where the GPU renderer
 * works today (OpenGL through GLX; the Wayland host draws on the CPU):
 *   - the GPU renderer is on (BUI_GPU=1 or BUI_gpu_set_default(1) before the first window call),
 *   - BUI_WINDOW_BACKEND=x11.
 * BUI_WINDOW_BACKEND=wayland asks for Wayland even then.
 *
 * Only built with Wayland support (kit_window_linux.h).
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include "kit_window_linux.h"

#ifdef BUI_WINDOW_WAYLAND

#  include <stdlib.h>
#  include <string.h>

#  include "GPU_raster.h"

enum { BACKEND_UNDECIDED = 0, BACKEND_X11, BACKEND_WAYLAND };
static int g_backend = BACKEND_UNDECIDED;

/* Decided by the first call that needs a window host. BK_window_supported does not decide: the
 * GPU renderer can still be asked for after it (the Rust App sets it right after), and that
 * changes the answer. */
static int backend(void)
{
  if (g_backend == BACKEND_UNDECIDED) {
    const char *force = getenv("BUI_WINDOW_BACKEND");
    bool wayland;
    if (force && strcmp(force, "wayland") == 0) {
      wayland = true;
    }
    else if (force && strcmp(force, "x11") == 0) {
      wayland = false;
    }
    else {
      const char *display = getenv("WAYLAND_DISPLAY");
      wayland = display && display[0] && !BUI_gpu_default();
    }
    g_backend = (wayland && bk_wl_window_supported()) ? BACKEND_WAYLAND : BACKEND_X11;
  }
  return g_backend;
}

#  define BK_WINDOW_DISPATCH(ret, name, params, args) \
    ret BK_window_##name params \
    { \
      return backend() == BACKEND_WAYLAND ? bk_wl_window_##name args : bk_x11_window_##name args; \
    }
BK_WINDOW_FUNCS(BK_WINDOW_DISPATCH)
#  undef BK_WINDOW_DISPATCH

/* Whether windows can be opened at all: either host will do. */
int BK_window_supported(void)
{
  return g_backend == BACKEND_WAYLAND ? 1 :
         g_backend == BACKEND_X11     ? bk_x11_window_supported() :
                                        (bk_x11_window_supported() || bk_wl_window_supported());
}

/* Process-wide state goes to both hosts (see kit_window_linux.h). */
void BK_window_quit(void)
{
  bk_x11_window_quit();
  bk_wl_window_quit();
}

void BK_window_set_stop_check(int (*stop)(void))
{
  bk_x11_window_set_stop_check(stop);
  bk_wl_window_set_stop_check(stop);
}

void BK_window_set_keep_alive(int on)
{
  bk_x11_window_set_keep_alive(on);
  bk_wl_window_set_keep_alive(on);
}

int BK_window_wake_init(void (*cb)(void))
{
  const int x11 = bk_x11_window_wake_init(cb);
  return bk_wl_window_wake_init(cb) && x11;
}

void BK_window_wake(void)
{
  if (g_backend != BACKEND_WAYLAND) {
    bk_x11_window_wake();
  }
  if (g_backend != BACKEND_X11) { /* undecided: either loop may be the one that runs */
    bk_wl_window_wake();
  }
}

int bk_window_wake_ready(void)
{
  const int x11 = bk_x11_window_wake_ready();
  return bk_wl_window_wake_ready() && x11;
}

#endif /* BUI_WINDOW_WAYLAND */
