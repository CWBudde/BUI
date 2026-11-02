/* BUI software raster backend — see GPU_raster.h.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#include "GPU_raster.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

#define TILE_W 64
#define TILE_H 32

struct BuiRaster {
  int w, h;
  uint32_t *pix;  /* current frame */
  uint32_t *prev; /* last presented frame, for damage */
  int full_damage;

  /* clip, exclusive max, always inside the buffer */
  int cx0, cy0, cx1, cy1;

  int (*damage)[4];
  int damage_len, damage_cap;

  char font_family[64];
  int font_flags;

  BuiGpuSink sink;
};

/* -------------------------------------------------------------------- */
/** \name Pixel helpers
 * \{ */

typedef struct Paint {
  uint32_t rgb; /* 0x00RRGGBB */
  int a;        /* 0..256 */
} Paint;

static inline int clampi(int v, int lo, int hi)
{
  return v < lo ? lo : (v > hi ? hi : v);
}

static inline float clampf(float v, float lo, float hi)
{
  return v < lo ? lo : (v > hi ? hi : v);
}

static inline Paint paint_from(const float c[4])
{
  Paint p;
  int r = (int)(clampf(c[0], 0.0f, 1.0f) * 255.0f + 0.5f);
  int g = (int)(clampf(c[1], 0.0f, 1.0f) * 255.0f + 0.5f);
  int b = (int)(clampf(c[2], 0.0f, 1.0f) * 255.0f + 0.5f);
  p.rgb = ((uint32_t)r << 16) | ((uint32_t)g << 8) | (uint32_t)b;
  p.a = (int)(clampf(c[3], 0.0f, 1.0f) * 256.0f + 0.5f);
  return p;
}

/* Blend s over d with factor f in 0..256. */
static inline uint32_t blend(uint32_t d, uint32_t s, uint32_t f)
{
  const uint32_t nf = 256 - f;
  const uint32_t rb = (((s & 0xFF00FFu) * f + (d & 0xFF00FFu) * nf) >> 8) & 0xFF00FFu;
  const uint32_t g = (((s & 0x00FF00u) * f + (d & 0x00FF00u) * nf) >> 8) & 0x00FF00u;
  return rb | g;
}

/* Write with coverage cov (0..1) scaled by paint alpha. */
static inline void put_cov(uint32_t *px, Paint p, float cov)
{
  if (cov <= 0.0f) {
    return;
  }
  int f = (int)(cov * (float)p.a + 0.5f);
  if (f >= 256) {
    *px = p.rgb;
  }
  else if (f > 0) {
    *px = blend(*px, p.rgb, (uint32_t)f);
  }
}

