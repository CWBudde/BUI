/* BUI Lua binding: BKContext userdata and BKHost callback bridge.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#include "bui_internal.h"

int luabui_host_requested = 0;

/* The Lua state used by the host callbacks. Since contexts are per-window
 * but all share one Lua state, we store it as a global reference.
 * This is set during luabui_init_host_callbacks. */
static lua_State *g_host_L = NULL;

/* Forward: host callbacks */
static void host_request(void *data);
static void host_set_cursor(void *data, int cursor);
static char *host_clipboard_get(void *data);
static void host_clipboard_set(void *data, const char *text);
static void host_schedule(void *data, int ms, int timer);
static void host_emit(void *data, BKNode *node, int kind, void *ref, double a, double b);

/* ------------------------------------------------------------------ */
/* BKHost implementation                                              */
/* ------------------------------------------------------------------ */

static void host_request(void *data) {
  (void)data;
  luabui_host_requested = 1;
}

static void host_set_cursor(void *data, int cursor) {
  (void)data;
  (void)cursor;
  /* In a native window BK_window handles this automatically via the host.
   * For headless/custom: store the cursor and let the host query it. */
}

static char *host_clipboard_get(void *data) {
  (void)data;
  return NULL;
}

static void host_clipboard_set(void *data, const char *text) {
  (void)data;
  (void)text;
}

static void host_schedule(void *data, int ms, int timer_id) {
  (void)data;
  /* For native windows, BK_window handles timers automatically.
   * For headless/embedded: queue the timer and dispatch from pump loop. */
  if (BK_window_supported()) {
    /* Native windows: schedule a one-shot timer via BK_window_timer.
     * We need a BKWindow handle here - this callback comes from the context.
     * In native mode the window host handles this through the BKHost interface
     * that the native window always installs on top of our host. */
  }
}

/* Call debug.traceback on the error at the top of the stack, write to stderr,
 * then pop the error and any temporaries. Leaves the stack one item lighter. */
static void report_error(lua_State *L) {
  const char *err = lua_tostring(L, -1);
  if (!err) err = "(error without message)";
  fprintf(stderr, "bui: callback error: %s\n", err);
  lua_pop(L, 1);
}

static void host_emit(void *data, BKNode *node, int kind, void *ref, double a, double b) {
  lua_State *L = g_host_L;
  if (!L) return;
  int top = lua_gettop(L);

  if (kind == BK_EV_CALL && ref) {
    int func_ref = (int)(intptr_t)ref;
    lua_rawgeti(L, LUA_REGISTRYINDEX, func_ref);
    if (!lua_isfunction(L, -1)) {
      lua_settop(L, top);
      return;
    }
    if (lua_pcall(L, 0, 0, 0) != LUA_OK) {
      report_error(L);
    }
    lua_settop(L, top);
    return;
  }

  if (!node) return;
  lua_getfield(L, LUA_REGISTRYINDEX, LUABUI_NODE_MAP);
  lua_pushlightuserdata(L, node);
  lua_rawget(L, -2);
  if (lua_isnil(L, -1)) { lua_settop(L, top); return; }
  if (lua_getuservalue(L, -1) == LUA_TNIL) { lua_settop(L, top); return; }
  lua_pushinteger(L, kind);
  lua_rawget(L, -2);

  if (lua_isfunction(L, -1)) {
    lua_pushvalue(L, -3);
    lua_pushnumber(L, a);
    lua_pushnumber(L, b);
    if (lua_pcall(L, 3, 0, 0) != LUA_OK) {
      report_error(L);
    }
  }
  lua_settop(L, top);
}

/* ------------------------------------------------------------------ */
/* Initialization                                                     */
/* ------------------------------------------------------------------ */

void luabui_init_host_callbacks(lua_State *L, BKContext *ctx) {
  g_host_L = L;

  BKHost host;
  memset(&host, 0, sizeof(host));
  host.data = L;
  host.request       = host_request;
  host.set_cursor    = host_set_cursor;
  host.clipboard_get = host_clipboard_get;
  host.clipboard_set = host_clipboard_set;
  host.schedule      = host_schedule;
  host.emit          = host_emit;
  BK_context_set_host(ctx, &host);
}

/* ------------------------------------------------------------------ */
/* Context methods                                                    */
/* ------------------------------------------------------------------ */

