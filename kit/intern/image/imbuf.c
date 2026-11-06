/* BKImBuf: Blender's ImBuf (imbuf/intern/allocimbuf.cc) reduced to one
 * premultiplied 8-bit buffer per frame, plus the pixel operations the load
 * path needs: box-filter downscale (IMB_scale's Box filter), crop, EXIF
 * orientation, premultiply / unpremultiply, and CSS object-fit.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <math.h>
#ifdef _MSC_VER
#  include <intrin.h> /* _InterlockedIncrement64 */
#endif
#include <string.h>

#include "MEM_guardedalloc.h"

#include "image.h"

/* -------------------------------------------------------------------- */
/** \name Allocation
 * \{ */

static uint64_t next_key(void)
{
  static volatile uint64_t key = 0;
#ifdef _MSC_VER
  return (uint64_t)_InterlockedIncrement64((volatile long long *)&key);
#else
  return __atomic_add_fetch(&key, 1, __ATOMIC_RELAXED);
#endif
}

BKImBuf *imb_alloc(int x, int y, int frames)
{
  if (x <= 0 || y <= 0 || frames <= 0 || x > BK_IMB_MAX_SIDE || y > BK_IMB_MAX_SIDE ||
      (size_t)x * y * frames > BK_IMB_MAX_PIXELS)
  {
    return NULL;
  }
  /* header, delays, then pixels on a 16-byte boundary (SIMD-friendly rows) */
  const size_t head = (sizeof(BKImBuf) + (frames > 1 ? sizeof(int) * (size_t)frames : 0) + 15) &
                      ~(size_t)15;
  const size_t size = head + sizeof(uint32_t) * (size_t)x * y * frames;
  BKImBuf *ib = (BKImBuf *)MEM_mallocN(size, "BKImBuf");
  if (!ib) {
    return NULL;
  }
  memset(ib, 0, sizeof(*ib));
  ib->x = ib->src_x = x;
  ib->y = ib->src_y = y;
  ib->frames = frames;
  ib->key = next_key();
  ib->size = size;
  ib->delays = frames > 1 ? (int *)(ib + 1) : NULL;
  ib->rect = (uint32_t *)((char *)ib + head);
  ib->crop[2] = (float)x;
  ib->crop[3] = (float)y;
  return ib;
}

void imb_rekey(BKImBuf *ib)
{
  ib->key = next_key();
}

/* A new buffer of w x h carrying `ib`'s metadata (frames, delays, source). */
static BKImBuf *imb_alloc_like(const BKImBuf *ib, int w, int h)
{
  BKImBuf *out = imb_alloc(w, h, ib->frames);
  if (out) {
    out->ftype = ib->ftype;
    out->src_x = ib->src_x;
    out->src_y = ib->src_y;
    out->loops = ib->loops;
    if (ib->delays && out->delays) { /* frames may have shrunk to 1 (truncated) */
      memcpy(out->delays, ib->delays, sizeof(int) * (size_t)ib->frames);
    }
  }
  return out;
}

