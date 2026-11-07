/* GPU start and window frames:
 *  - offscreen: the GPU device starts on a worker thread, BUI_raster_gpu_pending
 *    reports it, and BUI_raster_gpu_poll switches the raster when it is ready;
 *  - an idle window (no input, no animation) switches to the GPU;
 *  - a frame callback that closes its window ends the loop without an X error;
 *  - a window that closes while its GPU device starts releases the device first, so
 *    Mesa prints no "failed to create drawable" (CMakeLists.txt fails the test on it).
 * Each check passes, fails or is skipped (no GPU, no display). Exits 77 when
 * all checks are skipped.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */
#include <stdio.h>
#ifdef _WIN32
#  define WIN32_LEAN_AND_MEAN
#  include <windows.h>
#  define sleep_ms(ms) Sleep(ms)
#else
#  include <unistd.h>
#  define sleep_ms(ms) usleep((ms) * 1000)
#endif

#include "BUI_kit.h"
#include "GPU_raster.h"

enum { PASS = 0, FAIL = 1, SKIP = 77 };

static int offscreen_start(void)
{
  BuiRaster *r = BUI_raster_create(64, 64);
  if (!BUI_raster_gpu_attach(r, NULL, 0)) {
    BUI_raster_free(r);
    return SKIP; /* a build without the GPU renderer */
  }
  const int was_pending = BUI_raster_gpu_pending(r);
  int switched = 0;
  for (int i = 0; i < 1000 && BUI_raster_gpu_pending(r); i++) {
    switched |= BUI_raster_gpu_poll(r);
    sleep_ms(5);
  }
  const int pending = BUI_raster_gpu_pending(r);
  const int gpu = BUI_raster_is_gpu(r);
  BUI_raster_free(r);
  if (!pending && !gpu) {
    return SKIP; /* no GPU device on this machine */
  }
  printf("offscreen: pending at start %d, switched %d, pending at end %d\n", was_pending, switched,
         pending);
  return was_pending && switched && gpu && !pending ? PASS : FAIL;
}

static void quit_on_timer(void *data, BKWindow *win, int id)
{
  (void)data;
  (void)win;
  (void)id;
  BK_window_quit();
}

static int idle_window_switch(void)
{
  BKNode *root = BK_node_new("column");
  BKContext *ctx = BK_context_new(root);
  BKWindow *win = BK_window_new(ctx, "BUI GPU idle switch", 320, 200);
  if (!win) {
    BK_context_free(ctx);
    BK_node_free(root);
    return FAIL;
  }
  BKWindowCallbacks cb = {NULL, quit_on_timer, NULL, NULL};
  BK_window_set_callbacks(win, &cb);
  BK_window_set_gpu(win, 1);
  BK_window_show(win);
  BK_window_timer(win, 2000); /* no input and no animation until then */
  BK_window_run();
  const int gpu = BK_window_gpu(win);
  printf("idle window: gpu %d\n", gpu);
  BK_window_free(win);
  BK_context_free(ctx);
  BK_node_free(root);
  return gpu ? PASS : FAIL;
}

static void close_on_frame(void *data, BKWindow *win)
{
  (void)data;
  BK_window_close(win);
}

static int close_in_frame_callback(void)
{
  BKNode *root = BK_node_new("column");
  BKContext *ctx = BK_context_new(root);
  BKWindow *win = BK_window_new(ctx, "BUI close in frame", 320, 200);
  if (!win) {
    BK_context_free(ctx);
    BK_node_free(root);
    return FAIL;
  }
  BKWindowCallbacks cb = {NULL, quit_on_timer, NULL, close_on_frame};
  BK_window_set_callbacks(win, &cb);
  BK_window_show(win);
  BK_window_request_frame(win, 1);
  BK_window_timer(win, 3000); /* ends the loop if the frame never runs */
  BK_window_run();
  const int open = BK_window_is_open(win);
  printf("close in frame callback: open %d\n", open);
  BK_window_free(win);
  /* X errors arrive later: run a second window, so a pending error ends the process here */
  BKWindow *probe = BK_window_new(ctx, "BUI close in frame (probe)", 160, 100);
  if (probe) {
    BK_window_set_callbacks(probe, &cb);
    BK_window_show(probe);
    BK_window_timer(probe, 300);
    BK_window_run();
    BK_window_free(probe);
  }
  BK_context_free(ctx);
  BK_node_free(root);
  return open ? FAIL : PASS;
}

static int close_before_gpu_ready(void)
{
  BKNode *root = BK_node_new("column");
  BKContext *ctx = BK_context_new(root);
  BKWindow *win = BK_window_new(ctx, "BUI close before GPU", 320, 200);
  if (!win) {
    BK_context_free(ctx);
    BK_node_free(root);
    return FAIL;
  }
  BK_window_set_gpu(win, 1);
  BK_window_show(win);
  BK_window_close(win); /* the device is still starting on its worker thread */
  const int open = BK_window_is_open(win);
  BK_window_free(win);
  BK_context_free(ctx);
  BK_node_free(root);
  return open ? FAIL : PASS;
}

int main(void)
{
  const int display = BK_window_supported();
  const int offscreen = offscreen_start();
  const int results[] = {
      offscreen,
      offscreen == PASS && display ? idle_window_switch() : SKIP,
      display ? close_in_frame_callback() : SKIP,
      offscreen == PASS && display ? close_before_gpu_ready() : SKIP,
  };
  const char *names[] = {"offscreen start", "idle window switch", "close in frame callback",
                         "close before GPU ready"};
  const int count = (int)(sizeof(results) / sizeof(results[0]));
  int failed = 0, skipped = 0;
  for (int i = 0; i < count; i++) {
    printf("%-24s %s\n", names[i], results[i] == PASS ? "pass" : results[i] == SKIP ? "skip" : "FAIL");
    failed += results[i] == FAIL;
    skipped += results[i] == SKIP;
  }
  return failed ? 1 : skipped == count ? SKIP : PASS;
}
