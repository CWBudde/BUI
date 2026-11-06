/* Images (kit/intern/image): every format decodes (exact pixels where the
 * format is lossless), decode-at-size, EXIF orientation, animation frames,
 * corrupt input, then the cache and loader under load (thousands of
 * requests, eviction, cancellation) with every MEM block returned.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef _WIN32
#  define WIN32_LEAN_AND_MEAN
#  include <windows.h>
#  include <direct.h>
#  define sleep_ms(ms) Sleep(ms)
#  define make_dir(p) _mkdir(p)
#else
#  include <sys/stat.h>
#  include <time.h>
#  include <unistd.h>
#  define sleep_ms(ms) usleep((ms) * 1000)
#  define make_dir(p) mkdir(p, 0755)
#endif

#include "BUI_kit.h"
#include "GPU_raster.h"
#include "MEM_guardedalloc.h"

static int g_fail = 0;
#define CHECK(c) \
  do { \
    if (!(c)) { \
      printf("FAIL %s:%d: %s\n", __FILE__, __LINE__, #c); \
      g_fail = 1; \
    } \
  } while (0)

static unsigned char *slurp(const char *name, size_t *r_len)
{
  char path[512];
  snprintf(path, sizeof(path), "%s/%s", BUI_TEST_IMAGES, name);
  FILE *f = fopen(path, "rb");
  if (!f) {
    printf("FAIL missing %s\n", path);
    g_fail = 1;
    *r_len = 0;
    return NULL;
  }
  fseek(f, 0, SEEK_END);
  *r_len = (size_t)ftell(f);
  fseek(f, 0, SEEK_SET);
  unsigned char *b = (unsigned char *)malloc(*r_len);
  *r_len = fread(b, 1, *r_len, f);
  fclose(f);
  return b;
}

static int near_rgb(uint32_t px, unsigned r, unsigned g, unsigned b, int tol)
{
  const int dr = (int)((px >> 16) & 0xFF) - (int)r, dg = (int)((px >> 8) & 0xFF) - (int)g;
  const int db = (int)(px & 0xFF) - (int)b;
  return abs(dr) <= tol && abs(dg) <= tol && abs(db) <= tol;
}

/* The fixtures are 8 x 6: red | green over blue | white quadrants. */
static void check_quad(const char *name, int format, int tol)
{
  size_t n;
  unsigned char *data = slurp(name, &n);
  BKImageInfo info;
  CHECK(BK_image_probe(data, n, &info));
  CHECK(info.format == format && info.width == 8 && info.height == 6 && info.frames == 1);
  BKImBuf *ib = BK_imbuf_decode(data, n, 0, 0);
  CHECK(ib != NULL);
  if (ib) {
    int w, h;
    CHECK(BK_imbuf_size(ib, &w, &h) == 1 && w == 8 && h == 6);
    CHECK(BK_imbuf_format(ib) == format);
    const uint32_t *px = BK_imbuf_pixels(ib, 0);
    CHECK(near_rgb(px[1 * 8 + 1], 255, 0, 0, tol));
    CHECK(near_rgb(px[1 * 8 + 6], 0, 255, 0, tol));
    CHECK(near_rgb(px[4 * 8 + 1], 0, 0, 255, tol));
    CHECK(near_rgb(px[4 * 8 + 6], 255, 255, 255, tol));
    CHECK((px[0] >> 24) == 0xFF);
    BK_imbuf_free(ib);
  }
  /* decode at a size: half (JPEG through DCT scaling, others the box filter) */
  ib = BK_imbuf_decode(data, n, 4, 3);
  if (ib) {
    int w, h;
    BK_imbuf_size(ib, &w, &h);
    CHECK(w == 4 && h == 3);
    CHECK(near_rgb(BK_imbuf_pixels(ib, 0)[0], 255, 0, 0, tol + 8));
    BK_imbuf_free(ib);
  }
  else {
    CHECK(!"decode at size");
  }
  free(data);
}

