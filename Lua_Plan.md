# BUI Lua Host Client — Implementation Plan

## 1. Architecture Overview

```
┌──────────────────────────────────────────────────────────────────┐
│                    bui-lua executable                             │
│                                                                   │
│  ┌──────────────────────────────────────────────────────┐        │
│  │              Lua 5.4 VM (bundled)                     │        │
│  │  • Script execution: bui-lua app.lua                  │        │
│  │  • REPL mode:        bui-lua                          │        │
│  │  • Standard Lua libs + bui.* module                   │        │
│  └──────────┬───────────────────────────────────────────┘        │
│             │ C API calls                                         │
│  ┌──────────▼───────────────────────────────────────────┐        │
│  │              C Binding Layer (bui_lua.c)              │        │
│  │  • luabui_Node    – BKNode* userdata + metatable     │        │
│  │  • luabui_Context – BKContext* userdata + BKHost     │        │
│  │  • luabui_Window  – BKWindow* userdata               │        │
│  │  • luabui_TreeItem– BKTreeItem* userdata             │        │
│  │  • luabui_Image   – BKImBuf* helpers                 │        │
│  │  • luabui_Video   – BKVideo* helpers                 │        │
│  └──────────┬───────────────────────────────────────────┘        │
│             │ static link                                         │
│  ┌──────────▼───────────────────────────────────────────┐        │
│  │            BUI C Libraries (bui_kit, bui_gpu,         │        │
│  │            bui_core, bui_api)                         │        │
│  └──────────────────────────────────────────────────────┘        │
└──────────────────────────────────────────────────────────────────┘
```

**Key decisions:**
- **Lua 5.4** (latest stable), bundled as source — no system dependency
- **Standalone executable** — not a loadable module (like the QuickJS runtime pattern)
- **Full kit API** (`BUI_kit.h`), not just the stable C ABI — gives full control over nodes, context, windows, and input
- **Native windows via `BK_window_*`** — cross-platform for free (Win32/X11/Cocoa)

---

## 2. Build System Integration

### 2.1 Directory Structure

```
bindings/lua/
├── CMakeLists.txt          # Build: bundles Lua, links BUI, creates executable
├── src/
│   ├── main.c              # Entry point: CLI parsing, REPL, script runner
│   ├── bui_lua.c           # Module init, type registration, constants
│   ├── bui_node.c          # BKNode userdata + metatable
│   ├── bui_context.c       # BKContext userdata + BKHost callbacks
│   ├── bui_window.c        # BKWindow userdata
│   ├── bui_image.c         # Image/video loading helpers
│   └── bui_internal.h      # Shared internal declarations
├── lua-5.4.7/              # Bundled Lua 5.4 source
│   └── ... (lapi.c, lua.h, etc. from https://www.lua.org/ftp/lua-5.4.7.tar.gz)
├── extern/
│   └── linenoise/          # Bundled linenoise (MIT) for REPL line editing
│       ├── linenoise.h
│       └── linenoise.c
├── examples/
│   ├── hello.lua
│   ├── counter.lua
│   ├── todos.lua
│   └── gallery.lua
└── README.md
```

### 2.2 CMakeLists.txt

```cmake
# bindings/lua/CMakeLists.txt
# Bundles Lua 5.4 as a static library, links BUI, produces bui-lua executable.

set(LUA_DIR ${CMAKE_CURRENT_SOURCE_DIR}/lua-5.4.7)
set(LUA_SRC
    ${LUA_DIR}/lapi.c      ${LUA_DIR}/lauxlib.c    ${LUA_DIR}/lbaselib.c
    ${LUA_DIR}/lcode.c     ${LUA_DIR}/lcorolib.c   ${LUA_DIR}/lctype.c
    ${LUA_DIR}/ldebug.c    ${LUA_DIR}/ldo.c        ${LUA_DIR}/ldump.c
    ${LUA_DIR}/lfunc.c     ${LUA_DIR}/lgc.c        ${LUA_DIR}/linit.c
    ${LUA_DIR}/liolib.c    ${LUA_DIR}/llex.c       ${LUA_DIR}/lmathlib.c
    ${LUA_DIR}/lmem.c      ${LUA_DIR}/loadlib.c    ${LUA_DIR}/lobject.c
    ${LUA_DIR}/lopcodes.c  ${LUA_DIR}/loslib.c     ${LUA_DIR}/lparser.c
    ${LUA_DIR}/lstate.c    ${LUA_DIR}/lstring.c    ${LUA_DIR}/lstrlib.c
    ${LUA_DIR}/ltable.c    ${LUA_DIR}/ltablib.c    ${LUA_DIR}/ltm.c
    ${LUA_DIR}/lundump.c   ${LUA_DIR}/lutf8lib.c   ${LUA_DIR}/lvm.c
    ${LUA_DIR}/lzio.c)

add_library(bui_lua STATIC ${LUA_SRC})
target_include_directories(bui_lua PUBLIC ${LUA_DIR})
if(MSVC)
  target_compile_definitions(bui_lua PRIVATE LUA_BUILD_AS_DLL)
else()
  target_compile_options(bui_lua PRIVATE -fvisibility=hidden)
endif()

# linenoise for REPL
set(LINENOISE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/extern/linenoise)
add_library(linenoise STATIC ${LINENOISE_DIR}/linenoise.c)
target_include_directories(linenoise PUBLIC ${LINENOISE_DIR})

# The bui-lua executable
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
    bui::api)
target_include_directories(bui-lua PRIVATE src)
```

