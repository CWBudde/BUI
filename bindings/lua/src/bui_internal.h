/* BUI Lua binding: shared internal types and helpers.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#pragma once

#include "BUI_kit.h"
#include "bui_c_api.h"
#include "lua.h"
#include "lauxlib.h"
#include "lualib.h"
#include <string.h>
#include <stdlib.h>

/* ------------------------------------------------------------------ */
/* Userdata wrapper structs                                           */
/* ------------------------------------------------------------------ */

typedef struct {
  BKNode *node;
} luabui_Node;

typedef struct {
  BKContext *ctx;
} luabui_Context;

typedef struct {
  BKWindow *win;
} luabui_Window;

typedef struct {
  BKTreeItem *item;
} luabui_TreeItem;

typedef struct {
  BKImBuf *ib;
} luabui_ImBuf;

typedef struct {
  BKVideo *video;
} luabui_Video;

/* ------------------------------------------------------------------ */
/* Registry key names (metatables + node map)                         */
/* ------------------------------------------------------------------ */

#define LUABUI_NODE_MT      "bui.Node"
#define LUABUI_CONTEXT_MT   "bui.Context"
#define LUABUI_WINDOW_MT    "bui.Window"
#define LUABUI_TREEITEM_MT  "bui.TreeItem"
#define LUABUI_IMBUF_MT     "bui.ImBuf"
#define LUABUI_VIDEO_MT     "bui.Video"
#define LUABUI_NODE_MAP      "bui._node_map"
#define LUABUI_TREEITEM_MAP  "bui._treeitem_map"
#define LUABUI_TIMER_QUEUE   "bui._timer_queue"

/* ------------------------------------------------------------------ */
/* Macros                                                             */
/* ------------------------------------------------------------------ */

#define CHECK_NODE(L, idx)    ((luabui_Node *)luaL_checkudata(L, idx, LUABUI_NODE_MT))
#define CHECK_CTX(L, idx)     ((luabui_Context *)luaL_checkudata(L, idx, LUABUI_CONTEXT_MT))
#define CHECK_WIN(L, idx)     ((luabui_Window *)luaL_checkudata(L, idx, LUABUI_WINDOW_MT))
#define CHECK_TREEITEM(L, idx)((luabui_TreeItem *)luaL_checkudata(L, idx, LUABUI_TREEITEM_MT))
#define CHECK_IMBUF(L, idx)   ((luabui_ImBuf *)luaL_checkudata(L, idx, LUABUI_IMBUF_MT))
#define CHECK_VIDEO(L, idx)   ((luabui_Video *)luaL_checkudata(L, idx, LUABUI_VIDEO_MT))

/* ------------------------------------------------------------------ */
/* Registration functions                                             */
/* ------------------------------------------------------------------ */

void luabui_register_node(lua_State *L);
void luabui_register_context(lua_State *L);
void luabui_register_window(lua_State *L);
void luabui_register_treeitem(lua_State *L);
void luabui_register_imbuf(lua_State *L);
void luabui_register_video(lua_State *L);

/* ------------------------------------------------------------------ */
/* Node map helpers (BKNode* <-> Lua userdata)                        */
/* ------------------------------------------------------------------ */

/* Store node -> userdata in the weak-keyed node map. */
void luabui_map_node(lua_State *L, BKNode *node, int ud_idx);

/* Look up userdata for a BKNode*. Returns NULL if not found. */
void *luabui_lookup_node(lua_State *L, BKNode *node);

/* Remove a node from the map. */
void luabui_unmap_node(lua_State *L, BKNode *node);

/* ------------------------------------------------------------------ */
/* Tree item map helpers (BKTreeItem* <-> Lua userdata)               */
/* ------------------------------------------------------------------ */

void luabui_map_treeitem(lua_State *L, BKTreeItem *item, int ud_idx);
void *luabui_lookup_treeitem(lua_State *L, BKTreeItem *item);
void luabui_unmap_treeitem(lua_State *L, BKTreeItem *item);

/* Nullify all tree item userdata whose C pointer has been freed. */
void luabui_nullify_treeitem(lua_State *L, BKTreeItem *item);

/* ------------------------------------------------------------------ */
/* Host callbacks (implemented in bui_context.c)                      */
/* ------------------------------------------------------------------ */

extern int luabui_host_requested;

void luabui_init_host_callbacks(lua_State *L, BKContext *ctx);

/* ------------------------------------------------------------------ */
/* Known numeric property names (for node:get() dispatch)             */
/* ------------------------------------------------------------------ */

int luabui_is_numeric_key(const char *key);

/* Lua module entry point (defined in bui_lua.c) */
LUAMOD_API int luaopen_bui(lua_State *L);

/* Factory functions (defined in bui_image.c) */
int l_image_decode(lua_State *L);
int l_image_load(lua_State *L);
int l_video_new(lua_State *L);
