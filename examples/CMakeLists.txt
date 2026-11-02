# BUI examples.
# SPDX-License-Identifier: GPL-2.0-or-later

# Renders a showcase scene through the GPU sink into an SVG file. Works with
# the null backend, no window system needed.
add_executable(bui_svg_demo svg_demo.c)
target_link_libraries(bui_svg_demo PRIVATE bui_api bui_gpu)