### 2.3 Top-Level CMakeLists.txt Addition

Add after existing binding options (around line 123):
```cmake
option(BUI_BUILD_LUA_BINDINGS "Build the bui-lua host executable" OFF)
...
if(BUI_BUILD_LUA_BINDINGS)
  add_subdirectory(bindings/lua)
endif()
```

---

## 3. C Binding Layer Specification

### 3.1 Lua Userdata Types

```c
// bui_internal.h

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

// ImBuf userdata — decoded image (can be attached to nodes)
typedef struct {
  BKImBuf *ib;
} luabui_ImBuf;

// Video userdata
typedef struct {
  BKVideo *video;
} luabui_Video;

// Registry key names
#define LUABUI_NODE_MT      "bui.Node"
#define LUABUI_CONTEXT_MT   "bui.Context"
#define LUABUI_WINDOW_MT    "bui.Window"
#define LUABUI_TREEITEM_MT  "bui.TreeItem"
#define LUABUI_IMBUF_MT     "bui.ImBuf"
#define LUABUI_VIDEO_MT     "bui.Video"
#define LUABUI_NODE_MAP     "bui._node_map"    // weak-keyed table: BKNode* -> userdata
```

### 3.2 Module Registration (`bui_lua.c`)

```c
int luaopen_bui(lua_State *L) {
  // Create weak-keyed node map in registry
  lua_newtable(L);
  lua_newtable(L);
  lua_pushstring(L, "k");
  lua_setfield(L, -2, "__mode");
  lua_setmetatable(L, -2);
  lua_setfield(L, LUA_REGISTRYINDEX, LUABUI_NODE_MAP);

  // Register metatables
  luabui_register_node(L);
  luabui_register_context(L);
  luabui_register_window(L);
  luabui_register_treeitem(L);
  luabui_register_imbuf(L);
  luabui_register_video(L);

  // Module table
  lua_newtable(L);

  lua_pushcfunction(L, luabui_new_node);     lua_setfield(L, -2, "Node");
  lua_pushcfunction(L, luabui_new_context);  lua_setfield(L, -2, "Context");
  lua_pushcfunction(L, luabui_new_window);   lua_setfield(L, -2, "Window");
  lua_pushcfunction(L, luabui_new_treeitem); lua_setfield(L, -2, "TreeItem");

  lua_pushcfunction(L, luabui_run);          lua_setfield(L, -2, "run");
  lua_pushcfunction(L, luabui_pump);         lua_setfield(L, -2, "pump");
  lua_pushcfunction(L, luabui_set_scale);    lua_setfield(L, -2, "set_scale");
  lua_pushcfunction(L, luabui_set_theme);    lua_setfield(L, -2, "set_theme");
  lua_pushcfunction(L, luabui_theme_name);   lua_setfield(L, -2, "theme_name");
  lua_pushcfunction(L, luabui_theme_color);  lua_setfield(L, -2, "theme_color");
  lua_pushcfunction(L, luabui_unit);         lua_setfield(L, -2, "unit");
  lua_pushcfunction(L, luabui_version);      lua_setfield(L, -2, "version");
  lua_pushcfunction(L, luabui_eval);         lua_setfield(L, -2, "eval");
  lua_pushcfunction(L, luabui_image_decode); lua_setfield(L, -2, "image_decode");
  lua_pushcfunction(L, luabui_image_load);   lua_setfield(L, -2, "image_load");

  // Event constants
  lua_pushinteger(L, BK_EV_CHANGE);  lua_setfield(L, -2, "EV_CHANGE");
  lua_pushinteger(L, BK_EV_CLICK);   lua_setfield(L, -2, "EV_CLICK");
  lua_pushinteger(L, BK_EV_CALL);    lua_setfield(L, -2, "EV_CALL");
  lua_pushinteger(L, BK_EV_RENAME);  lua_setfield(L, -2, "EV_RENAME");
  lua_pushinteger(L, BK_EV_ACTIVATE);lua_setfield(L, -2, "EV_ACTIVATE");
  lua_pushinteger(L, BK_EV_TOGGLE);  lua_setfield(L, -2, "EV_TOGGLE");
  lua_pushinteger(L, BK_EV_BROWSE);  lua_setfield(L, -2, "EV_BROWSE");
  lua_pushinteger(L, BK_EV_PRESS);   lua_setfield(L, -2, "EV_PRESS");
  lua_pushinteger(L, BK_EV_RELEASE); lua_setfield(L, -2, "EV_RELEASE");
  lua_pushinteger(L, BK_EV_DOUBLE);  lua_setfield(L, -2, "EV_DOUBLE");
  lua_pushinteger(L, BK_EV_MOTION);  lua_setfield(L, -2, "EV_MOTION");
  lua_pushinteger(L, BK_EV_ENTER);   lua_setfield(L, -2, "EV_ENTER");
  lua_pushinteger(L, BK_EV_LEAVE);   lua_setfield(L, -2, "EV_LEAVE");
  lua_pushinteger(L, BK_EV_KEY);     lua_setfield(L, -2, "EV_KEY");
  lua_pushinteger(L, BK_EV_WHEEL);   lua_setfield(L, -2, "EV_WHEEL");
  lua_pushinteger(L, BK_EV_SUBMIT);  lua_setfield(L, -2, "EV_SUBMIT");
  lua_pushinteger(L, BK_EV_VIEW);    lua_setfield(L, -2, "EV_VIEW");
  lua_pushinteger(L, BK_EV_SCROLL);  lua_setfield(L, -2, "EV_SCROLL");
  lua_pushinteger(L, BK_EV_CONFIGURE);lua_setfield(L, -2, "EV_CONFIGURE");

  // Cursor constants
  lua_pushinteger(L, BK_CURSOR_DEFAULT);    lua_setfield(L, -2, "CURSOR_DEFAULT");
  lua_pushinteger(L, BK_CURSOR_EW);         lua_setfield(L, -2, "CURSOR_EW");
  lua_pushinteger(L, BK_CURSOR_NS);         lua_setfield(L, -2, "CURSOR_NS");
  lua_pushinteger(L, BK_CURSOR_TEXT);       lua_setfield(L, -2, "CURSOR_TEXT");
  lua_pushinteger(L, BK_CURSOR_CROSSHAIR);  lua_setfield(L, -2, "CURSOR_CROSSHAIR");
  lua_pushinteger(L, BK_CURSOR_HAND);       lua_setfield(L, -2, "CURSOR_HAND");
  lua_pushinteger(L, BK_CURSOR_MOVE);       lua_setfield(L, -2, "CURSOR_MOVE");
  lua_pushinteger(L, BK_CURSOR_NONE);       lua_setfield(L, -2, "CURSOR_NONE");

  // Geometry constants
  lua_pushinteger(L, BK_GEOM_NONE); lua_setfield(L, -2, "GEOM_NONE");
  lua_pushinteger(L, BK_GEOM_PACK); lua_setfield(L, -2, "GEOM_PACK");
  lua_pushinteger(L, BK_GEOM_GRID); lua_setfield(L, -2, "GEOM_GRID");
  lua_pushinteger(L, BK_GEOM_PLACE);lua_setfield(L, -2, "GEOM_PLACE");

  // Modifier bits
  lua_pushinteger(L, BK_SHIFT); lua_setfield(L, -2, "MOD_SHIFT");
  lua_pushinteger(L, BK_CTRL);  lua_setfield(L, -2, "MOD_CTRL");
  lua_pushinteger(L, BK_ALT);   lua_setfield(L, -2, "MOD_ALT");

  return 1;
}
```

