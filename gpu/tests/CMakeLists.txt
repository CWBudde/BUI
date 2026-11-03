# bui_gpu tests
# SPDX-License-Identifier: GPL-2.0-or-later

add_executable(bui_gpu_parity test_gpu_parity.c)
target_link_libraries(bui_gpu_parity PRIVATE bui_gpu)
if(NOT MSVC)
  target_link_libraries(bui_gpu_parity PRIVATE m)
endif()
add_test(NAME gpu_parity COMMAND bui_gpu_parity)
