# bui_gpu - drawing abstraction with a null backend
#
# The UI toolkit draws exclusively through these seam headers. Ship a real
# renderer by implementing the same API (GL / Metal / software rasterizer).
#
# SPDX-License-Identifier: GPL-2.0-or-later

set(SRC
  intern/gpu_null.c
)

add_library(bui_gpu STATIC ${SRC})
add_library(bui::gpu ALIAS bui_gpu)

target_include_directories(bui_gpu PUBLIC
  ${CMAKE_CURRENT_SOURCE_DIR}/include
  ${CMAKE_SOURCE_DIR}/core/include
)
target_link_libraries(bui_gpu PUBLIC bui::core)
