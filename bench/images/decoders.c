/* Decoder shoot-out (website: Images > Performance, bench.ts imageDecode).
 * Decodes each file N times from memory into 8-bit, 4-channel pixels and
 * prints the median. Each -DUSE_* adds a decoder; the third-party ones are
 * not in the repo (download them next to this file, under dl/):
 *   wuffs-v0.4.c (google/wuffs release/c), stb_image.h (nothings/stb),
 *   libspng-0.7.4 (+ zlib), libjpeg-turbo-3.1.2 and libwebp-1.6.0 built with
 *   their own CMake (libjpeg-turbo: -DWITH_SIMD=0).
 * USE_BUI links the kit (BK_imbuf_decode), as built by this repo's CMake:
 *   gcc -O2 -DUSE_BUI -Ikit/include -Igpu/include bench/images/decoders.c \
 *       build/kit/libbui_kit.a build/gpu/libbui_gpu.a build/core/libbui_core.a \
 *       -lstdc++ -luser32 -lgdi32 -o decoders
 *   decoders file.png file.jpg ...
 * Test images: one photo scaled and re-encoded with ffmpeg (png, jpg -q:v 3,
 * webp -quality 80 / -lossless 1, gif, bmp, tga, qoi, hdr).
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

static double now_ms(void)
{
  struct timespec ts;
  timespec_get(&ts, TIME_UTC);
  return ts.tv_sec * 1e3 + ts.tv_nsec / 1e6;
}

static uint8_t *slurp(const char *p, size_t *n)
{
  FILE *f = fopen(p, "rb");
  if (!f) return NULL;
  fseek(f, 0, SEEK_END);
  *n = (size_t)ftell(f);
  fseek(f, 0, SEEK_SET);
  uint8_t *b = malloc(*n);
  fread(b, 1, *n, f);
  fclose(f);
  return b;
}

#ifdef USE_WUFFS
#  define WUFFS_IMPLEMENTATION
#  define WUFFS_CONFIG__STATIC_FUNCTIONS
#  define WUFFS_CONFIG__MODULES
#  define WUFFS_CONFIG__MODULE__BASE
#  define WUFFS_CONFIG__MODULE__ADLER32
#  define WUFFS_CONFIG__MODULE__CRC32
#  define WUFFS_CONFIG__MODULE__DEFLATE
#  define WUFFS_CONFIG__MODULE__ZLIB
#  define WUFFS_CONFIG__MODULE__PNG
#  define WUFFS_CONFIG__MODULE__JPEG
#  define WUFFS_CONFIG__MODULE__GIF
#  define WUFFS_CONFIG__MODULE__LZW
#  define WUFFS_CONFIG__MODULE__BMP
#  define WUFFS_CONFIG__MODULE__TARGA
#  define WUFFS_CONFIG__MODULE__QOI
#  define WUFFS_CONFIG__MODULE__WEBP
#  define WUFFS_CONFIG__MODULE__VP8
#  include "dl/wuffs-v0.4.c"
static int dec_wuffs(const uint8_t *d, size_t n, int *w, int *h)
{
  wuffs_base__image_decoder *dec = NULL;
  if (n > 8 && !memcmp(d, "\x89PNG", 4)) dec = wuffs_png__decoder__alloc_as__wuffs_base__image_decoder();
  else if (n > 3 && d[0] == 0xFF && d[1] == 0xD8) dec = wuffs_jpeg__decoder__alloc_as__wuffs_base__image_decoder();
  else if (n > 12 && !memcmp(d + 8, "WEBP", 4)) dec = wuffs_webp__decoder__alloc_as__wuffs_base__image_decoder();
  else if (!memcmp(d, "GIF8", 4)) dec = wuffs_gif__decoder__alloc_as__wuffs_base__image_decoder();
  else if (!memcmp(d, "BM", 2)) dec = wuffs_bmp__decoder__alloc_as__wuffs_base__image_decoder();
  else if (!memcmp(d, "qoif", 4)) dec = wuffs_qoi__decoder__alloc_as__wuffs_base__image_decoder();
  else dec = wuffs_targa__decoder__alloc_as__wuffs_base__image_decoder();
  if (!dec) return 0;
  wuffs_base__image_decoder__set_quirk(dec, WUFFS_BASE__QUIRK_IGNORE_CHECKSUM, 1);
  wuffs_base__image_config ic = {0};
  wuffs_base__io_buffer src = wuffs_base__ptr_u8__reader((uint8_t *)d, n, true);
  wuffs_base__status st = wuffs_base__image_decoder__decode_image_config(dec, &ic, &src);
  if (st.repr) { fprintf(stderr, "wuffs cfg: %s\n", st.repr); free(dec); return 0; }
  *w = (int)wuffs_base__pixel_config__width(&ic.pixcfg);
  *h = (int)wuffs_base__pixel_config__height(&ic.pixcfg);
  wuffs_base__pixel_config__set(&ic.pixcfg, WUFFS_BASE__PIXEL_FORMAT__BGRA_PREMUL,
                                WUFFS_BASE__PIXEL_SUBSAMPLING__NONE, *w, *h);
  size_t sz = (size_t)*w * *h * 4;
  uint8_t *px = malloc(sz);
  wuffs_base__pixel_buffer pb;
  wuffs_base__pixel_buffer__set_from_slice(&pb, &ic.pixcfg, wuffs_base__make_slice_u8(px, sz));
  uint64_t wl = wuffs_base__image_decoder__workbuf_len(dec).max_incl;
  uint8_t *work = wl ? malloc(wl) : NULL;
  st = wuffs_base__image_decoder__decode_frame(dec, &pb, &src, WUFFS_BASE__PIXEL_BLEND__SRC,
                                               wuffs_base__make_slice_u8(work, wl), NULL);
  if (st.repr) fprintf(stderr, "wuffs frame: %s\n", st.repr);
  free(work);
  free(px);
  free(dec);
  return st.repr == NULL;
}
#endif

#ifdef USE_STB
#  define STB_IMAGE_IMPLEMENTATION
#  define STBI_NO_STDIO
#  include "dl/stb_image.h"
static int dec_stb(const uint8_t *d, size_t n, int *w, int *h)
{
  int c;
  if (n > 10 && !memcmp(d, "#?RADIANCE", 10)) {
    float *f = stbi_loadf_from_memory(d, (int)n, w, h, &c, 4);
    if (f) { /* to 8 bit like the others */
      uint8_t *o = malloc((size_t)*w * *h * 4);
      for (size_t i = 0; i < (size_t)*w * *h * 4; i++) { float v = f[i]; o[i] = (uint8_t)(v >= 1 ? 255 : v <= 0 ? 0 : v * 255); }
      free(o); stbi_image_free(f); return 1;
    }
    return 0;
  }
  uint8_t *p = stbi_load_from_memory(d, (int)n, w, h, &c, 4);
  if (!p) return 0;
  stbi_image_free(p);
  return 1;
}
#endif

