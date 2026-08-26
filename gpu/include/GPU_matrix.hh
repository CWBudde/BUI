/* BUI GPU seam: matrix stack.
 * Software implementation sufficient for 2D UI drawing.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#pragma once

#ifdef __cplusplus
extern "C" {
#endif

void GPU_matrix_identity_set(void);

void GPU_matrix_push(void);
void GPU_matrix_pop(void);

void GPU_matrix_push_projection(void);
void GPU_matrix_pop_projection(void);

void GPU_matrix_translate_2f(float x, float y);
void GPU_matrix_scale_1f(float scale);
void GPU_matrix_scale_2f(float x, float y);

/* Set an orthographic projection over the given extents (z near/far too). */
void GPU_matrix_ortho_set_z(float near, float far);
void GPU_matrix_projection_get(float result[4][4]);

#ifdef __cplusplus
}
#endif
