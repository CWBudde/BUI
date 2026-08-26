# bui_compat - seam headers and stubs
#
# These headers carry the same names as the Blender-app APIs the extracted UI
# code calls (RNA, CTX, BLF, IMB, ED, WM). Implementations here are BUI-owned:
# opaque types, stubs, or thin extension points. Over time each seam either
# gets a real implementation or the toolkit call-sites migrate to a native
# BUI API.
#
# SPDX-License-Identifier: GPL-2.0-or-later

add_library(bui_compat INTERFACE)
add_library(bui::compat ALIAS bui_compat)

target_include_directories(bui_compat INTERFACE
  ${CMAKE_CURRENT_SOURCE_DIR}/include
  ${CMAKE_SOURCE_DIR}/types/dna/include
  ${CMAKE_SOURCE_DIR}/gpu/include
  ${CMAKE_SOURCE_DIR}/core/include
)

target_link_libraries(bui_compat INTERFACE bui::core bui::types bui::gpu)
