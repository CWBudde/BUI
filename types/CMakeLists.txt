# bui_types - shared data-type definitions (headers only)
#
# Contains the verbatim DNA_*_types.h struct definitions the toolkit operates
# on (regions, areas, themes, events). These are pure data shapes: no Blender
# application logic, no serialization runtime.
#
# SPDX-License-Identifier: GPL-2.0-or-later

add_library(bui_types INTERFACE)
add_library(bui::types ALIAS bui_types)

target_include_directories(bui_types INTERFACE
  ${CMAKE_CURRENT_SOURCE_DIR}/dna/include
  ${CMAKE_SOURCE_DIR}/core/include
)

target_link_libraries(bui_types INTERFACE bui::core)
