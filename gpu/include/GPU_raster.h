/* BUI software raster backend.
 *
 * A complete frame renderer for hosts without a GPU context (Tk, plain Qt
 * widgets, headless). It plays the role Blender's GPU module + BLF play: the
 * whole UI is drawn into one RGB32 frame buffer in native code, then the host
 * presents it with a single blit — instead of turning every primitive into a
 * host-toolkit object.
 *
 *  - Anti-aliased analytic primitives (rect, rounded box, circle, line) using
 *    signed-distance coverage, like Blender's widget-base shaders.
 *  - Generic triangles with per-vertex color (for the BuiGpuSink seam).
 *  - Native text (GDI glyph-run cache on Windows); hosts fall back to their
 *    own text drawing where BUI_text_native() is 0.
 *  - Damage tracking: after each frame, the tiles that actually changed are
 *    reported so hosts upload only those.
 *
 * Pixel layout: uint32 0x00RRGGBB, top-down rows, stride == width. This is
 * byte-compatible with a Win32 32bpp BI_RGB DIB and QImage::Format_RGB32.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */
#pragma once

#include <stdint.h>

#include "GPU_backend_sink.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct BuiRaster BuiRaster;

enum {
  BUI_FONT_BOLD = 1 << 0,
  BUI_FONT_ITALIC = 1 << 1,
  BUI_FONT_UNDERLINE = 1 << 2,
};

BuiRaster *BUI_raster_create(int width, int height);
void BUI_raster_free(BuiRaster *r);

/* Frame bracket. begin_frame resizes (and fully damages) when the size
 * changes; end_frame computes the damage list against the previous frame. */
void BUI_raster_begin_frame(BuiRaster *r, int width, int height);
void BUI_raster_end_frame(BuiRaster *r);

int BUI_raster_width(const BuiRaster *r);
int BUI_raster_height(const BuiRaster *r);
const uint32_t *BUI_raster_pixels(const BuiRaster *r);

/* Damage from the last end_frame as {x, y, w, h} rects; returns the count. */
int BUI_raster_damage(const BuiRaster *r, const int (**r_rects)[4]);
/* Force the next end_frame to report the full frame as damaged. */
void BUI_raster_invalidate(BuiRaster *r);

/* Clip subsequent drawing; w/h <= 0 removes the clip. */
void BUI_raster_set_clip(BuiRaster *r, int x, int y, int w, int h);

void BUI_raster_fill_rect(BuiRaster *r, float x, float y, float w, float h, const float rgba[4]);
void BUI_raster_frame_rect(
    BuiRaster *r, float x, float y, float w, float h, float thickness, const float rgba[4]);
void BUI_raster_round_rect(
    BuiRaster *r, float x, float y, float w, float h, float radius, const float rgba[4]);
/* line_width <= 0 draws a filled disc. */
void BUI_raster_circle(
    BuiRaster *r, float cx, float cy, float radius, float line_width, const float rgba[4]);
void BUI_raster_line(
    BuiRaster *r, float x1, float y1, float x2, float y2, float width, const float rgba[4]);
/* Axis-aligned ellipse; line_width <= 0 fills. */
void BUI_raster_ellipse(BuiRaster *r,
                        float cx,
                        float cy,
                        float rx,
                        float ry,
                        float line_width,
                        const float rgba[4]);
/* Filled polygon (even-odd), anti-aliased; `points` holds `n` x/y pairs. */
void BUI_raster_polygon(BuiRaster *r, const float (*points)[2], int n, const float rgba[4]);

/* Rounded-box corner mask (Blender's UI_CNR_*). */
enum {
  BUI_CORNER_TOP_LEFT = 1 << 0,
  BUI_CORNER_TOP_RIGHT = 1 << 1,
  BUI_CORNER_BOTTOM_RIGHT = 1 << 2,
  BUI_CORNER_BOTTOM_LEFT = 1 << 3,
  BUI_CORNER_ALL = 15,
};

/* One Blender widget base (the GPU_SHADER_2D_WIDGET_BASE look) in a single
 * pass: inner fill shaded from `inner_top` to `inner_bottom`, an
 * `outline_width` outline inside the edge, and an emboss line just below
 * the box. Corners not in `corners` are square. Any color may be NULL. */
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
                       const float emboss[4]);

/* Parameters of Blender's widget-base shader (WidgetBaseParameters), in
 * screen space (y down). */