static int l_ctx_set_header(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  if (!cud->ctx) return luaL_error(L, "context was freed");
  luabui_Node *ud = CHECK_NODE(L, 2);
  BK_context_set_header(cud->ctx, ud->node);
  return 0;
}

static int l_ctx_root(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  if (!cud->ctx) return 0;
  BKNode *root = BK_context_root(cud->ctx);
  if (!root) return 0;
  if (luabui_lookup_node(L, root)) return 1;
  /* Create new wrapper */
  luabui_Node *ud = (luabui_Node *)lua_newuserdata(L, sizeof(luabui_Node));
  ud->node = root;
  luaL_getmetatable(L, LUABUI_NODE_MT);
  lua_setmetatable(L, -2);
  lua_newtable(L);
  lua_setuservalue(L, -2);
  luabui_map_node(L, root, -1);
  return 1;
}

static int l_ctx_render(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  if (!cud->ctx) return 0;
  int w = (int)luaL_checkinteger(L, 2);
  int h = (int)luaL_checkinteger(L, 3);
  int full = (int)luaL_optinteger(L, 4, 0);

  /* Create a temporary raster */
  BuiRaster *r = BUI_raster_create(w, h);
  if (!r) { lua_pushnil(L); return 1; }

  int regions = BK_render(cud->ctx, r, w, h, full);

  /* Return pixel data as a Lua string (BGRA premultiplied) */
  lua_pushlstring(L, (const char *)BUI_raster_pixels(r), (size_t)w * h * 4);
  lua_pushinteger(L, regions);
  lua_pushinteger(L, w);
  lua_pushinteger(L, h);

  BUI_raster_free(r);
  return 4;
}

static int l_ctx_needs_render(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  lua_pushboolean(L, cud->ctx ? BK_needs_render(cud->ctx) : 0);
  return 1;
}

/* Input forwarding */

static int l_ctx_press(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  if (!cud->ctx) return 0;
  int x = (int)luaL_checkinteger(L, 2);
  int y = (int)luaL_checkinteger(L, 3);
  int mods = (int)luaL_optinteger(L, 4, 0);
  lua_pushboolean(L, BK_press(cud->ctx, x, y, mods));
  return 1;
}

static int l_ctx_release(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  if (!cud->ctx) return 0;
  int x = (int)luaL_checkinteger(L, 2);
  int y = (int)luaL_checkinteger(L, 3);
  int mods = (int)luaL_optinteger(L, 4, 0);
  lua_pushboolean(L, BK_release(cud->ctx, x, y, mods));
  return 1;
}

static int l_ctx_motion(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  if (!cud->ctx) return 0;
  int x = (int)luaL_checkinteger(L, 2);
  int y = (int)luaL_checkinteger(L, 3);
  int mods = (int)luaL_optinteger(L, 4, 0);
  lua_pushboolean(L, BK_motion(cud->ctx, x, y, mods));
  return 1;
}

static int l_ctx_wheel(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  if (!cud->ctx) return 0;
  int x = (int)luaL_checkinteger(L, 2);
  int y = (int)luaL_checkinteger(L, 3);
  int delta = (int)luaL_checkinteger(L, 4);
  int mods = (int)luaL_optinteger(L, 5, 0);
  lua_pushboolean(L, BK_wheel(cud->ctx, x, y, delta, mods));
  return 1;
}

static int l_ctx_scroll(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  if (!cud->ctx) return 0;
  int x = (int)luaL_checkinteger(L, 2);
  int y = (int)luaL_checkinteger(L, 3);
  float dx = (float)luaL_checknumber(L, 4);
  float dy = (float)luaL_checknumber(L, 5);
  int mods = (int)luaL_optinteger(L, 6, 0);
  lua_pushboolean(L, BK_scroll(cud->ctx, x, y, dx, dy, mods));
  return 1;
}

static int l_ctx_key(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  if (!cud->ctx) return 0;
  const char *text = luaL_checkstring(L, 2);
  const char *keysym = luaL_optstring(L, 3, "");
  int mods = (int)luaL_optinteger(L, 4, 0);
  lua_pushboolean(L, BK_key(cud->ctx, text, keysym, mods));
  return 1;
}

