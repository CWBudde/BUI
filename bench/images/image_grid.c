/* Thumbnail-grid benchmark (website: Images > Performance).
 *
 *   bui_image_bench <dir with source images> <count> [async|sync] [gpu]
 *
 * <count> files (hard links to the sources, round robin) are shown as a
 * virtualized grid of 120 x 120 thumbnails (object-fit: cover) in a 1280 x 800
 * headless window, scrolled top to bottom at 40 px per frame, paced to 60 Hz
 * (the frame's own CPU time is what is measured). Prints JSON:
 *  - first: time from the first frame until every visible thumbnail is drawn
 *  - frame p50 / p99 (ms) while scrolling, and frames that showed a missing tile
 *  - peak / steady working set and the image module's own bytes (MEM)
 *  - MEM allocations in a settled (unchanged) full redraw: expected 0
 *  - working set and MEM after BK_image_exit (memory back to baseline)
 * "sync" is the previous BUI path for <img> (blenderui's png crate): a
 * synchronous full-size decode on the UI thread the first time an image
 * shows, kept forever.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <psapi.h>

#include "BUI_kit.h"
#include "GPU_raster.h"
#include "MEM_guardedalloc.h"

#define W 1280
#define H 800
#define CELL 120
#define GAP 8
#define COLS ((W - GAP) / (CELL + GAP))
#define STEP 40
#define BK_MAX(a, b) ((a) > (b) ? (a) : (b))
#define BK_MIN(a, b) ((a) < (b) ? (a) : (b))

static double now_ms(void)
{
  LARGE_INTEGER f, c;
  QueryPerformanceFrequency(&f);
  QueryPerformanceCounter(&c);
  return (double)c.QuadPart * 1000.0 / (double)f.QuadPart;
}

static size_t working_set(size_t *r_peak)
{
  PROCESS_MEMORY_COUNTERS pmc;
  GetProcessMemoryInfo(GetCurrentProcess(), &pmc, sizeof(pmc));
  if (r_peak) {
    *r_peak = pmc.PeakWorkingSetSize;
  }
  return pmc.WorkingSetSize;
}

static int cmp(const void *a, const void *b)
{
  const double x = *(const double *)a, y = *(const double *)b;
  return x < y ? -1 : x > y;
}

static char g_dir[MAX_PATH - 32]; /* room for the file names */
static int g_count;
static BKImBuf **g_sync; /* sync mode: decoded once, kept forever */

static void path_of(char *buf, int i)
{
  snprintf(buf, MAX_PATH, "%s/bench_%05d.img", g_dir, i);
}

/* The rows visible at `scroll`, as canvas items; returns how many items. */
static int show(BKNode *canvas, int scroll, int *r_missing)
{
  char p[MAX_PATH];
  const int row0 = scroll / (CELL + GAP), rows = H / (CELL + GAP) + 2;
  int n = 0, missing = 0;
  for (int r = row0; r < row0 + rows; r++) {
    for (int c = 0; c < COLS; c++) {
      const int i = r * COLS + c;
      if (i >= g_count) {
        break;
      }
      const float x = (float)(GAP + c * (CELL + GAP)), y = (float)(GAP + r * (CELL + GAP) - scroll);
      const float coords[4] = {x, y, x + CELL, y + CELL};
      BKCanvasItem it;
      memset(&it, 0, sizeof(it));
      it.id = n + 1;
      it.type = BK_ITEM_IMAGE;
      it.coords = coords;
      it.ncoords = 4;
      it.fit = BK_FIT_COVER;
      path_of(p, i);
      if (g_sync) {
        if (!g_sync[i]) {
          g_sync[i] = BK_imbuf_load(p, 0, 0);
        }
        it.imbuf = g_sync[i];
      }
      else {
        it.src = p;
      }
      BK_canvas_put_item(canvas, n++, &it);
    }
  }
  BK_canvas_truncate(canvas, n);
  if (r_missing) {
    BKImageStats st;
    BK_image_stats(&st);
    *r_missing = st.pending;
  }
  (void)missing;
  return n;
}

/* -- many animations at once ----------------------------------------------------------
 *   bui_image_bench <dir> anim <count> [gif|webp] [gpu]
 * <count> animated images (hard links to anim120.gif / anim120.webp: 120 x 120,
 * 20 frames of 100 ms, looping) shown at 60 x 60 in a 1280 x 800 window, all
 * on screen, for 5 s, driven like a window loop: the context's frame timer
 * (BKHost.schedule), decodes arriving, renders. Then they scroll off screen
 * for 3 s. Prints JSON: render time p50 / p99, the time spent rendering and
 * decoding per second (decode time is measured around each decode on the
 * workers: Windows' process CPU times are sampled at the clock tick and
 * miss most of these short bursts), memory, MEM allocations per rendered
 * frame and per decoded animation frame, and the off-screen phase's
 * decodes, renders and time. */