void BK_imbuf_free(BKImBuf *ib)
{
  if (ib && (ib->x > BUI_IMAGE_ATLAS_MAX || ib->y > BUI_IMAGE_ATLAS_MAX)) {
    BUI_image_forget(ib->key); /* its own GPU texture goes with it */
  }
  MEM_freeN(ib); /* one block (MEM_freeN(NULL) is a no-op) */
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Box filter (downscale)
 *
 * Area averaging, separable: each output pixel is the coverage-weighted mean
 * of the source pixels under it, computed on premultiplied channels (so
 * transparent pixels don't darken edges). Weights are 16-bit fixed point
 * summing to 65536 per output pixel.
 * \{ */

typedef struct BoxTaps {
  int *first, *count; /* per output pixel */
  uint32_t *w;        /* weights, sum(count) of them */
} BoxTaps;

static bool box_taps(BoxTaps *t, int src, int dst)
{
  const double scale = (double)src / dst;
  const int max_taps = (int)ceil(scale) + 1;
  t->first = (int *)MEM_mallocN(sizeof(int) * (size_t)dst * 2, "box taps");
  t->w = (uint32_t *)MEM_mallocN(sizeof(uint32_t) * (size_t)dst * max_taps, "box weights");
  if (!t->first || !t->w) {
    MEM_freeN(t->first);
    MEM_freeN(t->w);
    return false;
  }
  t->count = t->first + dst;
  uint32_t *w = t->w;
  for (int i = 0; i < dst; i++) {
    const double a = i * scale, b = (i + 1) * scale;
    const int i0 = (int)a, i1 = BK_MIN((int)ceil(b), src);
    uint32_t sum = 0;
    int n = 0;
    for (int k = i0; k < i1; k++, n++) {
      const double cover = fmin(b, k + 1.0) - fmax(a, (double)k);
      w[n] = (uint32_t)(cover / scale * 65536.0 + 0.5);
      sum += w[n];
    }
    if (n > 0) {
      w[n - 1] += 65536u - sum; /* exact unit sum: flat areas stay flat */
    }
    t->first[i] = i0;
    t->count[i] = n;
    w += n;
  }
  return true;
}

static void box_taps_free(BoxTaps *t)
{
  MEM_freeN(t->first);
  MEM_freeN(t->w);
}

static inline uint32_t box_sum(const uint32_t *p, size_t step, const uint32_t *w, int n)
{
  uint32_t a = 32768, r = 32768, g = 32768, b = 32768;
  for (int k = 0; k < n; k++, p += step) {
    const uint32_t c = *p, wk = w[k];
    a += (c >> 24) * wk;
    r += ((c >> 16) & 0xFF) * wk;
    g += ((c >> 8) & 0xFF) * wk;
    b += (c & 0xFF) * wk;
  }
  return ((a >> 16) << 24) | ((r >> 16) << 16) | ((g >> 16) << 8) | (b >> 16);
}

/* One frame: `src` (sw x sh) -> `dst` (dw x dh); `tmp` holds dw x sh. */
static void box_frame(const uint32_t *src, int sw, int sh, uint32_t *dst, int dw, int dh,
                      uint32_t *tmp, const BoxTaps *tx, const BoxTaps *ty)
{
  const uint32_t *rows = src;
  if (tx) {
    for (int y = 0; y < sh; y++) {
      const uint32_t *s = src + (size_t)y * sw, *w = tx->w;
      uint32_t *d = tmp + (size_t)y * dw;
      for (int x = 0; x < dw; x++) {
        d[x] = box_sum(s + tx->first[x], 1, w, tx->count[x]);
        w += tx->count[x];
      }
    }
    rows = tmp;
  }
  if (ty) {
    const uint32_t *w = ty->w;
    for (int y = 0; y < dh; y++) {
      const uint32_t *s = rows + (size_t)ty->first[y] * dw;
      uint32_t *d = dst + (size_t)y * dw;
      for (int x = 0; x < dw; x++) {
        d[x] = box_sum(s + x, (size_t)dw, w, ty->count[y]);
      }
      w += ty->count[y];
    }
  }
  else {
    memcpy(dst, rows, sizeof(uint32_t) * (size_t)dw * dh);
  }
}

BKImBuf *imb_scale(BKImBuf *ib, int w, int h)
{
  if (!ib || w <= 0 || h <= 0 || (w >= ib->x && h >= ib->y)) {
    return ib; /* downscale only: drawing upscales */
  }
  w = BK_MIN(w, ib->x);
  h = BK_MIN(h, ib->y);
  BKImBuf *out = imb_alloc_like(ib, w, h);
  BoxTaps tx = {0}, ty = {0};
  const bool sx = w != ib->x, sy = h != ib->y;
  uint32_t *tmp = sx && sy ? (uint32_t *)MEM_mallocN(sizeof(uint32_t) * (size_t)w * ib->y,
                                                       "box tmp") :
                             NULL;
  bool ok = out && (!sx || box_taps(&tx, ib->x, w));
  ok = ok && (!sy || box_taps(&ty, ib->y, h)) && (tmp || !(sx && sy));
  if (ok) {
    for (int f = 0; f < ib->frames; f++) {
      /* one pass writes straight into the result */
      uint32_t *pass = sx && sy ? tmp : imb_frame(out, f);
      box_frame(imb_frame(ib, f), ib->x, ib->y, imb_frame(out, f), w, h, pass, sx ? &tx : NULL,
                sy ? &ty : NULL);
    }
  }
  if (sx && tx.first) {
    box_taps_free(&tx);
  }
  if (sy && ty.first) {
    box_taps_free(&ty);
  }
  MEM_freeN(tmp);
  if (!ok) {
    MEM_freeN(out); /* never shown: no GPU copy to forget */
    return ib; /* out of memory: keep the larger image */
  }
  MEM_freeN(ib); /* an intermediate, never shown */
  return out;
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Crop, orientation
 * \{ */

BKImBuf *imb_crop(BKImBuf *ib, int x, int y, int w, int h)
{
  x = BK_CLAMP(x, 0, ib->x - 1);
  y = BK_CLAMP(y, 0, ib->y - 1);
  w = BK_CLAMP(w, 1, ib->x - x);
  h = BK_CLAMP(h, 1, ib->y - y);
  if (w == ib->x && h == ib->y) {
    return ib;
  }
  BKImBuf *out = imb_alloc_like(ib, w, h);
  if (!out) {
    return ib;
  }
  for (int f = 0; f < ib->frames; f++) {
    for (int r = 0; r < h; r++) {
      memcpy(imb_frame(out, f) + (size_t)r * w, imb_frame(ib, f) + (size_t)(y + r) * ib->x + x,
             sizeof(uint32_t) * (size_t)w);
    }
  }
  MEM_freeN(ib); /* an intermediate, never shown */
  return out;
}

BKImBuf *imb_orient(BKImBuf *ib, int o)
{
  if (o < 2 || o > 8) {
    return ib;
  }
  const int w = ib->x, h = ib->y;
  const bool swap = o >= 5;
  BKImBuf *out = imb_alloc_like(ib, swap ? h : w, swap ? w : h);
  if (!out) {
    return ib;
  }
  if (swap) {
    out->src_x = ib->src_y;
    out->src_y = ib->src_x;
  }
  const int ow = out->x, oh = out->y;
  for (int f = 0; f < ib->frames; f++) {
    const uint32_t *s = imb_frame(ib, f);
    uint32_t *d = imb_frame(out, f);
    for (int dy = 0; dy < oh; dy++) {
      for (int dx = 0; dx < ow; dx++) {
        int sx, sy; /* EXIF: the source pixel shown at (dx, dy) */
        switch (o) {
          case 2: sx = w - 1 - dx; sy = dy; break;
          case 3: sx = w - 1 - dx; sy = h - 1 - dy; break;
          case 4: sx = dx; sy = h - 1 - dy; break;
          case 5: sx = dy; sy = dx; break;
          case 6: sx = dy; sy = h - 1 - dx; break;
          case 7: sx = w - 1 - dy; sy = h - 1 - dx; break;
          default: sx = w - 1 - dy; sy = dx; break; /* 8 */
        }
        d[(size_t)dy * ow + dx] = s[(size_t)sy * w + sx];
      }
    }
  }
  MEM_freeN(ib); /* an intermediate, never shown */
  return out;
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Premultiplied alpha
 * \{ */

void imb_premultiply_rgba(uint32_t *dst, const uint8_t *rgba, int n)
{
  for (int i = 0; i < n; i++, rgba += 4) {
    const uint32_t a = rgba[3];
    if (a == 255) {
      dst[i] = 0xFF000000u | ((uint32_t)rgba[0] << 16) | ((uint32_t)rgba[1] << 8) | rgba[2];
    }
    else {
      /* x * a / 255, rounded */
      uint32_t r = rgba[0] * a + 128, g = rgba[1] * a + 128, b = rgba[2] * a + 128;
      r = (r + (r >> 8)) >> 8;
      g = (g + (g >> 8)) >> 8;
      b = (b + (b >> 8)) >> 8;
      dst[i] = (a << 24) | (r << 16) | (g << 8) | b;
    }
  }
}

void imb_unpremultiply(uint8_t *rgba, const uint32_t *src, int n)
{
  for (int i = 0; i < n; i++, rgba += 4) {
    const uint32_t c = src[i], a = c >> 24;
    uint32_t r = (c >> 16) & 0xFF, g = (c >> 8) & 0xFF, b = c & 0xFF;
    if (a && a < 255) {
      r = BK_MIN(255u, (r * 255 + a / 2) / a);
      g = BK_MIN(255u, (g * 255 + a / 2) / a);
      b = BK_MIN(255u, (b * 255 + a / 2) / a);
    }
    rgba[0] = (uint8_t)r;
    rgba[1] = (uint8_t)g;
    rgba[2] = (uint8_t)b;
    rgba[3] = (uint8_t)a;
  }
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Object fit
 * \{ */

void bk_image_fit(const BKImageFit *f, int iw, int ih, float dst[4], float src[4])
{
  const float bw = f->box_w > 0 ? (float)f->box_w : (float)iw;
  const float bh = f->box_h > 0 ? (float)f->box_h : (float)ih;
  const float kx = bw / (float)iw, ky = bh / (float)ih;
  float sx, sy;
  switch (f->fit) {
    case BK_FIT_FILL:
      sx = kx;
      sy = ky;
      break;
    case BK_FIT_COVER:
      sx = sy = fmaxf(kx, ky);
      break;
    case BK_FIT_NONE:
      sx = sy = 1.0f;
      break;
    case BK_FIT_SCALE_DOWN:
      sx = sy = fminf(1.0f, fminf(kx, ky));
      break;
    default: /* BK_FIT_CONTAIN */
      sx = sy = fminf(kx, ky);
      break;
  }
  const float w = (float)iw * sx, h = (float)ih * sy;
  const float x = (bw - w) * f->pos_x, y = (bh - h) * f->pos_y;
  const float x0 = fmaxf(0.0f, x), y0 = fmaxf(0.0f, y);
  const float x1 = fminf(bw, x + w), y1 = fminf(bh, y + h);
  dst[0] = x0;
  dst[1] = y0;
  dst[2] = fmaxf(0.0f, x1 - x0);
  dst[3] = fmaxf(0.0f, y1 - y0);
  src[0] = (x0 - x) / sx;
  src[1] = (y0 - y) / sy;
  src[2] = dst[2] / sx;
  src[3] = dst[3] / sy;
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Public accessors (BUI_kit.h)
 * \{ */

int BK_imbuf_size(const BKImBuf *ib, int *r_w, int *r_h)
{
  if (r_w) {
    *r_w = ib ? ib->x : 0;
  }
  if (r_h) {
    *r_h = ib ? ib->y : 0;
  }
  return ib ? ib->frames : 0;
}

int BK_imbuf_format(const BKImBuf *ib)
{
  return ib ? ib->ftype : BK_IMAGE_NONE;
}

const uint32_t *BK_imbuf_pixels(const BKImBuf *ib, int frame)
{
  return ib && frame >= 0 && frame < ib->frames ? imb_frame(ib, frame) : NULL;
}

int BK_imbuf_delay(const BKImBuf *ib, int frame)
{
  return ib && ib->delays && frame >= 0 && frame < ib->frames ? ib->delays[frame] : 0;
}

int BK_imbuf_loops(const BKImBuf *ib)
{
  return ib ? ib->loops : 0;
}

int BK_imbuf_rgba(const BKImBuf *ib, int frame, unsigned char *dst)
{
  const uint32_t *px = BK_imbuf_pixels(ib, frame);
  if (!px || !dst) {
    return 0;
  }
  imb_unpremultiply(dst, px, ib->x * ib->y);
  return 1;
}

BKImBuf *BK_imbuf_from_rgba(const unsigned char *rgba, int w, int h, int stride)
{
  if (!rgba) {
    return NULL;
  }
  BKImBuf *ib = imb_alloc(w, h, 1);
  if (!ib) {
    return NULL;
  }
  stride = stride > 0 ? stride : w * 4;
  for (int y = 0; y < h; y++) {
    imb_premultiply_rgba(ib->rect + (size_t)y * w, rgba + (size_t)y * stride, w);
  }
  return ib;
}

BKImBuf *BK_imbuf_from_argb(const uint32_t *argb, int w, int h)
{
  BKImBuf *ib = argb ? imb_alloc(w, h, 1) : NULL;
  if (ib) {
    memcpy(ib->rect, argb, sizeof(uint32_t) * (size_t)w * h);
  }
  return ib;
}

/** \} */