static int l_ctx_key_release(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  if (!cud->ctx) return 0;
  const char *keysym = luaL_checkstring(L, 2);
  int mods = (int)luaL_optinteger(L, 3, 0);
  lua_pushboolean(L, BK_key_release(cud->ctx, keysym, mods));
  return 1;
}

static int l_ctx_button(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  if (!cud->ctx) return 0;
  int x = (int)luaL_checkinteger(L, 2);
  int y = (int)luaL_checkinteger(L, 3);
  int button = (int)luaL_checkinteger(L, 4);
  int release = (int)luaL_checkinteger(L, 5);
  int mods = (int)luaL_optinteger(L, 6, 0);
  lua_pushboolean(L, BK_button(cud->ctx, x, y, button, release, mods));
  return 1;
}

static int l_ctx_context_menu(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  if (!cud->ctx) return 0;
  int x = (int)luaL_checkinteger(L, 2);
  int y = (int)luaL_checkinteger(L, 3);
  int mods = (int)luaL_optinteger(L, 4, 0);
  lua_pushboolean(L, BK_context_menu(cud->ctx, x, y, mods));
  return 1;
}

static int l_ctx_timer(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  if (cud->ctx) BK_timer(cud->ctx, (int)luaL_checkinteger(L, 2));
  return 0;
}

/* Registry key for the bind-refs table: combo string -> {ref, prev_ref} */
#define LUABUI_BIND_REFS "bui._bind_refs"

static int l_ctx_bind(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  if (!cud->ctx) return 0;
  const char *combo = luaL_checkstring(L, 2);
  luaL_checktype(L, 3, LUA_TFUNCTION);

  /* Unref any previous bind for this combo */
  lua_getfield(L, LUA_REGISTRYINDEX, LUABUI_BIND_REFS);
  if (lua_isnil(L, -1)) {
    lua_pop(L, 1);
    lua_newtable(L);
    lua_pushvalue(L, -1);
    lua_setfield(L, LUA_REGISTRYINDEX, LUABUI_BIND_REFS);
  }
  lua_pushstring(L, combo);
  lua_rawget(L, -2);
  if (!lua_isnil(L, -1)) {
    int old_ref = (int)lua_tointeger(L, -1);
    luaL_unref(L, LUA_REGISTRYINDEX, old_ref);
  }
  lua_pop(L, 1); /* pop old ref value or nil */

  /* Store new ref */
  lua_pushvalue(L, 3); /* copy function for luaL_ref */
  int ref = luaL_ref(L, LUA_REGISTRYINDEX);
  lua_pushstring(L, combo);
  lua_pushinteger(L, ref);
  lua_rawset(L, -3); /* bind_refs[combo] = ref */
  lua_pop(L, 1); /* bind_refs table */

  BK_bind(cud->ctx, combo, (void *)(intptr_t)ref);
  return 0;
}

static int l_ctx_close_popups(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  if (cud->ctx) BK_close_popups(cud->ctx);
  return 0;
}

static int l_ctx_focus(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  if (!cud->ctx) return 0;
  BKNode *n = BK_focus(cud->ctx);
  if (!n) return 0;
  if (luabui_lookup_node(L, n)) return 1;
  luabui_Node *ud = (luabui_Node *)lua_newuserdata(L, sizeof(luabui_Node));
  ud->node = n;
  luaL_getmetatable(L, LUABUI_NODE_MT);
  lua_setmetatable(L, -2);
  lua_newtable(L);
  lua_setuservalue(L, -2);
  luabui_map_node(L, n, -1);
  return 1;
}

static int l_ctx_hover(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  if (!cud->ctx) return 0;
  BKNode *n = BK_hover(cud->ctx);
  if (!n) return 0;
  if (luabui_lookup_node(L, n)) return 1;
  luabui_Node *ud = (luabui_Node *)lua_newuserdata(L, sizeof(luabui_Node));
  ud->node = n;
  luaL_getmetatable(L, LUABUI_NODE_MT);
  lua_setmetatable(L, -2);
  lua_newtable(L);
  lua_setuservalue(L, -2);
  luabui_map_node(L, n, -1);
  return 1;
}

static int l_ctx_cursor(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  lua_pushinteger(L, cud->ctx ? BK_cursor(cud->ctx) : 0);
  return 1;
}

