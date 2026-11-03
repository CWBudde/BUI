# bui_gpu - drawing abstraction with a null backend
#
# The UI toolkit draws exclusively through these seam headers. Ship a real
# renderer by implementing the same API (GL / Metal / software rasterizer).
#
# SPDX-License-Identifier: GPL-2.0-or-later

set(SRC
  intern/gpu_null.c
  intern/gpu_raster.c
  intern/gpu_d3d11.c
)

add_library(bui_gpu STATIC ${SRC})
add_library(bui::gpu ALIAS bui_gpu)

target_include_directories(bui_gpu PUBLIC
  ${CMAKE_CURRENT_SOURCE_DIR}/include
  ${CMAKE_SOURCE_DIR}/core/include
)
target_link_libraries(bui_gpu PUBLIC bui::core)
if(NOT MSVC)
  target_link_libraries(bui_gpu PRIVATE m)
endif()
# Text: the portable stb_truetype engine everywhere (Inter embedded as the
# default face, like Blender; glyph outlines feed the GPU's Slug renderer).
# BUI_TEXT_GDI=ON switches Windows to the native GDI engine (system fonts,
# ClearType-style hinting, no Slug). BUI_TEXT_STB is kept as a no-op.
option(BUI_TEXT_GDI "Use the native GDI text engine on Windows" OFF)
option(BUI_TEXT_STB "Deprecated: the portable engine is the default" ON)
if(BUI_TEXT_GDI)
  target_compile_definitions(bui_gpu PRIVATE BUI_TEXT_GDI=1)
endif()
if(WIN32)
  # gpu_raster.c: GDI glyph-run text engine + native window present.
  target_link_libraries(bui_gpu PUBLIC gdi32 user32)
endif()
