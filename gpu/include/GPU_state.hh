/* BUI GPU seam: state management.
 * Minimal replacement for upstream GPU_state.hh. Backed by a null backend;
 * plug a real renderer behind these calls.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#pragma once

#include "BLI_sys_types.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef enum eGPUBlend {
  GPU_BLEND_NONE = 0,
  GPU_BLEND_ALPHA,
  GPU_BLEND_ALPHA_PREMULT,
  GPU_BLEND_ADDITIVE,
} eGPUBlend;

typedef enum eGPUFaceCullTest {
  GPU_CULL_NONE = 0,
  GPU_CULL_FRONT,
  GPU_CULL_BACK,
} eGPUFaceCullTest;

void GPU_blend(eGPUBlend blend);
eGPUBlend GPU_blend_get(void);

void GPU_line_width(float width);
void GPU_line_smooth(bool enable);
void GPU_polygon_smooth(bool enable);

void GPU_point_size(float size);
void GPU_program_point_size(bool enable);

void GPU_scissor(int x, int y, int width, int height);
void GPU_scissor_test(bool enable);
void GPU_scissor_get(int coords[4]);

void GPU_face_culling(eGPUFaceCullTest culling);
void GPU_clear_color(float r, float g, float b, float a);

void GPU_viewport_size_get_f(float viewport[4]);

#ifdef __cplusplus
}
#endif