static int l_ctx_focus_node(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  if (!cud->ctx) return 0;
  if (lua_isnil(L, 2)) {
    BK_focus_node(cud->ctx, NULL);
  } else {
    luabui_Node *ud = CHECK_NODE(L, 2);
    BK_focus_node(cud->ctx, ud->node);
  }
  return 0;
}

static int l_ctx_popup_owner(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  if (!cud->ctx) return 0;
  int i = (int)luaL_checkinteger(L, 2);
  if (i < 0) i = BK_popup_count(cud->ctx) + i;
  BKNode *n = BK_popup_owner(cud->ctx, i);
  if (!n) return 0;
  if (luabui_lookup_node(L, n)) return 1;
  luabui_Node *ud = (luabui_Node *)lua_newuserdata(L, sizeof(luabui_Node));
  ud->node = n; luaL_getmetatable(L, LUABUI_NODE_MT); lua_setmetatable(L, -2);
  lua_newtable(L); lua_setuservalue(L, -2); luabui_map_node(L, n, -1);
  return 1;
}

static int l_ctx_menu_rows(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  if (!cud->ctx) return 0;
  int i = (int)luaL_checkinteger(L, 2);
  if (i < 0) i = BK_popup_count(cud->ctx) + i;
  BKNode *p = BK_popup(cud->ctx, i);
  if (!p) { lua_pushnil(L); return 1; }
  int n = BK_menu_row_count(p);
  lua_createtable(L, n, 0);
  for (int k = 0; k < n; k++) {
    int r[4];
    BK_menu_row_rect(p, k, r);
    lua_createtable(L, 0, 5);
    lua_pushstring(L, BK_menu_row_text(p, k)); lua_setfield(L, -2, "text");
    lua_pushinteger(L, r[0]); lua_setfield(L, -2, "x");
    lua_pushinteger(L, r[1]); lua_setfield(L, -2, "y");
    lua_pushinteger(L, r[2]); lua_setfield(L, -2, "w");
    lua_pushinteger(L, r[3]); lua_setfield(L, -2, "h");
    lua_rawseti(L, -2, k + 1);
  }
  return 1;
}

static int l_ctx_picker_part(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  if (!cud->ctx) return 0;
  const char *name = luaL_checkstring(L, 2);
  int i = (int)luaL_optinteger(L, 3, -1);
  if (i < 0) i = BK_popup_count(cud->ctx) + i;
  BKNode *p = BK_popup(cud->ctx, i);
  if (!p) return 0;
  BKNode *n = BK_picker_part(p, name);
  if (!n) return 0;
  if (luabui_lookup_node(L, n)) return 1;
  luabui_Node *ud = (luabui_Node *)lua_newuserdata(L, sizeof(luabui_Node));
  ud->node = n; luaL_getmetatable(L, LUABUI_NODE_MT); lua_setmetatable(L, -2);
  lua_newtable(L); lua_setuservalue(L, -2); luabui_map_node(L, n, -1);
  return 1;
}

static int l_ctx_modal(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  lua_pushboolean(L, cud->ctx ? BK_modal_active(cud->ctx) : 0);
  return 1;
}

static int l_ctx_tooltip(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  lua_pushboolean(L, cud->ctx ? BK_tooltip_visible(cud->ctx) : 0);
  return 1;
}

static int l_ctx_undo(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  lua_pushboolean(L, cud->ctx ? BK_undo(cud->ctx) : 0);
  return 1;
}

static int l_ctx_redo(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  lua_pushboolean(L, cud->ctx ? BK_redo(cud->ctx) : 0);
  return 1;
}

static int l_ctx_undo_depth(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  int redo = 0;
  int undo = cud->ctx ? BK_undo_depth(cud->ctx, &redo) : 0;
  lua_pushinteger(L, undo);
  lua_pushinteger(L, redo);
  return 2;
}

static int l_ctx_animating(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  lua_pushboolean(L, cud->ctx ? BK_animating(cud->ctx) : 0);
  return 1;
}

static int l_ctx_clipboard_get(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  if (!cud->ctx) return 0;
  char *text = BK_clipboard_get(cud->ctx);
  lua_pushstring(L, text ? text : "");
  free(text);
  return 1;
}

static int l_ctx_clipboard_set(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  if (cud->ctx) BK_clipboard_set(cud->ctx, luaL_checkstring(L, 2));
  return 0;
}

