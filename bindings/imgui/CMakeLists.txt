# bui_imgui - Dear ImGui (vendored, extern/imgui) running on BUI: a platform +
# renderer backend (imgui_impl_bui.cpp) and Blender's look (imgui_bui_style.cpp).
# See README.md.
#
# SPDX-License-Identifier: GPL-2.0-or-later

set(IMGUI_DIR ${CMAKE_CURRENT_SOURCE_DIR}/extern/imgui)
set(BUI_IMGUI_CORE
  ${IMGUI_DIR}/imgui.cpp
  ${IMGUI_DIR}/imgui_draw.cpp
  ${IMGUI_DIR}/imgui_tables.cpp
  ${IMGUI_DIR}/imgui_widgets.cpp
  ${IMGUI_DIR}/imgui_demo.cpp
  ${IMGUI_DIR}/misc/cpp/imgui_stdlib.cpp
)
set(BUI_IMGUI_BACKEND
  ${CMAKE_CURRENT_SOURCE_DIR}/imgui_impl_bui.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/imgui_bui_style.cpp
)
# Dear ImGui's own sources keep their upstream warning level.
set_source_files_properties(${BUI_IMGUI_CORE} PROPERTIES COMPILE_OPTIONS "$<IF:$<CXX_COMPILER_ID:MSVC>,/W1,-w>")

add_library(bui_imgui STATIC ${BUI_IMGUI_CORE} ${BUI_IMGUI_BACKEND})
add_library(bui::imgui ALIAS bui_imgui)
target_include_directories(bui_imgui PUBLIC
  ${CMAKE_CURRENT_SOURCE_DIR}
  ${IMGUI_DIR}
  ${IMGUI_DIR}/misc/cpp
)
target_link_libraries(bui_imgui PUBLIC bui::kit)

# examples/imgui_demo.cpp: ImGui::ShowDemoWindow() in a BUI window (or headless with
# --screenshot).
add_executable(bui_imgui_demo ${CMAKE_SOURCE_DIR}/examples/imgui_demo.cpp)
target_include_directories(bui_imgui_demo PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/tests)
target_link_libraries(bui_imgui_demo PRIVATE bui::imgui)

if(BUI_BUILD_TESTS)
  # The test compiles its own Dear ImGui with the test-engine hooks (to open every
  # section of the demo), so it does not link bui_imgui.
  add_executable(bui_imgui_test tests/test_imgui_bui.cpp ${BUI_IMGUI_CORE} ${BUI_IMGUI_BACKEND})
  target_compile_definitions(bui_imgui_test PRIVATE IMGUI_ENABLE_TEST_ENGINE)
  target_include_directories(bui_imgui_test PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR} ${IMGUI_DIR} ${IMGUI_DIR}/misc/cpp ${CMAKE_CURRENT_SOURCE_DIR}/tests)
  target_link_libraries(bui_imgui_test PRIVATE bui::kit)
  add_test(NAME imgui_backend COMMAND bui_imgui_test)
  set_tests_properties(imgui_backend PROPERTIES TIMEOUT 300)
endif()