#ifdef USE_SPNG
#  include "dl/libspng-0.7.4/spng/spng.h"
static int dec_spng(const uint8_t *d, size_t n, int *w, int *h)
{
  spng_ctx *ctx = spng_ctx_new(0);
  spng_set_crc_action(ctx, SPNG_CRC_USE, SPNG_CRC_USE);
  spng_set_png_buffer(ctx, d, n);
  struct spng_ihdr ih;
  if (spng_get_ihdr(ctx, &ih)) { spng_ctx_free(ctx); return 0; }
  *w = (int)ih.width; *h = (int)ih.height;
  size_t sz;
  spng_decoded_image_size(ctx, SPNG_FMT_RGBA8, &sz);
  uint8_t *px = malloc(sz);
  int r = spng_decode_image(ctx, px, sz, SPNG_FMT_RGBA8, 0);
  free(px);
  spng_ctx_free(ctx);
  return r == 0;
}
#endif

#ifdef USE_LJT
#  include <setjmp.h>
#  include "jpeglib.h"
static int g_scale = 1; /* 1, 2, 4, 8: DCT scaling denominator */
static int dec_ljt(const uint8_t *d, size_t n, int *w, int *h)
{
  struct jpeg_decompress_struct cinfo;
  struct jpeg_error_mgr jerr;
  cinfo.err = jpeg_std_error(&jerr);
  jpeg_create_decompress(&cinfo);
  jpeg_mem_src(&cinfo, d, (unsigned long)n);
  jpeg_read_header(&cinfo, TRUE);
  cinfo.out_color_space = JCS_EXT_BGRA;
  cinfo.scale_num = 1;
  cinfo.scale_denom = (unsigned)g_scale;
  jpeg_start_decompress(&cinfo);
  *w = (int)cinfo.output_width; *h = (int)cinfo.output_height;
  uint8_t *px = malloc((size_t)*w * *h * 4);
  while (cinfo.output_scanline < cinfo.output_height) {
    JSAMPROW row = px + (size_t)cinfo.output_scanline * *w * 4;
    jpeg_read_scanlines(&cinfo, &row, 1);
  }
  jpeg_finish_decompress(&cinfo);
  jpeg_destroy_decompress(&cinfo);
  free(px);
  return 1;
}
#endif