static int l_ctx_set_pointer(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  if (cud->ctx) BK_set_pointer(cud->ctx, (int)luaL_checkinteger(L, 2), (int)luaL_checkinteger(L, 3));
  return 0;
}

static int l_ctx_popup_count(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  lua_pushinteger(L, cud->ctx ? BK_popup_count(cud->ctx) : 0);
  return 1;
}

static int l_ctx_popup(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  if (!cud->ctx) return 0;
  int i = (int)luaL_checkinteger(L, 2);
  if (i < 0) i = BK_popup_count(cud->ctx) + i;
  BKNode *n = BK_popup(cud->ctx, i);
  if (!n) return 0;
  if (luabui_lookup_node(L, n)) return 1;
  luabui_Node *ud = (luabui_Node *)lua_newuserdata(L, sizeof(luabui_Node));
  ud->node = n;
  luaL_getmetatable(L, LUABUI_NODE_MT);
  lua_setmetatable(L, -2);
  lua_newtable(L);
  lua_setuservalue(L, -2);
  luabui_map_node(L, n, -1);
  return 1;
}

static int l_ctx_open_menu(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  if (!cud->ctx) return 0;
  /* owner node (optional), items table, title (optional), at_x/at_y (optional) */
  BKNode *owner = NULL;
  if (!lua_isnil(L, 2)) {
    luabui_Node *oud = CHECK_NODE(L, 2);
    if (oud->node) owner = oud->node;
  }
  luaL_checktype(L, 3, LUA_TTABLE);
  int n = (int)lua_rawlen(L, 3);
  BKMenuSpec *specs = (BKMenuSpec *)lua_newuserdata(L, n * sizeof(BKMenuSpec));
  memset(specs, 0, n * sizeof(BKMenuSpec));
  for (int i = 0; i < n; i++) {
    lua_rawgeti(L, 3, i + 1);
    if (lua_istable(L, -1)) {
      lua_getfield(L, -1, "text"); specs[i].text = lua_tostring(L, -1); lua_pop(L, 1);
      lua_getfield(L, -1, "icon"); specs[i].icon = (int)luaL_optinteger(L, -1, 0); lua_pop(L, 1);
      lua_getfield(L, -1, "enabled"); specs[i].enabled = (int)luaL_optinteger(L, -1, 1); lua_pop(L, 1);
      lua_getfield(L, -1, "checked"); specs[i].checked = (int)luaL_optinteger(L, -1, -1); lua_pop(L, 1);
    }
    lua_pop(L, 1);
  }
  const char *title = luaL_optstring(L, 4, NULL);
  int ax = (int)luaL_optinteger(L, 5, -100000);
  int ay = (int)luaL_optinteger(L, 6, -100000);
  BK_open_menu(cud->ctx, owner, specs, n, title, ax, ay);
  return 0;
}

static int l_ctx_open_pie(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  if (!cud->ctx) return 0;
  luaL_checktype(L, 2, LUA_TTABLE);
  int n = (int)lua_rawlen(L, 2);
  BKMenuSpec *specs = (BKMenuSpec *)lua_newuserdata(L, n * sizeof(BKMenuSpec));
  memset(specs, 0, n * sizeof(BKMenuSpec));
  for (int i = 0; i < n; i++) {
    lua_rawgeti(L, 2, i + 1);
    if (lua_istable(L, -1)) {
      lua_getfield(L, -1, "text"); specs[i].text = lua_tostring(L, -1); lua_pop(L, 1);
      lua_getfield(L, -1, "icon"); specs[i].icon = (int)luaL_optinteger(L, -1, 0); lua_pop(L, 1);
    }
    lua_pop(L, 1);
  }
  const char *title = luaL_optstring(L, 3, NULL);
  BK_open_pie(cud->ctx, specs, n, title);
  return 0;
}

/* Nullify all node userdata pointers in the node map. Called before freeing
 * a context so that node __gc (which runs later) sees ud->node == NULL and
 * does not try to access freed C memory. */
static void nullify_node_map(lua_State *L) {
  lua_getfield(L, LUA_REGISTRYINDEX, LUABUI_NODE_MAP);
  /* Iterate the weak-keyed node map: key = lightuserdata(BKNode*), value = userdata */
  lua_pushnil(L);
  while (lua_next(L, -2)) {
    /* stack: map, key, value */
    if (lua_isuserdata(L, -1) && luaL_testudata(L, -1, LUABUI_NODE_MT)) {
      luabui_Node *ud = (luabui_Node *)lua_touserdata(L, -1);
      ud->node = NULL;
    }
    lua_pop(L, 1); /* pop value */
  }
  lua_pop(L, 1); /* pop map */
}

