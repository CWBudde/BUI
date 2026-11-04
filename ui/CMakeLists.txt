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
  # Upstream headers rely on transitive math includes that the de-namespaced
  # core no longer provides implicitly.
  target_compile_options(bui_ui PRIVATE
    "-includeBLI_math_vector.h"
    "-includeBLI_math_color.h"
  )
else()
  # Headers-only target: lets consumers resolve UI_* headers without compiling.
  add_library(bui_ui INTERFACE)
  add_library(bui::ui ALIAS bui_ui)
  target_include_directories(bui_ui INTERFACE ${UI_INC})
endif()

# Blender's File Browser (editors/space_file + ED_fileselect.hh), staged in
# space_file/ from upstream d0ee3e1db6f (2026-11-04) with only the mechanical
# include pass (BLI_*.hh -> BUI's BLI_*.h). Not compiled: upstream has moved
# past the vendored blenlib/DNA (MutableCStr, ListBaseT methods, enum types) and
# the rest needs the asset system, BLO, ImBuf thumbnails and WM operators.
# The running port is kit/intern/kit_filebrowser.c ("filebrowser" node), which
# names the upstream function behind each part; diff against these files when
# syncing it.
set(UI_SPACE_FILE_STAGED
  space_file/ED_fileselect.hh
  space_file/file_intern.hh
  space_file/filelist.hh
  space_file/fsmenu.hh
  space_file/fsmenu.cc
  space_file/fsmenu_system_unix.cc
  space_file/fsmenu_system_win32.cc
  space_file/fsmenu_system_macos.mm
  space_file/folder_history.cc
  space_file/file_draw.cc
  space_file/file_ops.cc
  space_file/file_panels.cc
  space_file/file_utils.cc
  space_file/file_context.cc
  space_file/filesel.cc
  space_file/space_file.cc
  space_file/filelist/filelist.cc
  space_file/filelist/filelist_filter.cc
  space_file/filelist/filelist_intern.hh
  space_file/filelist/filelist_readjob.cc
  space_file/filelist/filelist_readjob.hh
  space_file/filelist/filelist_readjob_common.cc
  space_file/filelist/filelist_readjob_directories.cc
  space_file/filelist/filelist_sort.cc
)