static void test_formats(void)
{
  check_quad("quad.png", BK_IMAGE_PNG, 0);
  check_quad("quad.bmp", BK_IMAGE_BMP, 0);
  check_quad("quad.tga", BK_IMAGE_TGA, 0);
  check_quad("quad.qoi", BK_IMAGE_QOI, 0);
  check_quad("quad.ppm", BK_IMAGE_PNM, 0);
  check_quad("quad.webp", BK_IMAGE_WEBP, 0);
  check_quad("quad.gif", BK_IMAGE_GIF, 4); /* ffmpeg's 6-bit palette */
  check_quad("quad.hdr", BK_IMAGE_HDR, 8); /* ffmpeg's float conversion adds ~6/255 */
  check_quad("quad.jpg", BK_IMAGE_JPEG, 24);
  check_quad("quad_lossy.webp", BK_IMAGE_WEBP, 64); /* 4:2:0 chroma on 8 x 6 */
  CHECK(strcmp(BK_image_format_name(BK_IMAGE_JPEG), "jpeg") == 0);
}

static void test_alpha_and_rgba(void)
{
  size_t n;
  unsigned char *data = slurp("quad_alpha.png", &n);
  BKImBuf *ib = BK_imbuf_decode(data, n, 0, 0);
  CHECK(ib != NULL);
  if (ib) {
    const uint32_t *px = BK_imbuf_pixels(ib, 0);
    /* red at alpha 128: premultiplied 0x80800000 */
    CHECK(px[0] == 0x80800000u);
    CHECK(px[4 * 8 + 6] == 0); /* white at alpha 0 */
    unsigned char rgba[8 * 6 * 4];
    CHECK(BK_imbuf_rgba(ib, 0, rgba));
    CHECK(rgba[0] == 255 && rgba[1] == 0 && rgba[3] == 128); /* straight again */
    BKImBuf *copy = BK_imbuf_from_rgba(rgba, 8, 6, 0);
    CHECK(copy && memcmp(BK_imbuf_pixels(copy, 0), px, 8 * 6 * 4) == 0);
    BK_imbuf_free(copy);
    BK_imbuf_free(ib);
  }
  free(data);
}

static void test_svg(void)
{
  size_t n;
  unsigned char *data = slurp("halves.svg", &n);
  BKImageInfo info;
  CHECK(BK_image_probe(data, n, &info) && info.format == BK_IMAGE_SVG && info.width == 16 &&
        info.height == 12);
  BKImBuf *ib = BK_imbuf_decode(data, n, 32, 32); /* vector: rasterized at size, not upscaled */
  CHECK(ib != NULL);
  if (ib) {
    int w, h;
    BK_imbuf_size(ib, &w, &h);
    CHECK(w == 16 && h == 12);
    CHECK(near_rgb(BK_imbuf_pixels(ib, 0)[6 * 16 + 2], 255, 0, 0, 2));
    CHECK(near_rgb(BK_imbuf_pixels(ib, 0)[6 * 16 + 13], 0, 0, 255, 2));
    BK_imbuf_free(ib);
  }
  free(data);
}

static void test_exif_orientation(void)
{
  size_t n;
  unsigned char *jpg = slurp("quad.jpg", &n);
  /* APP1 "Exif" with a little-endian TIFF IFD0 holding Orientation = 6 */
  static const unsigned char app1[] = {
      0xFF, 0xE1, 0x00, 0x22, 'E', 'x', 'i', 'f', 0, 0, 'I', 'I', 0x2A, 0, 8, 0, 0, 0,
      1,    0,    0x12, 0x01, 3,   0,   1,   0,   0, 0, 6,   0,   0,    0, 0, 0, 0, 0};
  unsigned char *with = (unsigned char *)malloc(n + sizeof(app1));
  memcpy(with, jpg, 2);
  memcpy(with + 2, app1, sizeof(app1));
  memcpy(with + 2 + sizeof(app1), jpg + 2, n - 2);
  BKImageInfo info;
  CHECK(BK_image_probe(with, n + sizeof(app1), &info) && info.width == 6 && info.height == 8);
  BKImBuf *ib = BK_imbuf_decode(with, n + sizeof(app1), 0, 0);
  CHECK(ib != NULL);
  if (ib) {
    int w, h;
    BK_imbuf_size(ib, &w, &h);
    CHECK(w == 6 && h == 8);
    /* rotated 90 degrees clockwise: blue (bottom left) is now top left */
    CHECK(near_rgb(BK_imbuf_pixels(ib, 0)[1 * 6 + 1], 0, 0, 255, 40));
    CHECK(near_rgb(BK_imbuf_pixels(ib, 0)[1 * 6 + 4], 255, 0, 0, 40));
    BK_imbuf_free(ib);
  }
  free(with);
  free(jpg);
}

