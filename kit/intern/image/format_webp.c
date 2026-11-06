/* WebP through libwebp, as Blender's imbuf (format_webp.cc): lossy and
 * lossless stills decode straight into the BKImBuf as premultiplied BGRA
 * (MODE_bgrA), scaled by libwebp's rescaler when smaller is wanted;
 * animations go through WebPAnimDecoder (composited frames), one at a time.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <string.h>

#include "MEM_guardedalloc.h"

#include "image.h"

#include "src/webp/decode.h"
#include "src/webp/demux.h"

bool imb_is_a_webp(const uint8_t *mem, size_t size)
{
  return size >= 16 && memcmp(mem, "RIFF", 4) == 0 && memcmp(mem + 8, "WEBP", 4) == 0;
}

bool imb_header_webp(const uint8_t *mem, size_t size, BKImageInfo *r_info)
{
  WebPBitstreamFeatures f;
  if (WebPGetFeatures(mem, size, &f) != VP8_STATUS_OK) {
    return false;
  }
  r_info->width = f.width;
  r_info->height = f.height;
  r_info->frames = 1;
  if (f.has_animation) {
    const WebPData data = {mem, size};
    WebPDemuxer *dmux = WebPDemux(&data);
    if (dmux) {
      r_info->frames = (int)WebPDemuxGetI(dmux, WEBP_FF_FRAME_COUNT);
      WebPDemuxDelete(dmux);
    }
  }
  return true;
}

BKImBuf *imb_load_webp(const uint8_t *mem, size_t size, int want_w, int want_h)
{
  WebPDecoderConfig cfg;
  if (!WebPInitDecoderConfig(&cfg) || WebPGetFeatures(mem, size, &cfg.input) != VP8_STATUS_OK) {
    return NULL;
  }
  if (cfg.input.has_animation) {
    return NULL; /* animations go through imb_webp_codec (imb_load) */
  }
  int w = cfg.input.width, h = cfg.input.height;
  if (want_w > 0 && want_h > 0 && want_w < w && want_h < h) {
    /* libwebp's rescaler (area average) instead of a full-size buffer */
    cfg.options.use_scaling = 1;
    cfg.options.scaled_width = w = want_w;
    cfg.options.scaled_height = h = want_h;
  }
  BKImBuf *ib = imb_alloc(w, h, 1);
  if (!ib) {
    return NULL;
  }
  cfg.output.colorspace = MODE_bgrA; /* premultiplied 0xAARRGGBB on little-endian */
  cfg.output.is_external_memory = 1;
  cfg.output.u.RGBA.rgba = (uint8_t *)ib->rect;
  cfg.output.u.RGBA.stride = w * 4;
  cfg.output.u.RGBA.size = sizeof(uint32_t) * (size_t)w * h;
  if (WebPDecode(mem, size, &cfg) != VP8_STATUS_OK) {
    BK_imbuf_free(ib);
    ib = NULL;
  }
  WebPFreeDecBuffer(&cfg.output);
  return ib;
}

/* -------------------------------------------------------------------- */
/** \name Animated WebP, one frame at a time (anim.c)
 * \{ */

bool imb_webp_animated(const uint8_t *mem, size_t size)
{
  /* the VP8X chunk's animation flag */
  return size >= 21 && imb_is_a_webp(mem, size) && memcmp(mem + 12, "VP8X", 4) == 0 &&
         (mem[20] & 0x02);
}

typedef struct WebpAnim {
  WebPAnimDecoder *dec;
  int w, h, prev_ts;
} WebpAnim;

static void webp_close(void *state)
{
  WebpAnim *a = (WebpAnim *)state;
  WebPAnimDecoderDelete(a->dec);
  MEM_freeN(a);
}

static void *webp_open(const char *path, const uint8_t *mem, size_t size, int want_w, int want_h,
                       BKImAnimInfo *r_info)
{
  (void)path;
  (void)want_w;
  (void)want_h;
  WebPAnimDecoderOptions opt;
  if (!WebPAnimDecoderOptionsInit(&opt)) {
    return NULL;
  }
  opt.color_mode = MODE_bgrA; /* premultiplied 0xAARRGGBB on little-endian */
  opt.use_threads = 0;
  const WebPData data = {mem, size};
  WebpAnim *a = (WebpAnim *)MEM_callocN(sizeof(WebpAnim), "WebpAnim");
  a->dec = WebPAnimDecoderNew(&data, &opt);
  WebPAnimInfo info;
  if (!a->dec || !WebPAnimDecoderGetInfo(a->dec, &info) || info.canvas_width == 0 ||
      info.canvas_height == 0 || info.canvas_width > BK_IMB_MAX_SIDE ||
      info.canvas_height > BK_IMB_MAX_SIDE)
  {
    webp_close(a);
    return NULL;
  }
  r_info->w = r_info->src_w = a->w = (int)info.canvas_width;
  r_info->h = r_info->src_h = a->h = (int)info.canvas_height;
  r_info->format = BUI_PIXMAP_ARGB;
  r_info->loops = (int)info.loop_count;
  return a;
}

static int webp_next(void *state, BKImBuf *dst, double *r_pts, double *r_dur)
{
  (void)r_pts;
  WebpAnim *a = (WebpAnim *)state;
  uint8_t *buf;
  int ts;
  if (!WebPAnimDecoderHasMoreFrames(a->dec)) {
    return 0;
  }
  if (!WebPAnimDecoderGetNext(a->dec, &buf, &ts)) {
    return -1;
  }
  memcpy(dst->rect, buf, sizeof(uint32_t) * (size_t)a->w * a->h);
  *r_dur = (ts - a->prev_ts) / 1000.0; /* timestamps (ms) are frame ends */
  a->prev_ts = ts;
  return 1;
}

static bool webp_seek(void *state, double t)
{
  (void)t; /* from the top: anim.c skips to t */
  WebpAnim *a = (WebpAnim *)state;
  WebPAnimDecoderReset(a->dec);
  a->prev_ts = 0;
  return true;
}

static size_t webp_bytes(const void *state)
{
  const WebpAnim *a = (const WebpAnim *)state;
  return sizeof(*a) + 2 * sizeof(uint32_t) * (size_t)a->w * a->h; /* its canvas and previous */
}

const BKImAnimCodec imb_webp_codec = {"webp", false, imb_webp_animated, webp_open, webp_next, webp_seek, webp_bytes, webp_close};

/** \} */