### 3.3 BKNode Metatable (`bui_node.c`)

```c
static const struct luaL_Reg luabui_node_methods[] = {
  {"add",         luabui_node_add},
  {"remove",      luabui_node_remove},
  {"child",       luabui_node_child},
  {"children",    luabui_node_children},
  {"child_count", luabui_node_child_count},
  {"parent",      luabui_node_parent},
  {"type",        luabui_node_type},
  {"rect",        luabui_node_rect},
  {"natural",     luabui_node_natural},
  {"set",         luabui_node_set},
  {"get",         luabui_node_get},
  {"get_color",   luabui_node_get_color},
  {"set_strs",    luabui_node_set_strs},
  {"get_strs",    luabui_node_get_strs},
  {"invalidate",  luabui_node_invalidate},
  {"relayout",    luabui_node_relayout},
  {"free",        luabui_node_free},
  {"on",          luabui_node_on},
  {"off",         luabui_node_off},
  {"geom",        luabui_node_geom},
  {"geom_get",    luabui_node_geom_get},
  {"set_image",   luabui_node_set_image},
  {"set_imbuf",   luabui_node_set_imbuf},
  {"set_video",   luabui_node_set_video},
  {"set_image_source", luabui_node_set_image_source},
  // textarea
  {"text_at",     luabui_node_text_index},
  {"caret",       luabui_node_caret},
  {"set_caret",   luabui_node_set_caret},
  {"selection",   luabui_node_selection},
  // tree
  {"set_roots",   luabui_node_set_roots},
  {"selected",    luabui_node_selected},
  {"select",      luabui_node_select},
  {"row_count",   luabui_node_row_count},
  // menus
  {"set_menus",   luabui_node_set_menus},
  // custom nodes
  {"custom_set",  luabui_node_custom_set},
  {"custom_data", luabui_node_custom_data},
  // ramp/curve/histogram
  {"ramp_count",  luabui_node_ramp_count},
  {"ramp_get",    luabui_node_ramp_get},
  {"ramp_set",    luabui_node_ramp_set},
  {"curve_get",   luabui_node_curve_get},
  {"curve_set",   luabui_node_curve_set},
  {"histogram_set", luabui_node_histogram_set},
  // lifecycle
  {"__gc",        luabui_node_gc},
  {"__tostring",  luabui_node_tostring},
  {NULL, NULL}
};
```

**`set` auto-dispatch** — `node:set(key, value)` detects value type:

| Lua type | C API called |
|----------|-------------|
| `string` | `BK_set_str(node, key, val)` |
| `number` | `BK_set_num(node, key, val)` |
| `boolean` | `BK_set_num(node, key, val ? 1 : 0)` |
| `table` (4 numbers) | `BK_set_color(node, key, rgba)` |
| `table` (n strings) | `BK_set_strs(node, key, vals, n)` |

