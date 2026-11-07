/* BUI Lua binding: BKNode userdata and metatable.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#include "bui_internal.h"

/* ------------------------------------------------------------------ */
/* Node map implementation (weak-keyed registry table)                */
/* ------------------------------------------------------------------ */

void luabui_map_node(lua_State *L, BKNode *node, int ud_idx) {
  lua_getfield(L, LUA_REGISTRYINDEX, LUABUI_NODE_MAP);
  lua_pushlightuserdata(L, node);
  lua_pushvalue(L, ud_idx < 0 ? ud_idx - 1 : ud_idx);
  lua_rawset(L, -3);
  lua_pop(L, 1);
}

void *luabui_lookup_node(lua_State *L, BKNode *node) {
  lua_getfield(L, LUA_REGISTRYINDEX, LUABUI_NODE_MAP);
  lua_pushlightuserdata(L, node);
  lua_rawget(L, -2);
  void *ud = lua_touserdata(L, -1);
  lua_pop(L, 2);
  return ud;
}

void luabui_unmap_node(lua_State *L, BKNode *node) {
  lua_getfield(L, LUA_REGISTRYINDEX, LUABUI_NODE_MAP);
  lua_pushlightuserdata(L, node);
  lua_pushnil(L);
  lua_rawset(L, -3);
  lua_pop(L, 1);
}

/* ------------------------------------------------------------------ */
/* Helper: push a four-value color from float[4]                      */
/* ------------------------------------------------------------------ */
static void push_color(lua_State *L, const float rgba[4]) {
  lua_pushnumber(L, rgba[0]);
  lua_pushnumber(L, rgba[1]);
  lua_pushnumber(L, rgba[2]);
  lua_pushnumber(L, rgba[3]);
}

/* ------------------------------------------------------------------ */
/* Node methods                                                       */
/* ------------------------------------------------------------------ */

static int l_node_add(lua_State *L) {
  luabui_Node *parent = CHECK_NODE(L, 1);
  luabui_Node *child  = CHECK_NODE(L, 2);
  int index = (int)luaL_optinteger(L, 3, -1);
  if (!parent->node || !child->node)
    return luaL_error(L, "attempt to use freed node");
  BK_node_add(parent->node, child->node, index);
  return 0;
}

static int l_node_remove(lua_State *L) {
  luabui_Node *parent = CHECK_NODE(L, 1);
  luabui_Node *child  = CHECK_NODE(L, 2);
  if (!parent->node || !child->node)
    return luaL_error(L, "attempt to use freed node");
  BK_node_remove(parent->node, child->node);
  /* Also invalidate the child Lua reference */
  luabui_unmap_node(L, child->node);
  child->node = NULL;
  return 0;
}

static int l_node_child(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  if (!ud->node) return 0;
  int i = (int)luaL_checkinteger(L, 2);
  BKNode *child = BK_node_child(ud->node, i);
  if (!child) return 0;
  /* Return the Lua userdata for this child, creating one if needed */
  void *existing = luabui_lookup_node(L, child);
  if (existing) {
    lua_pushvalue(L, (int)(intptr_t)existing);
    return 1;
  }
  /* Create a new wrapper (the node already exists in the C tree) */
  luabui_Node *cud = (luabui_Node *)lua_newuserdata(L, sizeof(luabui_Node));
  cud->node = child;
  luaL_getmetatable(L, LUABUI_NODE_MT);
  lua_setmetatable(L, -2);
  lua_newtable(L);
  lua_setuservalue(L, -2);
  luabui_map_node(L, child, -1);
  return 1;
}

static int l_node_children(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  if (!ud->node) return 0;
  int n = BK_node_child_count(ud->node);
  lua_createtable(L, n, 0);
  for (int i = 0; i < n; i++) {
    BKNode *child = BK_node_child(ud->node, i);
    void *existing = luabui_lookup_node(L, child);
    if (existing) {
      lua_pushvalue(L, (int)(intptr_t)existing);
    } else {
      luabui_Node *cud = (luabui_Node *)lua_newuserdata(L, sizeof(luabui_Node));
      cud->node = child;
      luaL_getmetatable(L, LUABUI_NODE_MT);
      lua_setmetatable(L, -2);
      lua_newtable(L);
      lua_setuservalue(L, -2);
      luabui_map_node(L, child, -1);
    }
    lua_rawseti(L, -2, i + 1);
  }
  return 1;
}

