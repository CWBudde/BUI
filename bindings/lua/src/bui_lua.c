/* BUI Lua binding: module entry point and constant registration.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#include "bui_internal.h"

/* Numeric property keys (for node:get() dispatch) */
static const char *g_numeric_keys[] = {
  "value", "min", "max", "step", "precision", "padding", "margin",
  "width", "height", "reqwidth", "reqheight", "fill", "expand",
  "side", "padx0", "padx1", "pady0", "pady1", "ipadx", "ipady",
  "row", "column", "rowspan", "columnspan", "sticky",
  "relx", "rely", "relwidth", "relheight",
  "anchor", "scale", "factor", "progress", "value_min", "value_max",
  "mode", "multiple", "hidden", "sort", "state", "count",
  "icon", "checked", "enabled", "selected", "open",
  "rows", "cols", "wrap", "readonly", "password",
  "x", "y", "w", "h",
  "html", "editable",
  NULL
};

int luabui_is_numeric_key(const char *key) {
  for (int i = 0; g_numeric_keys[i]; i++) {
    if (strcmp(key, g_numeric_keys[i]) == 0) return 1;
  }
  return 0;
}

/* ------------------------------------------------------------------ */
/* Global functions                                                   */
/* ------------------------------------------------------------------ */

static int l_run(lua_State *L) {
  (void)L;
  BK_window_run();
  return 0;
}

static int l_pump(lua_State *L) {
  int open = BK_window_pump();
  lua_pushinteger(L, open);
  return 1;
}

static int l_set_scale(lua_State *L) {
  BK_set_scale((float)luaL_checknumber(L, 1));
  return 0;
}

static int l_set_theme(lua_State *L) {
  int ok = BK_set_theme(luaL_checkstring(L, 1));
  lua_pushinteger(L, ok);
  return 1;
}

static int l_theme_name(lua_State *L) {
  lua_pushstring(L, BK_theme_name());
  return 1;
}

static int l_theme_color(lua_State *L) {
  float rgba[4];
  if (BK_theme_color(luaL_checkstring(L, 1), rgba)) {
    lua_pushnumber(L, rgba[0]);
    lua_pushnumber(L, rgba[1]);
    lua_pushnumber(L, rgba[2]);
    lua_pushnumber(L, rgba[3]);
    return 4;
  }
  return 0;
}

static int l_unit(lua_State *L) {
  lua_pushinteger(L, BK_unit());
  return 1;
}

static int l_version(lua_State *L) {
  int major, minor, patch;
  bui_get_version(&major, &minor, &patch);
  lua_pushinteger(L, major);
  lua_pushinteger(L, minor);
  lua_pushinteger(L, patch);
  return 3;
}

static int l_eval(lua_State *L) {
  double result;
  if (BK_eval_number(luaL_checkstring(L, 1), &result)) {
    lua_pushnumber(L, result);
    return 1;
  }
  return 0;
}

/* ------------------------------------------------------------------ */
/* Factory functions                                                  */
/* ------------------------------------------------------------------ */

static int l_new_node(lua_State *L) {
  const char *type = luaL_checkstring(L, 1);
  BKNode *node = BK_node_new(type);
  if (!node) { lua_pushnil(L); return 1; }

  luabui_Node *ud = (luabui_Node *)lua_newuserdata(L, sizeof(luabui_Node));
  ud->node = node;
  luaL_getmetatable(L, LUABUI_NODE_MT);
  lua_setmetatable(L, -2);

  /* Create uservalue table for event callbacks */
  lua_newtable(L);
  lua_setuservalue(L, -2);

  luabui_map_node(L, node, -1);
  return 1;
}

static int l_new_context(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  if (!ud->node) { lua_pushnil(L); return 1; }

  BKContext *ctx = BK_context_new(ud->node);
  if (!ctx) { lua_pushnil(L); return 1; }

  luabui_Context *cud = (luabui_Context *)lua_newuserdata(L, sizeof(luabui_Context));
  cud->ctx = ctx;
  luaL_getmetatable(L, LUABUI_CONTEXT_MT);
  lua_setmetatable(L, -2);

  /* Set up host callbacks with Lua state as data */
  luabui_init_host_callbacks(L, ctx);
  return 1;
}

static int l_new_window(lua_State *L) {
  luabui_Context *cud = CHECK_CTX(L, 1);
  if (!cud->ctx) { lua_pushnil(L); return 1; }

  const char *title = luaL_checkstring(L, 2);
  int w = (int)luaL_checkinteger(L, 3);
  int h = (int)luaL_checkinteger(L, 4);

  if (!BK_window_supported()) { lua_pushnil(L); return 1; }

  BKWindow *win = BK_window_new(cud->ctx, title, w, h);
  if (!win) { lua_pushnil(L); return 1; }

  luabui_Window *wud = (luabui_Window *)lua_newuserdata(L, sizeof(luabui_Window));
  wud->win = win;
  luaL_getmetatable(L, LUABUI_WINDOW_MT);
  lua_setmetatable(L, -2);
  return 1;
}

