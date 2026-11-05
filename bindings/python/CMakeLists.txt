# buipy — CPython bindings for BUI (no third-party build dependencies).
# SPDX-License-Identifier: GPL-2.0-or-later

find_package(Python3 QUIET COMPONENTS Interpreter Development.Module)

if(NOT Python3_FOUND)
  message(STATUS "BUI: Python3 development files not found — buipy bindings skipped")
  return()
endif()

message(STATUS "BUI: building buipy Python bindings for Python ${Python3_VERSION}")

Python3_add_library(buipy MODULE WITH_SOABI src/buipy_module.c src/buipy_kit.c)
target_link_libraries(buipy PRIVATE bui_api bui_gpu bui_kit)

# ($<0:> keeps multi-config generators from adding a Release/ subdirectory.)
# Keep the extension next to the pure-Python 'bui' package in the build tree so
# tests and examples can import both with a single sys.path entry.
set_target_properties(buipy PROPERTIES
  LIBRARY_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bindings/python$<0:>
  RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bindings/python$<0:>
)

# Dear ImGui (bindings/imgui) for Python: the _buiimgui extension behind blenderui.imgui /
# bui.imgui. Its API is generated from imgui.h (src/buiimgui_gen.inc, bindings/imgui/gen).
if(TARGET bui_cimgui)
  Python3_add_library(_buiimgui MODULE WITH_SOABI src/buiimgui_module.cpp)
  target_link_libraries(_buiimgui PRIVATE bui_cimgui)
  if(MSVC)
    target_compile_options(_buiimgui PRIVATE /bigobj)
  endif()
  set_target_properties(_buiimgui PROPERTIES
    LIBRARY_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bindings/python$<0:>
    RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bindings/python$<0:>
  )
  if(SKBUILD)
    install(TARGETS _buiimgui LIBRARY DESTINATION . RUNTIME DESTINATION .)
    if(MINGW)
      target_link_options(_buiimgui PRIVATE -static-libgcc -static)
    endif()
  endif()
endif()

# Wheel layout (pip install blenderui / scikit-build-core): the extension and
# the pure-Python packages side by side in site-packages.
if(SKBUILD)
  install(TARGETS buipy LIBRARY DESTINATION . RUNTIME DESTINATION .)
  install(DIRECTORY bui blenderui DESTINATION .
          PATTERN "__pycache__" EXCLUDE PATTERN "*.pyc" EXCLUDE)
  if(MINGW)  # no MinGW runtime DLLs next to the wheel's .pyd
    target_link_options(buipy PRIVATE -static-libgcc -static)
  endif()
endif()

if(BUI_BUILD_TESTS)
  # buipy comes from the build dir; the 'bui' package from this source dir.
  if(WIN32)
    set(BUIPY_TEST_PYTHONPATH "${CMAKE_BINARY_DIR}/bindings/python\;${CMAKE_CURRENT_SOURCE_DIR}")
  else()
    set(BUIPY_TEST_PYTHONPATH "${CMAKE_BINARY_DIR}/bindings/python:${CMAKE_CURRENT_SOURCE_DIR}")
  endif()
  add_test(
    NAME buipy
    COMMAND ${Python3_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/tests/test_buipy.py
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
  )
  set_tests_properties(buipy PROPERTIES ENVIRONMENT "PYTHONPATH=${BUIPY_TEST_PYTHONPATH}")

  # High-level syntax layers (Qt syntax + tkinter syntax), rendered headless.
  add_test(
    NAME buipy_syntax
    COMMAND ${Python3_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/tests/test_syntax.py
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
  )
  set_tests_properties(buipy_syntax PROPERTIES ENVIRONMENT "PYTHONPATH=${BUIPY_TEST_PYTHONPATH}")

  # Advanced surface: Text/Treeview/Table/Notebook/Menu/PhotoImage/QPainter.
  add_test(
    NAME buipy_advanced
    COMMAND ${Python3_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/tests/test_advanced.py
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
  )
  set_tests_properties(buipy_advanced PROPERTIES ENVIRONMENT "PYTHONPATH=${BUIPY_TEST_PYTHONPATH}")

  # Native raster pipeline, damage tracking, redraw coalescing, input/layout
  # regressions (hover vs drag, popup z-order, Toplevel, nested layouts).
  add_test(
    NAME buipy_render
    COMMAND ${Python3_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/tests/test_render.py
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
  )
  set_tests_properties(buipy_render PROPERTIES ENVIRONMENT "PYTHONPATH=${BUIPY_TEST_PYTHONPATH}")

  # bui.components: Blender component library (layout, interactions, popups).
  add_test(
    NAME buipy_components
    COMMAND ${Python3_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/tests/test_components.py
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
  )
  set_tests_properties(buipy_components PROPERTIES ENVIRONMENT "PYTHONPATH=${BUIPY_TEST_PYTHONPATH}")
  add_test(
    NAME buipy_nodes
    COMMAND ${Python3_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/tests/test_nodes.py
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
  )
  set_tests_properties(buipy_nodes PROPERTIES ENVIRONMENT "PYTHONPATH=${BUIPY_TEST_PYTHONPATH}")
  add_test(
    NAME buipy_tk_compat
    COMMAND ${Python3_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/tests/test_tk_compat.py
  )
  set_tests_properties(buipy_tk_compat PROPERTIES ENVIRONMENT "PYTHONPATH=${BUIPY_TEST_PYTHONPATH}")
  if(TARGET _buiimgui)
    add_test(
      NAME buipy_imgui
      COMMAND ${Python3_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/tests/test_imgui.py
    )
    set_tests_properties(buipy_imgui PROPERTIES ENVIRONMENT "PYTHONPATH=${BUIPY_TEST_PYTHONPATH}")
  endif()
  add_test(
    NAME buipy_tk_input
    COMMAND ${Python3_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/tests/test_tk_input.py
  )
  set_tests_properties(buipy_tk_input PROPERTIES ENVIRONMENT "PYTHONPATH=${BUIPY_TEST_PYTHONPATH}")
  add_test(
    NAME buipy_filebrowser
    COMMAND ${Python3_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/tests/test_filebrowser.py
  )
  set_tests_properties(buipy_filebrowser PROPERTIES ENVIRONMENT "PYTHONPATH=${BUIPY_TEST_PYTHONPATH}")
endif()