static int l_node_child_count(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  lua_pushinteger(L, ud->node ? BK_node_child_count(ud->node) : 0);
  return 1;
}

static int l_node_parent(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  if (!ud->node) return 0;
  BKNode *parent = BK_node_parent(ud->node);
  if (!parent) return 0;
  void *existing = luabui_lookup_node(L, parent);
  if (existing) { lua_pushvalue(L, (int)(intptr_t)existing); return 1; }
  luabui_Node *pud = (luabui_Node *)lua_newuserdata(L, sizeof(luabui_Node));
  pud->node = parent;
  luaL_getmetatable(L, LUABUI_NODE_MT);
  lua_setmetatable(L, -2);
  lua_newtable(L);
  lua_setuservalue(L, -2);
  luabui_map_node(L, parent, -1);
  return 1;
}

static int l_node_type(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  lua_pushstring(L, ud->node ? BK_node_type(ud->node) : "(freed)");
  return 1;
}

static int l_node_rect(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  if (!ud->node) return 0;
  int r[4];
  BK_node_rect(ud->node, r);
  lua_pushinteger(L, r[0]); lua_pushinteger(L, r[1]);
  lua_pushinteger(L, r[2]); lua_pushinteger(L, r[3]);
  return 4;
}

static int l_node_natural(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  if (!ud->node) return 0;
  int s[2];
  BK_node_natural(ud->node, s);
  lua_pushinteger(L, s[0]); lua_pushinteger(L, s[1]);
  return 2;
}

static int l_node_set(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  if (!ud->node) return luaL_error(L, "attempt to use freed node");
  const char *key = luaL_checkstring(L, 2);
  int t = lua_type(L, 3);

  switch (t) {
    case LUA_TSTRING: {
      BK_set_str(ud->node, key, lua_tostring(L, 3));
      break;
    }
    case LUA_TNUMBER: {
      BK_set_num(ud->node, key, lua_tonumber(L, 3));
      break;
    }
    case LUA_TBOOLEAN: {
      BK_set_num(ud->node, key, lua_toboolean(L, 3) ? 1.0 : 0.0);
      break;
    }
    case LUA_TTABLE: {
      /* Heuristic: if table is array of 4 numbers, treat as color */
      int len = (int)lua_rawlen(L, 3);
      if (len == 4) {
        lua_rawgeti(L, 3, 1);
        lua_rawgeti(L, 3, 2);
        lua_rawgeti(L, 3, 3);
        lua_rawgeti(L, 3, 4);
        if (lua_isnumber(L, -4) && lua_isnumber(L, -3) &&
            lua_isnumber(L, -2) && lua_isnumber(L, -1)) {
          float rgba[4] = {
            (float)lua_tonumber(L, -4),
            (float)lua_tonumber(L, -3),
            (float)lua_tonumber(L, -2),
            (float)lua_tonumber(L, -1)
          };
          BK_set_color(ud->node, key, rgba);
          lua_pop(L, 4);
          break;
        }
        lua_pop(L, 4);
      }
      /* Otherwise treat as string list */
      const char **vals = (const char **)lua_newuserdata(L, len * sizeof(char *));
      for (int i = 0; i < len; i++) {
        lua_rawgeti(L, 3, i + 1);
        vals[i] = lua_tostring(L, -1);
      }
      BK_set_strs(ud->node, key, vals, len);
      /* Pop the string values */
      for (int i = 0; i < len; i++) lua_pop(L, 1);
      break;
    }
    default:
      return luaL_typeerror(L, 3, "string, number, boolean, or table");
  }
  return 0;
}

