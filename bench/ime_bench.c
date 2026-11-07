/* What the IME support costs the kit: a keystroke plus its repaint in a text field, an idle
 * frame, the host's per-frame BK_ime_state poll, and a frame while an IME composes.
 *
 * Build against the kit (CMake: target bui::kit), run, compare with a build without the IME
 * code (define BENCH_NO_IME to leave out the IME calls). Each figure is the median of 5 runs.
 *
 *   cl /O2 /DBENCH_NO_IME bench\ime_bench.c ... (against an older kit) or without the define.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "BUI_kit.h"
#include "GPU_raster.h"

#ifdef _WIN32
#  define WIN32_LEAN_AND_MEAN
#  include <windows.h>
static double now_ns(void)
{
  LARGE_INTEGER f, c;
  QueryPerformanceFrequency(&f);
  QueryPerformanceCounter(&c);
  return (double)c.QuadPart * 1e9 / (double)f.QuadPart;
}
#else
#  include <time.h>
static double now_ns(void)
{
  struct timespec t;
  clock_gettime(CLOCK_MONOTONIC, &t);
  return (double)t.tv_sec * 1e9 + (double)t.tv_nsec;
}
#endif

enum { W = 480, H = 240, RUNS = 5 };

static BKContext *g_ctx;
static BuiRaster *g_raster;

static int cmp(const void *a, const void *b)
{
  const double x = *(const double *)a, y = *(const double *)b;
  return (x > y) - (x < y);
}

static double median(double *v, int n)
{
  qsort(v, (size_t)n, sizeof(double), cmp);
  return v[n / 2];
}

#define BENCH(label, iters, body) \
  do { \
    double t[RUNS]; \
    for (int run = 0; run < RUNS; run++) { \
      const double t0 = now_ns(); \
      for (int i = 0; i < (iters); i++) { \
        body; \
      } \
      t[run] = (now_ns() - t0) / (iters); \
    } \
    printf("%-46s %10.1f ns\n", label, median(t, RUNS)); \
  } while (0)

#ifndef BENCH_NO_IME
/* Japanese hiragana, 10 and 11 characters (30 and 33 bytes), alternating */
static void compose_step(int tick)
{
  static const char *forms[2] = {
      "\xE3\x81\xAB\xE3\x81\xBB\xE3\x82\x93\xE3\x81\x94\xE3\x81\xAB\xE3\x81\xBB\xE3\x82\x93\xE3\x81\x94\xE3\x81\xAB\xE3\x81\xBB",
      "\xE3\x81\xAB\xE3\x81\xBB\xE3\x82\x93\xE3\x81\x94\xE3\x81\xAB\xE3\x81\xBB\xE3\x82\x93\xE3\x81\x94\xE3\x81\xAB\xE3\x81\xBB\xE3\x82\x93"};
  const BKImeEvent e = {"", forms[tick & 1], -1, 0, 6};
  BK_ime_event(g_ctx, BK_IME_COMPOSITION, &e);
  BK_render(g_ctx, g_raster, W, H, 0);
}
#endif

int main(void)
{
  BKNode *col = BK_node_new("column");
  BKNode *text = BK_node_new("text");
  BK_set_str(text, "value", "The quick brown fox");
  BK_node_add(col, text, -1);
  for (int i = 0; i < 6; i++) {
    BKNode *b = BK_node_new("button");
    BK_set_str(b, "text", "Button");
    BK_node_add(col, b, -1);
  }
  g_ctx = BK_context_new(col);
  g_raster = BUI_raster_create(W, H);
  BK_render(g_ctx, g_raster, W, H, 1);

  int r[4];
  BK_node_rect(text, r);
  BK_press(g_ctx, r[0] + r[2] - 4, r[1] + r[3] / 2, 0);
  BK_release(g_ctx, r[0] + r[2] - 4, r[1] + r[3] / 2, 0);
  BK_render(g_ctx, g_raster, W, H, 1);

  /* a keystroke in the focused text field and the repaint it causes */
  BENCH("keystroke + repaint (editing a text field)", 4000, {
    BK_key(g_ctx, "a", "a", 0);
    BK_key(g_ctx, "", "BackSpace", 0);
    BK_render(g_ctx, g_raster, W, H, 0);
  });
  /* nothing changed: the idle frame */
  BENCH("idle frame (nothing damaged)", 200000, BK_render(g_ctx, g_raster, W, H, 0));
  /* a full repaint of the window */
  BENCH("full repaint, editing", 2000, BK_render(g_ctx, g_raster, W, H, 1));

#ifndef BENCH_NO_IME
  BKImeState s;
  BENCH("host poll: BK_ime_state (unchanged)", 2000000, BK_ime_state(g_ctx, &s));

  BKImeEvent start = {"", "", -1, -1, -1};
  BK_ime_event(g_ctx, BK_IME_COMPOSITION_START, &start);
  int tick = 0;
  BENCH("composition update + repaint (10-11 chars)", 4000, compose_step(tick++));
  BK_ime_event(g_ctx, BK_IME_COMPOSITION_END, &start);
#endif
  BUI_raster_free(g_raster);
  BK_context_free(g_ctx);
  return 0;
}
