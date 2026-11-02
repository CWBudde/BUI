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
  GPU_SHADER_2D_POINT_UNIFORM_SIZE_UNIFORM_COLOR_AA,
  GPU_SHADER_2D_POINT_UNIFORM_SIZE_UNIFORM_COLOR_OUTLINE_AA,
  GPU_SHADER_3D_UNIFORM_COLOR,
  GPU_SHADER_3D_FLAT_COLOR,
  GPU_SHADER_3D_POINT_FLAT_COLOR,
  GPU_SHADER_3D_SMOOTH_COLOR,
  GPU_SHADER_3D_DEPTH_ONLY,
  GPU_SHADER_3D_IMAGE_COLOR,
  GPU_SHADER_3D_LINE_DASHED_UNIFORM_COLOR,
  GPU_SHADER_3D_POINT_UNIFORM_SIZE_UNIFORM_COLOR_AA,
  GPU_SHADER_3D_POINT_VARYING_SIZE_VARYING_COLOR,
  /* Widget shaders. */
  GPU_SHADER_2D_WIDGET_BASE,
  GPU_SHADER_2D_WIDGET_BASE_INST,
  GPU_SHADER_2D_WIDGET_SHADOW,
  GPU_SHADER_SIMPLE_LIGHTING,
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
extern void (*bui_imm_theme_color_hook)(int region_color_id, float r_color[4]);
extern void (*bui_imm_theme_color_alpha_hook)(int region_color_id, float alpha, float r_color[4]);
void immUniformThemeColor(int color_id);
void immUniformThemeColorAlpha(int color_id, float alpha);

#ifdef __cplusplus
}
#endif
