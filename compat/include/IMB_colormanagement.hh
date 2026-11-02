/* BUI seam: minimal colormanagement surface used by the toolkit.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#pragma once

#include "BLI_sys_types.h"
#include "DNA_vec_types.h"

#ifdef __cplusplus
#include "OCIO_scope.hh"
#endif

#ifdef __cplusplus
namespace blender {
struct ColorManagedDisplaySettings;
namespace ocio {
class Display;
}  // namespace ocio
using ColorManagedDisplay = ocio::Display;
}  // namespace blender
#endif

float IMB_colormanagement_get_luminance(const float rgb[3]);

#ifdef __cplusplus
/* Returns an empty scope description; wire a real processor later. */
blender::ocio::ScopeInfo IMB_colormanagement_get_scope_info(
    const blender::ColorManagedDisplaySettings *display_settings, const char *view_name);

/* Display-space conversion used by color widgets. Null-op with the stub
 * color management (scene-linear == display space). */
void IMB_colormanagement_scene_linear_to_display_v3(float pixel[3],
                                                    const blender::ColorManagedDisplay *display);
void IMB_colormanagement_display_to_scene_linear_v3(float pixel[3],
                                                    const blender::ColorManagedDisplay *display);

/* Color-picking space (stub: identity). */
void IMB_colormanagement_scene_linear_to_color_picking_v3(float color_picking[3],
                                                          const float scene_linear[3]);
void IMB_colormanagement_color_picking_to_scene_linear_v3(float scene_linear[3],
                                                          const float color_picking[3]);
#endif
