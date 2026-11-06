/* PNG, GIF (animated), BMP, TGA and QOI through Wuffs: memory-safe
 * decoders that write premultiplied BGRA (0xAARRGGBB in memory order on
 * little-endian machines) straight into the BKImBuf. The roles of Blender's
 * format_png.cc / format_bmp.cc / format_targa.cc.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <string.h>

#include "MEM_guardedalloc.h"

#include "image.h"

#define WUFFS_IMPLEMENTATION
#define WUFFS_CONFIG__STATIC_FUNCTIONS
#define WUFFS_CONFIG__MODULES
#define WUFFS_CONFIG__MODULE__BASE
#define WUFFS_CONFIG__MODULE__ADLER32
#define WUFFS_CONFIG__MODULE__CRC32
#define WUFFS_CONFIG__MODULE__DEFLATE
#define WUFFS_CONFIG__MODULE__ZLIB
#define WUFFS_CONFIG__MODULE__PNG
#define WUFFS_CONFIG__MODULE__LZW
#define WUFFS_CONFIG__MODULE__GIF
#define WUFFS_CONFIG__MODULE__BMP
#define WUFFS_CONFIG__MODULE__TARGA
#define WUFFS_CONFIG__MODULE__QOI
#include "wuffs-v0.4.c"

/* -------------------------------------------------------------------- */
/** \name Signatures
 * \{ */

bool imb_is_a_png(const uint8_t *mem, size_t size)
{
  return size >= 8 && memcmp(mem, "\x89PNG\r\n\x1a\n", 8) == 0;
}

bool imb_is_a_gif(const uint8_t *mem, size_t size)
{
  return size >= 6 && (memcmp(mem, "GIF87a", 6) == 0 || memcmp(mem, "GIF89a", 6) == 0);
}

bool imb_is_a_bmp(const uint8_t *mem, size_t size)
{
  return size >= 14 && mem[0] == 'B' && mem[1] == 'M';
}

bool imb_is_a_qoi(const uint8_t *mem, size_t size)
{
  return size >= 14 && memcmp(mem, "qoif", 4) == 0;
}

