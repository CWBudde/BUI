/* BUI GPU null backend.
 * Records state, discards geometry. Swap in a real renderer (GL/Metal/Vulkan/
 * software rasterizer) by implementing the same seam headers.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#include <stdlib.h>
#include <string.h>

#include "BLI_utildefines.h"

#include "GPU_batch.hh"
#include "GPU_immediate.hh"
#include "GPU_immediate_util.hh"
#include "GPU_matrix.hh"
#include "GPU_state.hh"

/* -------------------------------------------------------------------- */
/** \name State
 * \{ */

static eGPUBlend bui_blend = GPU_BLEND_NONE;
static int bui_scissor[4] = {0, 0, 0, 0};
static bool bui_scissor_enabled = false;
static float bui_viewport[4] = {0.0f, 0.0f, 0.0f, 0.0f};

void GPU_blend(eGPUBlend blend)
{
  bui_blend = blend;
}
eGPUBlend GPU_blend_get(void)
{
  return bui_blend;
}
void GPU_line_width(float UNUSED(width)) {}
void GPU_line_smooth(bool UNUSED(enable)) {}
void GPU_polygon_smooth(bool UNUSED(enable)) {}
void GPU_point_size(float UNUSED(size)) {}
void GPU_program_point_size(bool UNUSED(enable)) {}
void GPU_scissor(int x, int y, int width, int height)
{
  bui_scissor[0] = x;
  bui_scissor[1] = y;
  bui_scissor[2] = width;
  bui_scissor[3] = height;
}
void GPU_scissor_test(bool enable)
{
  bui_scissor_enabled = enable;
}
void GPU_scissor_get(int coords[4])
{
  memcpy(coords, bui_scissor, sizeof(bui_scissor));
}
void GPU_face_culling(eGPUFaceCullTest UNUSED(culling)) {}
void GPU_clear_color(float r, float g, float b, float a)
{
  /* Record as viewport clear state; nothing to present in the null backend. */
  bui_viewport[0] = r;
  bui_viewport[1] = g;
  bui_viewport[2] = b;
  bui_viewport[3] = a;
}
void GPU_viewport_size_get_f(float viewport[4])
{
  memcpy(viewport, bui_viewport, sizeof(bui_viewport));
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Matrix stack
 * \{ */

#define BUI_MATRIX_MAX_DEPTH 32
static float bui_model[BUI_MATRIX_MAX_DEPTH][16];
static int bui_model_depth = 0;
static float bui_projection_stack[BUI_MATRIX_MAX_DEPTH][16];
static int bui_projection_depth = 0;

static void bui_mat_identity(float m[16])
{
  memset(m, 0, sizeof(float) * 16);
  m[0] = m[5] = m[10] = m[15] = 1.0f;
}

void GPU_matrix_identity_set(void)
{
  bui_mat_identity(bui_model[bui_model_depth]);
}
void GPU_matrix_push(void)
{
  if (bui_model_depth + 1 < BUI_MATRIX_MAX_DEPTH) {
    memcpy(bui_model[bui_model_depth + 1], bui_model[bui_model_depth], sizeof(float) * 16);
    bui_model_depth++;
  }
}
void GPU_matrix_pop(void)
{
  if (bui_model_depth > 0) {
    bui_model_depth--;
  }
}
void GPU_matrix_push_projection(void)
{
  if (bui_projection_depth + 1 < BUI_MATRIX_MAX_DEPTH) {
    bui_projection_depth++;
  }
}
void GPU_matrix_pop_projection(void)
{
  if (bui_projection_depth > 0) {
    bui_projection_depth--;
  }
}
void GPU_matrix_translate_2f(float x, float y)
{
  float *m = bui_model[bui_model_depth];
  m[12] += m[0] * x + m[4] * y;
  m[13] += m[1] * x + m[5] * y;
}
void GPU_matrix_scale_1f(float scale)
{
  GPU_matrix_scale_2f(scale, scale);
}
void GPU_matrix_scale_2f(float x, float y)
{
  float *m = bui_model[bui_model_depth];
  m[0] *= x;
  m[1] *= x;
  m[4] *= y;
  m[5] *= y;
}
void GPU_matrix_ortho_set_z(float near_z, float far_z)
{
  float *m = bui_projection_stack[bui_projection_depth];
  m[10] = -2.0f / (far_z - near_z);
  m[14] = -(far_z + near_z) / (far_z - near_z);
}
void GPU_matrix_projection_get(float result[4][4])
{
  memcpy(result, bui_projection_stack[bui_projection_depth], sizeof(float) * 16);
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Immediate mode (null sink)
 * \{ */

static int bui_bound_shader = -1;

unsigned int immVertexFormat(void)
{
  return 0;
}
bool immIsShaderBound(void)
{
  return bui_bound_shader != -1;
}
void immBindBuiltinProgram(int shader_id)
{
  bui_bound_shader = shader_id;
}
void immUnbindProgram(void)
{
  bui_bound_shader = -1;
}
void immBegin(eGPUPrimType UNUSED(primitive), unsigned int UNUSED(vertex_len)) {}
void immBeginAtMost(eGPUPrimType UNUSED(primitive), unsigned int UNUSED(vertex_len)) {}
void immEnd() {}
void immVertex2f(unsigned int UNUSED(attrib), float UNUSED(x), float UNUSED(y)) {}
void immVertex2fv(unsigned int UNUSED(attrib), const float UNUSED(data[2])) {}
void immAttr3fv(unsigned int UNUSED(attrib), const float UNUSED(data[3])) {}
void immAttr4f(unsigned int UNUSED(attrib), float UNUSED(x), float UNUSED(y), float UNUSED(z),
               float UNUSED(w))
{
}
void immAttr4fv(unsigned int UNUSED(attrib), const float UNUSED(data[4])) {}
void immAttr4ub(unsigned int UNUSED(attrib), unsigned char UNUSED(r), unsigned char UNUSED(g),
                unsigned char UNUSED(b), unsigned char UNUSED(a))
{
}
void immAttr4ubv(unsigned int UNUSED(attrib), const unsigned char UNUSED(data[4])) {}
void immAttrSkip(unsigned int UNUSED(attrib)) {}

void immUniform1f(const char *UNUSED(name), float UNUSED(x)) {}
void immUniform1i(const char *UNUSED(name), int UNUSED(x)) {}
void immUniform2f(const char *UNUSED(name), float UNUSED(x), float UNUSED(y)) {}
void immUniform2fv(const char *UNUSED(name), const float UNUSED(data[2])) {}
void immUniform4f(const char *UNUSED(name), float UNUSED(x), float UNUSED(y), float UNUSED(z),
                  float UNUSED(w))
{
}
void immUniform4fv(const char *UNUSED(name), const float UNUSED(data[4])) {}

void immUniformColor4f(float UNUSED(r), float UNUSED(g), float UNUSED(b), float UNUSED(a)) {}
void immUniformColor4fv(const float UNUSED(rgba[4])) {}
void immUniformColor3f(float UNUSED(r), float UNUSED(g), float UNUSED(b)) {}
void immUniformColor3fv(const float UNUSED(rgb[3])) {}
void immUniformColor3fvAlpha(const float UNUSED(rgb[3]), float UNUSED(a)) {}
void immUniformColor3ub(unsigned char UNUSED(r), unsigned char UNUSED(g), unsigned char UNUSED(b))
{
}
void immUniformColor3ubv(const unsigned char UNUSED(rgb[3])) {}
void immUniformColor3ubvAlpha(const unsigned char UNUSED(rgb[3]), float UNUSED(alpha)) {}
void immUniformColor4ub(unsigned char UNUSED(r), unsigned char UNUSED(g), unsigned char UNUSED(b),
                        unsigned char UNUSED(a))
{
}
void immUniformColor4ubv(const unsigned char UNUSED(rgba[4])) {}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Immediate utilities
 * \{ */

void immRectf(int pos_id, float x1, float y1, float x2, float y2)
{
  immBegin(GPU_PRIM_TRI_FAN, 4);
  immVertex2f(pos_id, x1, y1);
  immVertex2f(pos_id, x2, y1);
  immVertex2f(pos_id, x2, y2);
  immVertex2f(pos_id, x1, y2);
  immEnd();
}

void imm_draw_box_wire_2d(unsigned int pos_id, float x1, float y1, float x2, float y2)
{
  immBegin(GPU_PRIM_LINE_STRIP, 5);
  immVertex2f(pos_id, x1, y1);
  immVertex2f(pos_id, x2, y1);
  immVertex2f(pos_id, x2, y2);
  immVertex2f(pos_id, x1, y2);
  immVertex2f(pos_id, x1, y1);
  immEnd();
}

void imm_draw_box_checker_2d_ex(float UNUSED(x1), float UNUSED(y1), float UNUSED(x2),
                                float UNUSED(y2), int UNUSED(checker_size))
{
}
void imm_draw_box_checker_2d(float UNUSED(x1), float UNUSED(y1), float UNUSED(x2),
                             float UNUSED(y2))
{
}
void imm_draw_circle_wire_2d(unsigned int UNUSED(pos), float UNUSED(x), float UNUSED(y),
                             float UNUSED(radius), int UNUSED(nsegments))
{
}
void imm_draw_disk_partial_fill_2d(unsigned int UNUSED(pos), float UNUSED(x), float UNUSED(y),
                                   float UNUSED(radius_inner), float UNUSED(radius_outer),
                                   float UNUSED(start_angle), float UNUSED(sweep_angle),
                                   int UNUSED(nsegments))
{
}

/* Theme hooks default to NULL: ui installs them at startup. */
void (*bui_imm_theme_color_hook)(int color_id, float r_color[4]) = NULL;
void (*bui_imm_theme_color_alpha_hook)(int color_id, float alpha, float r_color[4]) = NULL;

void immUniformThemeColor(int color_id)
{
  float rgba[4];
  if (bui_imm_theme_color_hook) {
    bui_imm_theme_color_hook(color_id, rgba);
    immUniformColor4fv(rgba);
  }
}
void immUniformThemeColorAlpha(int color_id, float alpha)
{
  float rgba[4];
  if (bui_imm_theme_color_alpha_hook) {
    bui_imm_theme_color_alpha_hook(color_id, alpha, rgba);
    immUniformColor4fv(rgba);
  }
}

void immDrawPixelsTexSetup(int UNUSED(shader_id)) {}
void immDrawPixelsTexScaledFullSize(float UNUSED(x), float UNUSED(y), float UNUSED(img_w),
                                    float UNUSED(img_h), float UNUSED(tex_w), float UNUSED(tex_h),
                                    float UNUSED(scale_x), float UNUSED(scale_y),
                                    int UNUSED(clip_min_x), int UNUSED(clip_min_y),
                                    int UNUSED(clip_max_x), int UNUSED(clip_max_y),
                                    bool UNUSED(linear_filter), const void *UNUSED(rect),
                                    int UNUSED(pixels_w), int UNUSED(pixels_h),
                                    int UNUSED(components), const float UNUSED(pixel_outline[4]))
{
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Buffers & batches
 * \{ */

struct GPUVertFormatRaw {
  int placeholder;
};
struct GPUVertFormat {
  int attr_len;
};
struct GPUVertBuf {
  int vertex_len;
};
struct GPUIndexBuf {
  int index_len;
};
struct GPUBatch {
  int prim_type;
};
struct GPUUniformBuf {
  int size;
};

unsigned int GPU_vertformat_attr_add(GPUVertFormat *UNUSED(format), const char *UNUSED(name),
                                     eGPUAttrType UNUSED(type), int UNUSED(components),
                                     eGPUAttrFetchMode UNUSED(fetch))
{
  return 0;
}
GPUVertBuf *GPU_vertbuf_create_with_format(GPUVertFormat *UNUSED(format))
{
  return (GPUVertBuf *)calloc(1, sizeof(GPUVertBuf));
}
void GPU_vertbuf_data_alloc(GPUVertBuf *verts, unsigned int vertex_len)
{
  verts->vertex_len = (int)vertex_len;
}
void GPU_vertbuf_attr_fill(GPUVertBuf *UNUSED(verts), unsigned int UNUSED(attrib),
                           const void *UNUSED(data))
{
}
void GPU_vertbuf_attr_get_raw_data(GPUVertBuf *UNUSED(verts), unsigned int UNUSED(attrib),
                                   void **r_data)
{
  static char raw[4096];
  *r_data = raw;
}
void *GPU_vertbuf_raw_step(void **iter)
{
  return *iter;
}
void GPU_vertbuf_tag_dirty(GPUVertBuf *UNUSED(verts)) {}
void GPU_vertbuf_use(GPUVertBuf *UNUSED(verts)) {}

void GPU_indexbuf_init(void **ib, int UNUSED(primitive_type), unsigned int UNUSED(index_len),
                       unsigned int UNUSED(vertex_len))
{
  *ib = calloc(1, sizeof(GPUIndexBuf));
}
void GPU_indexbuf_add_tri_verts(void *UNUSED(ib), unsigned int UNUSED(v1), unsigned int UNUSED(v2),
                                unsigned int UNUSED(v3))
{
}
GPUIndexBuf *GPU_indexbuf_build(void **ib)
{
  GPUIndexBuf *built = (GPUIndexBuf *)*ib;
  return built;
}

GPUBatch *GPU_batch_create_ex(int primitive_type, GPUVertBuf *UNUSED(verts),
                              GPUIndexBuf *UNUSED(index_buffer), unsigned int UNUSED(owns))
{
  GPUBatch *batch = (GPUBatch *)calloc(1, sizeof(GPUBatch));
  batch->prim_type = primitive_type;
  return batch;
}
void GPU_batch_discard(GPUBatch *batch)
{
  free(batch);
}
void GPU_batch_draw(GPUBatch *UNUSED(batch)) {}
void GPU_batch_draw_instance_range(GPUBatch *UNUSED(batch), int UNUSED(i_first),
                                   int UNUSED(i_count))
{
}
void GPU_batch_program_set_builtin(GPUBatch *UNUSED(batch), int UNUSED(shader_id)) {}
void GPU_batch_uniform_1f(GPUBatch *UNUSED(batch), const char *UNUSED(name), float UNUSED(v)) {}
void GPU_batch_uniform_3fv(GPUBatch *UNUSED(batch), const char *UNUSED(name),
                           const float UNUSED(data[3]))
{
}
void GPU_batch_uniform_4f(GPUBatch *UNUSED(batch), const char *UNUSED(name), float UNUSED(x),
                          float UNUSED(y), float UNUSED(z), float UNUSED(w))
{
}
void GPU_batch_uniform_4fv_array(GPUBatch *UNUSED(batch), const char *UNUSED(name),
                                 int UNUSED(len), const float *UNUSED(data))
{
}
void GPU_batch_uniformbuf_bind(GPUBatch *UNUSED(batch), GPUUniformBuf *UNUSED(uniformbuf)) {}

void gpu_batch_presets_register() {}
GPUBatch *GPU_batch_preset_sphere(int UNUSED(level))
{
  return GPU_batch_create_ex(GPU_PRIM_TRIS, NULL, NULL, 0);
}

/** \} */
