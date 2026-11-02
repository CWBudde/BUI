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

option(BUI_BUILD_TESTS "Build BUI test executables" ON)
option(BUI_BUILD_EXAMPLES "Build BUI example applications" OFF)
option(BUI_WARNINGS_AS_ERRORS "Treat compiler warnings as errors" OFF)
option(BUI_BUILD_PYTHON_BINDINGS "Build the buipy CPython bindings (needs Python3 dev files)" ON)
option(BUI_BUILD_QT_BACKEND "Build the Qt/QPainter backend (needs Qt5/Qt6 with Widgets)" OFF)

if(NOT CMAKE_BUILD_TYPE)
  set(CMAKE_BUILD_TYPE Release)
endif()

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

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
endif()

if(BUI_BUILD_PYTHON_BINDINGS)
  add_subdirectory(bindings/python)
endif()

if(BUI_BUILD_QT_BACKEND)
  add_subdirectory(backends/qt)
endif()

if(BUI_BUILD_EXAMPLES)
  add_subdirectory(examples)
endif()