typedef struct BuiWidgetBase {
  float rect[4];        /* x, y, w, h of the outer shape */
  float rad, radi;      /* outer / inner corner radius */
  int corners;          /* BUI_CORNER_* mask */
  float inner1[4];      /* shade start: top (or left when shade_horizontal) */
  float inner2[4];      /* shade end: bottom (or right) */
  float outline[4];
  float emboss[4];
  int shade_horizontal; /* Blender's shade_dir == 0 */
  int alpha_checker;    /* checkerboard under the inner color */
  float pixelsize;      /* U.pixelsize */
} BuiWidgetBase;

void BUI_raster_widget_base(BuiRaster *r, const BuiWidgetBase *params);

/* Image (0xAARRGGBB, iw x ih) scaled into x, y, w, h (nearest), alpha blended. */
void BUI_raster_image(BuiRaster *r, float x, float y, float w, float h, const uint32_t *argb,
                      int iw, int ih, float alpha);

/* end_frame that only diffs the given region (everything else is known to
 * be unchanged): partial redraws cost O(region), not O(window). */
void BUI_raster_end_frame_region(BuiRaster *r, int x, int y, int w, int h);

/* Soft drop shadow around a rounded box (menus/popups), fading out over
 * `size` pixels from `alpha`. */
void BUI_raster_shadow(
    BuiRaster *r, float x, float y, float w, float h, float radius, float size, float alpha);
void BUI_raster_triangles(BuiRaster *r,
                          const float (*coords)[2],
                          const float (*colors)[4],
                          int count);
/* HSV color wheel (Blender's layout: red at the bottom, saturation grows
 * outward) at brightness `value`, anti-aliased edge. */
void BUI_raster_hsv_circle(BuiRaster *r, float cx, float cy, float radius, float value);
/* Rectangle with bilinear corner colors (top-left, top-right, bottom-right,
 * bottom-left). */
void BUI_raster_gradient(BuiRaster *r,
                         float x,
                         float y,
                         float w,
                         float h,
                         const float tl[4],
                         const float tr[4],
                         const float br[4],
                         const float bl[4]);
/* Lit sphere (normal ball): `light` is the direction (x right, y up, z out). */
void BUI_raster_sphere(
    BuiRaster *r, float cx, float cy, float radius, const float light[3], const float rgba[4]);
void BUI_raster_points(BuiRaster *r,
                       const float (*coords)[2],
                       const float (*colors)[4],
                       int count,
                       float size);

/* Text. `family` NULL/"" means the platform UI font; `size` is the em size in
 * pixels; y is the baseline. Returns 0 when no native text engine exists. */
int BUI_text_native(void);
void BUI_raster_set_font(BuiRaster *r, const char *family, int flags);
int BUI_raster_text(BuiRaster *r,
                    float x,
                    float y,
                    const char *utf8,
                    int len,
                    float size,
                    const float rgba[4]);
int BUI_text_measure(
    const char *family, int flags, float size, const char *utf8, int len, float *r_width);

/* Parallel rasterisation. A view draws into `src`'s pixels, clipped to
 * src's clip and the rectangle [x0, x1) x [y0, y1); views over disjoint
 * rectangles may be drawn into from different threads. `dst` points at
 * BUI_raster_view_size() bytes. Views only draw: no frames, damage or present. */
size_t BUI_raster_view_size(void);
void BUI_raster_view(const BuiRaster *src, BuiRaster *dst, int x0, int y0, int x1, int y1);

/* Text in two steps so the drawing can run on several threads: prepare
 * (one thread; builds the glyph mask) and draw the prepared run (any thread,
 * read-only). Prepared runs stay valid while BUI_text_generation() is
 * unchanged. Prepare returns NULL when there is nothing to draw or no native
 * text engine. */
typedef struct BuiTextRun BuiTextRun;
const BuiTextRun *BUI_text_prepare(
    const char *family, int flags, float size, const char *utf8, int len, float *r_width);
unsigned BUI_text_generation(void);
void BUI_raster_text_run(BuiRaster *r, float x, float y, const BuiTextRun *run, const float rgba[4]);

/* A BuiGpuSink that renders into `r` (user pointer is the raster). */
const BuiGpuSink *BUI_raster_sink(BuiRaster *r);

/* Blit damaged regions (or everything when `full`) straight to a native
 * window: an HWND on Windows. Returns 0 when unsupported on this platform. */
int BUI_raster_present_native(BuiRaster *r, void *window_handle, int full);

#ifdef __cplusplus
}
#endif