**`get` return** — `node:get(key)` tries number first (if key is in a known list of numeric properties), else returns string; falls back to nil for unknown keys.

### 3.4 BKHost Bridge (`bui_context.c`)

The `BKHost` struct is the critical integration seam between BUI's C runtime and Lua:

```c
// The BKHost instance used by this context
static BKHost g_host_template = {
  NULL,                       // data = lua_State*
  luabui_host_request,
  luabui_host_set_cursor,
  NULL,                       // warp (optional)
  luabui_host_clipboard_get,
  luabui_host_clipboard_set,
  luabui_host_schedule,
  luabui_host_emit,
};

// Called by BUI whenever a node fires an event
static void luabui_host_emit(void *data, BKNode *node, int kind, void *ref, double a, double b) {
  lua_State *L = (lua_State *)data;

  // BK_EV_CALL with a function ref (menu items, hotkeys)
  if (kind == BK_EV_CALL && ref) {
    int func_ref = (int)(intptr_t)ref;
    lua_rawgeti(L, LUA_REGISTRYINDEX, func_ref);
    if (lua_pcall(L, 0, 0, 0) != LUA_OK) {
      fprintf(stderr, "bui: callback error: %s\n", lua_tostring(L, -1));
      lua_pop(L, 1);
    }
    return;
  }

  // Look up the Lua userdata for this node via the weak node map
  lua_getfield(L, LUA_REGISTRYINDEX, LUABUI_NODE_MAP);
  lua_pushlightuserdata(L, node);
  lua_rawget(L, -2);
  if (lua_isnil(L, -1)) { lua_pop(L, 2); return; }
  // stack: node_map, userdata

  // Get the uservalue table (where callbacks are stored)
  lua_getuservalue(L, -1);
  lua_pushinteger(L, kind);
  lua_rawget(L, -2);
  // stack: node_map, userdata, uservalue, callback

  if (!lua_isnil(L, -1)) {
    lua_pushvalue(L, -3);     // self = node userdata
    lua_pushnumber(L, a);
    lua_pushnumber(L, b);
    if (lua_pcall(L, 3, 0, 0) != LUA_OK) {
      fprintf(stderr, "bui: event callback error: %s\n", lua_tostring(L, -1));
      lua_pop(L, 1);
    }
  }
  lua_pop(L, 4);  // clean stack
}

static void luabui_host_request(void *data) {
  // Signal the main loop to call BK_render soon
  // (atomic flag checked in the pump cycle)
}

static void luabui_host_set_cursor(void *data, int cursor) {
  // The host sets the system cursor via the native window
  // (or stores it for the next pump cycle)
}

static char *luabui_host_clipboard_get(void *data) {
  // Use platform APIs (or defer to Lua)
  return NULL;
}

static void luabui_host_clipboard_set(void *data, const char *text) {
  // Use platform APIs
}

static void luabui_host_schedule(void *data, int ms, int timer_id) {
  // Queue a timer event: call BK_timer(ctx, timer_id) after ms ms
}
```

### 3.5 BKContext Metatable

```c
static const struct luaL_Reg luabui_context_methods[] = {
  {"set_header",   luabui_ctx_set_header},
  {"root",         luabui_ctx_root},
  {"render",       luabui_ctx_render},
  {"needs_render", luabui_ctx_needs_render},
  {"press",        luabui_ctx_press},
  {"release",      luabui_ctx_release},
  {"motion",       luabui_ctx_motion},
  {"wheel",        luabui_ctx_wheel},
  {"scroll",       luabui_ctx_scroll},
  {"key",          luabui_ctx_key},
  {"key_release",  luabui_ctx_key_release},
  {"button",       luabui_ctx_button},
  {"context_menu", luabui_ctx_context_menu},
  {"timer",        luabui_ctx_timer},
  {"bind",         luabui_ctx_bind},
  {"close_popups", luabui_ctx_close_popups},
  {"focus",        luabui_ctx_focus},
  {"hover",        luabui_ctx_hover},
  {"cursor",       luabui_ctx_cursor},
  {"undo",         luabui_ctx_undo},
  {"redo",         luabui_ctx_redo},
  {"free",         luabui_ctx_free},
  {"__gc",         luabui_ctx_gc},
  {"__tostring",   luabui_ctx_tostring},
  {NULL, NULL}
};
```

### 3.6 BKWindow Metatable (`bui_window.c`)

```c
static const struct luaL_Reg luabui_window_methods[] = {
  {"show",         luabui_win_show},
  {"close",        luabui_win_close},
  {"is_open",      luabui_win_is_open},
  {"set_title",    luabui_win_set_title},
  {"set_size",     luabui_win_set_size},
  {"get_size",     luabui_win_get_size},
  {"set_gpu",      luabui_win_set_gpu},
  {"render_now",   luabui_win_render_now},
  {"timer",        luabui_win_timer},
  {"cancel_timer", luabui_win_cancel_timer},
  {"on_close",     luabui_win_set_on_close},
  {"on_frame",     luabui_win_set_on_frame},
  {"handle",       luabui_win_handle},
  {"wm",           luabui_win_wm},
  {"wm_get",       luabui_win_wm_get},
  {"free",         luabui_win_free},
  {"__gc",         luabui_win_gc},
  {"__tostring",   luabui_win_tostring},
  {NULL, NULL}
};
```

