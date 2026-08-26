# bui_core tests
# SPDX-License-Identifier: GPL-2.0-or-later

add_executable(bui_core_tests test_core.c)
target_link_libraries(bui_core_tests PRIVATE bui_core)
target_include_directories(bui_core_tests PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/../intern)

if(CMAKE_C_COMPILER_ID MATCHES "GNU|Clang")
  target_compile_options(bui_core_tests PRIVATE
    "-include${CMAKE_CURRENT_SOURCE_DIR}/../intern/bui_c_compat.h"
  )
endif()

add_test(NAME core COMMAND bui_core_tests)
