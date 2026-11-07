# bui_core - pure C foundation (extracted from Blender's blenlib)
#
# SPDX-License-Identifier: GPL-2.0-or-later

set(SRC
  intern/listbase.c
  intern/BLI_dynstr.c
  intern/BLI_memarena.c
  intern/BLI_mempool.c
  intern/BLI_ghash.c
  intern/BLI_ghash_utils.c
  intern/BLI_linklist.c
  intern/rct.c
  intern/math_base.c
  intern/math_vector.c
  intern/math_matrix_c.c
  intern/math_rotation_c.c
  intern/math_color.c
  intern/math_geom.c
  intern/matrix_invert.c
  intern/string.c
  intern/string_utf8.c
  intern/string_cursor_utf8.c
  intern/endian_switch.c
  intern/array_utils_c.c
  intern/sort_utils.c
  intern/hash_mm3.c
  intern/hash_mm2.c
  intern/svd_m3.c
  intern/fnmatch.c
  intern/gsqueue.c
  intern/stack.c
  intern/BLI_assert.c
  intern/guardedalloc.c
  intern/bui_threads.c
  intern/bui_system_stubs.c
  intern/wcwidth/wcwidth.c
)

# Blender C++ (blenlib) used through C wrappers (include/BUI_blender_c.h):
# an object library (no forced C shim) whose objects go into bui_core.
add_library(bui_core_cxx OBJECT
  intern/expr_pylike_eval.cc
  intern/string_search.cc
  intern/bui_blender_c.cc
)
target_include_directories(bui_core_cxx PRIVATE
  ${CMAKE_CURRENT_SOURCE_DIR}/include
  ${BUI_SOURCE_DIR}/types/dna/include
  ${CMAKE_CURRENT_SOURCE_DIR}
  ${CMAKE_CURRENT_SOURCE_DIR}/intern
)

add_library(bui_core STATIC ${SRC} $<TARGET_OBJECTS:bui_core_cxx>)

# Blender builds with unsigned `char` (-funsigned-char, /J); blenlib's UTF-8
# decoding (string_utf8.c, `c < 128` / `c >= 192` tests) depends on it.
foreach(_t bui_core bui_core_cxx)
  if(MSVC)
    target_compile_options(${_t} PRIVATE /J)
  else()
    target_compile_options(${_t} PRIVATE -funsigned-char)
  endif()
endforeach()
if(MSVC)
  target_compile_options(bui_core_cxx PRIVATE /permissive-)
endif()
add_library(bui::core ALIAS bui_core)

target_include_directories(bui_core PUBLIC
  ${CMAKE_CURRENT_SOURCE_DIR}/include
  ${BUI_SOURCE_DIR}/types/dna/include
  # headers #include "intern/*.c" inline implementations, relative to module root
  ${CMAKE_CURRENT_SOURCE_DIR}
)
target_include_directories(bui_core PRIVATE
  ${CMAKE_CURRENT_SOURCE_DIR}/intern
  ${CMAKE_CURRENT_SOURCE_DIR}/intern/atomic
  ${CMAKE_CURRENT_SOURCE_DIR}/intern/wcwidth
)

# Forced compat shim so upstream sources compile as-is in C mode (GCC/Clang).
if(MSVC)  # cl and clang-cl spell the forced include /FI
  target_compile_options(bui_core PRIVATE
    "/FI${CMAKE_CURRENT_SOURCE_DIR}/intern/bui_c_compat.h"
  )
elseif(CMAKE_C_COMPILER_ID MATCHES "GNU|Clang")
  target_compile_options(bui_core PRIVATE
    "-include${CMAKE_CURRENT_SOURCE_DIR}/intern/bui_c_compat.h"
  )
endif()

find_package(Threads REQUIRED)
target_link_libraries(bui_core PUBLIC Threads::Threads)
if(UNIX)
  target_link_libraries(bui_core PUBLIC m)
endif()