static int l_node_get(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  if (!ud->node) return 0;
  const char *key = luaL_checkstring(L, 2);

  if (luabui_is_numeric_key(key)) {
    double val = BK_get_num(ud->node, key);
    lua_pushnumber(L, val);
  } else {
    const char *s = BK_get_str(ud->node, key);
    if (s) lua_pushstring(L, s);
    else   lua_pushnil(L);
  }
  return 1;
}

static int l_node_get_color(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  if (!ud->node) return 0;
  float rgba[4];
  if (BK_get_color(ud->node, luaL_checkstring(L, 2), rgba)) {
    push_color(L, rgba);
    return 4;
  }
  return 0;
}

static int l_node_set_strs(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  if (!ud->node) return luaL_error(L, "attempt to use freed node");
  const char *key = luaL_checkstring(L, 2);
  luaL_checktype(L, 3, LUA_TTABLE);
  int n = (int)lua_rawlen(L, 3);
  const char **vals = (const char **)lua_newuserdata(L, n * sizeof(char *));
  for (int i = 0; i < n; i++) {
    lua_rawgeti(L, 3, i + 1);
    vals[i] = lua_tostring(L, -1);
  }
  BK_set_strs(ud->node, key, vals, n);
  for (int i = 0; i < n; i++) lua_pop(L, 1);
  return 0;
}

static int l_node_get_strs(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  if (!ud->node) return 0;
  const char *key = luaL_checkstring(L, 2);
  int n = BK_strs_count(ud->node, key);
  lua_createtable(L, n, 0);
  for (int i = 0; i < n; i++) {
    lua_pushstring(L, BK_strs_get(ud->node, key, i));
    lua_rawseti(L, -2, i + 1);
  }
  return 1;
}

static int l_node_invalidate(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  if (ud->node) BK_node_invalidate(ud->node);
  return 0;
}

static int l_node_relayout(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  if (ud->node) BK_node_relayout(ud->node);
  return 0;
}

static int l_node_free(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  if (ud->node) {
    luabui_unmap_node(L, ud->node);
    BK_node_free(ud->node);
    ud->node = NULL;
  }
  return 0;
}

static int l_node_on(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  if (!ud->node) return luaL_error(L, "attempt to use freed node");
  int kind = (int)luaL_checkinteger(L, 2);
  luaL_checktype(L, 3, LUA_TFUNCTION);

  /* Store callback in uservalue table: [kind] = function */
  lua_getuservalue(L, 1);
  lua_pushinteger(L, kind);
  lua_pushvalue(L, 3);
  lua_rawset(L, -3);
  lua_pop(L, 1);
  return 0;
}

static int l_node_off(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  if (!ud->node) return 0;
  int kind = (int)luaL_checkinteger(L, 2);

  lua_getuservalue(L, 1);
  lua_pushinteger(L, kind);
  lua_pushnil(L);
  lua_rawset(L, -3);
  lua_pop(L, 1);
  return 0;
}

static int l_node_geom(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  if (!ud->node) return luaL_error(L, "attempt to use freed node");
  int mgr = (int)luaL_checkinteger(L, 2);
  BK_geom_set_manager(ud->node, mgr);

  if (lua_istable(L, 3)) {
    lua_pushnil(L);
    while (lua_next(L, 3)) {
      /* key at -2, value at -1 */
      const char *k = lua_tostring(L, -2);
      if (lua_isnumber(L, -1))
        BK_geom_set(ud->node, k, lua_tonumber(L, -1));
      else if (lua_isboolean(L, -1))
        BK_geom_set(ud->node, k, lua_toboolean(L, -1) ? 1.0 : 0.0);
      lua_pop(L, 1); /* pop value */
    }
  }
  return 0;
}

static int l_node_geom_get(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  if (!ud->node) return 0;
  double val = BK_geom_get(ud->node, luaL_checkstring(L, 2));
  lua_pushnumber(L, val);
  return 1;
}

static int l_node_set_image_source(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  if (!ud->node) return luaL_error(L, "attempt to use freed node");
  const char *src = luaL_checkstring(L, 2);
  int fit = (int)luaL_optinteger(L, 3, BK_FIT_FILL);
  BK_node_set_image_source(ud->node, src, fit);
  return 0;
}

