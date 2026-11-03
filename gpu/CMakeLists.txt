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
# Text: GDI on Windows; the portable stb_truetype engine everywhere else, or
# on Windows too with BUI_TEXT_STB=ON.
option(BUI_TEXT_STB "Use the portable (stb_truetype) text engine on Windows too" OFF)
if(BUI_TEXT_STB)
  target_compile_definitions(bui_gpu PRIVATE BUI_TEXT_STB=1)
endif()
if(WIN32)
  # gpu_raster.c: GDI glyph-run text engine + native window present.
  target_link_libraries(bui_gpu PUBLIC gdi32 user32)
endif()