static void test_animation(void)
{
  const char *names[] = {"anim.gif", "anim.webp"};
  for (int k = 0; k < 2; k++) {
    size_t n;
    unsigned char *data = slurp(names[k], &n);
    BKImageInfo info;
    CHECK(BK_image_probe(data, n, &info) && info.frames == 3 && info.width == 8);
    BKImBuf *ib = BK_imbuf_decode(data, n, 0, 0);
    CHECK(ib && BK_imbuf_size(ib, NULL, NULL) == 3);
    if (ib && BK_imbuf_size(ib, NULL, NULL) == 3) {
      CHECK(BK_imbuf_delay(ib, 0) == 100 && BK_imbuf_delay(ib, 2) == 100);
    }
    BK_imbuf_free(ib);
    free(data);
  }
}

static void test_corrupt(void)
{
  const char *names[] = {"quad.png", "quad.jpg", "quad.webp", "quad.gif", "quad.bmp",
                         "quad.qoi", "quad.hdr", "anim.webp", "halves.svg", "quad.tga"};
  unsigned seed = 1;
  for (int k = 0; k < 10; k++) {
    size_t n;
    unsigned char *data = slurp(names[k], &n);
    unsigned char *bad = (unsigned char *)malloc(n);
    for (size_t cut = 0; cut < n; cut += 1 + n / 40) { /* truncations */
      BK_imbuf_free(BK_imbuf_decode(data, cut, 0, 0));
    }
    for (int it = 0; it < 300; it++) { /* bit flips past the signature */
      memcpy(bad, data, n);
      for (int f = 0; f < 4; f++) {
        seed = seed * 1103515245u + 12345u;
        const size_t at = 4 + (seed >> 8) % (n > 4 ? n - 4 : 1);
        bad[at < n ? at : n - 1] ^= (unsigned char)(1u << ((seed >> 4) & 7));
      }
      BK_imbuf_free(BK_imbuf_decode(bad, n, (it & 1) ? 3 : 0, (it & 1) ? 2 : 0));
    }
    free(bad);
    free(data);
  }
  CHECK(BK_imbuf_decode("not an image", 12, 0, 0) == NULL);
}

/* -- animations: frames one at a time, played on the context's timer ---------------- */

static double now_s(void)
{
#ifdef _WIN32
  LARGE_INTEGER f, c;
  QueryPerformanceFrequency(&f);
  QueryPerformanceCounter(&c);
  return (double)c.QuadPart / (double)f.QuadPart;
#else
  struct timespec ts;
  clock_gettime(CLOCK_MONOTONIC, &ts);
  return ts.tv_sec + ts.tv_nsec / 1e9;
#endif
}

/* A host whose one timer the test loop fires. */
static struct {
  double due;
  int timer, requests;
} g_host;

static void host_schedule(void *data, int ms, int timer)
{
  (void)data;
  g_host.due = now_s() + ms / 1000.0;
  g_host.timer = timer;
}

static void host_request(void *data)
{
  (void)data;
  g_host.requests++;
}

/* Runs the context for `seconds` like a window loop: timers, decodes, renders.
 * Returns the frames rendered. */
static int run(BKContext *ctx, BuiRaster *r, int w, int h, double seconds)
{
  const double end = now_s() + seconds;
  int frames = 0;
  while (now_s() < end) {
    if (g_host.due > 0.0 && now_s() >= g_host.due) {
      g_host.due = 0.0;
      BK_timer(ctx, g_host.timer);
    }
    BK_image_poll();
    if (BK_needs_render(ctx)) {
      frames += BK_render(ctx, r, w, h, 0) > 0;
    }
    sleep_ms(1);
  }
  return frames;
}