static int l_node_set_imbuf(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  if (!ud->node) return luaL_error(L, "attempt to use freed node");
  luabui_ImBuf *iud = CHECK_IMBUF(L, 2);
  if (iud->ib) BK_node_set_imbuf(ud->node, iud->ib);
  return 0;
}

static int l_node_set_image(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  if (!ud->node) return luaL_error(L, "attempt to use freed node");
  size_t len;
  const char *data = luaL_checklstring(L, 2, &len);
  int w = (int)luaL_checkinteger(L, 3);
  int h = (int)luaL_checkinteger(L, 4);
  BK_node_set_image(ud->node, (const unsigned char *)data, w, h);
  return 0;
}

static int l_node_set_video(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  if (!ud->node) return luaL_error(L, "attempt to use freed node");
  luabui_Video *vud = CHECK_VIDEO(L, 2);
  int fit = (int)luaL_optinteger(L, 3, BK_FIT_FILL);
  if (vud->video) BK_node_set_video(ud->node, vud->video, fit);
  return 0;
}

static int l_node_set_roots(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  if (!ud->node) return luaL_error(L, "attempt to use freed node");
  luaL_checktype(L, 2, LUA_TTABLE);
  int n = (int)lua_rawlen(L, 2);
  BKTreeItem **roots = (BKTreeItem **)lua_newuserdata(L, n * sizeof(BKTreeItem *));
  for (int i = 0; i < n; i++) {
    lua_rawgeti(L, 2, i + 1);
    luabui_TreeItem *tud = (luabui_TreeItem *)luaL_checkudata(L, -1, LUABUI_TREEITEM_MT);
    roots[i] = tud->item;
    lua_pop(L, 1);
  }
  BK_tree_set_roots(ud->node, roots, n);
  return 0;
}

static int l_node_selected(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  if (!ud->node) return 0;
  BKTreeItem *item = BK_tree_selected(ud->node);
  if (!item) return 0;
  /* Create or look up a Lua wrapper for the item */
  luabui_TreeItem *tud = (luabui_TreeItem *)lua_newuserdata(L, sizeof(luabui_TreeItem));
  tud->item = item;
  luaL_getmetatable(L, LUABUI_TREEITEM_MT);
  lua_setmetatable(L, -2);
  return 1;
}

static int l_node_select(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  if (!ud->node) return 0;
  luabui_TreeItem *tud = CHECK_TREEITEM(L, 2);
  BK_tree_select(ud->node, tud->item);
  return 0;
}

static int l_node_row_count(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  lua_pushinteger(L, ud->node ? BK_tree_row_count(ud->node) : 0);
  return 1;
}

static int l_node_text_index(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  if (!ud->node) return 0;
  int x = (int)luaL_checkinteger(L, 2);
  int y = (int)luaL_checkinteger(L, 3);
  lua_pushinteger(L, BK_textarea_index_at(ud->node, x, y));
  return 1;
}

static int l_node_caret(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  lua_pushinteger(L, ud->node ? BK_textarea_caret(ud->node) : 0);
  return 1;
}

static int l_node_set_caret(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  if (ud->node) BK_textarea_set_caret(ud->node, (int)luaL_checkinteger(L, 2));
  return 0;
}

static int l_node_selection(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  if (!ud->node) return 0;
  int start, end;
  BK_textarea_selection(ud->node, &start, &end);
  lua_pushinteger(L, start);
  lua_pushinteger(L, end);
  return 2;
}