static struct {
  double due;
  int timer;
} g_timer;

static void anim_schedule(void *data, int ms, int timer)
{
  (void)data;
  g_timer.due = now_ms() + ms;
  g_timer.timer = timer;
}

typedef struct LoopStats {
  int renders;
  double render_ms;
  double *ft;
  int nft, cap;
} LoopStats;

static void loop_for(BKContext *ctx, BuiRaster *r, double ms, LoopStats *s)
{
  const double end = now_ms() + ms;
  while (now_ms() < end) {
    double wait = end - now_ms();
    if (g_timer.due > 0.0) {
      wait = BK_MIN(wait, g_timer.due - now_ms());
    }
    if (wait > 1.0) {
      Sleep((DWORD)BK_MIN(wait, 2.0)); /* decodes arrive meanwhile */
    }
    if (g_timer.due > 0.0 && now_ms() >= g_timer.due) {
      g_timer.due = 0.0;
      BK_timer(ctx, g_timer.timer);
    }
    BK_image_poll();
    if (BK_needs_render(ctx)) {
      const double a = now_ms();
      BK_render(ctx, r, W, H, 0);
      s->render_ms += now_ms() - a;
      if (s->nft < s->cap) {
        s->ft[s->nft++] = now_ms() - a;
      }
      s->renders++;
    }
  }
}

static int bench_anim(const char *dir, int count, int webp, int gpu)
{
  snprintf(g_dir, sizeof(g_dir), "%s/grid", dir);
  CreateDirectoryA(g_dir, NULL);
  char src[MAX_PATH];
  snprintf(src, sizeof(src), "%s/%s", dir, webp ? "anim120.webp" : "anim120.gif");
  for (int i = 0; i < count; i++) {
    char p[MAX_PATH];
    path_of(p, i);
    if (GetFileAttributesA(p) == INVALID_FILE_ATTRIBUTES && !CreateHardLinkA(p, src, NULL)) {
      CopyFileA(src, p, TRUE);
    }
  }
  g_count = count;
  const unsigned blocks0 = MEM_get_memory_blocks_in_use();
  BKNode *canvas = BK_node_new("canvas");
  BKContext *ctx = BK_context_new(canvas);
  BKHost host = {0};
  host.schedule = anim_schedule;
  BK_context_set_host(ctx, &host);
  BuiRaster *r = BUI_raster_create(W, H);
  if (gpu && !BUI_raster_gpu_attach(r, NULL, 1)) {
    fprintf(stderr, "no GPU renderer\n");
    return 1;
  }
  const int cell = 64, cols = W / cell;
  for (int i = 0; i < count; i++) {
    char p[MAX_PATH];
    path_of(p, i);
    const float x = (float)(i % cols * cell + 2), y = (float)(i / cols * cell + 2);
    const float coords[4] = {x, y, x + 60, y + 60};
    BKCanvasItem it;
    memset(&it, 0, sizeof(it));
    it.id = i + 1;
    it.type = BK_ITEM_IMAGE;
    it.coords = coords;
    it.ncoords = 4;
    it.fit = BK_FIT_COVER;
    it.src = p;
    BK_canvas_put_item(canvas, i, &it);
  }
  BK_canvas_truncate(canvas, count);
  BK_render(ctx, r, W, H, 1);

  LoopStats s = {.ft = (double *)malloc(sizeof(double) * 100000), .cap = 100000};
  loop_for(ctx, r, 1000.0, &s); /* warm up: first frames, clocks started */
  BKImageStats st;
  BK_image_stats(&st);
  const int dec0 = st.decoded;
  const double dms0 = st.decode_ms;
  const size_t alloc0 = MEM_get_allocations_total();
  const double t0 = now_ms();
  s.renders = s.nft = 0;
  s.render_ms = 0.0;
  loop_for(ctx, r, 5000.0, &s);
  const double secs = (now_ms() - t0) / 1000.0;
  BK_image_stats(&st);
  const double decode_ms = st.decode_ms - dms0, render_ms = s.render_ms;
  const int decoded = st.decoded - dec0;
  const size_t allocs = MEM_get_allocations_total() - alloc0;
  size_t peak_ws;
  const size_t ws = working_set(&peak_ws);
  const size_t image_mem = st.bytes;
  qsort(s.ft, (size_t)s.nft, sizeof(double), cmp);
  const double p50 = s.nft ? s.ft[s.nft / 2] : 0, p99 = s.nft ? s.ft[(int)(s.nft * 0.99)] : 0;
  const int renders = s.renders;

  /* off screen: the canvas shows nothing of them any more */
  BK_canvas_truncate(canvas, 0);
  BK_render(ctx, r, W, H, 1);
  loop_for(ctx, r, 300.0, &s); /* steps already in flight land */
  BK_image_stats(&st);
  const int off_dec0 = st.decoded;
  const double off_dms0 = st.decode_ms;
  s.renders = 0;
  s.render_ms = 0.0;
  loop_for(ctx, r, 3000.0, &s);
  BK_image_stats(&st);
  const int off_decoded = st.decoded - off_dec0, off_renders = s.renders;
  const double off_ms = s.render_ms + st.decode_ms - off_dms0;

  BK_context_free(ctx);
  BK_node_free(canvas);
  BUI_raster_free(r);
  BK_image_exit();
  printf("{\"animations\": %d, \"format\": \"%s\", \"renderer\": \"%s\", \"seconds\": %.1f, "
         "\"renders_per_s\": %.1f, \"render_p50_ms\": %.2f, \"render_p99_ms\": %.2f, "
         "\"render_ms_per_s\": %.1f, \"decode_ms_per_s\": %.1f, \"decoded_frames_per_s\": %.0f, "
         "\"mem_allocs_per_render\": %.1f, \"mem_allocs_per_decoded_frame\": %.1f, "
         "\"working_set_mb\": %.1f, \"peak_working_set_mb\": %.1f, \"image_cache_mb\": %.1f, "
         "\"offscreen_3s_decodes\": %d, \"offscreen_3s_renders\": %d, "
         "\"offscreen_3s_work_ms\": %.1f, \"mem_blocks_leaked\": %d}\n",
         count, webp ? "webp" : "gif", gpu ? "gpu" : "cpu", secs, renders / secs, p50, p99,
         render_ms / secs, decode_ms / secs, decoded / secs, renders ? (double)allocs / renders : 0.0,
         decoded ? (double)allocs / decoded : 0.0, ws / 1048576.0, peak_ws / 1048576.0,
         image_mem / 1048576.0, off_decoded, off_renders, off_ms,
         (int)MEM_get_memory_blocks_in_use() - (int)blocks0);
  free(s.ft);
  for (int i = 0; i < count; i++) {
    char p[MAX_PATH];
    path_of(p, i);
    DeleteFileA(p);
  }
  RemoveDirectoryA(g_dir);
  return 0;
}

