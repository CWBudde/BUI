/* BUI GPU seam: batches, vertex/index buffers, uniform buffers.
 * Minimal API-compatible surface for the extracted UI toolkit.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#pragma once

#include "GPU_state.hh"

#include <stdlib.h>

/* Opaque C types + shared enums first so both C and C++ consumers see them
 * before the C++ alias block below. */

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
  /* blender::gpu::VertAttrType aliases used by toolkit call-sites. */
  SFLOAT_32 = GPU_COMP_F32,
  SFLOAT_32_32,
  SFLOAT_32_32_32,
  SFLOAT_32_32_32_32,
  UBYTE_8_UINT,
  UINT_32 = GPU_COMP_U32,
  UNORM_8,
  UNORM_8_8_8_8,
} eGPUAttrType;

typedef enum eGPUAttrFetchMode {
  GPU_FETCH_FLOAT = 0,
  GPU_FETCH_INT,
  GPU_FETCH_UBYTE_TO_FLOAT,
} eGPUAttrFetchMode;

/* Batch ownership flags (upstream: eGPUBatchFlag). */
typedef enum eGPUBatchFlag {
  GPU_BATCH_INVALID = (1 << 0),
  GPU_BATCH_OWNS_VBO = (1 << 1),
  GPU_BATCH_OWNS_IBO = (1 << 2),
  GPU_BATCH_DIRTY = (1 << 3),
} eGPUBatchFlag;
/* Upstream spell-check: the index-buffer ownership flag is called OWNS_INDEX. */
#define GPU_BATCH_OWNS_INDEX GPU_BATCH_OWNS_IBO

/* Concrete format struct: call-sites stack-allocate these. */
#define BUI_VERTFORMAT_MAX_ATTRS 8
typedef struct GPUVertFormat GPUVertFormat;
struct GPUVertFormat {
  unsigned int attr_len;
  unsigned int stride;
  struct {
    char name[32];
    unsigned int attr_type;
    unsigned int components;
    unsigned int fetch_mode;
    unsigned int offset;
  } attrs[BUI_VERTFORMAT_MAX_ATTRS];
};

/* Raw attribute cursor used to fill a vertex buffer attribute in place
 * (upstream shape). */
typedef struct GPUVertBufRaw {
  unsigned int size;
  void *data;
  void *data_init;
  unsigned int stride;
} GPUVertBufRaw;

/* Index buffer builder (upstream shape). The null backend only records the
 * element count; a real renderer uploads `data`. */
typedef struct GPUIndexBufBuilder {
  unsigned int max_allowed_index;
  unsigned int max_index;
  unsigned int index_len;
  unsigned int used;
  unsigned int index_min;
  unsigned int index_max;
  int prim_type;
  unsigned int *data;
} GPUIndexBufBuilder;

#ifdef __cplusplus

namespace blender::gpu {

/* The toolkit only ever fills vertex buffers through the C API or through
 * #data<T>(). The first four members form a layout contract with the C
 * backends in gpu/intern (see GPUVertBufInternal there). */
struct VertBuf {
  unsigned int vertex_len = 0;
  unsigned int stride = 0;
  void *cpu_data = nullptr;
  GPUVertFormat *format = nullptr; /* heap copy owned by the buffer */

  template<typename T> struct DataView {
    T *ptr;
    T *data()
    {
      return ptr;
    }
  };
  /* Lazily backs the buffer with CPU memory so callers can write vertices
   * even with the null backend (which discards them at draw time). */
  template<typename T> DataView<T> data()
  {
    if (cpu_data == nullptr) {
      size_t bytes = size_t(vertex_len) * (stride ? stride : sizeof(T));
      if (bytes == 0) {
        bytes = sizeof(T);
      }
      cpu_data = calloc(1, bytes);
    }
    return DataView<T>(static_cast<T *>(cpu_data));
  }
};

enum class TextureFormat : unsigned char {
  RGBA8,
  RGBA8UI,
  RGBA16F,
  RGBA32F,
  R8,
  RG16,
  RGBA16,
  UNORM_8_8_8_8,
  UNORM_8,
};

using Batch = ::GPUBatch;
using VertAttrType = ::eGPUAttrType;
using VertFetchMode = ::eGPUAttrFetchMode;
using UniformBuf = ::GPUUniformBuf;

}  // namespace blender::gpu

