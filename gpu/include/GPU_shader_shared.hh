/* BUI GPU seam: shader interface structs shared between CPU and shaders.
 * Mirrors upstream GPU_shader_shared.hh for the subset the toolkit uses.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#pragma once

#ifdef __cplusplus
#include "BLI_math_vector_types.hh"

namespace blender::gpu {

/* UBO for the simple-lighting sphere preview (unit-vector button). */
struct SimpleLightingData {
  float4 l_color;
  float3 light;
  float _pad = 0.0f;
};

}  // namespace blender::gpu
#endif
