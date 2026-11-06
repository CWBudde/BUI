# bui-lua: standalone Lua 5.4 host for the BUI toolkit.
# Downloads Lua 5.4.7 via FetchContent at configure time.
# SPDX-License-Identifier: GPL-2.0-or-later

# Suppress FetchContent_Populate deprecation warning (we don't want
# FetchContent_MakeAvailable because Lua has its own build system).
cmake_policy(PUSH)
cmake_policy(SET CMP0169 OLD)

include(FetchContent)

FetchContent_Declare(
  lua-5.4.7
  URL      https://www.lua.org/ftp/lua-5.4.7.tar.gz
  URL_HASH SHA256=9fbf5e28ef86c69858f6d3d34eccc32e911c1a28b4120ff3e84aaa70cfbf1e30
)

FetchContent_GetProperties(lua-5.4.7)
if(NOT lua-5.4.7_POPULATED)
  FetchContent_Populate(lua-5.4.7)
endif()

cmake_policy(POP)

set(LUA_SRC_DIR ${lua-5.4.7_SOURCE_DIR}/src)

set(LUA_SRC
    ${LUA_SRC_DIR}/lapi.c      ${LUA_SRC_DIR}/lauxlib.c
    ${LUA_SRC_DIR}/lbaselib.c   ${LUA_SRC_DIR}/lcode.c
    ${LUA_SRC_DIR}/lcorolib.c   ${LUA_SRC_DIR}/lctype.c
    ${LUA_SRC_DIR}/ldblib.c     ${LUA_SRC_DIR}/ldebug.c
    ${LUA_SRC_DIR}/ldo.c        ${LUA_SRC_DIR}/ldump.c
    ${LUA_SRC_DIR}/lfunc.c      ${LUA_SRC_DIR}/lgc.c
    ${LUA_SRC_DIR}/linit.c      ${LUA_SRC_DIR}/liolib.c
    ${LUA_SRC_DIR}/llex.c       ${LUA_SRC_DIR}/lmathlib.c
    ${LUA_SRC_DIR}/lmem.c       ${LUA_SRC_DIR}/loadlib.c
    ${LUA_SRC_DIR}/lobject.c    ${LUA_SRC_DIR}/lopcodes.c
    ${LUA_SRC_DIR}/loslib.c     ${LUA_SRC_DIR}/lparser.c
    ${LUA_SRC_DIR}/lstate.c     ${LUA_SRC_DIR}/lstring.c
    ${LUA_SRC_DIR}/lstrlib.c    ${LUA_SRC_DIR}/ltable.c
    ${LUA_SRC_DIR}/ltablib.c    ${LUA_SRC_DIR}/ltm.c
    ${LUA_SRC_DIR}/lundump.c    ${LUA_SRC_DIR}/lutf8lib.c
    ${LUA_SRC_DIR}/lvm.c        ${LUA_SRC_DIR}/lzio.c)

add_library(bui_lua STATIC ${LUA_SRC})
target_include_directories(bui_lua PUBLIC ${LUA_SRC_DIR})

if(MSVC)
  target_compile_definitions(bui_lua PRIVATE LUA_BUILD_AS_DLL)
  target_compile_options(bui_lua PRIVATE /W0)
else()
  target_compile_options(bui_lua PRIVATE -w -fvisibility=hidden)
endif()

# linenoise (vendored, single-file MIT library)
add_library(linenoise STATIC
    ${CMAKE_CURRENT_SOURCE_DIR}/extern/linenoise/linenoise.c)
target_include_directories(linenoise PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/extern/linenoise)

# bui-lua executable
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