/* Clear all callback refs from the context */
static void unref_bind_refs(lua_State *L) {
  lua_getfield(L, LUA_REGISTRYINDEX, LUABUI_BIND_REFS);
  if (lua_istable(L, -1)) {
    lua_pushnil(L);
    while (lua_next(L, -2)) {
      /* key (combo), value (ref) */
      int ref = (int)lua_tointeger(L, -1);
      luaL_unref(L, LUA_REGISTRYINDEX, ref);
      lua_pop(L, 1); /* pop value, keep key for next */
    }
    lua_pushnil(L);
    lua_setfield(L, LUA_REGISTRYINDEX, LUABUI_BIND_REFS);
  }
  lua_pop(L, 1);
}

static int l_ctx_free(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  if (cud->ctx) {
    unref_bind_refs(L);
    nullify_node_map(L);
    BK_context_free(cud->ctx);
    cud->ctx = NULL;
  }
  return 0;
}

static int l_ctx_gc(lua_State *L) {
  luabui_Context *cud = (luabui_Context *)luaL_checkudata(L, 1, LUABUI_CONTEXT_MT);
  if (cud->ctx) {
    unref_bind_refs(L);
    nullify_node_map(L);
    BK_context_free(cud->ctx);
    cud->ctx = NULL;
  }
  return 0;
}

static int l_ctx_tostring(lua_State *L) {
  luabui_Context *cud = (luabui_Context *)luaL_checkudata(L, 1, LUABUI_CONTEXT_MT);
  lua_pushfstring(L, "bui.Context (%p)", (void *)(cud->ctx));
  return 1;
}

/* ------------------------------------------------------------------ */
/* Registration                                                       */
/* ------------------------------------------------------------------ */

static const luaL_Reg ctx_methods[] = {
  {"set_header",    l_ctx_set_header},
  {"root",          l_ctx_root},
  {"render",        l_ctx_render},
  {"needs_render",  l_ctx_needs_render},
  {"press",         l_ctx_press},
  {"release",       l_ctx_release},
  {"motion",        l_ctx_motion},
  {"wheel",         l_ctx_wheel},
  {"scroll",        l_ctx_scroll},
  {"key",           l_ctx_key},
  {"key_release",   l_ctx_key_release},
  {"button",        l_ctx_button},
  {"context_menu",  l_ctx_context_menu},
  {"timer",         l_ctx_timer},
  {"bind",          l_ctx_bind},
  {"close_popups",  l_ctx_close_popups},
  {"focus",         l_ctx_focus},
  {"focus_node",    l_ctx_focus_node},
  {"hover",         l_ctx_hover},
  {"cursor",        l_ctx_cursor},
  {"set_pointer",   l_ctx_set_pointer},
  {"modal",         l_ctx_modal},
  {"tooltip",       l_ctx_tooltip},
  {"undo",          l_ctx_undo},
  {"redo",          l_ctx_redo},
  {"undo_depth",    l_ctx_undo_depth},
  {"animating",     l_ctx_animating},
  {"clipboard_get", l_ctx_clipboard_get},
  {"clipboard_set", l_ctx_clipboard_set},
  {"popup_count",   l_ctx_popup_count},
  {"popup",         l_ctx_popup},
  {"popup_owner",   l_ctx_popup_owner},
  {"menu_rows",     l_ctx_menu_rows},
  {"picker_part",   l_ctx_picker_part},
  {"open_menu",     l_ctx_open_menu},
  {"open_pie",      l_ctx_open_pie},
  {"free",          l_ctx_free},
  {"__gc",          l_ctx_gc},
  {"__tostring",    l_ctx_tostring},
  {NULL, NULL}
};

void luabui_register_context(lua_State *L) {
  luaL_newmetatable(L, LUABUI_CONTEXT_MT);
  luaL_setfuncs(L, ctx_methods, 0);
  lua_pushvalue(L, -1);
  lua_setfield(L, -2, "__index");
  lua_pop(L, 1);
}
