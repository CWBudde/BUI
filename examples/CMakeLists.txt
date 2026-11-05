# BUI examples.
# SPDX-License-Identifier: GPL-2.0-or-later

# Renders a showcase scene through the GPU sink into an SVG file. Works with
# the null backend, no window system needed.
add_executable(bui_svg_demo svg_demo.c)
target_link_libraries(bui_svg_demo PRIVATE bui_api bui_gpu)

# The C kit from C: nodes, geometry, events, native window (docs/c-api.md).
add_executable(bui_kit_hello kit_hello.c)
target_link_libraries(bui_kit_hello PRIVATE bui::kit)

# The website's screenshot apps (examples/site/; tools/gen_doc_images.py --site):
# a modifier stack from C, and a scene inspector in Dear ImGui (C++). Both take
# --screenshot out.png (off-screen, BUI_THEME and BUI_UI_SCALE apply).
add_executable(bui_site_kit_app site/kit_modifiers.c)
target_link_libraries(bui_site_kit_app PRIVATE bui::kit)
if(TARGET bui_imgui)
  add_executable(bui_site_imgui site/imgui_inspector.cpp)
  target_link_libraries(bui_site_imgui PRIVATE bui::imgui)
endif()
