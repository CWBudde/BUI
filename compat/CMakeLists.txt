# bui_compat - seam headers and stub implementations
#
# SPDX-License-Identifier: GPL-2.0-or-later

add_library(bui_compat STATIC
  intern/bui_stubs.cc
)
add_library(bui::compat ALIAS bui_compat)

target_include_directories(bui_compat PUBLIC
  ${CMAKE_CURRENT_SOURCE_DIR}/include
  ${BUI_SOURCE_DIR}/types/dna/include
  ${BUI_SOURCE_DIR}/gpu/include
  ${BUI_SOURCE_DIR}/core/include
)
target_link_libraries(bui_compat PUBLIC bui::core bui::types bui::gpu)