---

## 4. Lua API Surface (User-Facing)

### 4.1 Module-Level Functions

```lua
local bui = require("bui")

-- Factories
bui.Node(type)                     -> node userdata
bui.Context(root_node)             -> ctx userdata
bui.Window(ctx, title, width, height) -> win userdata or nil
bui.TreeItem(label [, icon])       -> item userdata

-- Globals
bui.run()                          -- enter message loop (blocks until windows close)
bui.pump()                         -- pump one round of events (non-blocking)
bui.set_scale(factor)              -- resolution scale (1 = 20px widgets)
bui.set_theme("dark"|"light")      -- switch theme
bui.theme_name()                   -> "dark" | "light"
bui.theme_color(key)               -> r, g, b, a
bui.unit()                         -> pixels per widget unit
bui.version()                      -> major, minor, patch
bui.eval(expr)                     -> evaluated result (Blender math syntax)
bui.image_decode(data, max_w, max_h) -> imbuf userdata or nil
bui.image_load(path, max_w, max_h)   -> imbuf userdata or nil

-- Constants (partial list)
bui.EV_CHANGE    bui.EV_CLICK    bui.EV_CALL
bui.EV_PRESS     bui.EV_RELEASE  bui.EV_MOTION
bui.EV_KEY       bui.EV_WHEEL    bui.EV_SUBMIT
bui.EV_SCROLL    bui.EV_ENTER    bui.EV_LEAVE

bui.CURSOR_DEFAULT   bui.CURSOR_HAND   bui.CURSOR_TEXT   bui.CURSOR_MOVE

bui.GEOM_NONE        bui.GEOM_PACK     bui.GEOM_GRID     bui.GEOM_PLACE
bui.MOD_SHIFT        bui.MOD_CTRL      bui.MOD_ALT

bui.WIDGET_BUTTON    bui.WIDGET_TOGGLE  bui.WIDGET_SLIDER
bui.WIDGET_TEXT      bui.WIDGET_CHECKBOX bui.WIDGET_RADIO
bui.WIDGET_LABEL     bui.WIDGET_SEPARATOR
```

### 4.2 Node Methods

```lua
-- Tree manipulation
node:add(child [, index])          -- add child; index -1 = append (default)
node:remove(child)                 -- detach child
node:child(index)                  -> child node (0-indexed)
node:children()                    -> {child1, child2, ...}
node:child_count()                 -> number
node:parent()                      -> parent node or nil
node:type()                        -> "button", "slider", etc.
node:rect()                        -> x, y, w, h
node:natural()                     -> w, h

-- Properties (auto-dispatch type)
node:set(key, value)               -- string / number / bool / table
node:get(key)                      -> value (number or string) or nil
node:get_color(key)                -> r, g, b, a

-- String lists (dropdown items, etc.)
node:set_strs(key, {str1, str2, ...})
node:get_strs(key)                 -> {str1, str2, ...}
node:strs_count(key)               -> number
node:strs_get(key, i)              -> string

-- Damage / layout
node:invalidate()                  -- mark for redraw
node:relayout()                    -- mark for relayout

-- Events
node:on(event_kind, callback)      -- attach event handler (callback=self,a,b)
node:off(event_kind)               -- detach

-- Geometry manager (child of a "frame")
node:geom(manager, opts)           -- set geom manager + options as table
node:geom_get(key)                 -> value

-- Images
node:set_image(rgba_data, w, h)          -- raw RGBA pixels (copied)
node:set_image_source(path, fit)         -- load from file via cache
node:set_imbuf(imbuf)                    -- attach a decoded BKImBuf
node:set_image_frame(frame)              -- animated: 0=play, n>0=frame n-1

-- Video
node:set_video(video, fit)               -- attach a BKVideo player

-- Textarea
node:text_at(x, y)                 -> byte offset
node:caret()                       -> caret position
node:set_caret(offset)
node:selection()                   -> start, end

-- Tree view
node:set_roots({item1, item2, ...})-- set root items
node:selected()                    -> BKTreeItem or nil
node:select(item)                  -- select an item
node:row_count()                   -> number of visible rows

-- Menus
node:set_menus({spec1, spec2, ...})-- [{text, icon, shortcut, fn, ...}]

-- Custom nodes
node:custom_set({event, frame, draw, free, natural_w, natural_h})
node:custom_data()                 -> user data pointer

-- Color ramp / curve / histogram
node:ramp_count()                  -> number of stops
node:ramp_get(i)                   -> pos, r, g, b, a
node:ramp_set({pos, rgba}, ...)
node:curve_get(channel, i)         -> x, y
node:curve_set(channel, {x,y}, ...)
node:histogram_set(bins, n, channels)

-- Lifecycle
node:free()                        -- explicitly free C node
```

### 4.3 Context Methods