static int l_new_treeitem(lua_State *L) {
  const char *label = luaL_checkstring(L, 1);
  int icon = (int)luaL_optinteger(L, 2, 0);

  BKTreeItem *item = BK_tree_item_new(label, icon);
  if (!item) { lua_pushnil(L); return 1; }

  luabui_TreeItem *tud = (luabui_TreeItem *)lua_newuserdata(L, sizeof(luabui_TreeItem));
  tud->item = item;
  luaL_getmetatable(L, LUABUI_TREEITEM_MT);
  lua_setmetatable(L, -2);
  luabui_map_treeitem(L, item, -1);
  return 1;
}

/* ------------------------------------------------------------------ */
/* Module entry point                                                 */
/* ------------------------------------------------------------------ */

static const struct luaL_Reg bui_module[] = {
  {"Node",      l_new_node},
  {"Context",   l_new_context},
  {"Window",    l_new_window},
  {"TreeItem",  l_new_treeitem},
  {"run",       l_run},
  {"pump",      l_pump},
  {"set_scale", l_set_scale},
  {"set_theme", l_set_theme},
  {"theme_name",l_theme_name},
  {"theme_color",l_theme_color},
  {"unit",      l_unit},
  {"version",   l_version},
  {"eval",      l_eval},
  {"image_decode", l_image_decode},
  {"image_load",   l_image_load},
  {"Video",        l_video_new},
  {NULL, NULL}
};