static int l_node_set_menus(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  if (!ud->node) return luaL_error(L, "attempt to use freed node");
  luaL_checktype(L, 2, LUA_TTABLE);
  int n = (int)lua_rawlen(L, 2);
  BKMenuSpec *specs = (BKMenuSpec *)lua_newuserdata(L, n * sizeof(BKMenuSpec));
  memset(specs, 0, n * sizeof(BKMenuSpec));
  for (int i = 0; i < n; i++) {
    lua_rawgeti(L, 2, i + 1);
    luaL_checktype(L, -1, LUA_TTABLE);
    lua_getfield(L, -1, "text");
    specs[i].text = lua_tostring(L, -1);
    lua_getfield(L, -2, "icon");
    specs[i].icon = (int)luaL_optinteger(L, -1, 0);
    lua_getfield(L, -3, "shortcut");
    specs[i].shortcut = lua_tostring(L, -1);
    lua_getfield(L, -4, "checked");
    specs[i].checked = (int)luaL_optinteger(L, -1, -1);
    lua_getfield(L, -5, "enabled");
    specs[i].enabled = (int)luaL_optinteger(L, -1, 1);
    /* Store function ref in ref field */
    lua_getfield(L, -6, "fn");
    if (lua_isfunction(L, -1)) {
      specs[i].ref = (void *)(intptr_t)luaL_ref(L, LUA_REGISTRYINDEX);
    }
    lua_pop(L, 7); /* clean up: table + 6 fields */
  }
  BK_menubutton_set_items(ud->node, specs, n);
  return 0;
}

static int l_node_custom_set(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  if (!ud->node) return luaL_error(L, "attempt to use freed node");
  luaL_checktype(L, 2, LUA_TTABLE);
  BKCustomClass cls = {0};
  cls.natural_w = 0;
  cls.natural_h = 0;

  lua_getfield(L, 2, "natural_w");
  cls.natural_w = (int)luaL_optinteger(L, -1, 0);
  lua_getfield(L, 2, "natural_h");
  cls.natural_h = (int)luaL_optinteger(L, -1, 0);
  lua_pop(L, 2);

  /* Store the class table as userdata on the node's ref for event dispatch */
  lua_pushvalue(L, 2);
  BK_node_set_ref(ud->node, (void *)(intptr_t)luaL_ref(L, LUA_REGISTRYINDEX));

  BK_custom_set(ud->node, &cls, (void *)L);
  return 0;
}

static int l_node_custom_data(lua_State *L) {
  luabui_Node *ud = CHECK_NODE(L, 1);
  if (!ud->node) return 0;
  void *data = BK_custom_data(ud->node);
  lua_pushlightuserdata(L, data);
  return 1;
}

static int l_node_gc(lua_State *L) {
  luabui_Node *ud = (luabui_Node *)luaL_checkudata(L, 1, LUABUI_NODE_MT);
  if (ud->node) {
    luabui_unmap_node(L, ud->node);
    /* Ownership model: the C tree owns the node, NOT the Lua wrapper.
     * Only free if explicitly requested via node:free(). */
  }
  ud->node = NULL;
  return 0;
}

static int l_node_tostring(lua_State *L) {
  luabui_Node *ud = (luabui_Node *)luaL_checkudata(L, 1, LUABUI_NODE_MT);
  if (ud->node) {
    lua_pushfstring(L, "bui.Node: %s (%p)", BK_node_type(ud->node), (void *)ud->node);
  } else {
    lua_pushfstring(L, "bui.Node: (freed)");
  }
  return 1;
}

/* ------------------------------------------------------------------ */
/* Registration                                                       */
/* ------------------------------------------------------------------ */