```lua
ctx:set_header(node)               -- set fixed top bar node
ctx:root()                         -> root node

-- Rendering
ctx:render(w, h [, full])          -- render damage into context, returns region count
ctx:needs_render()                 -> bool

-- Input forwarding (for custom hosts / testing)
ctx:press(x, y [, mods])           -- returns bool (something changed)
ctx:release(x, y [, mods])
ctx:motion(x, y [, mods])
ctx:wheel(x, y, delta [, mods])
ctx:scroll(x, y, dx, dy [, mods])
ctx:key(text, keysym [, mods])
ctx:key_release(keysym [, mods])
ctx:button(x, y, button, release [, mods])
ctx:context_menu(x, y [, mods])

-- Utilities
ctx:timer(timer_id)                -- manual timer trigger
ctx:bind(combo, callback_fn)       -- keyboard shortcut ("ctrl+s")
ctx:close_popups()
ctx:focus()                        -> focused node or nil
ctx:hover()                        -> hovered node or nil
ctx:cursor()                       -> current cursor id
ctx:undo()
ctx:redo()

ctx:free()
```

### 4.4 Window Methods

```lua
win:show()
win:close()
win:is_open()                      -> bool
win:set_title(title)
win:set_size(w, h)
win:get_size()                     -> w, h
win:set_gpu(on)                    -- enable GPU rendering
win:render_now()                   -- force immediate render
win:timer(ms)                      -> id (one-shot timer)
win:cancel_timer(id)
win:on_close(callback)             -- return false to prevent close
win:on_frame(callback)             -- called before each render
win:handle()                       -> native handle (number)
win:wm(key, a, b)                  -- WM request
win:wm_get(key)                    -> a, b
win:free()
```

### 4.5 TreeItem Methods

```lua
item:add(child_item)               -- append to parent
item:remove(child_item)
item:clear()
item:child_count()                 -> number
item:child(i)                      -> item
item:set(key, value)               -- numeric properties
item:get(key)                      -> number
item:set_label(text)               -- display text
item:label()                       -> text
item:set_values({str1, ...})       -- column values
item:free()
```

### 4.6 ImBuf / Video Types

```lua
imbuf:size()                       -> w, h (and frame count)
imbuf:format()                     -> number (BK_IMAGE_*)
imbuf:pixels([frame])              -> pixel bytes (for custom use)
imbuf:free()

video:play()
video:pause()
video:seek(seconds)
video:set_rate(rate)
video:set_loop(bool)
video:set_poster(path)
video:set_controls(on)
video:state()                      -> BK_VIDEO_*
video:time()                       -> seconds
video:duration()                   -> seconds
video:size()                       -> w, h
video:free()
```

---

## 5. Lua API Usage Examples

### 5.1 Minimal App

```lua
-- hello.lua
local bui = require("bui")

local root = bui.Node("frame")
root:set("padding", 8)

local label = bui.Node("label")
label:set("text", "Hello from Lua!")
root:add(label)

local btn = bui.Node("button")
btn:set("text", "Click Me")
btn:on(bui.EV_CLICK, function()
  print("Button was clicked!")
end)
root:add(btn)
btn:geom(bui.GEOM_PACK, {fill = 1})

local ctx = bui.Context(root)
local win = bui.Window(ctx, "Lua BUI", 320, 120)
win:show()
bui.run()  -- blocks until window closes
```

### 5.2 Counter with Callbacks

```lua
-- counter.lua
local bui = require("bui")

local root = bui.Node("column")
root:set("padding", 12)

local label = bui.Node("label")
label:set("text", "Count: 0")
root:add(label)

local count = 0
local btn = bui.Node("button")
btn:set("text", "Increment")

btn:on(bui.EV_CLICK, function()
  count = count + 1
  label:set("text", "Count: " .. count)
end)

root:add(btn)
btn:geom(bui.GEOM_PACK, {fill = 1})

local reset = bui.Node("button")
reset:set("text", "Reset")
reset:on(bui.EV_CLICK, function()
  count = 0
  label:set("text", "Count: 0")
end)
root:add(reset)
reset:geom(bui.GEOM_PACK, {fill = 1})

local ctx = bui.Context(root)
local win = bui.Window(ctx, "Counter", 240, 150)
win:show()
bui.run()
```

### 5.3 Image Display

```lua
-- image_viewer.lua
local bui = require("bui")

local root = bui.Node("column")
root:set("padding", 8)

local img = bui.Node("image")
img:set_image_source("photo.jpg", bui.FIT_CONTAIN)
root:add(img)

local ctx = bui.Context(root)
local win = bui.Window(ctx, "Image Viewer", 800, 600)
win:show()
bui.run()
```

---

## 6. Event Loop Design

### 6.1 Script Mode (Blocking)

```c
// main.c
int main(int argc, char **argv) {
  lua_State *L = luaL_newstate();
  luaL_openlibs(L);

  luaL_requiref(L, "bui", luaopen_bui, 1);
  lua_pop(L, 1);

  if (argc > 1) {
    // Run script with args
    lua_createtable(L, argc - 2, 0);
    for (int i = 2; i < argc; i++) {
      lua_pushstring(L, argv[i]);
      lua_rawseti(L, -2, i - 1);
    }
    lua_setglobal(L, "arg");

    if (luaL_dofile(L, argv[1]) != LUA_OK) {
      fprintf(stderr, "bui: %s\n", lua_tostring(L, -1));
      lua_close(L);
      return 1;
    }
  } else {
    // REPL mode
    luabui_repl(L);
  }

  lua_close(L);
  return 0;
}
```