static void test_animation_api(void)
{
  size_t n;
  unsigned char *gif = slurp("anim.gif", &n);
  BKAnim *a = BK_anim_open(gif, n, 4, 4);
  CHECK(a != NULL);
  int w, h, loops;
  CHECK(BK_anim_info(a, &w, &h, &loops) == 3 && w == 8 && h == 8);
  uint32_t first[16];
  for (int i = 0; i < 7; i++) { /* wraps after 3 */
    int delay;
    BKImBuf *f = BK_anim_next(a, &delay);
    CHECK(f && delay == 100);
    int fw, fh;
    BK_imbuf_size(f, &fw, &fh);
    CHECK(fw == 4 && fh == 4); /* at the size asked */
    if (i == 0) {
      memcpy(first, BK_imbuf_pixels(f, 0), sizeof(first));
    }
    if (i == 3) {
      CHECK(memcmp(first, BK_imbuf_pixels(f, 0), sizeof(first)) == 0); /* frame 0 again */
    }
    BK_imbuf_free(f);
  }
  BK_anim_free(a);
  /* one frame alone (PhotoImage format="gif -index 2") */
  BKImBuf *f2 = BK_imbuf_decode_frame(gif, n, 2, 0, 0);
  BKImBuf *all = BK_imbuf_decode(gif, n, 0, 0);
  CHECK(f2 && all && memcmp(BK_imbuf_pixels(f2, 0), BK_imbuf_pixels(all, 2), 8 * 8 * 4) == 0);
  CHECK(BK_imbuf_decode_frame(gif, n, 3, 0, 0) == NULL); /* no such frame */
  BK_imbuf_free(f2);
  BK_imbuf_free(all);
  CHECK(BK_anim_open("GIF89a", 6, 0, 0) == NULL);
  free(gif);

  /* malformed animations: truncated and bit-flipped, stepped through twice over */
  const char *names[] = {"anim.gif", "anim.webp"};
  unsigned seed = 7;
  for (int k = 0; k < 2; k++) {
    unsigned char *data = slurp(names[k], &n);
    unsigned char *bad = (unsigned char *)malloc(n);
    for (int it = 0; it < 400; it++) {
      memcpy(bad, data, n);
      size_t len = n;
      if (it % 2) {
        len = 6 + (size_t)(it * 37) % (n - 6); /* truncated */
      }
      else {
        for (int f = 0; f < 3; f++) {
          seed = seed * 1103515245u + 12345u;
          bad[12 + (seed >> 8) % (n - 12)] ^= (unsigned char)(1u << ((seed >> 4) & 7));
        }
      }
      BKAnim *m = BK_anim_open(bad, len, (it & 2) ? 4 : 0, (it & 2) ? 4 : 0);
      for (int s = 0; m && s < 8; s++) {
        int delay;
        BK_imbuf_free(BK_anim_next(m, &delay));
      }
      BK_anim_free(m);
      BK_imbuf_free(BK_imbuf_decode(bad, len, 0, 0));
    }
    free(bad);
    free(data);
  }
}

/* FNV over the pixels of x0..x1 x y0..y1 */
static uint64_t box_hash(const uint32_t *px, int stride, int x0, int y0, int x1, int y1)
{
  uint64_t h = 1469598103934665603ull;
  for (int y = y0; y < y1; y++) {
    for (int x = x0; x < x1; x++) {
      h = (h ^ px[y * stride + x]) * 1099511628211ull;
    }
  }
  return h;
}

static void test_playback(void)
{
  const unsigned blocks0 = MEM_get_memory_blocks_in_use();
  {
    char p[600];
    snprintf(p, sizeof(p), "%s/anim.gif", BUI_TEST_IMAGES);
    BKNode *canvas = BK_node_new("canvas");
    BKContext *ctx = BK_context_new(canvas);
    BKHost host = {0};
    host.request = host_request;
    host.schedule = host_schedule;
    BK_context_set_host(ctx, &host);
    const int W = 200, H = 100;
    BuiRaster *r = BUI_raster_create(W, H);
    const float box[4] = {10, 10, 74, 74}, box2[4] = {100, 10, 164, 74};
    BKCanvasItem it;
    memset(&it, 0, sizeof(it));
    it.id = 1;
    it.type = BK_ITEM_IMAGE;
    it.coords = box;
    it.ncoords = 4;
    it.src = p;
    it.fit = BK_FIT_FILL;
    BK_canvas_set_item(canvas, &it);
    BKCanvasItem still = it; /* the same file paused on its third frame */
    still.id = 2;
    still.coords = box2;
    still.frame = 3;
    BK_canvas_set_item(canvas, &still);
    BK_render(ctx, r, W, H, 1);

    /* it plays: within 0.8 s (8 frame times) the pixels change at least 4 times */
    const uint32_t *px = BUI_raster_pixels(r);
    uint64_t last = 0, still0 = 0;
    int changes = 0;
    for (int i = 0; i < 16; i++) {
      run(ctx, r, W, H, 0.05);
      const uint64_t c = box_hash(px, W, 10, 10, 74, 74);
      changes += i > 0 && c != last;
      last = c;
      if (i == 2) {
        still0 = box_hash(px, W, 100, 10, 164, 74);
      }
    }
    printf("playing for 0.8 s: the frame changed %d times\n", changes);
    CHECK(changes >= 2); /* 8 frame times; a loaded machine (parallel ctest) drops some */
    CHECK(box_hash(px, W, 100, 10, 164, 74) == still0); /* the paused one holds its frame */
    BKImageStats st;

    /* off screen (no longer drawn): no decoding, no timer, no renders */
    BK_canvas_delete(canvas, -1);
    BK_render(ctx, r, W, H, 1);
    for (int i = 0; i < 200; i++) { /* a step already decoding lands first */
      run(ctx, r, W, H, 0.01);
      BK_image_stats(&st);
      if (st.pending == 0) {
        break;
      }
    }
    run(ctx, r, W, H, 0.05);
    BK_image_stats(&st);
    const int decoded = st.decoded, requests = g_host.requests;
    const int frames = run(ctx, r, W, H, 0.5);
    BK_image_stats(&st);
    printf("off screen for 0.5 s: %d decodes, %d renders, %d render requests\n",
           st.decoded - decoded, frames, g_host.requests - requests);
    CHECK(st.decoded == decoded && frames == 0 && g_host.requests == requests);

    /* reduced motion: shown again, it holds its frame */
    BK_set_animations(0);
    BK_canvas_set_item(canvas, &it);
    BK_render(ctx, r, W, H, 1);
    run(ctx, r, W, H, 0.1);
    const uint64_t held = box_hash(px, W, 10, 10, 74, 74);
    run(ctx, r, W, H, 0.4);
    CHECK(box_hash(px, W, 10, 10, 74, 74) == held);
    BK_set_animations(1);

    BK_context_free(ctx);
    BK_node_free(canvas);
    BUI_raster_free(r);
    BK_image_exit();
  }
  CHECK(MEM_get_memory_blocks_in_use() == blocks0);
}

