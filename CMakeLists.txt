# BUI - Blender-derived UI toolkit
#
# A decoupled UI library extracted from the Blender project.
# The application-specific layers (scene, nodes, depsgraph, operators on
# datablocks...) are NOT part of this library. Only reusable interface
# machinery lives here: core containers/math (C), UI types, drawing
# abstraction, and the widget toolkit.
#
# SPDX-License-Identifier: GPL-2.0-or-later

cmake_minimum_required(VERSION 3.20)

# One version for the C library, the Python package and the PyPI wheel.
file(STRINGS "${CMAKE_CURRENT_SOURCE_DIR}/bindings/python/bui/_version.py" _bui_version_line
     REGEX "^__version__")
string(REGEX MATCH "[0-9]+[.][0-9]+[.][0-9]+" BUI_VERSION "${_bui_version_line}")
project(bui LANGUAGES C CXX VERSION ${BUI_VERSION})
# macOS: the kit's native window host (kit/intern/kit_window_cocoa.m) is
# Objective-C. Enabled here, in the top directory, so every target linking
# bui_kit knows the language.
if(APPLE)
  enable_language(OBJC)
endif()
# static libraries end up in shared modules (Python, Node) on Linux and macOS
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

option(BUI_BUILD_TESTS "Build BUI test executables" ON)
option(BUI_BUILD_EXAMPLES "Build BUI example applications" OFF)
option(BUI_WARNINGS_AS_ERRORS "Treat compiler warnings as errors" OFF)
option(BUI_BUILD_PYTHON_BINDINGS "Build the buipy CPython bindings (needs Python3 dev files)" ON)
option(BUI_BUILD_QT_BACKEND "Build the Qt/QPainter backend (needs Qt5/Qt6 with Widgets)" OFF)
# The optional parts (image formats, video, GPU renderers, file browser, Dear
# ImGui on BUI, ...): one option each, from features.json.
include(features.cmake)

if(NOT CMAKE_BUILD_TYPE)
  set(CMAKE_BUILD_TYPE Release)
endif()

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Size and speed: every function and datum in its own section, so linkers drop
# what a program never calls (MSVC /OPT:REF, GNU --gc-sections: Rust passes
# both). Release is optimized explicitly: cmake-rs (blenderui-sys) replaces
# CMAKE_C_FLAGS_RELEASE with flags that carry no /O2, and the Visual Studio
# generator then compiled the kit without optimization.
if(MSVC)
  add_compile_options($<$<CONFIG:Release>:/O2> /Gy /Gw)
else()
  add_compile_options(-ffunction-sections -fdata-sections)
endif()

if(MSVC)
  add_compile_options(/W3 /utf-8)
  add_compile_definitions(_USE_MATH_DEFINES _CRT_SECURE_NO_WARNINGS)  # M_PI*; POSIX names
else()
  add_compile_options(-Wall -Wno-unused-parameter -Wno-missing-field-initializers)
  if(NOT BUI_WARNINGS_AS_ERRORS)
    add_compile_options(-Wno-error)
  endif()
endif()

set(BUI_SOURCE_DIR ${CMAKE_CURRENT_SOURCE_DIR})

# Dependency order matters: each layer may only use layers above it in this list.
add_subdirectory(core)   # pure C foundation
add_subdirectory(types)  # BUI-owned shared data types
add_subdirectory(gpu)    # draw abstraction (null backend + sink dispatch)
add_subdirectory(compat) # seam headers/stubs replacing Blender-app APIs
add_subdirectory(ui)     # the widget toolkit
add_subdirectory(kit)    # Blender UI runtime in C (widgets, input, damage rendering)
add_subdirectory(api)    # stable C ABI for hosts & bindings

if(BUI_BUILD_TESTS)
  enable_testing()
  add_subdirectory(core/tests)
  add_subdirectory(gpu/tests)
  # the native window host (Win32 / X11): exits 77 (skipped) without a display
  add_executable(bui_window_smoke tests/window_smoke.c)
  target_link_libraries(bui_window_smoke PRIVATE bui::kit)
  add_test(NAME bui_window_smoke COMMAND bui_window_smoke)
  set_tests_properties(bui_window_smoke PROPERTIES SKIP_RETURN_CODE 77 TIMEOUT 30)
  # themes: Blender's Dark / Light presets (BK_set_theme), dark stays pixel-identical
  add_executable(bui_theme_test tests/theme_test.c)
  target_link_libraries(bui_theme_test PRIVATE bui::kit)
  add_test(NAME bui_theme_test COMMAND bui_theme_test)
  # images: every format, decode at size, the cache and loader, no leaks
  add_executable(bui_image_test tests/image_test.c)
  target_link_libraries(bui_image_test PRIVATE bui::kit)
  target_compile_definitions(bui_image_test PRIVATE
    BUI_TEST_IMAGES="${CMAKE_CURRENT_SOURCE_DIR}/tests/images"
    BUI_TEST_TMP="${CMAKE_CURRENT_BINARY_DIR}/image_test_files")
  add_test(NAME bui_image_test COMMAND bui_image_test)
  set_tests_properties(bui_image_test PROPERTIES TIMEOUT 120)
  # video: play, pause, seek, rate, loop, off screen, malformed files, no leaks
  # (exits 77, skipped, without a backend that decodes the H.264 clip)
  add_executable(bui_video_test tests/video_test.c)
  target_link_libraries(bui_video_test PRIVATE bui::kit)
  target_compile_definitions(bui_video_test PRIVATE
    BUI_TEST_VIDEOS="${CMAKE_CURRENT_SOURCE_DIR}/tests/videos"
    BUI_TEST_TMP="${CMAKE_CURRENT_BINARY_DIR}")
  add_test(NAME bui_video_test COMMAND bui_video_test)
  set_tests_properties(bui_video_test PROPERTIES SKIP_RETURN_CODE 77 TIMEOUT 180)
  if(WIN32) # the thumbnail-grid benchmark (bench/images/image_grid.c; not a test)
    add_executable(bui_image_bench bench/images/image_grid.c)
    target_link_libraries(bui_image_bench PRIVATE bui::kit psapi)
    # players at once, CPU and GPU (bench/video/video_bench.c; not a test)
    add_executable(bui_video_bench bench/video/video_bench.c)
    target_link_libraries(bui_video_bench PRIVATE bui::kit psapi)
  endif()
endif()

# before the Python bindings: they add the blenderui.imgui extension when bui_cimgui exists
if(BUI_BUILD_IMGUI)
  add_subdirectory(bindings/imgui)
endif()

if(BUI_BUILD_PYTHON_BINDINGS)
  add_subdirectory(bindings/python)
endif()

if(BUI_BUILD_LUA_BINDINGS)
  add_subdirectory(bindings/lua)
endif()

if(BUI_BUILD_QT_BACKEND)
  add_subdirectory(backends/qt)
endif()


if(BUI_BUILD_EXAMPLES)
  add_subdirectory(examples)
endif()
