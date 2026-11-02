/* BUI GPU seam: immediate mode drawing.
 * Minimal API-compatible surface for the extracted UI toolkit.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#pragma once

#include "BLI_sys_types.h"
#include "GPU_batch.hh"
#include "GPU_matrix.hh"
#include "GPU_immediate_util.hh"

#ifdef __cplusplus
extern "C" {
#endif

typedef enum eGPUPrimType {
  GPU_PRIM_NONE = 0,
  GPU_PRIM_POINTS,
  GPU_PRIM_LINES,
  GPU_PRIM_LINE_STRIP,
  GPU_PRIM_LINE_LOOP,
  GPU_PRIM_TRIS,
  GPU_PRIM_TRI_STRIP,
  GPU_PRIM_TRI_FAN,
} eGPUPrimType;
typedef enum eGPUPrimType GPUPrimType;

/* Returns a scratch vertex format (upstream semantics). */
GPUVertFormat *immVertexFormat(void);
void immBindBuiltinProgram(int shader_id);
bool immIsShaderBound(void);
void immUnbindProgram(void);

void immBegin(eGPUPrimType primitive, unsigned int vertex_len);
void immBeginAtMost(eGPUPrimType primitive, unsigned int vertex_len);
void immEnd(void);

void immVertex2f(unsigned int attrib_id, float x, float y);
void immVertex2fv(unsigned int attrib_id, const float data[2]);
void immAttr3fv(unsigned int attrib_id, const float data[3]);
void immAttr4f(unsigned int attrib_id, float x, float y, float z, float w);
void immAttr4fv(unsigned int attrib_id, const float data[4]);
void immAttr4ub(unsigned int attrib_id, unsigned char r, unsigned char g, unsigned char b, unsigned char a);
void immAttr4ubv(unsigned int attrib_id, const unsigned char data[4]);
void immAttrSkip(unsigned int attrib_id);

void immUniform1f(const char *name, float x);
void immUniform1i(const char *name, int x);
void immUniform2f(const char *name, float x, float y);
void immUniform2fv(const char *name, const float data[2]);
void immUniform4f(const char *name, float x, float y, float z, float w);
void immUniform4fv(const char *name, const float data[4]);

void immUniformColor4f(float r, float g, float b, float a);
void immUniformColor4fv(const float rgba[4]);
void immUniformColor3f(float r, float g, float b);
void immUniformColor3fv(const float rgb[3]);
void immUniformColor3fvAlpha(const float rgb[3], float a);
void immUniformColor3ub(unsigned char r, unsigned char g, unsigned char b);
void immUniformColor3ubv(const unsigned char rgb[3]);
void immUniformColor3ubvAlpha(const unsigned char rgb[3], float alpha);
void immUniformColor4ub(unsigned char r, unsigned char g, unsigned char b, unsigned char a);
void immUniformColor4ubv(const unsigned char rgba[4]);

#ifdef __cplusplus
}
#endif