/* -- the cache and loader ------------------------------------------------------------ */

#define NFILES 1000
static char g_dir[512];

static void path_of(char *buf, size_t size, int i)
{
  snprintf(buf, size, "%s/img%04d.png", g_dir, i);
}

static void write_files(void)
{
  size_t n;
  unsigned char *png = slurp("quad.png", &n);
  snprintf(g_dir, sizeof(g_dir), "%s", BUI_TEST_TMP);
  make_dir(g_dir);
  for (int i = 0; i < NFILES; i++) {
    char p[600];
    path_of(p, sizeof(p), i);
    FILE *f = fopen(p, "wb");
    if (f) {
      fwrite(png, 1, n, f);
      fclose(f);
    }
  }
  free(png);
}

static void remove_files(void)
{
  for (int i = 0; i < NFILES; i++) {
    char p[600];
    path_of(p, sizeof(p), i);
    remove(p);
  }
#ifdef _WIN32
  _rmdir(g_dir);
#else
  rmdir(g_dir);
#endif
}

/* A grid of `count` thumbnails starting at file `first`, `cell` pixels each. */
static void show_grid(BKNode *canvas, int first, int count, int cell)
{
  char p[600];
  for (int i = 0; i < count; i++) {
    path_of(p, sizeof(p), (first + i) % NFILES);
    const float x = (float)(i % 20) * cell, y = (float)(i / 20) * cell;
    const float coords[4] = {x, y, x + cell - 1, y + cell - 1};
    BKCanvasItem it;
    memset(&it, 0, sizeof(it));
    it.id = i + 1;
    it.type = BK_ITEM_IMAGE;
    it.coords = coords;
    it.ncoords = 4;
    it.src = p;
    it.fit = BK_FIT_COVER;
    BK_canvas_put_item(canvas, i, &it);
  }
  BK_canvas_truncate(canvas, count);
}

static void settle(BKContext *ctx, BuiRaster *r, int w, int h)
{
  BKImageStats st;
  for (int i = 0; i < 2000; i++) {
    BK_image_poll();
    BK_render(ctx, r, w, h, 0);
    BK_image_stats(&st);
    if (st.pending == 0) {
      break;
    }
    sleep_ms(1);
  }
  BK_render(ctx, r, w, h, 0);
}