The `bui.run()` function calls `BK_window_run()` which blocks until all windows close:

```c
static int luabui_run(lua_State *L) {
  BK_window_run();
  return 0;
}
```

### 6.2 REPL Mode

Using bundled linenoise for line editing:

```c
static void luabui_repl(lua_State *L) {
  printf("BUI-Lua %d.%d.%d — Type 'exit' to quit\n",
         BUI_MAJOR, BUI_MINOR, BUI_PATCH);
  printf("  bui.Node, bui.Context, bui.Window, bui.run(), etc.\n\n");

  linenoiseHistorySetMaxLen(100);
  linenoiseHistoryLoad("~/.bui-lua-history");

  char *line;
  while ((line = linenoise("bui> ")) != NULL) {
    if (strcmp(line, "exit") == 0 || strcmp(line, "quit") == 0) {
      free(line);
      break;
    }
    linenoiseHistoryAdd(line);
    linenoiseHistorySave("~/.bui-lua-history");

    if (luaL_loadbuffer(L, line, strlen(line), "=stdin") != LUA_OK) {
      // Multi-line: detect incomplete input via "near '<eof>'" syntax error
      const char *msg = lua_tostring(L, -1);
      // Accumulate multi-line input if needed...
      fprintf(stderr, "error: %s\n", msg);
      lua_pop(L, 1);
      free(line);
      continue;
    }

    if (lua_pcall(L, 0, LUA_MULTRET, 0) != LUA_OK) {
      fprintf(stderr, "error: %s\n", lua_tostring(L, -1));
      lua_pop(L, 1);
    } else {
      // Print return values
      int n = lua_gettop(L);
      for (int i = 1; i <= n; i++) {
        // dispatch by type: string, number, table, userdata, etc.
      }
      lua_settop(L, 0);
    }
    free(line);
  }
  printf("\n");
}
```

### 6.3 Timer Queue

The `schedule` callback stores pending timers. A timer queue is checked on each pump cycle:

```c
typedef struct {
  int id;
  lua_Integer deadline_ms;
} PendingTimer;

// Stored in the context userdata or a global array
// Thread-local storage for the current lua_State
```

---

## 7. Memory Management

### 7.1 Node Lifecycle

- A `BKNode*` C node is owned by its parent in the tree
- When `node:add(child)` is called, the child C node becomes owned by the tree
- Lua userdata wrappers are **thin handles** — their `__gc` does NOT free the C node
- The C tree is freed when the root is freed or the context is freed
- The weak-keyed node map (`LUABUI_NODE_MAP` in registry) maps `BKNode*` → Lua userdata; when the userdata is GC'd, the entry is automatically removed by Lua's weak keys
- If a C node is freed while Lua still has a handle, subsequent method calls detect `ud->node == NULL` and return nil/raise

### 7.2 Callback Storage

- Event callbacks are stored in the **userdata's uservalue table**: `{ [BK_EV_CLICK] = fn, ... }`
- Lua owns these references — they are GC'd with the userdata
- `node:on(BK_EV_CLICK, fn)` → `uservalue[EV_CLICK] = fn`
- `node:off(BK_EV_CLICK)` → `uservalue[EV_CLICK] = nil`

### 7.3 Menu / Hotkey Callbacks

For `BK_EV_CALL`, the callback function is stored via `luaL_ref` (registry reference) in the `ref` field of `BKMenuSpec`. The reference is released when:
- The menu is closed (via the `BKContext` close sequence)
- The menu spec is replaced

---

## 8. Entry Point Design

### 8.1 CLI Interface

```
Usage: bui-lua [options] [script [args...]]

Options:
  -e <expr>       Execute inline Lua code
  --headless      Force software raster mode (no display needed)
  --theme <name>  Set initial theme (dark | light)
  --scale <n>     Set UI scale factor
  --version       Print version and exit
  --help          Print this help and exit

Script mode:
  bui-lua app.lua           Run app.lua (enters event loop on bui.run())
  bui-lua -e "code"         Run inline code, then exit
  bui-lua -e "code" -- args Pass args to inline code

REPL mode:
  bui-lua                   Interactive REPL prompt
```

### 8.2 main.c Flow

```
Parse args
  ├─ --version → print + exit
  ├─ --help    → print + exit
  └─ Recognize mode:
      ├─ script file arg[1]  → script mode
      ├─ -e expr             → eval mode
      └─ no args             → REPL mode

Initialize Lua state
Register bui module
Apply --theme / --scale if given
Set BUI_HEADLESS=1 if --headless

Execute mode:
  ├─ Script: set arg table, dofile, return
  ├─ Eval:   dostring, print results, return
  └─ REPL:   interactive loop

Cleanup (lua_close)
```

---

## 9. Implementation Phases

### Phase 1: Minimal Viable Product

| Step | File(s) | What |
|------|---------|------|
| 1 | `lua-5.4.7/` | Bundle Lua 5.4 source |
| 2 | `extern/linenoise/` | Bundle linenoise |
| 3 | `bindings/lua/CMakeLists.txt` | Build system linking Lua + BUI |
| 4 | `CMakeLists.txt` (top) | Add `BUI_BUILD_LUA_BINDINGS` option |
| 5 | `src/bui_internal.h` | Shared types and macros |
| 6 | `src/bui_lua.c` | Module init, constant registration |
| 7 | `src/bui_node.c` | BKNode wrapper (core subset: new, add, remove, set, get, free, on, off) |
| 8 | `src/bui_context.c` | BKContext wrapper, BKHost bridge (emit only) |
| 9 | `src/bui_window.c` | BKWindow wrapper (show, close, is_open, free) |
| 10 | `src/main.c` | CLI parsing, script mode, REPL mode |
| 11 | `examples/hello.lua` | Verification |

