/* BUI stable C API.
 *
 * A small, ABI-stable facade for hosts that embed BUI without compiling
 * against the full toolkit headers: version info plus the render-sink
 * installation that makes the toolkit renderable by any technology
 * (Qt/QPainter, PySide6, SVG recorder, terminal grid, ...).
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */
#ifndef __BUI_C_API_H__
#define __BUI_C_API_H__

#include "GPU_backend_sink.h"

#ifdef __cplusplus
extern "C" {
#endif

/* Library version, mirroring the CMake project() version. */
void bui_get_version(int *r_major, int *r_minor, int *r_patch);
const char *bui_version_string(void);

/* Install the primitive sink that receives everything the toolkit draws. */
void bui_set_render_sink(const BuiGpuSink *sink);
const BuiGpuSink *bui_get_render_sink(void);

/* Record the viewport the host renders into (pixels). */
void bui_set_viewport(int width, int height);

#ifdef __cplusplus
}
#endif

#endif /* __BUI_C_API_H__ */