static const luaL_Reg node_methods[] = {
  {"add",          l_node_add},
  {"remove",       l_node_remove},
  {"child",        l_node_child},
  {"children",     l_node_children},
  {"child_count",  l_node_child_count},
  {"parent",       l_node_parent},
  {"type",         l_node_type},
  {"rect",         l_node_rect},
  {"natural",      l_node_natural},
  {"set",          l_node_set},
  {"get",          l_node_get},
  {"get_color",    l_node_get_color},
  {"set_strs",     l_node_set_strs},
  {"get_strs",     l_node_get_strs},
  {"invalidate",   l_node_invalidate},
  {"relayout",     l_node_relayout},
  {"free",         l_node_free},
  {"on",           l_node_on},
  {"off",          l_node_off},
  {"geom",         l_node_geom},
  {"geom_get",     l_node_geom_get},
  {"set_image",        l_node_set_image},
  {"set_imbuf",        l_node_set_imbuf},
  {"set_image_source", l_node_set_image_source},
  {"set_video",        l_node_set_video},
  {"text_at",      l_node_text_index},
  {"caret",        l_node_caret},
  {"set_caret",    l_node_set_caret},
  {"selection",    l_node_selection},
  {"set_roots",    l_node_set_roots},
  {"selected",     l_node_selected},
  {"select",       l_node_select},
  {"row_count",    l_node_row_count},
  {"set_menus",    l_node_set_menus},
  {"custom_set",   l_node_custom_set},
  {"custom_data",  l_node_custom_data},
  {"__gc",         l_node_gc},
  {"__tostring",   l_node_tostring},
  {NULL, NULL}
};

void luabui_register_node(lua_State *L) {
  luaL_newmetatable(L, LUABUI_NODE_MT);
  /* methods */
  luaL_setfuncs(L, node_methods, 0);
  /* metatable.__index = metatable (so methods are accessible on the object) */
  lua_pushvalue(L, -1);
  lua_setfield(L, -2, "__index");
  lua_pop(L, 1);
}

/* ------------------------------------------------------------------ */
/* TreeItem map helpers                                               */
/* ------------------------------------------------------------------ */

void luabui_map_treeitem(lua_State *L, BKTreeItem *item, int ud_idx) {
  lua_getfield(L, LUA_REGISTRYINDEX, LUABUI_TREEITEM_MAP);
  lua_pushlightuserdata(L, item);
  lua_pushvalue(L, ud_idx < 0 ? ud_idx - 1 : ud_idx);
  lua_rawset(L, -3);
  lua_pop(L, 1);
}

void *luabui_lookup_treeitem(lua_State *L, BKTreeItem *item) {
  lua_getfield(L, LUA_REGISTRYINDEX, LUABUI_TREEITEM_MAP);
  lua_pushlightuserdata(L, item);
  lua_rawget(L, -2);
  void *ud = lua_touserdata(L, -1);
  lua_pop(L, 2);
  return ud;
}

void luabui_unmap_treeitem(lua_State *L, BKTreeItem *item) {
  lua_getfield(L, LUA_REGISTRYINDEX, LUABUI_TREEITEM_MAP);
  lua_pushlightuserdata(L, item);
  lua_pushnil(L);
  lua_rawset(L, -3);
  lua_pop(L, 1);
}

/* Recursively nullify all children of a tree item in the map. */
void luabui_nullify_treeitem(lua_State *L, BKTreeItem *item) {
  if (!item) return;
  int n = BK_tree_item_child_count(item);
  for (int i = 0; i < n; i++) {
    BKTreeItem *child = BK_tree_item_child(item, i);
    /* Look up this child's Lua wrapper and nullify its C pointer */
    lua_getfield(L, LUA_REGISTRYINDEX, LUABUI_TREEITEM_MAP);
    lua_pushlightuserdata(L, child);
    lua_rawget(L, -2);
    if (lua_isuserdata(L, -1) && luaL_testudata(L, -1, LUABUI_TREEITEM_MT)) {
      ((luabui_TreeItem *)lua_touserdata(L, -1))->item = NULL;
    }
    lua_pop(L, 2);
    /* Also nullify the child's children recursively before the child is freed */
    luabui_nullify_treeitem(L, child);
  }
}

/* ------------------------------------------------------------------ */
/* TreeItem methods                                                    */
/* ------------------------------------------------------------------ */

static int l_treeitem_add(lua_State *L) {
  luabui_TreeItem *parent = CHECK_TREEITEM(L, 1);
  luabui_TreeItem *child  = CHECK_TREEITEM(L, 2);
  if (!parent->item || !child->item) return 0;
  BK_tree_item_add(parent->item, child->item);
  return 0;
}