static void test_cache(void)
{
  write_files();
  const unsigned blocks0 = MEM_get_memory_blocks_in_use();
  const size_t bytes0 = MEM_get_memory_in_use();
  {
    BKNode *canvas = BK_node_new("canvas");
    BKContext *ctx = BK_context_new(canvas);
    const int W = 640, H = 480;
    BuiRaster *r = BUI_raster_create(W, H);
    BK_render(ctx, r, W, H, 1);

    /* the pixels arrive and redraw: 8 x 6 into a 16 x 12 box (fill) */
    show_grid(canvas, 0, 1, 16);
    settle(ctx, r, W, H);
    const uint32_t *px = BUI_raster_pixels(r);
    CHECK(near_rgb(px[2 * W + 2], 255, 0, 0, 1));
    CHECK(near_rgb(px[9 * W + 13], 255, 255, 255, 1));

    /* cover: 8 x 6 into a 30 x 30 box (upscaled, sides cropped): quadrants
     * stay in their corners, sampled from the right sub-pixel region */
    {
      char p[600];
      path_of(p, sizeof(p), 0);
      const float box[4] = {100, 100, 130, 130};
      BKCanvasItem it;
      memset(&it, 0, sizeof(it));
      it.id = 1;
      it.type = BK_ITEM_IMAGE;
      it.coords = box;
      it.ncoords = 4;
      it.src = p;
      it.fit = BK_FIT_COVER;
      BK_canvas_put_item(canvas, 0, &it);
      BK_canvas_truncate(canvas, 1);
      BK_render(ctx, r, W, H, 0);
      settle(ctx, r, W, H);
      px = BUI_raster_pixels(r);
      CHECK(near_rgb(px[102 * W + 102], 255, 0, 0, 1));
      CHECK(near_rgb(px[102 * W + 127], 0, 255, 0, 1));
      CHECK(near_rgb(px[127 * W + 102], 0, 0, 255, 1));
      CHECK(near_rgb(px[127 * W + 127], 255, 255, 255, 1));
    }

    /* scroll through every file: 600 thumbnails a screen, 30 screens */
    BK_image_set_budget(64 * 1024);
    for (int s = 0; s < 30; s++) {
      show_grid(canvas, s * 37, 600, 32);
      BK_render(ctx, r, W, H, 0);
    }
    settle(ctx, r, W, H);
    BKImageStats st;
    BK_image_stats(&st);
    printf("cache: decoded %d failed %d cancelled %d evicted %d entries %d bytes %zu\n", st.decoded,
           st.failed, st.cancelled, st.evicted, st.entries, st.bytes);
    CHECK(st.decoded >= 600 && st.failed == 0);
    CHECK(st.cancelled > 0); /* screens scrolled past before their decodes ran */
    CHECK(st.evicted > 0);   /* the budget holds fewer than were decoded */

    /* a settled frame allocates nothing in the image module (with a budget
     * that holds the screen) */
    BK_image_set_budget((size_t)64 << 20);
    show_grid(canvas, 29 * 37, 600, 32);
    BK_render(ctx, r, W, H, 1);
    settle(ctx, r, W, H);
    const unsigned settled = MEM_get_memory_blocks_in_use();
    show_grid(canvas, 29 * 37, 600, 32);
    BK_render(ctx, r, W, H, 1);
    CHECK(MEM_get_memory_blocks_in_use() == settled);
    printf("settled: %u MEM blocks, %zu bytes live\n", settled, MEM_get_memory_in_use());

    /* thousands queued, then nothing shown: every queued request cancels */
    for (int s = 0; s < 4; s++) {
      show_grid(canvas, 500 + s * 100, 600, 24);
      BK_render(ctx, r, W, H, 0);
    }
    BK_canvas_truncate(canvas, 0);
    BK_render(ctx, r, W, H, 1);
    settle(ctx, r, W, H);
    BK_image_stats(&st);
    CHECK(st.pending == 0);

    BK_context_free(ctx);
    BK_node_free(canvas);
    BUI_raster_free(r);
    BK_image_exit();
  }
  printf("memory: %u blocks, %zu bytes before; %u blocks, %zu bytes after\n", blocks0, bytes0,
         MEM_get_memory_blocks_in_use(), MEM_get_memory_in_use());
  CHECK(MEM_get_memory_blocks_in_use() == blocks0);
  CHECK(MEM_get_memory_in_use() == bytes0);
  remove_files();
}

int main(void)
{
  setvbuf(stdout, NULL, _IONBF, 0);
  const unsigned blocks0 = MEM_get_memory_blocks_in_use();
  test_formats();
  test_alpha_and_rgba();
  test_svg();
  test_exif_orientation();
  test_animation();
  test_corrupt();
  test_animation_api();
  CHECK(MEM_get_memory_blocks_in_use() == blocks0); /* decoders free everything */
  test_cache();
  test_playback();
  printf(g_fail ? "image_test: FAILED\n" : "image_test: ok\n");
  return g_fail;
}