int main(int argc, char **argv)
{
  if (argc < 3) {
    fprintf(stderr, "usage: %s <source dir> <count> [async|sync] [gpu]\n", argv[0]);
    return 2;
  }
  if (strcmp(argv[2], "anim") == 0 && argc > 3) {
    return bench_anim(argv[1], atoi(argv[3]), argc > 4 && strcmp(argv[4], "webp") == 0,
                      argc > 5 && strcmp(argv[5], "gpu") == 0);
  }
  g_count = atoi(argv[2]);
  const int sync = argc > 3 && strcmp(argv[3], "sync") == 0;
  const int gpu = argc > 4 && strcmp(argv[4], "gpu") == 0;
  /* the files: hard links to the sources (distinct paths, no copies) */
  const char *sources[] = {"j1024.jpg", "p1024.png", "l1024.webp", "a1024.png"};
  snprintf(g_dir, sizeof(g_dir), "%s/grid", argv[1]);
  CreateDirectoryA(g_dir, NULL);
  for (int i = 0; i < g_count; i++) {
    char p[MAX_PATH], src[MAX_PATH];
    path_of(p, i);
    snprintf(src, sizeof(src), "%s/%s", argv[1], sources[i % 4]);
    if (GetFileAttributesA(p) == INVALID_FILE_ATTRIBUTES && !CreateHardLinkA(p, src, NULL)) {
      CopyFileA(src, p, TRUE);
    }
  }
  if (sync) {
    g_sync = (BKImBuf **)calloc((size_t)g_count, sizeof(BKImBuf *));
  }
  const size_t ws0 = working_set(NULL);
  const size_t mem0 = MEM_get_memory_in_use();
  const unsigned blocks0 = MEM_get_memory_blocks_in_use();

  BKNode *canvas = BK_node_new("canvas");
  BKContext *ctx = BK_context_new(canvas);
  BuiRaster *r = BUI_raster_create(W, H);
  if (gpu && !BUI_raster_gpu_attach(r, NULL, 1)) {
    fprintf(stderr, "no GPU renderer\n");
    return 1;
  }

  /* first frame until every visible thumbnail is on screen */
  const double t0 = now_ms();
  show(canvas, 0, NULL);
  BK_render(ctx, r, W, H, 1);
  BKImageStats st;
  for (;;) {
    BK_image_stats(&st);
    if (st.pending == 0) {
      break;
    }
    Sleep(1);
    if (BK_image_poll() > 0) {
      BK_render(ctx, r, W, H, 0);
    }
  }
  BK_image_poll();
  BK_render(ctx, r, W, H, 0);
  const double first = now_ms() - t0;

  /* scroll top to bottom, paced to 60 Hz */
  const int rows = (g_count + COLS - 1) / COLS;
  const int max_scroll = BK_MAX(0, rows * (CELL + GAP) + GAP - H);
  const int frames = max_scroll / STEP + 1;
  double *ft = (double *)malloc(sizeof(double) * (size_t)frames);
  int late = 0;
  size_t peak_mem = 0;
  for (int f = 0; f < frames; f++) {
    const double a = now_ms();
    BK_image_poll();
    int missing;
    show(canvas, f * STEP, &missing);
    BK_render(ctx, r, W, H, 0);
    ft[f] = now_ms() - a;
    late += missing > 0;
    peak_mem = BK_MAX(peak_mem, MEM_get_memory_in_use() - mem0);
    const double rest = 1000.0 / 60.0 - (now_ms() - a);
    if (rest > 1.0) {
      Sleep((DWORD)rest);
    }
  }
  /* settle at the end */
  for (int i = 0; i < 5000; i++) {
    BK_image_stats(&st);
    if (st.pending == 0) {
      break;
    }
    Sleep(1);
  }
  BK_image_poll();
  show(canvas, (frames - 1) * STEP, NULL);
  BK_render(ctx, r, W, H, 1);
  size_t peak_ws;
  const size_t steady_ws = working_set(&peak_ws);
  const size_t steady_mem = MEM_get_memory_in_use() - mem0;
  /* a settled full redraw: what it allocates through MEM */
  const unsigned b0 = MEM_get_memory_blocks_in_use();
  double settled = now_ms();
  for (int i = 0; i < 10; i++) {
    show(canvas, (frames - 1) * STEP, NULL);
    BK_render(ctx, r, W, H, 1);
  }
  settled = (now_ms() - settled) / 10.0;
  const int settled_allocs = (int)MEM_get_memory_blocks_in_use() - (int)b0;
  BK_image_stats(&st);

  qsort(ft, (size_t)frames, sizeof(double), cmp);
  const double p50 = ft[frames / 2], p99 = ft[(int)(frames * 0.99)];

  /* close: everything back */
  BK_context_free(ctx);
  BK_node_free(canvas);
  BUI_raster_free(r);
  if (g_sync) {
    for (int i = 0; i < g_count; i++) {
      BK_imbuf_free(g_sync[i]);
    }
    free(g_sync);
  }
  BK_image_exit();
  const size_t ws_after = working_set(NULL);
  printf("{\"count\": %d, \"mode\": \"%s\", \"renderer\": \"%s\", \"first_ms\": %.1f, "
         "\"frames\": %d, \"frame_p50_ms\": %.2f, \"frame_p99_ms\": %.2f, "
         "\"frames_with_pending\": %d, \"settled_full_redraw_ms\": %.2f, "
         "\"settled_mem_allocs\": %d, \"peak_ws_mb\": %.1f, \"steady_ws_mb\": %.1f, "
         "\"image_mem_peak_mb\": %.1f, \"image_mem_steady_mb\": %.1f, \"decoded\": %d, "
         "\"cancelled\": %d, \"evicted\": %d, \"ws_start_mb\": %.1f, \"ws_after_close_mb\": %.1f, "
         "\"mem_blocks_leaked\": %d}\n",
         g_count, sync ? "sync" : "async", gpu ? "gpu" : "cpu", first, frames, p50, p99, late,
         settled, settled_allocs, peak_ws / 1048576.0, steady_ws / 1048576.0,
         peak_mem / 1048576.0, steady_mem / 1048576.0, st.decoded, st.cancelled, st.evicted,
         ws0 / 1048576.0, ws_after / 1048576.0,
         (int)MEM_get_memory_blocks_in_use() - (int)blocks0);
  free(ft);
  for (int i = 0; i < g_count; i++) {
    char p[MAX_PATH];
    path_of(p, i);
    DeleteFileA(p);
  }
  RemoveDirectoryA(g_dir);
  return 0;
}