static int l_treeitem_clear(lua_State *L) {
  luabui_TreeItem *tud = CHECK_TREEITEM(L, 1);
  if (!tud->item) return 0;
  int n = BK_tree_item_child_count(tud->item);
  /* Re-parent each child to null (detach) */
  BKTreeItem **children = NULL;
  if (n > 0) {
    children = (BKTreeItem **)lua_newuserdata(L, n * sizeof(BKTreeItem *));
    for (int i = 0; i < n; i++) {
      children[i] = BK_tree_item_child(tud->item, i);
    }
  }
  BK_tree_item_clear(tud->item);
  /* Nullify each child's Lua wrapper (they are detached, still alive) */
  for (int i = 0; i < n; i++) {
    BKTreeItem *child = children[i];
    lua_getfield(L, LUA_REGISTRYINDEX, LUABUI_TREEITEM_MAP);
    lua_pushlightuserdata(L, child);
    lua_rawget(L, -2);
    if (lua_isuserdata(L, -1) && luaL_testudata(L, -1, LUABUI_TREEITEM_MT)) {
      ((luabui_TreeItem *)lua_touserdata(L, -1))->item = NULL;
    }
    lua_pop(L, 2);
  }
  return 0;
}

static int l_treeitem_child_count(lua_State *L) {
  luabui_TreeItem *tud = CHECK_TREEITEM(L, 1);
  lua_pushinteger(L, tud->item ? BK_tree_item_child_count(tud->item) : 0);
  return 1;
}

static int l_treeitem_child(lua_State *L) {
  luabui_TreeItem *tud = CHECK_TREEITEM(L, 1);
  if (!tud->item) return 0;
  int i = (int)luaL_checkinteger(L, 2);
  BKTreeItem *item = BK_tree_item_child(tud->item, i);
  if (!item) return 0;
  /* Check if we already have a Lua wrapper */
  void *existing = luabui_lookup_treeitem(L, item);
  if (existing) { lua_pushvalue(L, (int)(intptr_t)existing); return 1; }
  luabui_TreeItem *cud = (luabui_TreeItem *)lua_newuserdata(L, sizeof(luabui_TreeItem));
  cud->item = item;
  luaL_getmetatable(L, LUABUI_TREEITEM_MT);
  lua_setmetatable(L, -2);
  luabui_map_treeitem(L, item, -1);
  return 1;
}

static int l_treeitem_set_label(lua_State *L) {
  luabui_TreeItem *tud = CHECK_TREEITEM(L, 1);
  if (tud->item) BK_tree_item_set_label(tud->item, luaL_checkstring(L, 2));
  return 0;
}

static int l_treeitem_label(lua_State *L) {
  luabui_TreeItem *tud = CHECK_TREEITEM(L, 1);
  lua_pushstring(L, tud->item ? BK_tree_item_label(tud->item) : "");
  return 1;
}

static int l_treeitem_set(lua_State *L) {
  luabui_TreeItem *tud = CHECK_TREEITEM(L, 1);
  if (tud->item) BK_tree_item_set(tud->item, luaL_checkstring(L, 2), luaL_checknumber(L, 3));
  return 0;
}

static int l_treeitem_get(lua_State *L) {
  luabui_TreeItem *tud = CHECK_TREEITEM(L, 1);
  lua_pushnumber(L, tud->item ? BK_tree_item_get(tud->item, luaL_checkstring(L, 2)) : 0);
  return 1;
}

static int l_treeitem_set_values(lua_State *L) {
  luabui_TreeItem *tud = CHECK_TREEITEM(L, 1);
  if (!tud->item) return 0;
  luaL_checktype(L, 2, LUA_TTABLE);
  int n = (int)lua_rawlen(L, 2);
  const char **vals = (const char **)lua_newuserdata(L, n * sizeof(char *));
  for (int i = 0; i < n; i++) {
    lua_rawgeti(L, 2, i + 1);
    vals[i] = lua_tostring(L, -1);
  }
  BK_tree_item_set_values(tud->item, vals, n);
  for (int i = 0; i < n; i++) lua_pop(L, 1);
  return 0;
}

