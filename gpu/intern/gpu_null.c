/* BUI GPU null backend.
 * Records state, discards geometry. Swap in a real renderer (GL/Metal/Vulkan/
 * software rasterizer) by implementing the same seam headers.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "BLI_utildefines.h"

#include "GPU_batch.hh"
#include "GPU_immediate.hh"
#include "GPU_immediate_util.hh"
#include "GPU_matrix.hh"
#include "GPU_state.hh"
#include "GPU_backend_sink.h"

/* -------------------------------------------------------------------- */
/** \name State
 * \{ */

static eGPUBlend bui_blend = GPU_BLEND_NONE;
static int bui_scissor[4] = {0, 0, 0, 0};
static bool bui_scissor_enabled = false;
static float bui_viewport[4] = {0.0f, 0.0f, 0.0f, 0.0f};
static int bui_viewport_wh[2] = {0, 0};
static const BuiGpuSink *bui_sink = NULL;
static float bui_point_size = 1.0f;

/* --- Immediate-mode vertex recording ------------------------------------- */
#define BUI_IMM_MAX_ATTRS BUI_VERTFORMAT_MAX_ATTRS
static GPUVertFormat *bui_imm_fmt = NULL;
static int bui_imm_prim = GPU_PRIM_NONE;
static unsigned int bui_imm_vertex_cap = 0;   /* rows the buffer can hold */
static unsigned int bui_imm_vertex_len = 0;   /* requested vertex count */
static unsigned int bui_imm_cursor = 0;       /* committed rows */
static unsigned char *bui_imm_buf = NULL;
static float bui_imm_color[4] = {1.0f, 1.0f, 1.0f, 1.0f};
static bool bui_imm_active = false;
typedef struct BuiImmPending {
  bool filled;
  float values[4];
} BuiImmPending;
static BuiImmPending bui_imm_pending[BUI_IMM_MAX_ATTRS];

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
void GPU_point_size(float size)
{
  bui_point_size = size;
}
void GPU_program_point_size(bool UNUSED(enable)) {}
void GPU_scissor(int x, int y, int width, int height)
{
  bui_scissor[0] = x;
  bui_scissor[1] = y;
  bui_scissor[2] = width;
  bui_scissor[3] = height;
  bui_scissor_enabled = true;
  if (bui_sink && bui_sink->scissor) {
    bui_sink->scissor(bui_sink->user, x, y, width, height);
  }
}
void GPU_scissor_test(bool enable)
{
  bui_scissor_enabled = enable;
  if (!enable && bui_sink && bui_sink->scissor) {
    bui_sink->scissor(bui_sink->user, 0, 0, 0, 0);
  }
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

void GPU_viewport_size_set(int width, int height)
{
  bui_viewport_wh[0] = width;
  bui_viewport_wh[1] = height;
}

void GPU_backend_sink_set(const BuiGpuSink *sink)
{
  bui_sink = sink;
}
const BuiGpuSink *GPU_backend_sink_get(void)
{
  return bui_sink;
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

/* The matrix stack starts at identity so draw calls before any matrix setup
 * pass pixel coordinates through unchanged. */
static bool bui_matrix_initialized = false;
static void bui_matrix_ensure_init(void)
{
  if (!bui_matrix_initialized) {
    bui_matrix_initialized = true;
    for (int i = 0; i < BUI_MATRIX_MAX_DEPTH; i++) {
      bui_mat_identity(bui_model[i]);
    }
  }
}

void GPU_matrix_identity_set(void)
{
  bui_matrix_ensure_init();
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

GPUVertFormat *immVertexFormat(void)
{
  static GPUVertFormat fmt;
  memset(&fmt, 0, sizeof(fmt));
  bui_imm_fmt = &fmt;
  return &fmt;
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

/* Transform a 2D position through the current model matrix. */
static void bui_imm_transform(float x, float y, float r_out[2])
{
  bui_matrix_ensure_init();
  const float *m = bui_model[bui_model_depth];
  r_out[0] = m[0] * x + m[4] * y + m[12];
  r_out[1] = m[1] * x + m[5] * y + m[13];
}

/* Returns the attribute index with `name`, or -1. */
static int bui_imm_attr_by_name(const char *name)
{
  if (!bui_imm_fmt) {
    return -1;
  }
  for (unsigned int i = 0; i < bui_imm_fmt->attr_len; i++) {
    if (strcmp(bui_imm_fmt->attrs[i].name, name) == 0) {
      return (int)i;
    }
  }
  return -1;
}

static void bui_sink_flush_prim(int prim,
                                const float (*coords)[2],
                                const float (*colors)[4],
                                int count)
{
  if (!bui_sink) {
    return;
  }
  switch (prim) {
    case GPU_PRIM_TRIS:
    case GPU_PRIM_TRI_STRIP:
    case GPU_PRIM_TRI_FAN:
      if (bui_sink->triangles) {
        bui_sink->triangles(bui_sink->user, coords, colors, count);
      }
      break;
    case GPU_PRIM_LINES:
    case GPU_PRIM_LINE_STRIP:
    case GPU_PRIM_LINE_LOOP:
      if (bui_sink->lines) {
        bui_sink->lines(bui_sink->user, coords, colors, count);
      }
      break;
    case GPU_PRIM_POINTS:
      if (bui_sink->points) {
        bui_sink->points(bui_sink->user, coords, colors, count, bui_point_size);
      }
      break;
    default:
      break;
  }
}

/* Expand the recorded primitive into lists and emit through the sink. */
static void bui_imm_flush(void)
{
  const unsigned int n = bui_imm_cursor;
  if (!bui_sink || n == 0) {
    return;
  }
  const int pos_idx = bui_imm_attr_by_name("pos");
  const int col_idx = bui_imm_attr_by_name("color");
  const unsigned int pos_off = (pos_idx >= 0) ? bui_imm_fmt->attrs[pos_idx].offset : 0;
  const bool has_col = col_idx >= 0;
  const unsigned int col_off = has_col ? bui_imm_fmt->attrs[col_idx].offset : 0;
  const unsigned int stride = bui_imm_fmt ? bui_imm_fmt->stride : 0;

  float(*coords)[2] = (float(*)[2])malloc(sizeof(float) * 2 * n);
  float(*colors)[4] = (float(*)[4])malloc(sizeof(float) * 4 * n);
  for (unsigned int i = 0; i < n; i++) {
    const unsigned char *row = bui_imm_buf + (size_t)i * stride;
    float px, py;
    memcpy(&px, row + pos_off, sizeof(float));
    memcpy(&py, row + pos_off + 4, sizeof(float));
    bui_imm_transform(px, py, coords[i]);
    if (has_col) {
      memcpy(colors[i], row + col_off, sizeof(float) * 4);
    }
    else {
      memcpy(colors[i], bui_imm_color, sizeof(float) * 4);
    }
  }
  bui_sink_flush_prim(bui_imm_prim, coords, colors, (int)n);
  free(coords);
  free(colors);
}

void immBegin(eGPUPrimType primitive, unsigned int vertex_len)
{
  bui_imm_prim = primitive;
  bui_imm_vertex_len = vertex_len;
  bui_imm_cursor = 0;
  bui_imm_active = true;
  if (bui_imm_fmt && vertex_len > bui_imm_vertex_cap) {
    bui_imm_vertex_cap = vertex_len;
    bui_imm_buf = (unsigned char *)realloc(bui_imm_buf, (size_t)bui_imm_fmt->stride * vertex_len);
  }
}
void immBeginAtMost(eGPUPrimType primitive, unsigned int vertex_len)
{
  immBegin(primitive, vertex_len);
}
void immEnd(void)
{
  if (bui_imm_active) {
    bui_imm_flush();
  }
  bui_imm_active = false;
  bui_imm_prim = GPU_PRIM_NONE;
  memset(bui_imm_pending, 0, sizeof(bui_imm_pending));
}
static void bui_imm_store(unsigned int attrib, const float *values)
{
  if (attrib < BUI_IMM_MAX_ATTRS) {
    bui_imm_pending[attrib].filled = true;
    memcpy(bui_imm_pending[attrib].values, values, sizeof(float) * 4);
  }
}
static void bui_imm_commit_vertex(unsigned int UNUSED(pos_attrib), float x, float y)
{
  if (!bui_imm_fmt || !bui_imm_buf || bui_imm_cursor >= bui_imm_vertex_cap) {
    return;
  }
  unsigned char *row = bui_imm_buf + (size_t)bui_imm_cursor * bui_imm_fmt->stride;
  for (unsigned int i = 0; i < bui_imm_fmt->attr_len; i++) {
    if (!bui_imm_pending[i].filled || strcmp(bui_imm_fmt->attrs[i].name, "pos") == 0) {
      continue;
    }
    const unsigned int comps = bui_imm_fmt->attrs[i].components;
    if (bui_imm_fmt->attrs[i].attr_type == (unsigned int)UBYTE_8_UINT) {
      for (unsigned int c = 0; c < comps; c++) {
        row[bui_imm_fmt->attrs[i].offset + c] = (unsigned char)(
            bui_imm_pending[i].values[c] * 255.0f + 0.5f);
      }
    }
    else {
      memcpy(row + bui_imm_fmt->attrs[i].offset, bui_imm_pending[i].values,
             sizeof(float) * comps);
    }
    bui_imm_pending[i].filled = false;
  }
  const int pos_idx = bui_imm_attr_by_name("pos");
  const unsigned int pos_off = (pos_idx >= 0) ? bui_imm_fmt->attrs[pos_idx].offset : 0;
  const float xy[2] = {x, y};
  memcpy(row + pos_off, xy, sizeof(float) * 2);
  bui_imm_cursor++;
}
void immVertex2f(unsigned int attrib, float x, float y)
{
  bui_imm_commit_vertex(attrib, x, y);
}
void immVertex2fv(unsigned int attrib, const float data[2])
{
  bui_imm_commit_vertex(attrib, data[0], data[1]);
}
void immAttr3fv(unsigned int attrib, const float data[3])
{
  float v[4] = {data[0], data[1], data[2], 1.0f};
  bui_imm_store(attrib, v);
}
void immAttr4f(unsigned int attrib, float x, float y, float z, float w)
{
  float v[4] = {x, y, z, w};
  bui_imm_store(attrib, v);
}
void immAttr4fv(unsigned int attrib, const float data[4])
{
  bui_imm_store(attrib, data);
}
void immAttr4ub(unsigned int attrib, unsigned char r, unsigned char g, unsigned char b,
                unsigned char a)
{
  float v[4] = {r / 255.0f, g / 255.0f, b / 255.0f, a / 255.0f};
  bui_imm_store(attrib, v);
}
void immAttr4ubv(unsigned int attrib, const unsigned char data[4])
{
  float v[4] = {data[0] / 255.0f, data[1] / 255.0f, data[2] / 255.0f, data[3] / 255.0f};
  bui_imm_store(attrib, v);
}
void immAttrSkip(unsigned int attrib)
{
  if (attrib < BUI_IMM_MAX_ATTRS) {
    bui_imm_pending[attrib].filled = false;
  }
}

void immUniform1f(const char *UNUSED(name), float UNUSED(x)) {}
void immUniform1i(const char *UNUSED(name), int UNUSED(x)) {}
void immUniform2f(const char *UNUSED(name), float UNUSED(x), float UNUSED(y)) {}
void immUniform2fv(const char *UNUSED(name), const float UNUSED(data[2])) {}
void immUniform4f(const char *UNUSED(name), float UNUSED(x), float UNUSED(y), float UNUSED(z),
                  float UNUSED(w))
{
}
void immUniform4fv(const char *UNUSED(name), const float UNUSED(data[4])) {}

void immUniformColor4f(float r, float g, float b, float a)
{
  bui_imm_color[0] = r;
  bui_imm_color[1] = g;
  bui_imm_color[2] = b;
  bui_imm_color[3] = a;
}
void immUniformColor4fv(const float rgba[4])
{
  immUniformColor4f(rgba[0], rgba[1], rgba[2], rgba[3]);
}
void immUniformColor3f(float r, float g, float b)
{
  immUniformColor4f(r, g, b, 1.0f);
}
void immUniformColor3fv(const float rgb[3])
{
  immUniformColor4f(rgb[0], rgb[1], rgb[2], 1.0f);
}
void immUniformColor3fvAlpha(const float rgb[3], float a)
{
  immUniformColor4f(rgb[0], rgb[1], rgb[2], a);
}
void immUniformColor3ub(unsigned char r, unsigned char g, unsigned char b)
{
  immUniformColor4f(r / 255.0f, g / 255.0f, b / 255.0f, 1.0f);
}
void immUniformColor3ubv(const unsigned char rgb[3])
{
  immUniformColor3ub(rgb[0], rgb[1], rgb[2]);
}
void immUniformColor3ubvAlpha(const unsigned char rgb[3], float alpha)
{
  immUniformColor4f(rgb[0] / 255.0f, rgb[1] / 255.0f, rgb[2] / 255.0f, alpha);
}
void immUniformColor4ub(unsigned char r, unsigned char g, unsigned char b, unsigned char a)
{
  immUniformColor4f(r / 255.0f, g / 255.0f, b / 255.0f, a / 255.0f);
}
void immUniformColor4ubv(const unsigned char rgba[4])
{
  immUniformColor4ub(rgba[0], rgba[1], rgba[2], rgba[3]);
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

/** \} */

/* -------------------------------------------------------------------- */
/** \name Buffers & batches
 * \{ */

/* Layout contract with blender::gpu::VertBuf (GPU_batch.hh): the members
 * must appear in this exact order so both languages share one allocation. */
typedef struct GPUVertBufInternal {
  unsigned int vertex_len;
  unsigned int stride;
  void *cpu_data;
  GPUVertFormat *format;
} GPUVertBufInternal;

struct GPUIndexBuf {
  unsigned int index_len;
  unsigned int *data;
};
struct GPUBatch {
  int prim_type;
  GPUVertBuf *verts;
  GPUIndexBuf *index_buffer;
  unsigned int owns_flags;
  float uniform_color[4];
};
struct GPUUniformBuf {
  size_t size;
};

static void bui_vertbuf_ensure_data(GPUVertBufInternal *verts)
{
  if (verts->cpu_data == NULL) {
    size_t bytes = (size_t)verts->stride * (verts->vertex_len ? verts->vertex_len : 1);
    verts->cpu_data = calloc(1, bytes ? bytes : 4);
  }
}

unsigned int GPU_vertformat_attr_add(GPUVertFormat *format, const char *name,
                                     eGPUAttrType type)
{
  if (!format || format->attr_len >= BUI_VERTFORMAT_MAX_ATTRS) {
    return 0;
  }
  unsigned int id = format->attr_len++;
  snprintf(format->attrs[id].name, sizeof(format->attrs[id].name), "%s", name);
  format->attrs[id].attr_type = (unsigned int)type;
  format->attrs[id].components = ((type == SFLOAT_32_32 || type == UBYTE_8_UINT) ? 2 :
                                  (type == SFLOAT_32_32_32) ? 3 : 4);
  format->attrs[id].offset = format->stride;
  format->stride += format->attrs[id].components * 4;
  return id;
}

#ifndef __cplusplus
GPUVertBuf *GPU_vertbuf_create_with_format(GPUVertFormat *format)
{
  GPUVertBufInternal *verts = (GPUVertBufInternal *)calloc(1, sizeof(GPUVertBufInternal));
  if (format) {
    verts->stride = format->stride;
    verts->format = (GPUVertFormat *)calloc(1, sizeof(GPUVertFormat));
    *verts->format = *format;
  }
  return (GPUVertBuf *)verts;
}
void GPU_vertbuf_data_alloc(GPUVertBuf *verts_ptr, unsigned int vertex_len)
{
  GPUVertBufInternal *verts = (GPUVertBufInternal *)verts_ptr;
  verts->vertex_len = vertex_len;
}
#endif

void GPU_vertbuf_attr_fill(GPUVertBuf *verts_ptr, unsigned int attrib, const void *data)
{
  GPUVertBufInternal *verts = (GPUVertBufInternal *)verts_ptr;
  if (!verts || !data) {
    return;
  }
  bui_vertbuf_ensure_data(verts);
  if (verts->format && attrib < verts->format->attr_len) {
    const unsigned int offset = verts->format->attrs[attrib].offset;
    const unsigned int comp_bytes = verts->format->attrs[attrib].components * 4;
    for (unsigned int i = 0; i < verts->vertex_len; i++) {
      memcpy((char *)verts->cpu_data + (size_t)i * verts->stride + offset,
             (const char *)data + (size_t)i * comp_bytes,
             comp_bytes);
    }
  }
}

void GPU_vertbuf_attr_get_raw_data(GPUVertBuf *verts_ptr, unsigned int attrib,
                                   GPUVertBufRaw *r_raw)
{
  GPUVertBufInternal *verts = (GPUVertBufInternal *)verts_ptr;
  bui_vertbuf_ensure_data(verts);
  const unsigned int offset = (verts->format && attrib < verts->format->attr_len) ?
                                  verts->format->attrs[attrib].offset : 0;
  r_raw->size = verts->vertex_len;
  r_raw->stride = verts->stride;
  r_raw->data_init = (char *)verts->cpu_data + offset;
  r_raw->data = r_raw->data_init;
}

void *GPU_vertbuf_raw_step(GPUVertBufRaw *raw)
{
  void *ptr = raw->data;
  raw->data = (char *)raw->data + raw->stride;
  raw->size--;
  return ptr;
}

void GPU_vertbuf_tag_dirty(GPUVertBuf *UNUSED(verts)) {}
void GPU_vertbuf_use(GPUVertBuf *UNUSED(verts)) {}

void GPU_indexbuf_init(GPUIndexBufBuilder *builder, int primitive_type,
                       unsigned int index_len, unsigned int max_allowed_index)
{
  memset(builder, 0, sizeof(*builder));
  builder->prim_type = primitive_type;
  builder->index_len = index_len;
  builder->max_allowed_index = max_allowed_index;
  builder->data = (unsigned int *)calloc(index_len ? index_len : 1, sizeof(unsigned int));
}
void GPU_indexbuf_add_tri_verts(GPUIndexBufBuilder *builder, unsigned int v1, unsigned int v2,
                                unsigned int v3)
{
  if (builder->used + 3 <= builder->index_len) {
    builder->data[builder->used++] = v1;
    builder->data[builder->used++] = v2;
    builder->data[builder->used++] = v3;
  }
  if (v1 > builder->max_index) builder->max_index = v1;
  if (v2 > builder->max_index) builder->max_index = v2;
  if (v3 > builder->max_index) builder->max_index = v3;
}
void GPU_indexbuf_add_line_verts(GPUIndexBufBuilder *builder, unsigned int v1, unsigned int v2)
{
  if (builder->used + 2 <= builder->index_len) {
    builder->data[builder->used++] = v1;
    builder->data[builder->used++] = v2;
  }
  if (v1 > builder->max_index) builder->max_index = v1;
  if (v2 > builder->max_index) builder->max_index = v2;
}
GPUIndexBuf *GPU_indexbuf_build(GPUIndexBufBuilder *builder)
{
  GPUIndexBuf *buf = (GPUIndexBuf *)calloc(1, sizeof(GPUIndexBuf));
  buf->index_len = builder->used;
  buf->data = builder->data;
  builder->data = NULL;
  return buf;
}
GPUIndexBuf *GPU_indexbuf_build_in_memory(GPUIndexBufBuilder *UNUSED(builder),
                                          void *UNUSED(memory))
{
  return (GPUIndexBuf *)calloc(1, sizeof(GPUIndexBuf));
}
void GPU_indexbuf_discard(GPUIndexBuf *index_buf)
{
  if (index_buf) {
    free(index_buf->data);
    free(index_buf);
  }
}

GPUBatch *GPU_batch_create_ex(int primitive_type, GPUVertBuf *verts,
                              GPUIndexBuf *index_buffer, unsigned int owns_flags)
{
  GPUBatch *batch = (GPUBatch *)calloc(1, sizeof(GPUBatch));
  batch->prim_type = primitive_type;
  batch->verts = verts;
  batch->index_buffer = index_buffer;
  batch->owns_flags = owns_flags;
  return batch;
}
void GPU_batch_discard(GPUBatch *batch)
{
  if (batch) {
    if (batch->owns_flags & GPU_BATCH_OWNS_IBO) {
      GPU_indexbuf_discard(batch->index_buffer);
    }
    if (batch->owns_flags & GPU_BATCH_OWNS_VBO) {
      GPUVertBufInternal *verts = (GPUVertBufInternal *)batch->verts;
      if (verts) {
        free(verts->cpu_data);
        free(verts->format);
      }
      free(verts);
    }
    free(batch);
  }
}
static void bui_batch_flush(GPUBatch *batch)
{
  if (!bui_sink || !batch || !batch->verts) {
    return;
  }
  GPUVertBufInternal *verts = (GPUVertBufInternal *)batch->verts;
  if (!verts->cpu_data || verts->vertex_len == 0 || !verts->format) {
    return;
  }
  int pos_idx = -1;
  int col_idx = -1;
  for (unsigned int i = 0; i < verts->format->attr_len; i++) {
    if (strcmp(verts->format->attrs[i].name, "pos") == 0) {
      pos_idx = (int)i;
    }
    else if (strcmp(verts->format->attrs[i].name, "color") == 0) {
      col_idx = (int)i;
    }
  }
  if (pos_idx < 0) {
    return;
  }
  const unsigned int pos_off = verts->format->attrs[pos_idx].offset;
  const bool has_col = col_idx >= 0;
  const unsigned int col_off = has_col ? verts->format->attrs[col_idx].offset : 0;
  const unsigned int stride = verts->stride;

  const unsigned int n = verts->vertex_len;
  float(*coords)[2] = (float(*)[2])malloc(sizeof(float) * 2 * n);
  float(*colors)[4] = (float(*)[4])malloc(sizeof(float) * 4 * n);
  for (unsigned int i = 0; i < n; i++) {
    const unsigned char *row = (unsigned char *)verts->cpu_data + (size_t)i * stride;
    memcpy(coords[i], row + pos_off, sizeof(float) * 2);
    if (has_col) {
      if (verts->format->attrs[col_idx].attr_type == (unsigned int)UBYTE_8_UINT) {
        for (int c = 0; c < 4; c++) {
          colors[i][c] = row[col_off + c] / 255.0f;
        }
      }
      else {
        memcpy(colors[i], row + col_off, sizeof(float) * 4);
      }
    }
    else {
      memcpy(colors[i], batch->uniform_color, sizeof(float) * 4);
    }
  }
  bui_sink_flush_prim(batch->prim_type, coords, colors, (int)n);
  free(coords);
  free(colors);
}

void GPU_batch_draw(GPUBatch *batch) { bui_batch_flush(batch); }
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
void GPU_batch_uniform_4f(GPUBatch *batch, const char *name, float x,
                          float y, float z, float w)
{
  if (batch && name && strcmp(name, "color") == 0) {
    batch->uniform_color[0] = x;
    batch->uniform_color[1] = y;
    batch->uniform_color[2] = z;
    batch->uniform_color[3] = w;
  }
}
void GPU_batch_uniform_4fv_array(GPUBatch *UNUSED(batch), const char *UNUSED(name),
                                 int UNUSED(len), const float (*data)[4])
{
  (void)data;
}
void GPU_batch_uniformbuf_bind(GPUBatch *UNUSED(batch), GPUUniformBuf *UNUSED(uniformbuf)) {}

GPUUniformBuf *GPU_uniformbuf_create_ex(size_t size, const void *UNUSED(data),
                                        const char *UNUSED(name))
{
  GPUUniformBuf *ubo = (GPUUniformBuf *)calloc(1, sizeof(GPUUniformBuf));
  ubo->size = size;
  return ubo;
}
void GPU_uniformbuf_update(GPUUniformBuf *UNUSED(uniformbuf), const void *UNUSED(data)) {}
void GPU_uniformbuf_free(GPUUniformBuf *uniformbuf)
{
  free(uniformbuf);
}

void gpu_batch_presets_register(GPUBatch *UNUSED(preset_batch)) {}
GPUBatch *GPU_batch_preset_sphere(int UNUSED(level))
{
  return GPU_batch_create_ex(GPU_PRIM_TRIS, NULL, NULL, 0);
}

/** \} */