typedef blender::gpu::VertBuf GPUVertBuf;
typedef blender::gpu::TextureFormat GPUTextureFormat;

/* C++ convenience overloads (upstream takes references). Allocated buffers use
 * calloc on purpose so the C and C++ sides share one free()-consistent
 * ownership story; zeroed bytes equal the default member initializers. */
inline GPUVertBuf *GPU_vertbuf_create_with_format(const GPUVertFormat &format)
{
  GPUVertBuf *verts = static_cast<GPUVertBuf *>(calloc(1, sizeof(blender::gpu::VertBuf)));
  verts->stride = format.stride;
  verts->format = static_cast<GPUVertFormat *>(calloc(1, sizeof(GPUVertFormat)));
  *verts->format = format;
  return verts;
}
inline void GPU_vertbuf_data_alloc(GPUVertBuf &verts, unsigned int vertex_len)
{
  verts.vertex_len = vertex_len;
}
inline void GPU_batch_uniformbuf_bind(GPUBatch * /*batch*/,
                                      const char * /*name*/,
                                      GPUUniformBuf *uniformbuf)
{
  (void)uniformbuf;
}

#else /* __cplusplus */

typedef struct GPUVertBuf GPUVertBuf;

#endif /* __cplusplus */

#ifdef __cplusplus
extern "C" {
#endif

unsigned int GPU_vertformat_attr_add(GPUVertFormat *format,
                                     const char *name,
                                     eGPUAttrType type);

#ifndef __cplusplus
GPUVertBuf *GPU_vertbuf_create_with_format(GPUVertFormat *format);
void GPU_vertbuf_data_alloc(GPUVertBuf *verts, unsigned int vertex_len);
#endif
void GPU_vertbuf_attr_fill(GPUVertBuf *verts, unsigned int attrib_id, const void *data);
void GPU_vertbuf_attr_get_raw_data(GPUVertBuf *verts, unsigned int attrib_id, GPUVertBufRaw *r_raw);
void *GPU_vertbuf_raw_step(GPUVertBufRaw *raw);
void GPU_vertbuf_tag_dirty(GPUVertBuf *verts);
void GPU_vertbuf_use(GPUVertBuf *verts);

void GPU_indexbuf_init(GPUIndexBufBuilder *builder,
                       int primitive_type,
                       unsigned int index_len,
                       unsigned int max_allowed_index);
void GPU_indexbuf_add_tri_verts(GPUIndexBufBuilder *builder,
                                unsigned int v1,
                                unsigned int v2,
                                unsigned int v3);
void GPU_indexbuf_add_line_verts(GPUIndexBufBuilder *builder,
                                 unsigned int v1,
                                 unsigned int v2);
GPUIndexBuf *GPU_indexbuf_build(GPUIndexBufBuilder *builder);
GPUIndexBuf *GPU_indexbuf_build_in_memory(GPUIndexBufBuilder *builder, void *memory);
void GPU_indexbuf_discard(GPUIndexBuf *index_buf);

GPUBatch *GPU_batch_create_ex(int primitive_type,
                              GPUVertBuf *verts,
                              GPUIndexBuf *index_buffer,
                              unsigned int owns_flags);
void GPU_batch_discard(GPUBatch *batch);
void GPU_batch_draw(GPUBatch *batch);
void GPU_batch_draw_instance_range(GPUBatch *batch, int i_first, int i_count);
void GPU_batch_program_set_builtin(GPUBatch *batch, int shader_id);
void GPU_batch_uniform_1f(GPUBatch *batch, const char *name, float value);
void GPU_batch_uniform_3fv(GPUBatch *batch, const char *name, const float data[3]);
void GPU_batch_uniform_4f(GPUBatch *batch, const char *name, float x, float y, float z, float w);
void GPU_batch_uniform_4fv_array(GPUBatch *batch,
                                 const char *name,
                                 int len,
                                 const float (*data)[4]);
void GPU_batch_uniformbuf_bind(GPUBatch *batch, GPUUniformBuf *uniformbuf);

/* Preset batches (spheres etc). The null backend returns empty batches. */
void gpu_batch_presets_register(GPUBatch *preset_batch);
GPUBatch *GPU_batch_preset_sphere(int level);

#ifdef __cplusplus
}
#endif