#ifdef USE_WEBP
#  include "dl/libwebp-1.6.0/src/webp/decode.h"
static int g_wscale = 1;
static int dec_webp(const uint8_t *d, size_t n, int *w, int *h)
{
  WebPDecoderConfig cfg;
  WebPInitDecoderConfig(&cfg);
  if (WebPGetFeatures(d, n, &cfg.input) != VP8_STATUS_OK) return 0;
  *w = cfg.input.width / g_wscale; *h = cfg.input.height / g_wscale;
  if (g_wscale > 1) { cfg.options.use_scaling = 1; cfg.options.scaled_width = *w; cfg.options.scaled_height = *h; }
  size_t sz = (size_t)*w * *h * 4;
  uint8_t *px = malloc(sz);
  cfg.output.colorspace = MODE_bgrA;
  cfg.output.is_external_memory = 1;
  cfg.output.u.RGBA.rgba = px;
  cfg.output.u.RGBA.stride = *w * 4;
  cfg.output.u.RGBA.size = sz;
  int ok = WebPDecode(d, n, &cfg) == VP8_STATUS_OK;
  WebPFreeDecBuffer(&cfg.output);
  free(px);
  return ok;
}
#endif

#ifdef USE_BUI
#  include "BUI_kit.h"
static int g_bmax = 0;
static int dec_bui(const uint8_t *d, size_t n, int *w, int *h)
{
  BKImBuf *ib = BK_imbuf_decode(d, n, g_bmax, g_bmax);
  if (!ib) return 0;
  BK_imbuf_size(ib, w, h);
  BK_imbuf_free(ib);
  return 1;
}
#endif

typedef int (*DecFn)(const uint8_t *, size_t, int *, int *);

static int cmp(const void *a, const void *b)
{
  double x = *(const double *)a, y = *(const double *)b;
  return x < y ? -1 : x > y;
}

static void run(const char *name, DecFn fn, const char *path, int reps)
{
  size_t n;
  uint8_t *d = slurp(path, &n);
  if (!d) return;
  int w = 0, h = 0;
  if (!fn(d, n, &w, &h)) { printf("%-10s %-14s FAIL\n", name, path); free(d); return; }
  double t[64];
  for (int i = 0; i < reps; i++) {
    double t0 = now_ms();
    fn(d, n, &w, &h);
    t[i] = now_ms() - t0;
  }
  qsort(t, reps, sizeof(double), cmp);
  printf("%-10s %-14s %5dx%-5d %9.3f ms %8.1f MP/s\n", name, path, w, h, t[reps / 2],
         (double)w * h / 1e3 / t[reps / 2]);
  free(d);
}

int main(int argc, char **argv)
{
  for (int i = 1; i < argc; i++) {
    const char *p = argv[i];
    const int reps = strstr(p, "3840") ? 7 : 21;
    const char *ext = strrchr(p, '.');
#ifdef USE_WUFFS
    run("wuffs", dec_wuffs, p, reps);
#endif
#ifdef USE_STB
    if (strcmp(ext, ".webp") && strcmp(ext, ".qoi")) run("stb", dec_stb, p, reps);
#endif
#ifdef USE_SPNG
    if (!strcmp(ext, ".png")) run("spng", dec_spng, p, reps);
#endif
#ifdef USE_LJT
    if (!strcmp(ext, ".jpg")) {
      for (g_scale = 1; g_scale <= 8; g_scale *= 2) {
        char nm[32];
        snprintf(nm, sizeof nm, "ljt/%d", g_scale);
        run(nm, dec_ljt, p, reps);
      }
      g_scale = 1;
    }
#endif
#ifdef USE_WEBP
    if (!strcmp(ext, ".webp")) {
      run("libwebp", dec_webp, p, reps);
      g_wscale = 8;
      run("libwebp/8", dec_webp, p, reps);
      g_wscale = 1;
    }
#endif
#ifdef USE_BUI
    g_bmax = 0;
    run("bui", dec_bui, p, reps);
    g_bmax = 256;
    run("bui/256", dec_bui, p, reps);
    g_bmax = 0;
#endif
    (void)ext;
  }
  return 0;
}