LUAMOD_API int luaopen_bui(lua_State *L) {
  /* Version check: refuse to load if the kit version doesn't match the header */
  int kit_ver = BK_version();
  int hdr_ver = BUI_KIT_VERSION;
  if (kit_ver != hdr_ver) {
    int kmaj = kit_ver / 10000, kmin = (kit_ver % 10000) / 100, kpat = kit_ver % 100;
    int hmaj = hdr_ver / 10000, hmin = (hdr_ver % 10000) / 100, hpat = hdr_ver % 100;
    lua_pushfstring(L, "bui: kit version mismatch (runtime %d.%d.%d, header %d.%d.%d). "
                       "Recompile the binding.\n"
                       "Expected \"%s\", linked \"%s\".",
                     kmaj, kmin, kpat, hmaj, hmin, hpat,
                     BUI_KIT_VERSION_STRING, BK_version_string());
    return lua_error(L);
  }

  /* Weak-keyed node map in registry */
  lua_newtable(L);
  lua_newtable(L);
  lua_pushstring(L, "k");
  lua_setfield(L, -2, "__mode");
  lua_setmetatable(L, -2);
  lua_setfield(L, LUA_REGISTRYINDEX, LUABUI_NODE_MAP);

  /* Weak-keyed tree item map in registry */
  lua_newtable(L);
  lua_newtable(L);
  lua_pushstring(L, "k");
  lua_setfield(L, -2, "__mode");
  lua_setmetatable(L, -2);
  lua_setfield(L, LUA_REGISTRYINDEX, LUABUI_TREEITEM_MAP);

  /* Register types */
  luabui_register_node(L);
  luabui_register_context(L);
  luabui_register_window(L);
  luabui_register_treeitem(L);
  luabui_register_imbuf(L);
  luabui_register_video(L);

  /* Module table */
  lua_newtable(L);
  luaL_setfuncs(L, bui_module, 0);

  /* Version constants */
  lua_pushstring(L, BUI_KIT_VERSION_STRING); lua_setfield(L, -2, "KIT_VERSION");
  {
    int vmaj, vmin, vpat;
    bui_get_version(&vmaj, &vmin, &vpat);
    char vbuf[32];
    int n = snprintf(vbuf, sizeof(vbuf), "%d.%d.%d", vmaj, vmin, vpat);
    lua_pushlstring(L, vbuf, (size_t)n);
    lua_setfield(L, -2, "_VERSION");
  }

  /* Event constants */
  lua_pushinteger(L, BK_EV_CHANGE); lua_setfield(L, -2, "EV_CHANGE");   lua_pushinteger(L, BK_EV_CLICK); lua_setfield(L, -2, "EV_CLICK");
  lua_pushinteger(L, BK_EV_CALL); lua_setfield(L, -2, "EV_CALL");     lua_pushinteger(L, BK_EV_RENAME); lua_setfield(L, -2, "EV_RENAME");
  lua_pushinteger(L, BK_EV_ACTIVATE); lua_setfield(L, -2, "EV_ACTIVATE"); lua_pushinteger(L, BK_EV_TOGGLE); lua_setfield(L, -2, "EV_TOGGLE");
  lua_pushinteger(L, BK_EV_BROWSE); lua_setfield(L, -2, "EV_BROWSE");   lua_pushinteger(L, BK_EV_PRESS); lua_setfield(L, -2, "EV_PRESS");
  lua_pushinteger(L, BK_EV_RELEASE); lua_setfield(L, -2, "EV_RELEASE");  lua_pushinteger(L, BK_EV_DOUBLE); lua_setfield(L, -2, "EV_DOUBLE");
  lua_pushinteger(L, BK_EV_MOTION); lua_setfield(L, -2, "EV_MOTION");   lua_pushinteger(L, BK_EV_ENTER); lua_setfield(L, -2, "EV_ENTER");
  lua_pushinteger(L, BK_EV_LEAVE); lua_setfield(L, -2, "EV_LEAVE");    lua_pushinteger(L, BK_EV_KEY); lua_setfield(L, -2, "EV_KEY");
  lua_pushinteger(L, BK_EV_WHEEL); lua_setfield(L, -2, "EV_WHEEL");    lua_pushinteger(L, BK_EV_SUBMIT); lua_setfield(L, -2, "EV_SUBMIT");
  lua_pushinteger(L, BK_EV_VIEW); lua_setfield(L, -2, "EV_VIEW");     lua_pushinteger(L, BK_EV_SCROLL); lua_setfield(L, -2, "EV_SCROLL");
  lua_pushinteger(L, BK_EV_CONFIGURE); lua_setfield(L, -2, "EV_CONFIGURE");lua_pushinteger(L, BK_EV_KEYRELEASE); lua_setfield(L, -2, "EV_KEYRELEASE");

  /* Cursor constants */
  lua_pushinteger(L, BK_CURSOR_DEFAULT); lua_setfield(L, -2, "CURSOR_DEFAULT");  lua_pushinteger(L, BK_CURSOR_EW); lua_setfield(L, -2, "CURSOR_EW");
  lua_pushinteger(L, BK_CURSOR_NS); lua_setfield(L, -2, "CURSOR_NS");       lua_pushinteger(L, BK_CURSOR_TEXT); lua_setfield(L, -2, "CURSOR_TEXT");
  lua_pushinteger(L, BK_CURSOR_CROSSHAIR); lua_setfield(L, -2, "CURSOR_CROSSHAIR");lua_pushinteger(L, BK_CURSOR_HAND); lua_setfield(L, -2, "CURSOR_HAND");
  lua_pushinteger(L, BK_CURSOR_MOVE); lua_setfield(L, -2, "CURSOR_MOVE");     lua_pushinteger(L, BK_CURSOR_NONE); lua_setfield(L, -2, "CURSOR_NONE");

  /* Geometry constants */
  lua_pushinteger(L, BK_GEOM_NONE); lua_setfield(L, -2, "GEOM_NONE");  lua_pushinteger(L, BK_GEOM_PACK); lua_setfield(L, -2, "GEOM_PACK");
  lua_pushinteger(L, BK_GEOM_GRID); lua_setfield(L, -2, "GEOM_GRID");  lua_pushinteger(L, BK_GEOM_PLACE); lua_setfield(L, -2, "GEOM_PLACE");

  /* Modifier bits */
  lua_pushinteger(L, BK_SHIFT); lua_setfield(L, -2, "MOD_SHIFT");  lua_pushinteger(L, BK_CTRL); lua_setfield(L, -2, "MOD_CTRL");
  lua_pushinteger(L, BK_ALT); lua_setfield(L, -2, "MOD_ALT");     lua_pushinteger(L, BK_BUTTON1); lua_setfield(L, -2, "MOD_BUTTON1");
  lua_pushinteger(L, BK_BUTTON2); lua_setfield(L, -2, "MOD_BUTTON2"); lua_pushinteger(L, BK_BUTTON3); lua_setfield(L, -2, "MOD_BUTTON3");

  /* Image formats */
  lua_pushinteger(L, BK_IMAGE_NONE); lua_setfield(L, -2, "IMAGE_NONE");  lua_pushinteger(L, BK_IMAGE_PNG); lua_setfield(L, -2, "IMAGE_PNG");
  lua_pushinteger(L, BK_IMAGE_JPEG); lua_setfield(L, -2, "IMAGE_JPEG");  lua_pushinteger(L, BK_IMAGE_WEBP); lua_setfield(L, -2, "IMAGE_WEBP");
  lua_pushinteger(L, BK_IMAGE_GIF); lua_setfield(L, -2, "IMAGE_GIF");   lua_pushinteger(L, BK_IMAGE_SVG); lua_setfield(L, -2, "IMAGE_SVG");
  lua_pushinteger(L, BK_IMAGE_VIDEO); lua_setfield(L, -2, "IMAGE_VIDEO");

  /* Object-fit */
  lua_pushinteger(L, BK_FIT_FILL); lua_setfield(L, -2, "FIT_FILL");    lua_pushinteger(L, BK_FIT_CONTAIN); lua_setfield(L, -2, "FIT_CONTAIN");
  lua_pushinteger(L, BK_FIT_COVER); lua_setfield(L, -2, "FIT_COVER");   lua_pushinteger(L, BK_FIT_NONE); lua_setfield(L, -2, "FIT_NONE");
  lua_pushinteger(L, BK_FIT_SCALE_DOWN); lua_setfield(L, -2, "FIT_SCALE_DOWN");

  /* Video states */
  lua_pushinteger(L, BK_VIDEO_LOADING); lua_setfield(L, -2, "VIDEO_LOADING"); lua_pushinteger(L, BK_VIDEO_PLAYING); lua_setfield(L, -2, "VIDEO_PLAYING");
  lua_pushinteger(L, BK_VIDEO_PAUSED); lua_setfield(L, -2, "VIDEO_PAUSED");  lua_pushinteger(L, BK_VIDEO_ENDED); lua_setfield(L, -2, "VIDEO_ENDED");
  lua_pushinteger(L, BK_VIDEO_ERROR); lua_setfield(L, -2, "VIDEO_ERROR");

  /* Canvas item types */
  lua_pushinteger(L, BK_ITEM_RECT); lua_setfield(L, -2, "ITEM_RECT");     lua_pushinteger(L, BK_ITEM_OVAL); lua_setfield(L, -2, "ITEM_OVAL");
  lua_pushinteger(L, BK_ITEM_LINE); lua_setfield(L, -2, "ITEM_LINE");     lua_pushinteger(L, BK_ITEM_POLYGON); lua_setfield(L, -2, "ITEM_POLYGON");
  lua_pushinteger(L, BK_ITEM_TEXT); lua_setfield(L, -2, "ITEM_TEXT");     lua_pushinteger(L, BK_ITEM_ARC); lua_setfield(L, -2, "ITEM_ARC");
  lua_pushinteger(L, BK_ITEM_IMAGE); lua_setfield(L, -2, "ITEM_IMAGE");    lua_pushinteger(L, BK_ITEM_ROUNDRECT); lua_setfield(L, -2, "ITEM_ROUNDRECT");

  /* Widget draw ids */
  lua_pushinteger(L, BK_WIDGET_BUTTON); lua_setfield(L, -2, "WIDGET_BUTTON");  lua_pushinteger(L, BK_WIDGET_TOGGLE); lua_setfield(L, -2, "WIDGET_TOGGLE");
  lua_pushinteger(L, BK_WIDGET_CHECKBOX); lua_setfield(L, -2, "WIDGET_CHECKBOX");lua_pushinteger(L, BK_WIDGET_RADIO); lua_setfield(L, -2, "WIDGET_RADIO");
  lua_pushinteger(L, BK_WIDGET_SLIDER); lua_setfield(L, -2, "WIDGET_SLIDER");  lua_pushinteger(L, BK_WIDGET_TEXT); lua_setfield(L, -2, "WIDGET_TEXT");
  lua_pushinteger(L, BK_WIDGET_LABEL); lua_setfield(L, -2, "WIDGET_LABEL");   lua_pushinteger(L, BK_WIDGET_SEPARATOR); lua_setfield(L, -2, "WIDGET_SEPARATOR");

  /* Widget states */
  lua_pushinteger(L, BK_WS_HOVER); lua_setfield(L, -2, "WS_HOVER");     lua_pushinteger(L, BK_WS_PRESSED); lua_setfield(L, -2, "WS_PRESSED");
  lua_pushinteger(L, BK_WS_SELECTED); lua_setfield(L, -2, "WS_SELECTED");  lua_pushinteger(L, BK_WS_DISABLED); lua_setfield(L, -2, "WS_DISABLED");
  lua_pushinteger(L, BK_WS_EDITING); lua_setfield(L, -2, "WS_EDITING");   lua_pushinteger(L, BK_WS_OPEN); lua_setfield(L, -2, "WS_OPEN");

  return 1;
}
