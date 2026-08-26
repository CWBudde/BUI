# bui_ui - the widget toolkit extracted from Blender's editors/interface
#
# STATUS: sources staged, compile-in-progress. This module is option-gated so
# the root build stays green while seams are completed. Enable with
# -DBUI_BUILD_UI=ON to work on it.
#
# SPDX-License-Identifier: GPL-2.0-or-later

option(BUI_BUILD_UI "Compile the ui toolkit module (seams WIP)" OFF)

set(UI_SRC
  intern/interface_align.cc
  intern/interface_button_group.cc
  intern/interface_button_sections.cc
  intern/interface_context_path.cc
  intern/interface_drag.cc
  intern/interface_draw.cc
  intern/interface_drop.cc
  intern/interface_query.cc
  intern/interface_string_search.cc
  intern/interface_style.cc
  intern/interface_utils.cc
  intern/interface_widgets.cc
  intern/resources.cc
  intern/interface_textbox.cc
  intern/view2d.cc
  intern/view2d_edge_pan.cc
)

set(UI_INC
  ${CMAKE_CURRENT_SOURCE_DIR}/include
  ${CMAKE_CURRENT_SOURCE_DIR}/intern
)

if(BUI_BUILD_UI)
  add_library(bui_ui STATIC ${UI_SRC})
  add_library(bui::ui ALIAS bui_ui)
  target_include_directories(bui_ui PUBLIC ${UI_INC})
  target_include_directories(bui_ui PRIVATE
    ${CMAKE_SOURCE_DIR}/compat/include
    ${CMAKE_SOURCE_DIR}/types/dna/include
    ${CMAKE_SOURCE_DIR}/gpu/include
  )
  target_link_libraries(bui_ui PUBLIC bui::core bui::types bui::gpu bui::compat)
else()
  # Headers-only target: lets consumers resolve UI_* headers without compiling.
  add_library(bui_ui INTERFACE)
  add_library(bui::ui ALIAS bui_ui)
  target_include_directories(bui_ui INTERFACE ${UI_INC})
endif()
