/* BUI GPU seam: immediate-mode utility helpers + builtin shader ids.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#pragma once

#include "GPU_immediate.hh"
#include "BLI_sys_types.h"

#ifdef __cplusplus
extern "C" {
#endif

/* Builtin shader identifiers (subset used by the UI toolkit). */
typedef enum eGPUBuiltinShader {
  GPU_SHADER_2D_UNIFORM_COLOR = 0,
  GPU_SHADER_2D_FLAT_COLOR,
  GPU_SHADER_2D_SMOOTH_COLOR,
  GPU_SHADER_2D_IMAGE,
  GPU_SHADER_2D_CHECKER,
  GPU_SHADER_2D_LINE_DASHED_UNIFORM_COLOR,
  /* Keep last. */
  GPU_SHADER_BUILTIN_LEN,
} eGPUBuiltinShader;

void immRectf(int pos_id, float x1, float y1, float x2, float y2);

void imm_draw_box_wire_2d(unsigned int pos_id, float x1, float y1, float x2, float y2);
void imm_draw_box_checker_2d(float x1, float y1, float x2, float y2);
void imm_draw_box_checker_2d_ex(float x1, float y1, float x2, float y2, int checker_size);
void imm_draw_circle_wire_2d(unsigned int pos_id, float x, float y, float radius, int nsegments);
void imm_draw_disk_partial_fill_2d(unsigned int pos_id,
                                   float x,
                                   float y,
                               float radius_inner,
                                   float radius_outer,
                                   float start_angle,
                                   float sweep_angle,
                                   int nsegments);

/* Theme-color uniforms are provided by the ui module via this hook to avoid a
 * gpu -> ui dependency. The toolkit installs it at startup. */
struct bThemeUIColor; /* opaque */
extern void (*bui_imm_theme_color_hook)(int region_color_id, float r_color[4]);
extern void (*bui_imm_theme_color_alpha_hook)(int region_color_id, float alpha, float r_color[4]);
void immUniformThemeColor(int color_id);
void immUniformThemeColorAlpha(int color_id, float alpha);

/* Pixel-texture drawing (icons etc). Null backend discards. */
void immDrawPixelsTexSetup(int shader_id); /* placeholder signature */
void immDrawPixelsTexScaledFullSize(
    float x, float y, float img_w, float img_h, float tex_w, float tex_h,
    float scale_x, float scale_y, int clip_min_x, int clip_min_y, int clip_max_x, int clip_max_y,
    bool linear_filter, const void *rect, int pixels_w, int pixels_h, int components,
    const float pixel_outline[4]);

#ifdef __cplusplus
}
#endif
