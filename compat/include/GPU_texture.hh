/* BUI GPU seam: textures (minimal).
 * SPDX-License-Identifier: GPL-2.0-or-later */
#pragma once

#ifdef __cplusplus
namespace blender::gpu {
class Texture;
enum class TextureFormat : unsigned char;
}  // namespace blender::gpu
using GPUTexture = blender::gpu::Texture;
#endif
