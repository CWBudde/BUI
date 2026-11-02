/* BUI GPU seam header (GPU_uniform_buffer.hh).
 * SPDX-License-Identifier: GPL-2.0-or-later */
#pragma once

#include "GPU_batch.hh"

#ifdef __cplusplus
extern "C" {
#endif

/* Uniform buffer lifecycle. The null backend keeps only a size record. */
GPUUniformBuf *GPU_uniformbuf_create_ex(size_t size, const void *data, const char *name);
void GPU_uniformbuf_update(GPUUniformBuf *uniformbuf, const void *data);
void GPU_uniformbuf_free(GPUUniformBuf *uniformbuf);

#ifdef __cplusplus
}
#endif
