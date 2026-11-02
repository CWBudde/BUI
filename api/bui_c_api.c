/* BUI stable C API implementation: thin forwarders into the toolkit.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#include "bui_c_api.h"

#include "GPU_backend_sink.h"

#define BUI_VERSION_MAJOR 0
#define BUI_VERSION_MINOR 1
#define BUI_VERSION_PATCH 0

void bui_get_version(int *r_major, int *r_minor, int *r_patch)
{
  if (r_major) {
    *r_major = BUI_VERSION_MAJOR;
  }
  if (r_minor) {
    *r_minor = BUI_VERSION_MINOR;
  }
  if (r_patch) {
    *r_patch = BUI_VERSION_PATCH;
  }
}

const char *bui_version_string(void)
{
  return "BUI 0.1.0";
}

void bui_set_render_sink(const BuiGpuSink *sink)
{
  GPU_backend_sink_set(sink);
}

const BuiGpuSink *bui_get_render_sink(void)
{
  return GPU_backend_sink_get();
}

void bui_set_viewport(int width, int height)
{
  GPU_viewport_size_set(width, height);
}
