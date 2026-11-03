/* BUI GPU renderer (Direct3D 11) — private interface for gpu_raster.c.
 *
 * A BuiRaster in GPU mode records its primitives as instances instead of
 * rasterizing them; this backend draws them into a persistent render target
 * (so damage-region redraws work exactly like the software raster) and
 * presents it with a flip-model swap chain plus dirty rects.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */
#pragma once

#include <stdint.h>

/* instance kinds: keep in sync with gpu/shaders/bui_ui.hlsl */
enum {
  BG_RECT = 0,
  BG_WIDGET = 1,
  BG_ELLIPSE = 2,
  BG_LINE = 3,
  BG_MASK = 4,
  BG_IMAGE = 5,
  BG_SHADOW = 6,
  BG_GRADIENT = 7,
  BG_HSV = 8,
  BG_SPHERE = 9,
  BG_SLUG = 10, /* glyph outline (Slug): rect = pen x, baseline y, px per unit, shear */
};
enum {
  BG_W_SHADE_H = 1,
  BG_W_CHECKER = 2,
  BG_W_LEGACY = 4,
  BG_W_INNER = 8,
};

typedef struct BuiGpuInst {
  float rect[4];
  float bounds[4]; /* x0, y0, x1, y1: pixel box, already clipped */
  float c0[4], c1[4], c2[4], c3[4];
  float p[4]; /* p[0] = kind */
  float uv[4];
} BuiGpuInst;

typedef struct BuiGpu BuiGpu;

/* Starts creating the device on a worker thread (the first frames keep
 * using the software raster). NULL when D3D11 isn't available. */
BuiGpu *bui_gpu_create(void *hwnd);
void bui_gpu_free(BuiGpu *g);
/* 1 once the device is ready, -1 when creation failed, 0 while pending
 * (never 0 with `wait`). */
int bui_gpu_state(BuiGpu *g, int wait);

void bui_gpu_begin(BuiGpu *g, int w, int h);
/* Appends an instance (zeroed except kind and bounds). */
BuiGpuInst *bui_gpu_inst(BuiGpu *g, int kind, int x0, int y0, int x1, int y1);
/* Atlas placement of an 8-bit mask keyed by `key` (NULL key: not cached).
 * Returns 0 when it can't be placed. */
int bui_gpu_mask(BuiGpu *g, const void *key, unsigned gen, const uint8_t *mask, int w, int h,
                 int *r_x, int *r_y);
/* Slug text (Eric Lengyel's algorithm): 1 when glyph outlines are
 * rendered on the GPU (default; BUI_SLUG=0 turns it off). */
int bui_gpu_slug_on(BuiGpu *g);
/* Curve/band data for glyph `glyph` of font face `face` (uploaded on first
 * use): its box in font units, the band transform (instance c1) and its
 * band location (instance uv). 0 for empty glyphs or when unavailable.
 * `gen` changes whenever faces may have been freed. */
int bui_gpu_slug_glyph(BuiGpu *g, int face, int glyph, unsigned gen, float r_bbox[4],
                       float r_band[4], int r_loc[4]);
/* Provided by the text engine (gpu_raster.c): a glyph's outline as
 * quadratic curves (6 floats each, font units, y up) and its box. */
int bui_text_glyph_outline(int face, int glyph, float **r_curves, int *r_n, float r_bbox[4]);

/* Atlas placement of a 0xAARRGGBB image (cached by content). */
int bui_gpu_image(BuiGpu *g, const uint32_t *argb, int w, int h, int *r_x, int *r_y);
void bui_gpu_triangles(BuiGpu *g,
                       const float (*co)[2],
                       const float (*col)[4],
                       int count,
                       const int clip[4]);
/* Records damage for present (x, y, w, h); w <= 0 means the whole frame. */
void bui_gpu_damage(BuiGpu *g, int x, int y, int w, int h);
/* Draws everything recorded and presents. Returns 0 on device loss. */
int bui_gpu_present(BuiGpu *g, int full);
/* Reads the frame back as 0x00RRGGBB rows. 0 when there is none yet. */
int bui_gpu_read(BuiGpu *g, uint32_t *dst, int w, int h);
