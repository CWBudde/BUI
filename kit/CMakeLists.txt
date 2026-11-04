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
  intern/kit_window_win32.c
  intern/kit_window_x11.c
  intern/kit_window_null.c
  intern/kit_icons_svg.c
  intern/kit_regions.c
  intern/kit_nodes.c
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
elseif(APPLE)
  # Native windows on macOS: an AppKit host after Blender's GHOST Cocoa backend.
  # Objective-C (enable_language(OBJC) is in the top-level CMakeLists.txt) with
  # manual retain/release, as GHOST; kit_window_null.c compiles to nothing.
  target_sources(bui_kit PRIVATE intern/kit_window_cocoa.m)
  set_source_files_properties(intern/kit_window_cocoa.m PROPERTIES COMPILE_OPTIONS "-fno-objc-arc")
  target_compile_definitions(bui_kit PRIVATE BUI_WINDOW_COCOA)
  target_link_libraries(bui_kit PUBLIC "-framework Cocoa" "-framework CoreGraphics" objc)
elseif(UNIX AND NOT APPLE)
  # Native windows on X11 (Wayland sessions run it through XWayland).
  # Only the headers are needed: libX11 is dlopen()ed at run time, so builds
  # (and wheels) run without X11 installed and open windows where it is.
  find_path(BUI_X11_INCLUDE X11/Xlib.h)
  if(BUI_X11_INCLUDE)
    target_compile_definitions(bui_kit PRIVATE BUI_WINDOW_X11)
    target_include_directories(bui_kit PRIVATE ${BUI_X11_INCLUDE})
    target_link_libraries(bui_kit PUBLIC ${CMAKE_DL_LIBS})
  else()
    message(STATUS "BUI: X11 headers not found; no native Linux windows (Tk / headless only)")
  endif()
endif()
