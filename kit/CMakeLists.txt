# bui_kit - Blender's interface as a retained-mode C runtime: nodes, layout,
# input handling (interface_handlers.cc rules) and widget drawing ported from
# interface_widgets.cc, rendered into a BuiRaster with damage tracking.
#
# SPDX-License-Identifier: GPL-2.0-or-later

add_library(bui_kit STATIC
  intern/kit_core.c
  intern/kit_draw.c
  intern/kit_icons_data.c
  intern/kit_widgets.c
  intern/kit_views.c
  intern/kit_menus.c
  intern/kit_color.c
  intern/kit_frame.c
  intern/kit_textarea.c
  intern/kit_window.c
  intern/kit_icons_svg.c
  intern/kit_regions.c
  extern/nanosvg/blender_nanosvg.c
  extern/nanosvg/blender_raster.c
)
# Blender's icons: its SVGs (kit_icons_svg.c) rasterized with its nanosvg.
target_include_directories(bui_kit PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/extern/nanosvg)
set_source_files_properties(extern/nanosvg/blender_nanosvg.c extern/nanosvg/blender_raster.c
                            PROPERTIES COMPILE_OPTIONS "$<IF:$<C_COMPILER_ID:MSVC>,/w,-w>")
add_library(bui::kit ALIAS bui_kit)
target_include_directories(bui_kit PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
target_include_directories(bui_kit PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/intern)
# bui_core: Blender's color math and UTF-8 cursor stepping; bui_gpu: raster.
target_link_libraries(bui_kit PUBLIC bui::gpu bui::core)
if(NOT MSVC)
  target_compile_options(bui_kit PRIVATE -O2)
  target_link_libraries(bui_kit PRIVATE m)
endif()
if(WIN32)
  target_link_libraries(bui_kit PRIVATE user32 gdi32)
endif()