**Milestone:** `bui-lua examples/hello.lua` shows a clickable button in a native window.

### Phase 2: Events & Input

| Step | What |
|------|------|
| 1 | Full `host_emit` bridge with node lookup and Lua callback dispatch |
| 2 | Userdata uservalue for callback storage |
| 3 | Menu/hotkey `BK_EV_CALL` dispatch via `luaL_ref` |
| 4 | Context input forwarding methods (press, release, motion, key, wheel, scroll, etc.) |
| 5 | Timer/schedule support |
| 6 | Bind shortcut support |

**Milestone:** App with buttons, sliders, text fields, and keyboard shortcuts.

### Phase 3: Full API Coverage

| Step | What |
|------|------|
| 1 | Geometry manager API (pack, grid, place) |
| 2 | Image loading and display |
| 3 | Tree view with TreeItem |
| 4 | Menu bar, popup menus, pie menus |
| 5 | Canvas items |
| 6 | Color ramp / curve / histogram |
| 7 | Textarea operations (insert, delete, caret, selection, spans) |
| 8 | Custom nodes |
| 9 | File browser |
| 10 | `BKWindowCallbacks` (on_close, on_frame) |

**Milestone:** All major BUI features accessible from Lua.

### Phase 4: REPL & Polish

| Step | What |
|------|------|
| 1 | REPL with linenoise, history, multi-line |
| 2 | `-e` inline evaluation |
| 3 | `--headless` flag |
| 4 | Error handling improvements (pcall wrapping on all callbacks) |
| 5 | `--version`, `--help` flags |
| 6 | Headless raster type (`bui.Raster`) for testing |
| 7 | Comprehensive error messages |

**Milestone:** Full-featured REPL and CLI tool.

### Phase 5: Advanced (Future)

| Feature | Description |
|---------|-------------|
| Live REPL + windows | Non-blocking pump with `select`/`poll`, windows update while REPL runs |
| `--watch` | Auto-reload Lua script on file change |
| Hot-reload | Re-run Lua script without destroying windows |
| Lua module path | Set up `package.path` for user Lua modules |
| Package support | Require path pointing to a BUI component library |
| Screenshot API | `bui.screenshot(path)` via `BUI_raster_write_png` |
| `bui-luac` | Compile Lua + BUI into standalone executable |

---

## 10. Files to Create (Summary)

| File | Lines (est.) |
|------|-------------|
| `bindings/lua/CMakeLists.txt` | 60 |
| `bindings/lua/src/main.c` | 200 |
| `bindings/lua/src/bui_lua.c` | 350 |
| `bindings/lua/src/bui_node.c` | 650 |
| `bindings/lua/src/bui_context.c` | 500 |
| `bindings/lua/src/bui_window.c` | 200 |
| `bindings/lua/src/bui_image.c` | 150 |
| `bindings/lua/src/bui_internal.h` | 90 |
| `bindings/lua/examples/hello.lua` | 20 |
| `bindings/lua/examples/counter.lua` | 30 |
| `bindings/lua/examples/todos.lua` | 70 |
| `bindings/lua/examples/gallery.lua` | 90 |
| `bindings/lua/README.md` | 100 |
| `CMakeLists.txt` (top, +4 lines) | 4 |
| **Lua 5.4.7 source bundle** | ~20000 (vendored) |
| **linenoise source bundle** | ~500 (vendored) |

**C source total:** ~2200 lines (excluding vendored Lua)

---

## 11. Key Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Lua version | **5.4** (bundled) | Latest stable, small (~300KB), no system dep |
| Executable type | **Standalone binary** | Simple distribution, self-contained |
| C API used | **Full kit** (`BUI_kit.h`) | Complete access, same as Python/Rust bindings |
| Window hosting | **BKWindow** (native) | Free Win32/X11/Cocoa support |
| Event dispatch | **Weak node map + uservalue** | Automatic GC, no manual ref management |
| Node lifecycle | **Tree-ownership model** | Context frees whole tree; Lua wrappers are thin views |
| Event loop | **`BK_window_run()` blocks** | Simplest correct approach for v1 |
| REPL | **linenoise** | Single-file, MIT-licensed, cross-platform |
| Build system | **CMake** | Matches existing BUI project pattern |

---

## 12. Compatibility

| Platform | Native Windows | Software Raster | GPU | Status |
|----------|---------------|-----------------|-----|--------|
| **Linux (X11)** | ✅ | ✅ | ✅ | All features |
| **macOS** | ✅ (Cocoa) | ✅ | ✅ (OpenGL) | All features |
| **Windows** | ✅ (Win32) | ✅ | ✅ (D3D11) | All features |
| **Linux (Wayland)** | ❌ (no host yet) | ✅ | ✅ | Software raster only |
| **Headless (CI)** | ❌ | ✅ | ❌ | `--headless` flag |