static void treeitem_free_safe(lua_State *L, luabui_TreeItem *tud) {
  if (!tud->item) return;

  /* WARNING: tree items use parent-ownership semantics. The C parent retains
   * child pointers. Freeing a child while it is still attached to a parent
   * leaves a dangling pointer in the parent. Therefore we MUST always free
   * from the root down (parent first, which cascades to children). */

  /* Step 1: detach and free all children recursively */
  int n = BK_tree_item_child_count(tud->item);
  BKTreeItem **children = NULL;
  if (n > 0) {
    children = (BKTreeItem **)lua_newuserdata(L, n * sizeof(BKTreeItem *));
    for (int i = 0; i < n; i++) {
      children[i] = BK_tree_item_child(tud->item, i);
    }
  }
  BK_tree_item_clear(tud->item);
  for (int i = 0; i < n; i++) {
    BKTreeItem *child = children[i];
    /* Nullify child's Lua wrapper */
    lua_getfield(L, LUA_REGISTRYINDEX, LUABUI_TREEITEM_MAP);
    lua_pushlightuserdata(L, child);
    lua_rawget(L, -2);
    if (lua_isuserdata(L, -1) && luaL_testudata(L, -1, LUABUI_TREEITEM_MT)) {
      ((luabui_TreeItem *)lua_touserdata(L, -1))->item = NULL;
    }
    lua_pop(L, 2);
    BK_tree_item_free(child);
  }

  /* Step 2: unmap and free self */
  luabui_unmap_treeitem(L, tud->item);
  BK_tree_item_free(tud->item);
  tud->item = NULL;
}

static int l_treeitem_free(lua_State *L) {
  luabui_TreeItem *tud = CHECK_TREEITEM(L, 1);
  treeitem_free_safe(L, tud);
  return 0;
}

static int l_treeitem_gc(lua_State *L) {
  luabui_TreeItem *tud = (luabui_TreeItem *)luaL_checkudata(L, 1, LUABUI_TREEITEM_MT);
  /* Tree items require explicit lifecycle management (add/remove from parent).
   * __gc does NOT free the C item — that's the caller's responsibility via :free().
   * We just remove from the map if it's still there. */
  if (tud->item) {
    /* Check if still in the map before trying to unmap */
    lua_getfield(L, LUA_REGISTRYINDEX, LUABUI_TREEITEM_MAP);
    lua_pushlightuserdata(L, tud->item);
    lua_rawget(L, -2);
    if (!lua_isnil(L, -1)) {
      lua_pop(L, 1);
      luabui_unmap_treeitem(L, tud->item);
    } else {
      lua_pop(L, 1);
    }
    tud->item = NULL;
  }
  return 0;
}

static int l_treeitem_tostring(lua_State *L) {
  luabui_TreeItem *tud = (luabui_TreeItem *)luaL_checkudata(L, 1, LUABUI_TREEITEM_MT);
  if (tud->item) {
    lua_pushfstring(L, "bui.TreeItem: \"%s\"", BK_tree_item_label(tud->item));
  } else {
    lua_pushfstring(L, "bui.TreeItem: (freed)");
  }
  return 1;
}

static const luaL_Reg treeitem_methods[] = {
  {"add",        l_treeitem_add},
  {"clear",      l_treeitem_clear},
  {"child_count",l_treeitem_child_count},
  {"child",      l_treeitem_child},
  {"set_label",  l_treeitem_set_label},
  {"label",      l_treeitem_label},
  {"set",        l_treeitem_set},
  {"get",        l_treeitem_get},
  {"set_values", l_treeitem_set_values},
  {"free",       l_treeitem_free},
  {"__gc",       l_treeitem_gc},
  {"__tostring", l_treeitem_tostring},
  {NULL, NULL}
};

void luabui_register_treeitem(lua_State *L) {
  luaL_newmetatable(L, LUABUI_TREEITEM_MT);
  luaL_setfuncs(L, treeitem_methods, 0);
  lua_pushvalue(L, -1);
  lua_setfield(L, -2, "__index");
  lua_pop(L, 1);
}
