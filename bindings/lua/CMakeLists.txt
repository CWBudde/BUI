# bui-lua: standalone Lua 5.4 host for the BUI toolkit.
# SPDX-License-Identifier: GPL-2.0-or-later

set(LUA_DIR ${CMAKE_CURRENT_SOURCE_DIR}/lua-5.4.7)
set(LUA_SRC
    ${LUA_DIR}/src/lapi.c      ${LUA_DIR}/src/lauxlib.c    ${LUA_DIR}/src/lbaselib.c
    ${LUA_DIR}/src/lcode.c     ${LUA_DIR}/src/lcorolib.c   ${LUA_DIR}/src/lctype.c
    ${LUA_DIR}/src/ldblib.c    ${LUA_DIR}/src/ldebug.c     ${LUA_DIR}/src/ldo.c
    ${LUA_DIR}/src/ldump.c     ${LUA_DIR}/src/lfunc.c      ${LUA_DIR}/src/lgc.c
    ${LUA_DIR}/src/linit.c     ${LUA_DIR}/src/liolib.c     ${LUA_DIR}/src/llex.c
    ${LUA_DIR}/src/lmathlib.c  ${LUA_DIR}/src/lmem.c       ${LUA_DIR}/src/loadlib.c
    ${LUA_DIR}/src/lobject.c   ${LUA_DIR}/src/lopcodes.c   ${LUA_DIR}/src/loslib.c
    ${LUA_DIR}/src/lparser.c   ${LUA_DIR}/src/lstate.c     ${LUA_DIR}/src/lstring.c
    ${LUA_DIR}/src/lstrlib.c   ${LUA_DIR}/src/ltable.c     ${LUA_DIR}/src/ltablib.c
    ${LUA_DIR}/src/ltm.c       ${LUA_DIR}/src/lundump.c    ${LUA_DIR}/src/lutf8lib.c
    ${LUA_DIR}/src/lvm.c       ${LUA_DIR}/src/lzio.c)

add_library(bui_lua STATIC ${LUA_SRC})
target_include_directories(bui_lua PUBLIC ${LUA_DIR}/src)
if(MSVC)
  target_compile_definitions(bui_lua PRIVATE LUA_BUILD_AS_DLL)
  target_compile_options(bui_lua PRIVATE /W0)
else()
  target_compile_options(bui_lua PRIVATE -w -fvisibility=hidden)
endif()

add_library(linenoise STATIC
    ${CMAKE_CURRENT_SOURCE_DIR}/extern/linenoise/linenoise.c)
target_include_directories(linenoise PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/extern/linenoise)

add_executable(bui-lua
    src/main.c
    src/bui_lua.c
    src/bui_node.c
    src/bui_context.c
    src/bui_window.c
    src/bui_image.c)
target_link_libraries(bui-lua PRIVATE
    bui_lua
    linenoise
    bui::kit
    bui::gpu
    bui::core
    bui_api)
target_include_directories(bui-lua PRIVATE src)
