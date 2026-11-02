# BUI stable C API — a thin facade for hosts and language bindings.
# SPDX-License-Identifier: GPL-2.0-or-later

add_library(bui_api STATIC bui_c_api.c)
target_include_directories(bui_api PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})
target_link_libraries(bui_api PUBLIC bui_gpu)