/* Integer bbox [x0,x1) x [y0,y1) clipped; returns 0 when empty. */
static int clip_box(const BuiRaster *r, float fx0, float fy0, float fx1, float fy1, int *x0, int *y0,
                    int *x1, int *y1)
{
  if (!(fx1 > fx0) || !(fy1 > fy0)) {
    return 0;
  }
  /* Guard against NaN / huge values before converting to int. */
  fx0 = clampf(fx0, -1e6f, 1e6f);
  fy0 = clampf(fy0, -1e6f, 1e6f);
  fx1 = clampf(fx1, -1e6f, 1e6f);
  fy1 = clampf(fy1, -1e6f, 1e6f);
  *x0 = clampi((int)floorf(fx0), r->cx0, r->cx1);
  *y0 = clampi((int)floorf(fy0), r->cy0, r->cy1);
  *x1 = clampi((int)ceilf(fx1), r->cx0, r->cx1);
  *y1 = clampi((int)ceilf(fy1), r->cy0, r->cy1);
  return *x1 > *x0 && *y1 > *y0;
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Lifetime / frame
 * \{ */

static void raster_reset_clip(BuiRaster *r)
{
  r->cx0 = 0;
  r->cy0 = 0;
  r->cx1 = r->w;
  r->cy1 = r->h;
}

static void raster_alloc(BuiRaster *r, int w, int h)
{
  free(r->pix);
  free(r->prev);
  r->w = w > 0 ? w : 1;
  r->h = h > 0 ? h : 1;
  r->pix = (uint32_t *)calloc((size_t)r->w * r->h, sizeof(uint32_t));
  r->prev = (uint32_t *)calloc((size_t)r->w * r->h, sizeof(uint32_t));
  r->full_damage = 1;
  raster_reset_clip(r);
}

static void sink_begin(void *u, int w, int h)
{
  BUI_raster_begin_frame((BuiRaster *)u, w, h);
}
static void sink_scissor(void *u, int x, int y, int w, int h)
{
  BUI_raster_set_clip((BuiRaster *)u, x, y, w, h);
}
static void sink_tris(void *u, const float (*co)[2], const float (*col)[4], int n)
{
  BUI_raster_triangles((BuiRaster *)u, co, col, n);
}
static void sink_lines(void *u, const float (*co)[2], const float (*col)[4], int n)
{
  for (int i = 0; i + 1 < n; i += 2) {
    BUI_raster_line((BuiRaster *)u, co[i][0], co[i][1], co[i + 1][0], co[i + 1][1], 1.0f, col[i]);
  }
}
static void sink_points(void *u, const float (*co)[2], const float (*col)[4], int n, float size)
{
  BUI_raster_points((BuiRaster *)u, co, col, n, size);
}
static void sink_text(void *u, float x, float y, const char *text, float size, const float rgba[4])
{
  BUI_raster_text((BuiRaster *)u, x, y, text, (int)strlen(text), size, rgba);
}
static void sink_end(void *u)
{
  BUI_raster_end_frame((BuiRaster *)u);
}

BuiRaster *BUI_raster_create(int width, int height)
{
  BuiRaster *r = (BuiRaster *)calloc(1, sizeof(BuiRaster));
  if (!r) {
    return NULL;
  }
  raster_alloc(r, width, height);
  r->sink.user = r;
  r->sink.begin_frame = sink_begin;
  r->sink.scissor = sink_scissor;
  r->sink.triangles = sink_tris;
  r->sink.lines = sink_lines;
  r->sink.points = sink_points;
  r->sink.text = sink_text;
  r->sink.end_frame = sink_end;
  return r;
}

void BUI_raster_free(BuiRaster *r)
{
  if (!r) {
    return;
  }
  free(r->pix);
  free(r->prev);
  free(r->damage);
  free(r);
}

const BuiGpuSink *BUI_raster_sink(BuiRaster *r)
{
  return &r->sink;
}

void BUI_raster_begin_frame(BuiRaster *r, int width, int height)
{
  if (width > 0 && height > 0 && (width != r->w || height != r->h)) {
    raster_alloc(r, width, height);
  }
  raster_reset_clip(r);
}

static void damage_push(BuiRaster *r, int x, int y, int w, int h)
{
  /* Merge with the previous rect when it is the same column span directly
   * above (common for tall widgets), keeping uploads few and large. */
  if (r->damage_len > 0) {
    int *last = r->damage[r->damage_len - 1];
    if (last[0] == x && last[2] == w && last[1] + last[3] == y) {
      last[3] += h;
      return;
    }
  }
  if (r->damage_len == r->damage_cap) {
    int cap = r->damage_cap ? r->damage_cap * 2 : 64;
    int(*nd)[4] = (int(*)[4])realloc(r->damage, sizeof(int[4]) * (size_t)cap);
    if (!nd) {
      return;
    }
    r->damage = nd;
    r->damage_cap = cap;
  }
  int *d = r->damage[r->damage_len++];
  d[0] = x;
  d[1] = y;
  d[2] = w;
  d[3] = h;
}

void BUI_raster_end_frame(BuiRaster *r)
{
  r->damage_len = 0;
  const size_t total = (size_t)r->w * r->h;
  if (r->full_damage) {
    damage_push(r, 0, 0, r->w, r->h);
    memcpy(r->prev, r->pix, total * sizeof(uint32_t));
    r->full_damage = 0;
    return;
  }
  for (int ty = 0; ty < r->h; ty += TILE_H) {
    const int th = (ty + TILE_H <= r->h) ? TILE_H : r->h - ty;
    int run_x = -1;
    /* One step past the last tile acts as a clean sentinel that closes a
     * run touching the right edge (for any width, not just multiples of
     * TILE_W — otherwise right-edge changes were never reported). */
    for (int tx = 0; tx < r->w + TILE_W; tx += TILE_W) {
      int dirty = 0;
      if (tx < r->w) {
        const int tw = (tx + TILE_W <= r->w) ? TILE_W : r->w - tx;
        for (int y = ty; y < ty + th; y++) {
          const size_t o = (size_t)y * r->w + tx;
          if (memcmp(r->pix + o, r->prev + o, (size_t)tw * sizeof(uint32_t)) != 0) {
            dirty = 1;
            break;
          }
        }
      }
      if (dirty && run_x < 0) {
        run_x = tx;
      }
      else if (!dirty && run_x >= 0) {
        const int end = tx < r->w ? tx : r->w;
        damage_push(r, run_x, ty, end - run_x, th);
        for (int y = ty; y < ty + th; y++) {
          const size_t o = (size_t)y * r->w + run_x;
          memcpy(r->prev + o, r->pix + o, (size_t)(end - run_x) * sizeof(uint32_t));
        }
        run_x = -1;
      }
    }
  }
}

int BUI_raster_width(const BuiRaster *r)
{
  return r->w;
}
int BUI_raster_height(const BuiRaster *r)
{
  return r->h;
}
const uint32_t *BUI_raster_pixels(const BuiRaster *r)
{
  return r->pix;
}

int BUI_raster_damage(const BuiRaster *r, const int (**r_rects)[4])
{
  *r_rects = (const int(*)[4])r->damage;
  return r->damage_len;
}

void BUI_raster_invalidate(BuiRaster *r)
{
  r->full_damage = 1;
}

void BUI_raster_set_clip(BuiRaster *r, int x, int y, int w, int h)
{
  if (w <= 0 || h <= 0) {
    raster_reset_clip(r);
    return;
  }
  r->cx0 = clampi(x, 0, r->w);
  r->cy0 = clampi(y, 0, r->h);
  r->cx1 = clampi(x + w, 0, r->w);
  r->cy1 = clampi(y + h, 0, r->h);
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Primitives
 * \{ */

void BUI_raster_fill_rect(BuiRaster *r, float x, float y, float w, float h, const float rgba[4])
{
  int x0, y0, x1, y1;
  if (!clip_box(r, x, y, x + w, y + h, &x0, &y0, &x1, &y1)) {
    return;
  }
  const Paint p = paint_from(rgba);
  if (p.a <= 0) {
    return;
  }
  const float fx0 = x, fx1 = x + w, fy0 = y, fy1 = y + h;
  for (int py = y0; py < y1; py++) {
    const float cy = fminf(fy1, (float)py + 1.0f) - fmaxf(fy0, (float)py);
    if (cy <= 0.0f) {
      continue;
    }
    uint32_t *row = r->pix + (size_t)py * r->w;
    /* fully covered interior span */
    const int ix0 = clampi((int)ceilf(fx0), x0, x1);
    const int ix1 = clampi((int)floorf(fx1), x0, x1);
    for (int px = x0; px < ix0 && px < x1; px++) {
      const float cx = fminf(fx1, (float)px + 1.0f) - fmaxf(fx0, (float)px);
      put_cov(row + px, p, cx * cy);
    }
    if (cy >= 1.0f && p.a >= 256) {
      for (int px = ix0; px < ix1; px++) {
        row[px] = p.rgb;
      }
    }
    else {
      const int f = (int)(cy * (float)p.a + 0.5f);
      if (f >= 256) {
        for (int px = ix0; px < ix1; px++) {
          row[px] = p.rgb;
        }
      }
      else if (f > 0) {
        for (int px = ix0; px < ix1; px++) {
          row[px] = blend(row[px], p.rgb, (uint32_t)f);
        }
      }
    }
    /* trailing partial column(s); when the rect sits inside one pixel the
     * leading loop already covered it (ix1 < ix0). */
    for (int px = (ix1 > ix0 ? ix1 : ix0); px < x1; px++) {
      const float cx = fminf(fx1, (float)px + 1.0f) - fmaxf(fx0, (float)px);
      put_cov(row + px, p, cx * cy);
    }
  }
}

void BUI_raster_frame_rect(
    BuiRaster *r, float x, float y, float w, float h, float t, const float rgba[4])
{
  if (t <= 0.0f || w <= 0.0f || h <= 0.0f) {
    return;
  }
  if (t * 2.0f >= w || t * 2.0f >= h) {
    BUI_raster_fill_rect(r, x, y, w, h, rgba);
    return;
  }
  /* Non-overlapping sides so translucent borders blend once. */
  BUI_raster_fill_rect(r, x, y, w, t, rgba);
  BUI_raster_fill_rect(r, x, y + h - t, w, t, rgba);
  BUI_raster_fill_rect(r, x, y + t, t, h - 2.0f * t, rgba);
  BUI_raster_fill_rect(r, x + w - t, y + t, t, h - 2.0f * t, rgba);
}

void BUI_raster_round_rect(
    BuiRaster *r, float x, float y, float w, float h, float radius, const float rgba[4])
{
  if (w <= 0.0f || h <= 0.0f) {
    return;
  }
  const float hw = w * 0.5f, hh = h * 0.5f;
  float rad = fminf(radius, fminf(hw, hh));
  if (rad <= 0.25f) {
    BUI_raster_fill_rect(r, x, y, w, h, rgba);
    return;
  }
  int x0, y0, x1, y1;
  if (!clip_box(r, x, y, x + w, y + h, &x0, &y0, &x1, &y1)) {
    return;
  }
  const Paint p = paint_from(rgba);
  if (p.a <= 0) {
    return;
  }
  const float cx = x + hw, cy = y + hh;
  const float bx = hw - rad, by = hh - rad;
  const float rin = rad - 0.5f; /* coverage is exactly 1 inside this radius */
  for (int py = y0; py < y1; py++) {
    uint32_t *row = r->pix + (size_t)py * r->w;
    const float qy = fabsf((float)py + 0.5f - cy) - by;
    /* Half-width (from cx) of this row's fully covered span: the interior
     * is filled directly; the SDF only runs on the anti-aliased rim. */
    float ih = -1.0f;
    if (rin > 0.0f) {
      if (qy <= 0.0f) {
        ih = bx + rin;
      }
      else if (qy < rin) {
        ih = bx + sqrtf(rin * rin - qy * qy);
      }
    }
    int sa = x1, sb = x0; /* solid span [sa, sb) */
    if (ih >= 0.0f) {
      sa = clampi((int)ceilf(cx - ih - 0.5f), x0, x1);
      sb = clampi((int)floorf(cx + ih - 0.5f) + 1, x0, x1);
      if (sb > sa) {
        if (p.a >= 256) {
          for (int px = sa; px < sb; px++) {
            row[px] = p.rgb;
          }
        }
        else {
          for (int px = sa; px < sb; px++) {
            row[px] = blend(row[px], p.rgb, (uint32_t)p.a);
          }
        }
      }
      else {
        sa = x1;
        sb = x0;
      }
    }
    for (int px = x0; px < x1; px++) {
      if (px == sa) {
        px = sb - 1; /* skip the solid span */
        continue;
      }
      const float qx = fabsf((float)px + 0.5f - cx) - bx;
      float d;
      if (qx <= 0.0f && qy <= 0.0f) {
        d = fmaxf(qx, qy) - rad;
        if (d <= -0.5f) {
          if (p.a >= 256) {
            row[px] = p.rgb;
          }
          else {
            row[px] = blend(row[px], p.rgb, (uint32_t)p.a);
          }
          continue;
        }
      }
      else {
        const float mx = fmaxf(qx, 0.0f), my = fmaxf(qy, 0.0f);
        d = sqrtf(mx * mx + my * my) + fminf(fmaxf(qx, qy), 0.0f) - rad;
      }
      put_cov(row + px, p, clampf(0.5f - d, 0.0f, 1.0f));
    }
  }
}

void BUI_raster_circle(
    BuiRaster *r, float cx, float cy, float radius, float line_width, const float rgba[4])
{
  if (radius <= 0.0f) {
    return;
  }
  const float ext = radius + (line_width > 0.0f ? line_width : 0.0f) + 1.0f;
  int x0, y0, x1, y1;
  if (!clip_box(r, cx - ext, cy - ext, cx + ext, cy + ext, &x0, &y0, &x1, &y1)) {
    return;
  }
  const Paint p = paint_from(rgba);
  if (p.a <= 0) {
    return;
  }
  const float half_lw = line_width * 0.5f;
  for (int py = y0; py < y1; py++) {
    uint32_t *row = r->pix + (size_t)py * r->w;
    const float dy = (float)py + 0.5f - cy;
    for (int px = x0; px < x1; px++) {
      const float dx = (float)px + 0.5f - cx;
      const float dist = sqrtf(dx * dx + dy * dy);
      float cov;
      if (line_width > 0.0f) {
        cov = clampf(half_lw - fabsf(dist - radius) + 0.5f, 0.0f, 1.0f);
      }
      else {
        cov = clampf(radius - dist + 0.5f, 0.0f, 1.0f);
      }
      put_cov(row + px, p, cov);
    }
  }
}

void BUI_raster_line(
    BuiRaster *r, float x1, float y1, float x2, float y2, float width, const float rgba[4])
{
  if (width <= 0.0f) {
    width = 1.0f;
  }
  /* Axis-aligned lines are pixel-exact spans (crisp, like GL lines on the
   * pixel grid). Integer coords address the pixel to the right/below. */
  if (y1 == y2) {
    const float xa = fminf(x1, x2), xb = fmaxf(x1, x2);
    BUI_raster_fill_rect(r, xa, y1 - (width - 1.0f) * 0.5f, fmaxf(xb - xa, 1.0f), width, rgba);
    return;
  }
  if (x1 == x2) {
    const float ya = fminf(y1, y2), yb = fmaxf(y1, y2);
    BUI_raster_fill_rect(r, x1 - (width - 1.0f) * 0.5f, ya, width, fmaxf(yb - ya, 1.0f), rgba);
    return;
  }
  /* Diagonal: anti-aliased capsule between pixel centers. */
  const float ax = x1 + 0.5f, ay = y1 + 0.5f, bx = x2 + 0.5f, by = y2 + 0.5f;
  const float hw = width * 0.5f;
  int ix0, iy0, ix1, iy1;
  if (!clip_box(r, fminf(ax, bx) - hw - 1.0f, fminf(ay, by) - hw - 1.0f,
                fmaxf(ax, bx) + hw + 1.0f, fmaxf(ay, by) + hw + 1.0f, &ix0, &iy0, &ix1, &iy1))
  {
    return;
  }
  const Paint p = paint_from(rgba);
  if (p.a <= 0) {
    return;
  }
  const float vx = bx - ax, vy = by - ay;
  const float len2 = vx * vx + vy * vy;
  for (int py = iy0; py < iy1; py++) {
    uint32_t *row = r->pix + (size_t)py * r->w;
    const float wy = (float)py + 0.5f - ay;
    for (int px = ix0; px < ix1; px++) {
      const float wx = (float)px + 0.5f - ax;
      const float t = clampf((wx * vx + wy * vy) / len2, 0.0f, 1.0f);
      const float ex = wx - vx * t, ey = wy - vy * t;
      const float d = sqrtf(ex * ex + ey * ey) - hw;
      put_cov(row + px, p, clampf(0.5f - d, 0.0f, 1.0f));
    }
  }
}

void BUI_raster_ellipse(BuiRaster *r,
                        float cx,
                        float cy,
                        float rx,
                        float ry,
                        float line_width,
                        const float rgba[4])
{
  rx = fabsf(rx);
  ry = fabsf(ry);
  if (rx <= 0.0f || ry <= 0.0f) {
    return;
  }
  if (fabsf(rx - ry) < 1e-3f) {
    BUI_raster_circle(r, cx, cy, rx, line_width, rgba);
    return;
  }
  const float ext = (line_width > 0.0f ? line_width : 0.0f) + 1.0f;
  int x0, y0, x1, y1;
  if (!clip_box(r, cx - rx - ext, cy - ry - ext, cx + rx + ext, cy + ry + ext, &x0, &y0, &x1,
                &y1))
  {
    return;
  }
  const Paint p = paint_from(rgba);
  if (p.a <= 0) {
    return;
  }
  const float half_lw = line_width * 0.5f;
  for (int py = y0; py < y1; py++) {
    uint32_t *row = r->pix + (size_t)py * r->w;
    const float dy = ((float)py + 0.5f - cy) / ry;
    for (int px = x0; px < x1; px++) {
      const float dx = ((float)px + 0.5f - cx) / rx;
      /* First-order distance: (|p/r| - 1) / |grad| — accurate near the edge,
       * which is all anti-aliasing needs. */
      const float k = sqrtf(dx * dx + dy * dy);
      const float gx = dx / rx, gy = dy / ry;
      const float g = sqrtf(gx * gx + gy * gy);
      const float d = g > 0.0f ? (k - 1.0f) * k / g : -fminf(rx, ry);
      float cov;
      if (line_width > 0.0f) {
        cov = clampf(half_lw - fabsf(d) + 0.5f, 0.0f, 1.0f);
      }
      else {
        cov = clampf(0.5f - d, 0.0f, 1.0f);
      }
      put_cov(row + px, p, cov);
    }
  }
}

/* Signed distance to a rounded box with a per-corner radius mask. Pixel
 * center (sx, sy); box center (cx, cy), half size (hw, hh). */
static inline float sd_widget_box(
    float sx, float sy, float cx, float cy, float hw, float hh, float rad, int corners)
{
  const float px = sx - cx, py = sy - cy;
  int bit;
  if (py < 0.0f) {
    bit = px < 0.0f ? BUI_CORNER_TOP_LEFT : BUI_CORNER_TOP_RIGHT;
  }
  else {
    bit = px < 0.0f ? BUI_CORNER_BOTTOM_LEFT : BUI_CORNER_BOTTOM_RIGHT;
  }
  const float r = (corners & bit) ? rad : 0.0f;
  const float qx = fabsf(px) - (hw - r), qy = fabsf(py) - (hh - r);
  /* Straight edges need no sqrt; only true corner pixels do. */
  if (qx <= 0.0f || qy <= 0.0f) {
    return fmaxf(qx, qy) - r;
  }
  return sqrtf(qx * qx + qy * qy) - r;
}

void BUI_raster_widget(BuiRaster *r,
                       float x,
                       float y,
                       float w,
                       float h,
                       float radius,
                       int corners,
                       const float inner_top[4],
                       const float inner_bottom[4],
                       const float outline[4],
                       float outline_width,
                       const float emboss[4])
{
  if (w <= 0.0f || h <= 0.0f) {
    return;
  }
  const float hw = w * 0.5f, hh = h * 0.5f;
  const float rad = clampf(radius, 0.0f, fminf(hw, hh));
  const float cx = x + hw, cy = y + hh;
  const float ow = outline ? fmaxf(outline_width, 0.0f) : 0.0f;
  const int has_emboss = emboss && emboss[3] > 0.0f;
  int x0, y0, x1, y1;
  if (!clip_box(r, x - 1.0f, y - 1.0f, x + w + 1.0f, y + h + (has_emboss ? 2.0f : 1.0f), &x0, &y0,
                &x1, &y1))
  {
    return;
  }
  const float *top = inner_top ? inner_top : inner_bottom;
  const float *bot = inner_bottom ? inner_bottom : inner_top;
  const int gradient = top && bot && memcmp(top, bot, sizeof(float[4])) != 0;
  Paint p_inner = top ? paint_from(top) : (Paint){0, 0};
  const Paint p_out = outline ? paint_from(outline) : (Paint){0, 0};
  const Paint p_emb = has_emboss ? paint_from(emboss) : (Paint){0, 0};
  /* Fully covered interior margin: no SDF needed farther than this from
   * every edge (skips sqrt for the bulk of each widget). */
  const float safe = rad + ow + 1.0f;

  for (int py = y0; py < y1; py++) {
    uint32_t *row = r->pix + (size_t)py * r->w;
    const float sy = (float)py + 0.5f;
    if (gradient) {
      const float t = clampf((sy - y) / h, 0.0f, 1.0f);
      float c[4];
      for (int k = 0; k < 4; k++) {
        c[k] = top[k] + (bot[k] - top[k]) * t;
      }
      p_inner = paint_from(c);
    }
    const int row_inside = (sy - y) >= safe && (y + h - sy) >= safe;
    /* Interior span of this row: filled directly, no per-pixel SDF. */
    int sa = x1, sb = x1;
    if (row_inside) {
      sa = clampi((int)ceilf(x + safe - 0.5f), x0, x1);
      sb = clampi((int)floorf(x + w - safe - 0.5f) + 1, x0, x1);
      if (sb <= sa) {
        sa = sb = x1;
      }
      else if (top) {
        if (p_inner.a >= 256) {
          for (int px = sa; px < sb; px++) {
            row[px] = p_inner.rgb;
          }
        }
        else if (p_inner.a > 0) {
          for (int px = sa; px < sb; px++) {
            row[px] = blend(row[px], p_inner.rgb, (uint32_t)p_inner.a);
          }
        }
      }
    }
    for (int px = x0; px < x1; px++) {
      if (px == sa) {
        px = sb - 1; /* skip the interior span */
        continue;
      }
      const float sx = (float)px + 0.5f;
      const float d = sd_widget_box(sx, sy, cx, cy, hw, hh, rad, corners);
      const float cov = clampf(0.5f - d, 0.0f, 1.0f);
      if (has_emboss && sy > cy) {
        /* Emboss: the same shape shifted 1px down, visible only outside. */
        const float de = sd_widget_box(sx, sy - 1.0f, cx, cy, hw, hh, rad, corners);
        const float ecov = clampf(0.5f - de, 0.0f, 1.0f) - cov;
        if (ecov > 0.0f) {
          put_cov(row + px, p_emb, ecov);
        }
      }
      if (cov <= 0.0f) {
        continue;
      }
      if (top) {
        put_cov(row + px, p_inner, cov);
      }
      if (ow > 0.0f) {
        /* Outline band: inside the edge, `ow` pixels wide. */
        const float inner_cov = clampf(0.5f - (d + ow), 0.0f, 1.0f);
        const float band = cov - inner_cov;
        if (band > 0.0f) {
          put_cov(row + px, p_out, band);
        }
      }
    }
  }
}

/* Blender's widget base (GPU_SHADER_2D_WIDGET_BASE) in software: the same
 * parameters interface_widgets.cc feeds the shader. Outer shape `rect`/`rad`,
 * inner shape `rect` inset by `pixelsize` with `radi`; the band between them
 * is the outline, the outer shape shifted down by one pixel (bottom half
 * only) is the emboss. */
void BUI_raster_widget_base(BuiRaster *r, const BuiWidgetBase *p)
{
  const float x = p->rect[0], y = p->rect[1], w = p->rect[2], h = p->rect[3];
  if (w <= 0.0f || h <= 0.0f) {
    return;
  }
  const float px1 = p->pixelsize > 0.0f ? p->pixelsize : 1.0f;
  const float hw = w * 0.5f, hh = h * 0.5f, cx = x + hw, cy = y + hh;
  const float rad = clampf(p->rad, 0.0f, fminf(hw, hh));
  const float hwi = hw - px1, hhi = hh - px1;
  const float radi = clampf(p->radi, 0.0f, fmaxf(0.0f, fminf(hwi, hhi)));
  const int has_inner = p->inner1[3] > 0.0f || p->inner2[3] > 0.0f || p->alpha_checker;
  const int has_outline = p->outline[3] > 0.0f;
  const int has_emboss = p->emboss[3] > 0.0f;
  if (!has_inner && !has_outline && !has_emboss) {
    return;
  }
  int x0, y0, x1, y1;
  if (!clip_box(r, x - 1.0f, y - 1.0f, x + w + 1.0f, y + h + px1 + 1.0f, &x0, &y0, &x1, &y1)) {
    return;
  }
  const int gradient = memcmp(p->inner1, p->inner2, sizeof(float[4])) != 0;
  const int per_pixel_inner = p->alpha_checker || (gradient && p->shade_horizontal);
  const Paint p_out = paint_from(p->outline);
  const Paint p_emb = paint_from(p->emboss);
  Paint p_in = paint_from(p->inner1);
  /* Columns farther than this from the left/right edges never see a corner:
   * there the distance field only depends on the row (computed once). */
  const float cornerw = fmaxf(rad, radi) + px1 + 1.0f;
  const int ma = clampi((int)ceilf(x + cornerw - 0.5f), x0, x1);
  const int mb = clampi((int)floorf(x + w - cornerw - 0.5f) + 1, ma, x1);

  for (int py = y0; py < y1; py++) {
    uint32_t *row = r->pix + (size_t)py * r->w;
    const float sy = (float)py + 0.5f;
    if (gradient && !p->shade_horizontal) {
      const float t = clampf((sy - (y + px1)) / fmaxf(1.0f, h - 2.0f * px1), 0.0f, 1.0f);
      float c[4];
      for (int k = 0; k < 4; k++) {
        c[k] = p->inner1[k] + (p->inner2[k] - p->inner1[k]) * t;
      }
      p_in = paint_from(c);
    }
    /* -- middle columns: row-constant coverage -- */
    int span_a = ma, span_b = mb;
    if (per_pixel_inner) {
      span_a = span_b = x1; /* rare: checker / horizontal shading -> exact path */
    }
    if (span_a < span_b) {
      const float ay = fabsf(sy - cy);
      const float co = clampf(0.5f - (ay - hh), 0.0f, 1.0f);
      const float ci = fminf(co, clampf(0.5f - (ay - hhi), 0.0f, 1.0f));
      float ec = 0.0f;
      if (has_emboss && sy > cy) {
        ec = clampf(0.5f - (fabsf(sy - px1 - cy) - hh), 0.0f, 1.0f) - co;
      }
      const float band = has_outline ? co - ci : 0.0f;
      const int fill_inner = has_inner && ci > 0.0f;
      if (!(ec > 0.0f) && !(band > 0.0f) && fill_inner && ci >= 1.0f && p_in.a >= 256) {
        for (int i = span_a; i < span_b; i++) {
          row[i] = p_in.rgb; /* opaque interior: plain store */
        }
      }
      else if (ec > 0.0f || band > 0.0f || fill_inner) {
        const int fe = (int)(ec * (float)p_emb.a + 0.5f);
        const int fi = fill_inner ? (int)(ci * (float)p_in.a + 0.5f) : 0;
        const int fo = (int)(band * (float)p_out.a + 0.5f);
        for (int i = span_a; i < span_b; i++) {
          uint32_t d = row[i];
          if (fe > 0) {
            d = fe >= 256 ? p_emb.rgb : blend(d, p_emb.rgb, (uint32_t)fe);
          }
          if (fi > 0) {
            d = fi >= 256 ? p_in.rgb : blend(d, p_in.rgb, (uint32_t)fi);
          }
          if (fo > 0) {
            d = fo >= 256 ? p_out.rgb : blend(d, p_out.rgb, (uint32_t)fo);
          }
          row[i] = d;
        }
      }
    }
    /* -- corner columns (and everything on the exact path): full SDF -- */
    for (int px = x0; px < x1; px++) {
      if (px == span_a && span_a < span_b) {
        px = span_b - 1;
        continue;
      }
      const float sx = (float)px + 0.5f;
      const float d = sd_widget_box(sx, sy, cx, cy, hw, hh, rad, p->corners);
      const float co = clampf(0.5f - d, 0.0f, 1.0f);
      if (has_emboss && sy > cy) {
        const float de = sd_widget_box(sx, sy - px1, cx, cy, hw, hh, rad, p->corners);
        const float ec = clampf(0.5f - de, 0.0f, 1.0f) - co;
        if (ec > 0.0f) {
          put_cov(row + px, p_emb, ec);
        }
      }
      if (co <= 0.0f) {
        continue;
      }
      const float di = sd_widget_box(sx, sy, cx, cy, hwi, hhi, radi, p->corners);
      const float ci = fminf(co, clampf(0.5f - di, 0.0f, 1.0f));
      if (has_inner && ci > 0.0f) {
        if (p->alpha_checker) {
          /* checkerColorAndSize: 100/255 and 160/255, 8 px cells */
          const int cell = ((int)((sx - x) / 8.0f) + (int)((sy - y) / 8.0f)) & 1;
          const float g = cell ? 160.0f / 255.0f : 100.0f / 255.0f;
          const float gc[4] = {g, g, g, 1.0f};
          put_cov(row + px, paint_from(gc), ci);
        }
        Paint pi = p_in;
        if (gradient && p->shade_horizontal) {
          const float t = clampf((sx - (x + px1)) / fmaxf(1.0f, w - 2.0f * px1), 0.0f, 1.0f);
          float c[4];
          for (int k = 0; k < 4; k++) {
            c[k] = p->inner1[k] + (p->inner2[k] - p->inner1[k]) * t;
          }
          pi = paint_from(c);
        }
        put_cov(row + px, pi, ci);
      }
      if (has_outline && co - ci > 0.0f) {
        put_cov(row + px, p_out, co - ci);
      }
    }
  }
}

void BUI_raster_image(BuiRaster *r,
                      float x,
                      float y,
                      float w,
                      float h,
                      const uint32_t *argb,
                      int iw,
                      int ih,
                      float alpha)
{
  int x0, y0, x1, y1;
  if (!argb || iw <= 0 || ih <= 0 || !clip_box(r, x, y, x + w, y + h, &x0, &y0, &x1, &y1)) {
    return;
  }
  const int ga = (int)(clampf(alpha, 0.0f, 1.0f) * 256.0f + 0.5f);
  for (int py = y0; py < y1; py++) {
    uint32_t *row = r->pix + (size_t)py * r->w;
    const int sy = clampi((int)(((float)py + 0.5f - y) * ih / h), 0, ih - 1);
    const uint32_t *src = argb + (size_t)sy * iw;
    for (int px = x0; px < x1; px++) {
      const int sx = clampi((int)(((float)px + 0.5f - x) * iw / w), 0, iw - 1);
      const uint32_t c = src[sx];
      const int a = (int)((c >> 24) * ga) >> 8;
      if (a >= 255) {
        row[px] = c & 0xFFFFFFu;
      }
      else if (a > 0) {
        row[px] = blend(row[px], c & 0xFFFFFFu, (uint32_t)(a + (a >> 7)));
      }
    }
  }
}

void BUI_raster_end_frame_region(BuiRaster *r, int rx, int ry, int rw, int rh)
{
  /* Like end_frame, but only compares tiles touching the region that was
   * redrawn: the rest of the frame is known to be unchanged. */
  if (r->full_damage || rw <= 0 || rh <= 0) {
    BUI_raster_end_frame(r);
    return;
  }
  r->damage_len = 0;
  const int tx0 = clampi(rx, 0, r->w) / TILE_W * TILE_W;
  const int ty0 = clampi(ry, 0, r->h) / TILE_H * TILE_H;
  const int tx1 = clampi(rx + rw, 0, r->w), ty1 = clampi(ry + rh, 0, r->h);
  for (int ty = ty0; ty < ty1; ty += TILE_H) {
    const int th = (ty + TILE_H <= r->h) ? TILE_H : r->h - ty;
    int run_x = -1;
    for (int tx = tx0; tx < tx1 + TILE_W; tx += TILE_W) {
      int dirty = 0;
      if (tx < tx1) {
        const int tw = (tx + TILE_W <= r->w) ? TILE_W : r->w - tx;
        for (int y = ty; y < ty + th; y++) {
          const size_t o = (size_t)y * r->w + tx;
          if (memcmp(r->pix + o, r->prev + o, (size_t)tw * sizeof(uint32_t)) != 0) {
            dirty = 1;
            break;
          }
        }
      }
      if (dirty && run_x < 0) {
        run_x = tx;
      }
      else if (!dirty && run_x >= 0) {
        const int end = tx < r->w ? tx : r->w;
        damage_push(r, run_x, ty, end - run_x, th);
        for (int y = ty; y < ty + th; y++) {
          const size_t o = (size_t)y * r->w + run_x;
          memcpy(r->prev + o, r->pix + o, (size_t)(end - run_x) * sizeof(uint32_t));
        }
        run_x = -1;
      }
    }
  }
}

void BUI_raster_shadow(
    BuiRaster *r, float x, float y, float w, float h, float radius, float size, float alpha)
{
  if (w <= 0.0f || h <= 0.0f || size <= 0.0f || alpha <= 0.0f) {
    return;
  }
  int x0, y0, x1, y1;
  if (!clip_box(r, x - size, y - size * 0.5f, x + w + size, y + h + size * 1.5f, &x0, &y0, &x1,
                &y1))
  {
    return;
  }
  const float hw = w * 0.5f, hh = h * 0.5f;
  const float rad = clampf(radius, 0.0f, fminf(hw, hh));
  /* Light comes from above: the shadow sits slightly lower. */
  const float cx = x + hw, cy = y + hh + size * 0.25f;
  const Paint black = {0x000000u, 256};
  for (int py = y0; py < y1; py++) {
    uint32_t *row = r->pix + (size_t)py * r->w;
    const float sy = (float)py + 0.5f;
    for (int px = x0; px < x1; px++) {
      const float sx = (float)px + 0.5f;
      const float d = sd_widget_box(sx, sy, cx, cy, hw, hh, rad, BUI_CORNER_ALL);
      /* Only outside the box itself (the popup covers the inside). */
      const float dbox = sd_widget_box(sx, sy, x + hw, y + hh, hw, hh, rad, BUI_CORNER_ALL);
      if (dbox <= -0.5f || d >= size) {
        continue;
      }
      float t = 1.0f - clampf(d / size, 0.0f, 1.0f);
      t = t * t * (3.0f - 2.0f * t); /* smoothstep falloff */
      put_cov(row + px, black, t * alpha * clampf(0.5f + dbox, 0.0f, 1.0f));
    }
  }
}

#define POLY_SUBSAMPLES 4

static int cmp_float(const void *a, const void *b)
{
  const float fa = *(const float *)a, fb = *(const float *)b;
  return (fa > fb) - (fa < fb);
}

void BUI_raster_polygon(BuiRaster *r, const float (*pts)[2], int n, const float rgba[4])
{
  if (n < 3) {
    return;
  }
  float minx = pts[0][0], maxx = pts[0][0], miny = pts[0][1], maxy = pts[0][1];
  for (int i = 1; i < n; i++) {
    minx = fminf(minx, pts[i][0]);
    maxx = fmaxf(maxx, pts[i][0]);
    miny = fminf(miny, pts[i][1]);
    maxy = fmaxf(maxy, pts[i][1]);
  }
  int x0, y0, x1, y1;
  if (!clip_box(r, minx, miny, maxx, maxy, &x0, &y0, &x1, &y1)) {
    return;
  }
  const Paint p = paint_from(rgba);
  if (p.a <= 0) {
    return;
  }
  const int span = x1 - x0;
  float *cov = (float *)calloc((size_t)span + 1, sizeof(float));
  float *xs = (float *)malloc(sizeof(float) * (size_t)n);
  if (!cov || !xs) {
    free(cov);
    free(xs);
    return;
  }
  const float sub_w = 1.0f / POLY_SUBSAMPLES;
  for (int py = y0; py < y1; py++) {
    memset(cov, 0, sizeof(float) * ((size_t)span + 1));
    int any = 0;
    for (int s = 0; s < POLY_SUBSAMPLES; s++) {
      const float sy = (float)py + ((float)s + 0.5f) * sub_w;
      int nx = 0;
      for (int i = 0, j = n - 1; i < n; j = i++) {
        const float ay = pts[j][1], by = pts[i][1];
        if ((ay <= sy && by > sy) || (by <= sy && ay > sy)) {
          const float t = (sy - ay) / (by - ay);
          xs[nx++] = pts[j][0] + t * (pts[i][0] - pts[j][0]);
        }
      }
      if (nx < 2) {
        continue;
      }
      qsort(xs, (size_t)nx, sizeof(float), cmp_float);
      /* Even-odd spans with fractional horizontal coverage at the ends. */
      for (int k = 0; k + 1 < nx; k += 2) {
        const float a = fmaxf(xs[k], (float)x0), b = fminf(xs[k + 1], (float)x1);
        if (b <= a) {
          continue;
        }
        any = 1;
        const int ia = (int)floorf(a), ib = (int)floorf(b);
        if (ia == ib) {
          cov[ia - x0] += (b - a) * sub_w;
          continue;
        }
        cov[ia - x0] += ((float)ia + 1.0f - a) * sub_w;
        for (int x = ia + 1; x < ib; x++) {
          cov[x - x0] += sub_w;
        }
        if (ib < x1) {
          cov[ib - x0] += (b - (float)ib) * sub_w;
        }
      }
    }
    if (!any) {
      continue;
    }
    uint32_t *row = r->pix + (size_t)py * r->w;
    for (int x = 0; x < span; x++) {
      put_cov(row + x0 + x, p, fminf(cov[x], 1.0f));
    }
  }
  free(cov);
  free(xs);
}

static inline float edge_fn(float ax, float ay, float bx, float by, float px, float py)
{
  return (bx - ax) * (py - ay) - (by - ay) * (px - ax);
}

static inline int is_top_left(float ax, float ay, float bx, float by)
{
  /* For counter-clockwise (in y-down space: positive area) winding. */
  return (ay == by && bx < ax) || (by > ay);
}

void BUI_raster_triangles(BuiRaster *r,
                          const float (*co)[2],
                          const float (*col)[4],
                          int count)
{
  for (int i = 0; i + 2 < count; i += 3) {
    float v[3][2] = {{co[i][0], co[i][1]}, {co[i + 1][0], co[i + 1][1]}, {co[i + 2][0], co[i + 2][1]}};
    const float *c[3] = {col[i], col[i + 1], col[i + 2]};
    float area = edge_fn(v[0][0], v[0][1], v[1][0], v[1][1], v[2][0], v[2][1]);
    if (area == 0.0f || area != area) {
      continue;
    }
    if (area < 0.0f) {
      float tv[2] = {v[1][0], v[1][1]};
      v[1][0] = v[2][0];
      v[1][1] = v[2][1];
      v[2][0] = tv[0];
      v[2][1] = tv[1];
      const float *tc = c[1];
      c[1] = c[2];
      c[2] = tc;
      area = -area;
    }
    int x0, y0, x1, y1;
    if (!clip_box(r, fminf(v[0][0], fminf(v[1][0], v[2][0])), fminf(v[0][1], fminf(v[1][1], v[2][1])),
                  fmaxf(v[0][0], fmaxf(v[1][0], v[2][0])), fmaxf(v[0][1], fmaxf(v[1][1], v[2][1])),
                  &x0, &y0, &x1, &y1))
    {
      continue;
    }
    const int flat = memcmp(c[0], c[1], sizeof(float[4])) == 0 &&
                     memcmp(c[0], c[2], sizeof(float[4])) == 0;
    const Paint flat_p = paint_from(c[0]);
    const int tl0 = is_top_left(v[1][0], v[1][1], v[2][0], v[2][1]);
    const int tl1 = is_top_left(v[2][0], v[2][1], v[0][0], v[0][1]);
    const int tl2 = is_top_left(v[0][0], v[0][1], v[1][0], v[1][1]);
    const float inv_area = 1.0f / area;
    for (int py = y0; py < y1; py++) {
      uint32_t *row = r->pix + (size_t)py * r->w;
      const float sy = (float)py + 0.5f;
      for (int px = x0; px < x1; px++) {
        const float sx = (float)px + 0.5f;
        const float w0 = edge_fn(v[1][0], v[1][1], v[2][0], v[2][1], sx, sy);
        const float w1 = edge_fn(v[2][0], v[2][1], v[0][0], v[0][1], sx, sy);
        const float w2 = edge_fn(v[0][0], v[0][1], v[1][0], v[1][1], sx, sy);
        if ((w0 < 0.0f || (w0 == 0.0f && !tl0)) || (w1 < 0.0f || (w1 == 0.0f && !tl1)) ||
            (w2 < 0.0f || (w2 == 0.0f && !tl2)))
        {
          continue;
        }
        if (flat) {
          put_cov(row + px, flat_p, 1.0f);
        }
        else {
          float rgba[4];
          const float b0 = w0 * inv_area, b1 = w1 * inv_area, b2 = w2 * inv_area;
          for (int k = 0; k < 4; k++) {
            rgba[k] = c[0][k] * b0 + c[1][k] * b1 + c[2][k] * b2;
          }
          put_cov(row + px, paint_from(rgba), 1.0f);
        }
      }
    }
  }
}

void BUI_raster_points(BuiRaster *r,
                       const float (*co)[2],
                       const float (*col)[4],
                       int count,
                       float size)
{
  const float rad = fmaxf(0.5f, size * 0.5f);
  for (int i = 0; i < count; i++) {
    BUI_raster_circle(r, co[i][0], co[i][1], rad, 0.0f, col[i]);
  }
}

/* -------------------------------------------------------------------- */
/** \name Color widgets (HSV wheel, gradients, lit sphere)
 * \{ */

static void hsv_to_rgb_f(float h, float s, float v, float out[3])
{
  h = h - floorf(h);
  const float h6 = h * 6.0f;
  const float r = clampf(fabsf(h6 - 3.0f) - 1.0f, 0.0f, 1.0f);
  const float g = clampf(2.0f - fabsf(h6 - 2.0f), 0.0f, 1.0f);
  const float b = clampf(2.0f - fabsf(h6 - 4.0f), 0.0f, 1.0f);
  out[0] = ((r - 1.0f) * s + 1.0f) * v;
  out[1] = ((g - 1.0f) * s + 1.0f) * v;
  out[2] = ((b - 1.0f) * s + 1.0f) * v;
}

void BUI_raster_hsv_circle(BuiRaster *r, float cx, float cy, float radius, float value)
{
  int x0, y0, x1, y1;
  if (radius <= 0.0f ||
      !clip_box(r, cx - radius - 1, cy - radius - 1, cx + radius + 1, cy + radius + 1, &x0, &y0,
                &x1, &y1))
  {
    return;
  }
  const float inv_2pi = 0.15915494309189535f;
  for (int py = y0; py < y1; py++) {
    uint32_t *row = r->pix + (size_t)py * r->w;
    const float dy = (float)py + 0.5f - cy;
    for (int px = x0; px < x1; px++) {
      const float dx = (float)px + 0.5f - cx;
      const float dist = sqrtf(dx * dx + dy * dy);
      const float cov = clampf(radius - dist + 0.5f, 0.0f, 1.0f);
      if (cov <= 0.0f) {
        continue;
      }
      /* Blender's wheel (ui_hsvcircle_vals_from_pos): hue from the angle
       * measured from "up", red at the bottom; saturation = distance. */
      const float hue = atan2f(dx, -dy) * inv_2pi + 0.5f;
      float rgba[4] = {0, 0, 0, 1.0f};
      hsv_to_rgb_f(hue, fminf(dist / radius, 1.0f), value, rgba);
      put_cov(row + px, paint_from(rgba), cov);
    }
  }
}

void BUI_raster_gradient(BuiRaster *r,
                         float x,
                         float y,
                         float w,
                         float h,
                         const float tl[4],
                         const float tr[4],
                         const float br[4],
                         const float bl[4])
{
  int x0, y0, x1, y1;
  if (!clip_box(r, x, y, x + w, y + h, &x0, &y0, &x1, &y1)) {
    return;
  }
  for (int py = y0; py < y1; py++) {
    uint32_t *row = r->pix + (size_t)py * r->w;
    const float v = clampf(((float)py + 0.5f - y) / h, 0.0f, 1.0f);
    float left[4], right[4];
    for (int k = 0; k < 4; k++) {
      left[k] = tl[k] + (bl[k] - tl[k]) * v;
      right[k] = tr[k] + (br[k] - tr[k]) * v;
    }
    for (int px = x0; px < x1; px++) {
      const float u = clampf(((float)px + 0.5f - x) / w, 0.0f, 1.0f);
      float c[4];
      for (int k = 0; k < 4; k++) {
        c[k] = left[k] + (right[k] - left[k]) * u;
      }
      put_cov(row + px, paint_from(c), 1.0f);
    }
  }
}

void BUI_raster_sphere(
    BuiRaster *r, float cx, float cy, float radius, const float light[3], const float rgba[4])
{
  int x0, y0, x1, y1;
  if (radius <= 0.0f ||
      !clip_box(r, cx - radius - 1, cy - radius - 1, cx + radius + 1, cy + radius + 1, &x0, &y0,
                &x1, &y1))
  {
    return;
  }
  float L[3] = {light[0], light[1], light[2]};
  const float len = sqrtf(L[0] * L[0] + L[1] * L[1] + L[2] * L[2]);
  if (len > 0.0f) {
    L[0] /= len;
    L[1] /= len;
    L[2] /= len;
  }
  else {
    L[2] = 1.0f;
  }
  /* Half vector for a viewer looking down -z. */
  float H[3] = {L[0], L[1], L[2] + 1.0f};
  const float hl = sqrtf(H[0] * H[0] + H[1] * H[1] + H[2] * H[2]);
  if (hl > 0.0f) {
    H[0] /= hl;
    H[1] /= hl;
    H[2] /= hl;
  }
  for (int py = y0; py < y1; py++) {
    uint32_t *row = r->pix + (size_t)py * r->w;
    const float ny = -((float)py + 0.5f - cy) / radius; /* up is +y */
    for (int px = x0; px < x1; px++) {
      const float nx = ((float)px + 0.5f - cx) / radius;
      const float d2 = nx * nx + ny * ny;
      const float cov = clampf((1.0f - sqrtf(d2)) * radius + 0.5f, 0.0f, 1.0f);
      if (cov <= 0.0f) {
        continue;
      }
      const float nz = sqrtf(fmaxf(0.0f, 1.0f - d2));
      const float diff = fmaxf(0.0f, nx * L[0] + ny * L[1] + nz * L[2]);
      float spec = fmaxf(0.0f, nx * H[0] + ny * H[1] + nz * H[2]);
      spec = spec * spec;
      spec = spec * spec;
      spec = spec * spec * spec * spec; /* ^32 */
      float c[4];
      for (int k = 0; k < 3; k++) {
        c[k] = rgba[k] * (0.12f + 0.88f * diff) + 0.6f * spec;
      }
      c[3] = rgba[3];
      put_cov(row + px, paint_from(c), cov);
    }
  }
}

/** \} */

void BUI_raster_set_font(BuiRaster *r, const char *family, int flags)
{
  if (!family) {
    family = "";
  }
  strncpy(r->font_family, family, sizeof(r->font_family) - 1);
  r->font_family[sizeof(r->font_family) - 1] = '\0';
  r->font_flags = flags;
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Native text + present (Win32 GDI)
 * \{ */

#ifdef _WIN32
#  ifndef WIN32_LEAN_AND_MEAN
#    define WIN32_LEAN_AND_MEAN
#  endif
#  include <windows.h>

typedef struct FontSlot {
  char family[64];
  int px;
  int flags;
  HFONT font;
  int ascent;
  int height;
} FontSlot;

#  define MAX_FONTS 48
static FontSlot g_fonts[MAX_FONTS];
static int g_font_count = 0;
static HDC g_dc = NULL;

typedef struct TextRun {
  uint64_t hash;
  int font;
  int len;
  char *text;
  float advance;
  /* mask, built lazily on first draw */
  int built;
  int mw, mh, ox; /* ox: left padding baked into the mask */
  uint8_t *mask;
} TextRun;

#  define RUN_CAP 8192 /* power of two */
#  define RUN_MAX_FILL 6000
static TextRun *g_runs = NULL;
static int g_run_count = 0;

static void runs_flush(void)
{
  if (!g_runs) {
    return;
  }
  for (int i = 0; i < RUN_CAP; i++) {
    free(g_runs[i].text);
    free(g_runs[i].mask);
  }
  memset(g_runs, 0, sizeof(TextRun) * RUN_CAP);
  g_run_count = 0;
}

static void fonts_flush(void)
{
  for (int i = 0; i < g_font_count; i++) {
    DeleteObject(g_fonts[i].font);
  }
  g_font_count = 0;
  runs_flush(); /* runs reference font slots */
}

static HDC text_dc(void)
{
  if (!g_dc) {
    g_dc = CreateCompatibleDC(NULL);
    SetBkMode(g_dc, TRANSPARENT);
    SetTextAlign(g_dc, TA_LEFT | TA_TOP | TA_NOUPDATECP);
  }
  return g_dc;
}

static int font_get(const char *family, int flags, float size)
{
  if (!family || !family[0] || strcmp(family, "TkDefaultFont") == 0) {
    family = "Segoe UI";
  }
  int px = (int)(size + 0.5f);
  if (px < 1) {
    px = 1;
  }
  if (px > 512) {
    px = 512;
  }
  for (int i = 0; i < g_font_count; i++) {
    if (g_fonts[i].px == px && g_fonts[i].flags == flags &&
        strcmp(g_fonts[i].family, family) == 0)
    {
      return i;
    }
  }
  if (g_font_count == MAX_FONTS) {
    fonts_flush();
  }
  wchar_t wfam[64];
  if (MultiByteToWideChar(CP_UTF8, 0, family, -1, wfam, 64) == 0) {
    wcscpy(wfam, L"Segoe UI");
  }
  HFONT font = CreateFontW(-px, 0, 0, 0, (flags & BUI_FONT_BOLD) ? FW_BOLD : FW_NORMAL,
                           (flags & BUI_FONT_ITALIC) ? TRUE : FALSE,
                           (flags & BUI_FONT_UNDERLINE) ? TRUE : FALSE, FALSE, DEFAULT_CHARSET,
                           OUT_TT_PRECIS, CLIP_DEFAULT_PRECIS, ANTIALIASED_QUALITY,
                           DEFAULT_PITCH | FF_DONTCARE, wfam);
  if (!font) {
    return -1;
  }
  FontSlot *s = &g_fonts[g_font_count];
  strncpy(s->family, family, sizeof(s->family) - 1);
  s->family[sizeof(s->family) - 1] = '\0';
  s->px = px;
  s->flags = flags;
  s->font = font;
  HDC dc = text_dc();
  HGDIOBJ old = SelectObject(dc, font);
  TEXTMETRICW tm;
  GetTextMetricsW(dc, &tm);
  SelectObject(dc, old);
  s->ascent = tm.tmAscent;
  s->height = tm.tmHeight;
  return g_font_count++;
}

static uint64_t run_hash(int font, const char *s, int len)
{
  uint64_t h = 1469598103934665603ull ^ (uint64_t)(font * 2654435761u);
  for (int i = 0; i < len; i++) {
    h ^= (uint8_t)s[i];
    h *= 1099511628211ull;
  }
  return h ? h : 1;
}

static wchar_t *to_wide(const char *s, int len, int *r_n)
{
  int n = MultiByteToWideChar(CP_UTF8, 0, s, len, NULL, 0);
  wchar_t *w = (wchar_t *)malloc(sizeof(wchar_t) * (size_t)(n + 1));
  if (!w) {
    *r_n = 0;
    return NULL;
  }
  MultiByteToWideChar(CP_UTF8, 0, s, len, w, n);
  w[n] = 0;
  *r_n = n;
  return w;
}

static TextRun *run_get(int font, const char *s, int len)
{
  if (!g_runs) {
    g_runs = (TextRun *)calloc(RUN_CAP, sizeof(TextRun));
    if (!g_runs) {
      return NULL;
    }
  }
  const uint64_t h = run_hash(font, s, len);
  int idx = (int)(h & (RUN_CAP - 1));
  for (;;) {
    TextRun *e = &g_runs[idx];
    if (e->hash == 0) {
      break;
    }
    if (e->hash == h && e->font == font && e->len == len && memcmp(e->text, s, (size_t)len) == 0) {
      return e;
    }
    idx = (idx + 1) & (RUN_CAP - 1);
  }
  if (g_run_count >= RUN_MAX_FILL) {
    runs_flush();
    return run_get(font, s, len);
  }
  TextRun *e = &g_runs[idx];
  e->text = (char *)malloc((size_t)len + 1);
  if (!e->text) {
    return NULL;
  }
  memcpy(e->text, s, (size_t)len);
  e->text[len] = 0;
  e->hash = h;
  e->font = font;
  e->len = len;
  g_run_count++;

  int wn;
  wchar_t *w = to_wide(s, len, &wn);
  HDC dc = text_dc();
  HGDIOBJ old = SelectObject(dc, g_fonts[font].font);
  SIZE sz = {0, 0};
  if (w && wn > 0) {
    GetTextExtentPoint32W(dc, w, wn, &sz);
  }
  SelectObject(dc, old);
  free(w);
  e->advance = (float)sz.cx;
  return e;
}

static void run_build(TextRun *e)
{
  e->built = 1;
  const FontSlot *fs = &g_fonts[e->font];
  int wn;
  wchar_t *w = to_wide(e->text, e->len, &wn);
  if (!w || wn == 0 || e->advance <= 0.0f) {
    free(w);
    return;
  }
  const int pad = 2 + fs->px / 4; /* bearings / italic overhang */
  const int mw = (int)e->advance + pad * 2;
  const int mh = fs->height + 1;
  BITMAPINFO bmi;
  memset(&bmi, 0, sizeof(bmi));
  bmi.bmiHeader.biSize = sizeof(BITMAPINFOHEADER);
  bmi.bmiHeader.biWidth = mw;
  bmi.bmiHeader.biHeight = -mh;
  bmi.bmiHeader.biPlanes = 1;
  bmi.bmiHeader.biBitCount = 32;
  bmi.bmiHeader.biCompression = BI_RGB;
  void *bits = NULL;
  HDC dc = text_dc();
  HBITMAP bmp = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &bits, NULL, 0);
  if (!bmp || !bits) {
    free(w);
    return;
  }
  memset(bits, 0, (size_t)mw * mh * 4);
  HGDIOBJ old_bmp = SelectObject(dc, bmp);
  HGDIOBJ old_font = SelectObject(dc, fs->font);
  SetTextColor(dc, RGB(255, 255, 255));
  TextOutW(dc, pad, 0, w, wn);
  GdiFlush();
  e->mask = (uint8_t *)malloc((size_t)mw * mh);
  if (e->mask) {
    const uint32_t *src = (const uint32_t *)bits;
    for (int i = 0; i < mw * mh; i++) {
      /* grayscale AA: channels match; take green (perceptual) */
      e->mask[i] = (uint8_t)((src[i] >> 8) & 0xFF);
    }
    e->mw = mw;
    e->mh = mh;
    e->ox = pad;
  }
  SelectObject(dc, old_font);
  SelectObject(dc, old_bmp);
  DeleteObject(bmp);
  free(w);
}

int BUI_text_native(void)
{
  return 1;
}

int BUI_text_measure(
    const char *family, int flags, float size, const char *utf8, int len, float *r_width)
{
  *r_width = 0.0f;
  if (len <= 0) {
    return 1;
  }
  const int f = font_get(family, flags, size);
  if (f < 0) {
    return 0;
  }
  TextRun *e = run_get(f, utf8, len);
  if (!e) {
    return 0;
  }
  *r_width = e->advance;
  return 1;
}

int BUI_raster_text(BuiRaster *r,
                    float x,
                    float y,
                    const char *utf8,
                    int len,
                    float size,
                    const float rgba[4])
{
  if (len <= 0) {
    return 1;
  }
  const int f = font_get(r->font_family, r->font_flags, size);
  if (f < 0) {
    return 0;
  }
  TextRun *e = run_get(f, utf8, len);
  if (!e) {
    return 0;
  }
  if (!e->built) {
    run_build(e);
  }
  if (!e->mask) {
    return 1; /* whitespace-only or empty */
  }
  const Paint p = paint_from(rgba);
  if (p.a <= 0) {
    return 1;
  }
  const int dx = (int)floorf(x + 0.5f) - e->ox;
  const int dy = (int)floorf(y + 0.5f) - g_fonts[f].ascent;
  const int x0 = clampi(dx, r->cx0, r->cx1), x1 = clampi(dx + e->mw, r->cx0, r->cx1);
  const int y0 = clampi(dy, r->cy0, r->cy1), y1 = clampi(dy + e->mh, r->cy0, r->cy1);
  for (int py = y0; py < y1; py++) {
    uint32_t *row = r->pix + (size_t)py * r->w;
    const uint8_t *m = e->mask + (size_t)(py - dy) * e->mw - dx;
    for (int px = x0; px < x1; px++) {
      const uint32_t cov = m[px];
      if (cov == 0) {
        continue;
      }
      const uint32_t fac = ((uint32_t)p.a * (cov + (cov >> 7))) >> 8;
      if (fac >= 256) {
        row[px] = p.rgb;
      }
      else if (fac > 0) {
        row[px] = blend(row[px], p.rgb, fac);
      }
    }
  }
  return 1;
}

int BUI_raster_present_native(BuiRaster *r, void *window_handle, int full)
{
  HWND hwnd = (HWND)window_handle;
  if (!hwnd || !IsWindow(hwnd)) {
    return 0;
  }
  HDC hdc = GetDC(hwnd);
  if (!hdc) {
    return 0;
  }
  BITMAPINFO bmi;
  memset(&bmi, 0, sizeof(bmi));
  bmi.bmiHeader.biSize = sizeof(BITMAPINFOHEADER);
  bmi.bmiHeader.biWidth = r->w;
  bmi.bmiHeader.biPlanes = 1;
  bmi.bmiHeader.biBitCount = 32;
  bmi.bmiHeader.biCompression = BI_RGB;

  int whole[1][4] = {{0, 0, r->w, r->h}};
  const int(*rects)[4] = full ? (const int(*)[4])whole : (const int(*)[4])r->damage;
  const int n = full ? 1 : r->damage_len;
  /* Each rect is copied into its own tight top-down DIB and blitted whole
   * (src origin 0,0, all scanlines). Sub-rect source offsets into a
   * top-down DIB are interpreted inconsistently by GDI, which left partial
   * updates on screen stale. */
  uint32_t *tmp = NULL;
  size_t tmp_cap = 0;
  for (int i = 0; i < n; i++) {
    const int rx = rects[i][0], ry = rects[i][1], rw = rects[i][2], rh = rects[i][3];
    if (rw <= 0 || rh <= 0) {
      continue;
    }
    const uint32_t *bits = r->pix;
    if (!(rx == 0 && rw == r->w)) {
      const size_t need = (size_t)rw * rh;
      if (need > tmp_cap) {
        uint32_t *nt = (uint32_t *)realloc(tmp, need * sizeof(uint32_t));
        if (!nt) {
          continue;
        }
        tmp = nt;
        tmp_cap = need;
      }
      for (int row = 0; row < rh; row++) {
        memcpy(tmp + (size_t)row * rw, r->pix + (size_t)(ry + row) * r->w + rx,
               (size_t)rw * sizeof(uint32_t));
      }
      bits = tmp;
    }
    else {
      bits = r->pix + (size_t)ry * r->w; /* full-width band is contiguous */
    }
    bmi.bmiHeader.biWidth = rw;
    bmi.bmiHeader.biHeight = -rh;
    SetDIBitsToDevice(hdc, rx, ry, (DWORD)rw, (DWORD)rh, 0, 0, 0, (UINT)rh, bits, &bmi,
                      DIB_RGB_COLORS);
  }
  free(tmp);
  GdiFlush();
  ReleaseDC(hwnd, hdc);
  return 1;
}

#else /* !_WIN32 */

int BUI_text_native(void)
{
  return 0;
}

int BUI_text_measure(
    const char *family, int flags, float size, const char *utf8, int len, float *r_width)
{
  (void)family;
  (void)flags;
  (void)size;
  (void)utf8;
  (void)len;
  *r_width = 0.0f;
  return 0;
}

int BUI_raster_text(BuiRaster *r,
                    float x,
                    float y,
                    const char *utf8,
                    int len,
                    float size,
                    const float rgba[4])
{
  (void)r;
  (void)x;
  (void)y;
  (void)utf8;
  (void)len;
  (void)size;
  (void)rgba;
  return 0;
}

int BUI_raster_present_native(BuiRaster *r, void *window_handle, int full)
{
  (void)r;
  (void)window_handle;
  (void)full;
  return 0;
}

#endif

/** \} */