bool imb_is_a_tga(const uint8_t *mem, size_t size)
{
  /* no signature: a plausible header (as Blender's imb_is_a_targa) */
  if (size < 18) {
    return false;
  }
  const int cmap = mem[1], type = mem[2], depth = mem[16];
  const bool type_ok = type == 1 || type == 2 || type == 3 || type == 9 || type == 10 ||
                       type == 11;
  const bool depth_ok = depth == 8 || depth == 15 || depth == 16 || depth == 24 || depth == 32;
  return cmap <= 1 && type_ok && depth_ok && (mem[12] | mem[13]) && (mem[14] | mem[15]);
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Decoders
 * \{ */

/* A decoder of `ftype` in a MEM block (freed with MEM_freeN). */
#define CODEC_NEW(name) \
  { \
    const size_t n = sizeof__wuffs_##name##__decoder(); \
    wuffs_##name##__decoder *d = (wuffs_##name##__decoder *)MEM_mallocN(n, "wuffs " #name); \
    if (d && !wuffs_##name##__decoder__initialize( \
                  d, n, WUFFS_VERSION, WUFFS_INITIALIZE__LEAVE_INTERNAL_BUFFERS_UNINITIALIZED) \
                  .repr) \
    { \
      return wuffs_##name##__decoder__upcast_as__wuffs_base__image_decoder(d); \
    } \
    MEM_freeN(d); \
    return NULL; \
  }

static wuffs_base__image_decoder *codec_new(const uint8_t *mem, size_t size)
{
  if (imb_is_a_png(mem, size)) CODEC_NEW(png)
  if (imb_is_a_gif(mem, size)) CODEC_NEW(gif)
  if (imb_is_a_bmp(mem, size)) CODEC_NEW(bmp)
  if (imb_is_a_qoi(mem, size)) CODEC_NEW(qoi)
  if (imb_is_a_tga(mem, size)) CODEC_NEW(targa)
  return NULL;
}

/* The decoder, past the image config (size checked). */
static wuffs_base__image_decoder *open_image(const uint8_t *mem,
                                             size_t size,
                                             wuffs_base__io_buffer *src,
                                             wuffs_base__image_config *ic)
{
  wuffs_base__image_decoder *dec = codec_new(mem, size);
  if (!dec) {
    return NULL;
  }
  /* Only quirks the codec knows: an unsupported one is an error, and an
   * error disables a Wuffs decoder. */
  if (imb_is_a_png(mem, size)) {
    /* browsers ignore zlib/PNG checksums too; saves a pass over the data */
    wuffs_base__image_decoder__set_quirk(dec, WUFFS_BASE__QUIRK_IGNORE_CHECKSUM, 1);
  }
  else if (imb_is_a_gif(mem, size)) {
    wuffs_base__image_decoder__set_quirk(dec, WUFFS_GIF__QUIRK_IGNORE_TOO_MUCH_PIXEL_DATA, 1);
  }
  *src = wuffs_base__ptr_u8__reader((uint8_t *)mem, size, true);
  memset(ic, 0, sizeof(*ic));
  const wuffs_base__status st = wuffs_base__image_decoder__decode_image_config(dec, ic, src);
  const uint32_t w = wuffs_base__pixel_config__width(&ic->pixcfg);
  const uint32_t h = wuffs_base__pixel_config__height(&ic->pixcfg);
  if (st.repr || w == 0 || h == 0 || w > BK_IMB_MAX_SIDE || h > BK_IMB_MAX_SIDE) {
    MEM_freeN(dec);
    return NULL;
  }
  return dec;
}

/* Frames of an animation (GIF), counted by skipping through it. */
static int count_frames(wuffs_base__image_decoder *dec, wuffs_base__io_buffer *src)
{
  int n = 0;
  wuffs_base__frame_config fc;
  while (!wuffs_base__image_decoder__decode_frame_config(dec, &fc, src).repr) {
    n++;
  }
  return n;
}

bool imb_header_wuffs(const uint8_t *mem, size_t size, BKImageInfo *r_info)
{
  wuffs_base__io_buffer src;
  wuffs_base__image_config ic;
  wuffs_base__image_decoder *dec = open_image(mem, size, &src, &ic);
  if (!dec) {
    return false;
  }
  r_info->width = (int)wuffs_base__pixel_config__width(&ic.pixcfg);
  r_info->height = (int)wuffs_base__pixel_config__height(&ic.pixcfg);
  const int frames = imb_is_a_gif(mem, size) ? count_frames(dec, &src) : 1;
  r_info->frames = BK_MAX(1, frames); /* (a macro: no calls inside) */
  MEM_freeN(dec);
  return true;
}

static void clear_rect(uint32_t *px, int stride, wuffs_base__rect_ie_u32 r)
{
  for (uint32_t y = r.min_incl_y; y < r.max_excl_y; y++) {
    memset(px + (size_t)y * stride + r.min_incl_x, 0,
           sizeof(uint32_t) * (r.max_excl_x - r.min_incl_x));
  }
}

static void copy_rect(uint32_t *dst, const uint32_t *src, int stride, wuffs_base__rect_ie_u32 r)
{
  for (uint32_t y = r.min_incl_y; y < r.max_excl_y; y++) {
    const size_t o = (size_t)y * stride + r.min_incl_x;
    memcpy(dst + o, src + o, sizeof(uint32_t) * (r.max_excl_x - r.min_incl_x));
  }
}

/* Decodes the next frame config and frame of `dec` into `canvas` (w x h)
 * blended as the frame asks. Returns the status of the frame config (a note
 * past the last frame) and the frame's in `r_frame`. */
static wuffs_base__status decode_next(wuffs_base__image_decoder *dec, wuffs_base__io_buffer *src,
                                      wuffs_base__image_config *ic, uint8_t *work, uint64_t wl,
                                      uint32_t *canvas, int first, wuffs_base__frame_config *fc,
                                      wuffs_base__status *r_frame)
{
  const wuffs_base__status st = wuffs_base__image_decoder__decode_frame_config(dec, fc, src);
  if (st.repr) {
    return st;
  }
  const int w = (int)wuffs_base__pixel_config__width(&ic->pixcfg);
  const int h = (int)wuffs_base__pixel_config__height(&ic->pixcfg);
  wuffs_base__pixel_buffer pb;
  wuffs_base__pixel_buffer__set_from_slice(
      &pb, &ic->pixcfg,
      wuffs_base__make_slice_u8((uint8_t *)canvas, sizeof(uint32_t) * (size_t)w * h));
  const wuffs_base__pixel_blend blend = first ||
                                                wuffs_base__frame_config__overwrite_instead_of_blend(
                                                    fc) ?
                                            WUFFS_BASE__PIXEL_BLEND__SRC :
                                            WUFFS_BASE__PIXEL_BLEND__SRC_OVER;
  *r_frame = wuffs_base__image_decoder__decode_frame(dec, &pb, src, blend,
                                                     wuffs_base__make_slice_u8(work, (size_t)wl),
                                                     NULL);
  return st;
}

BKImBuf *imb_load_wuffs(const uint8_t *mem, size_t size, int want_w, int want_h)
{
  (void)want_w; /* these formats decode at full size; imb_load scales */
  (void)want_h;
  wuffs_base__io_buffer src;
  wuffs_base__image_config ic;
  wuffs_base__image_decoder *dec = open_image(mem, size, &src, &ic);
  if (!dec) {
    return NULL;
  }
  const int w = (int)wuffs_base__pixel_config__width(&ic.pixcfg);
  const int h = (int)wuffs_base__pixel_config__height(&ic.pixcfg);
  BKImBuf *ib = imb_alloc(w, h, 1);
  const uint64_t wl = wuffs_base__image_decoder__workbuf_len(dec).max_incl;
  uint8_t *work = wl ? (uint8_t *)MEM_mallocN((size_t)wl, "wuffs work") : NULL;
  bool ok = ib && (work || !wl);
  if (ok) {
    wuffs_base__pixel_config__set(&ic.pixcfg, WUFFS_BASE__PIXEL_FORMAT__BGRA_PREMUL,
                                  WUFFS_BASE__PIXEL_SUBSAMPLING__NONE, (uint32_t)w, (uint32_t)h);
    wuffs_base__frame_config fc;
    wuffs_base__status frame;
    ok = !decode_next(dec, &src, &ic, work, wl, ib->rect, 1, &fc, &frame).repr && !frame.repr;
  }
  MEM_freeN(work);
  MEM_freeN(dec);
  if (!ok) {
    MEM_freeN(ib);
    return NULL;
  }
  return ib;
}

/* -------------------------------------------------------------------- */
/** \name GIF animation, one frame at a time (anim.c)
 * \{ */

typedef struct GifAnim {
  const uint8_t *mem;
  size_t size;
  wuffs_base__image_decoder *dec;
  wuffs_base__io_buffer src;
  wuffs_base__image_config ic;
  uint8_t *work;
  uint64_t wl;
  int w, h, index;
  uint32_t *canvas, *backup; /* backup: before a RESTORE_PREVIOUS frame */
  wuffs_base__frame_config prev;
} GifAnim;

static bool gif_start(GifAnim *g)
{
  MEM_freeN(g->dec);
  g->dec = open_image(g->mem, g->size, &g->src, &g->ic);
  if (!g->dec) {
    return false;
  }
  wuffs_base__pixel_config__set(&g->ic.pixcfg, WUFFS_BASE__PIXEL_FORMAT__BGRA_PREMUL,
                                WUFFS_BASE__PIXEL_SUBSAMPLING__NONE, (uint32_t)g->w,
                                (uint32_t)g->h);
  memset(g->canvas, 0, sizeof(uint32_t) * (size_t)g->w * g->h);
  g->index = 0;
  return true;
}

static void gif_close(void *state)
{
  GifAnim *g = (GifAnim *)state;
  MEM_freeN(g->dec);
  MEM_freeN(g->work);
  MEM_freeN(g->canvas);
  MEM_freeN(g->backup);
  MEM_freeN(g);
}

static void *gif_open(const uint8_t *mem, size_t size, int *r_w, int *r_h, int *r_loops)
{
  GifAnim *g = (GifAnim *)MEM_callocN(sizeof(GifAnim), "GifAnim");
  g->mem = mem;
  g->size = size;
  g->dec = open_image(mem, size, &g->src, &g->ic);
  if (!g->dec || !imb_is_a_gif(mem, size)) {
    gif_close(g);
    return NULL;
  }
  g->w = (int)wuffs_base__pixel_config__width(&g->ic.pixcfg);
  g->h = (int)wuffs_base__pixel_config__height(&g->ic.pixcfg);
  g->wl = wuffs_base__image_decoder__workbuf_len(g->dec).max_incl;
  g->work = g->wl ? (uint8_t *)MEM_mallocN((size_t)g->wl, "wuffs work") : NULL;
  g->canvas = (uint32_t *)MEM_mallocN(sizeof(uint32_t) * (size_t)g->w * g->h, "gif canvas");
  g->backup = (uint32_t *)MEM_mallocN(sizeof(uint32_t) * (size_t)g->w * g->h, "gif backup");
  if ((g->wl && !g->work) || !g->canvas || !g->backup || !gif_start(g)) {
    gif_close(g);
    return NULL;
  }
  *r_w = g->w;
  *r_h = g->h;
  /* plays (0: forever): known once the frames were seen (no NETSCAPE block:
   * one play), so ask a decoder that skipped through them */
  wuffs_base__io_buffer src;
  wuffs_base__image_config ic;
  wuffs_base__image_decoder *count = open_image(mem, size, &src, &ic);
  if (count) {
    count_frames(count, &src);
    *r_loops = (int)wuffs_base__image_decoder__num_animation_loops(count);
    MEM_freeN(count);
  }
  return g;
}

static void gif_rewind(void *state)
{
  gif_start((GifAnim *)state);
}

static int gif_next(void *state, uint32_t *dst, int *r_delay)
{
  GifAnim *g = (GifAnim *)state;
  if (!g->dec) {
    return -1;
  }
  if (g->index > 0) { /* the previous frame's disposal */
    const wuffs_base__rect_ie_u32 pb = wuffs_base__frame_config__bounds(&g->prev);
    switch (wuffs_base__frame_config__disposal(&g->prev)) {
      case WUFFS_BASE__ANIMATION_DISPOSAL__RESTORE_BACKGROUND:
        clear_rect(g->canvas, g->w, pb);
        break;
      case WUFFS_BASE__ANIMATION_DISPOSAL__RESTORE_PREVIOUS:
        copy_rect(g->canvas, g->backup, g->w, pb);
        break;
      default:
        break;
    }
  }
  /* peek the next frame's disposal: RESTORE_PREVIOUS needs the canvas before it */
  memcpy(g->backup, g->canvas, sizeof(uint32_t) * (size_t)g->w * g->h);
  wuffs_base__frame_config fc;
  wuffs_base__status frame;
  const wuffs_base__status st = decode_next(g->dec, &g->src, &g->ic, g->work, g->wl, g->canvas,
                                            g->index == 0, &fc, &frame);
  if (st.repr) {
    return st.repr == wuffs_base__note__end_of_data ? 0 : (g->index > 0 ? 0 : -1);
  }
  if (frame.repr && g->index == 0) {
    return -1; /* a bad first frame: no image (later bad frames end the animation) */
  }
  *r_delay = (int)(wuffs_base__frame_config__duration(&fc) / WUFFS_BASE__FLICKS_PER_MILLISECOND);
  g->prev = fc;
  g->index++;
  memcpy(dst, g->canvas, sizeof(uint32_t) * (size_t)g->w * g->h);
  return 1;
}

static size_t gif_bytes(const void *state)
{
  const GifAnim *g = (const GifAnim *)state;
  return sizeof(*g) + (size_t)g->wl + 2 * sizeof(uint32_t) * (size_t)g->w * g->h;
}

const BKImAnimCodec imb_gif_codec = {gif_open, gif_next, gif_rewind, gif_bytes, gif_close};

/** \} */
