/* BUI GPU seam: batches, vertex/index buffers, uniform buffers.
 * Minimal opaque handles + creation API for the UI toolkit.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#pragma once

#include "GPU_state.hh"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct GPUVertFormat GPUVertFormat;
typedef struct GPUVertBuf GPUVertBuf;
typedef struct GPUIndexBuf GPUIndexBuf;
typedef struct GPUBatch GPUBatch;
typedef struct GPUUniformBuf GPUUniformBuf;

typedef enum eGPUAttrType {
  GPU_COMP_F32 = 0,
  GPU_COMP_I8,
  GPU_COMP_U8,
  GPU_COMP_I16,
  GPU_COMP_U16,
  GPU_COMP_I32,
  GPU_COMP_U32,
} eGPUAttrType;

typedef enum eGPUAttrFetchMode {
  GPU_FETCH_FLOAT = 0,
  GPU_FETCH_INT,
  GPU_FETCH_UBYTE_TO_FLOAT,
} eGPUAttrFetchMode;


unsigned int GPU_vertformat_attr_add(GPUVertFormat *format,
                                     const char *name,
                                     eGPUAttrType type,
                                     int components,
                                     eGPUAttrFetchMode fetch);

GPUVertBuf *GPU_vertbuf_create_with_format(GPUVertFormat *format);
void GPU_vertbuf_data_alloc(GPUVertBuf *verts, unsigned int vertex_len);
void GPU_vertbuf_attr_fill(GPUVertBuf *verts, unsigned int attrib_id, const void *data);
void GPU_vertbuf_attr_get_raw_data(GPUVertBuf *verts, unsigned int attrib_id, void **r_data);
void *GPU_vertbuf_raw_step(void **iter);
void GPU_vertbuf_tag_dirty(GPUVertBuf *verts);
void GPU_vertbuf_use(GPUVertBuf *verts);

void GPU_indexbuf_init(void **ib, int primitive_type, unsigned int index_len, unsigned int vertex_len);
void GPU_indexbuf_add_tri_verts(void *ib, unsigned int v1, unsigned int v2, unsigned int v3);
GPUIndexBuf *GPU_indexbuf_build(void **ib);

GPUBatch *GPU_batch_create_ex(int primitive_type,
                              GPUVertBuf *verts,
                              GPUIndexBuf *index_buffer,
                              unsigned int owns_buffers);
void GPU_batch_discard(GPUBatch *batch);
void GPU_batch_draw(GPUBatch *batch);
void GPU_batch_draw_instance_range(GPUBatch *batch, int i_first, int i_count);
void GPU_batch_program_set_builtin(GPUBatch *batch, int shader_id);
void GPU_batch_uniform_1f(GPUBatch *batch, const char *name, float value);
void GPU_batch_uniform_3fv(GPUBatch *batch, const char *name, const float data[3]);
void GPU_batch_uniform_4f(GPUBatch *batch, const char *name, float x, float y, float z, float w);
void GPU_batch_uniform_4fv_array(GPUBatch *batch, const char *name, int len, const float *data);
void GPU_batch_uniformbuf_bind(GPUBatch *batch, GPUUniformBuf *uniformbuf);

/* Batch presets (spheres etc; null backend returns empty batch). */
void gpu_batch_presets_register(void);
GPUBatch *GPU_batch_preset_sphere(int level);

#ifdef __cplusplus
}
#endif

