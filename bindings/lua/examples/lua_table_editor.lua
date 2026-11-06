-- Lua Table Editor for bui-lua
-- Navigate, edit, and export any Lua table using BUI tree view.
--
-- Usage:  bui-lua bindings/lua/examples/lua_table_editor.lua
--
-- SPDX-License-Identifier: GPL-2.0-or-later

local bui = require("bui")

-- ============================================================
-- HELPERS
-- ============================================================

local function sorted_pairs(t)
  local keys = {}
  for k in pairs(t) do keys[#keys + 1] = k end
  table.sort(keys, function(a, b) return tostring(a) < tostring(b) end)
  local i = 0
  return function()
    i = i + 1; local k = keys[i]
    if k == nil then return nil end
    return k, t[k]
  end
end

local function count_items(t)
  local n = 0
  for _ in pairs(t) do n = n + 1 end
  return n
end

local function lua_type_name(v)
  local t = type(v)
  if t ~= "table" then return t end
  local n = count_items(v)
  if n == 0 then return "table" end
  -- check if array
  for i = 1, n do if v[i] == nil then return "table" end end
  return "array"
end

local function fmt_val(v)
  local t = type(v)
  if t == "string"  then return string.format("%q", v)
  elseif t == "number" then
    if v == math.floor(v) and math.abs(v) < 1e15 then return tostring(v)
    else return string.format("%.14g", v) end
  elseif t == "boolean" then return v and "true" or "false"
  elseif t == "nil"   then return "nil"
  elseif t == "table" then return "{" .. count_items(v) .. " items}"
  else return "(" .. t .. ")" end
end

-- ============================================================
-- DEMO TABLES
-- ============================================================

local DEMOS = {}

DEMOS["Person Record"] = {
  name = "Alice Smith",
  age = 30,
  email = "alice@example.com",
  active = true,
  score = 95.5,
  tags = {"developer", "designer", "rust"},
  address = {
    street = "123 Main St",
    city = "Portland",
    state = "OR",
    zip = 97201,
    coordinates = {lat = 45.5152, lon = -122.6784}
  },
  metadata = nil
}

DEMOS["Project Config"] = {
  name = "bui-lua",
  version = "0.1.0",
  debug = false,
  features = {
    images = true,
    video = false,
    gpu = true,
    limits = {
      max_windows = 10,
      max_fps = 60,
      cache_size_mb = 256
    }
  },
  paths = {
    data = "/usr/local/share/bui",
    cache = "/tmp/bui-cache",
    logs = "/var/log/bui"
  },
  users = {
    {id = 1, name = "admin", role = "owner"},
    {id = 2, name = "guest", role = "viewer"}
  }
}

DEMOS["Game State"] = {
  player = {
    name = "Hero",
    level = 12,
    health = 85,
    max_health = 100,
    mana = 42,
    max_mana = 80,
    inventory = {"sword", "shield", "potion", "ring", "map"},
    stats = {
      strength = 15,
      dexterity = 12,
      intelligence = 8,
      vitality = 14
    },
    position = {x = 120.5, y = 340.2, z = 0}
  },
  quests = {
    active = {"find_the_artifact", "defeat_the_dragon"},
    completed = {"tutorial", "first_blood", "side_quest_01"},
    failed = {}
  },
  settings = {
    volume = 0.8,
    fullscreen = true,
    resolution = {width = 1920, height = 1080},
    controls = {
      move_forward = "w",
      move_back = "s",
      jump = "space",
      interact = "e"
    }
  }
}

-- ============================================================
-- DATA MODEL
-- ============================================================

local data       = nil             -- current table being edited
local id_counter = 1000
local item_meta  = {}              -- id -> {type, key, parent, value}

local function next_id()
  id_counter = id_counter + 1
  return id_counter
end

-- ============================================================
-- TREE BUILDING
-- ============================================================

local function build_subtree(parent_item, t, parent_data)
  for k, v in sorted_pairs(t) do
    local key_str = tostring(k)
    local vt = lua_type_name(v)
    local item = bui.TreeItem(key_str, 0)
    item:set_values({key_str, vt, fmt_val(v)})
    local id = next_id()
    item:set("_id", id)
    item_meta[id] = {
      type   = vt,
      key    = k,
      value  = v,
      parent = parent_data,
      id     = id,
    }
    if type(v) == "table" then
      build_subtree(item, v, v)
    end
    if parent_item then
      parent_item:add(item)
    end
  end
end

local function rebuild_tree(tree, source, meta_clean)
  if meta_clean then item_meta = {} end
  id_counter = 1000
  local roots = {}
  build_subtree(nil, source, source)
  local items = {}
  -- Collect root items: items whose parent is `source`
  for k, v in sorted_pairs(source) do
    local key_str = tostring(k)
    local vt = lua_type_name(v)
    local item = bui.TreeItem(key_str, 0)
    item:set_values({key_str, vt, fmt_val(v)})
    local id = next_id()
    item:set("_id", id)
    item_meta[id] = {
      type   = vt,
      key    = k,
      value  = v,
      parent = source,
      id     = id,
    }
    if type(v) == "table" then
      build_subtree(item, v, v)
    end
    items[#items + 1] = item
  end
  tree:set_roots(items)
  tree:relayout()
end

-- ============================================================
-- SERIALIZATION
-- ============================================================

local function serialize(v, indent)
  indent = indent or ""
  local t = type(v)
  if t == "string" then return string.format("%q", v)
  elseif t == "number" then
    if v == math.floor(v) and math.abs(v) < 1e15 then return tostring(v)
    else return string.format("%.14g", v) end
  elseif t == "boolean" then return v and "true" or "false"
  elseif t == "nil"   then return "nil"
  elseif t ~= "table" then return tostring(v) end

  -- table
  local n = count_items(v)
  if n == 0 then return "{}" end
  local is_arr = true
  for i = 1, n do if v[i] == nil then is_arr = false; break end end

  local sub = indent .. "  "
  local parts = {}
  for k, val in sorted_pairs(v) do
    local line
    if is_arr then
      line = serialize(val, sub)
    elseif type(k) == "string" and k:match("^[%a_][%w_]*$") then
      line = k .. " = " .. serialize(val, sub)
    else
      local ks = type(k) == "string" and string.format("[%q]", k) or "[" .. tostring(k) .. "]"
      line = ks .. " = " .. serialize(val, sub)
    end
    parts[#parts + 1] = line
  end
  if #parts <= 3 and #table.concat(parts, ", ") < 60 then
    return "{ " .. table.concat(parts, ", ") .. " }"
  end
  return "{\n" .. sub .. table.concat(parts, ",\n" .. sub) .. "\n" .. indent .. "}"
end

-- ============================================================
-- UI STATE
-- ============================================================

local tree_node     = nil
local editor_key    = nil
local editor_type   = nil
local editor_val    = nil
local editor_bool   = nil
local val_label     = nil
local status_node   = nil
local demo_selector = nil
local current_demo  = ""
local edit_id       = nil   -- id of item being edited

-- ============================================================
-- EDITOR ACTIONS
-- ============================================================

local function refresh_editor()
  if not tree_node then return end
  local sel = tree_node:selected()
  if not sel then
    edit_id = nil
    if editor_key then editor_key:set("value", "") end
    if editor_type then editor_type:set("value", 0) end
    if editor_val then
      editor_val:set("value", "")
      editor_val:set("enabled", 0)
    end
    if editor_bool then
      editor_bool:set("value", 0)
      editor_bool:set("enabled", 0)
    end
    if val_label then val_label:set("text", "Value:") end
    return
  end

  local id = sel:get("_id")
  local meta = item_meta[id]
  if not meta then edit_id = nil; return end
  edit_id = id

  if editor_key then editor_key:set("value", tostring(meta.key)) end

  local type_names = {"string", "number", "boolean", "table", "array", "nil"}
  local type_idx = 0
  for i, tn in ipairs(type_names) do
    if tn == meta.type then type_idx = i - 1; break end
  end
  if editor_type then editor_type:set("value", type_idx) end

  -- Show/hide value vs bool widgets based on type
  if meta.type == "boolean" then
    if val_label then val_label:set("text", "Value:") end
    if editor_val then
      editor_val:set("value", meta.value and "true" or "false")
      editor_val:set("enabled", 1)
    end
    if editor_bool then
      editor_bool:set("value", meta.value and 1 or 0)
      editor_bool:set("enabled", 1)
    end
  elseif meta.type == "string" or meta.type == "number" then
    if val_label then val_label:set("text", "Value:") end
    if editor_val then
      local sv
      if meta.type == "string" then sv = meta.value or ""
      else sv = tostring(meta.value or 0) end
      editor_val:set("value", sv)
      editor_val:set("enabled", 1)
    end
    if editor_bool then
      editor_bool:set("enabled", 0)
    end
  else
    if val_label then val_label:set("text", "(read-only)") end
    if editor_val then
      editor_val:set("value", "")
      editor_val:set("enabled", 0)
    end
    if editor_bool then editor_bool:set("enabled", 0) end
  end
end

local function apply_edit()
  if not edit_id then return end
  local meta = item_meta[edit_id]
  if not meta then return end

  local parent = meta.parent
  local old_key = meta.key

  -- Read new key
  local new_key_str = editor_key:get("value")
  if not new_key_str or new_key_str == "" then return end

  -- Determine new type from dropdown
  local type_idx = editor_type:get("value")
  local type_names = {"string", "number", "boolean", "table", "array", "nil"}
  local new_type = type_names[type_idx + 1] or "string"

  -- Parse value
  local new_val
  if new_type == "string" then
    new_val = editor_val:get("value") or ""
  elseif new_type == "number" then
    new_val = tonumber(editor_val:get("value")) or 0
  elseif new_type == "boolean" then
    new_val = editor_bool:get("value") > 0
  elseif new_type == "table" or new_type == "array" then
    new_val = {}
  elseif new_type == "nil" then
    new_val = nil
  end

  -- Determine final key type
  local new_key = tonumber(new_key_str) or new_key_str

  -- Update parent table
  if old_key ~= new_key then
    parent[old_key] = nil
  end
  parent[new_key] = new_val

  rebuild_tree(tree_node, data)
  edit_id = nil
  if status_node then
    status_node:set("text", "Applied. Entries: " .. count_items(data))
  end
end

local function delete_selected()
  if not edit_id then return end
  local meta = item_meta[edit_id]
  if not meta then return end
  meta.parent[meta.key] = nil
  rebuild_tree(tree_node, data)
  edit_id = nil
  if status_node then
    status_node:set("text", "Deleted. Entries: " .. count_items(data))
  end
end

local function add_row()
  local target = data
  if edit_id then
    local meta = item_meta[edit_id]
    if meta and (meta.type == "table" or meta.type == "array") then
      target = meta.value
    end
  end
  local base = "new"
  local i = 1
  while target[base .. i] ~= nil do i = i + 1 end
  target[base .. i] = ""
  rebuild_tree(tree_node, data)
  edit_id = nil
  if status_node then
    status_node:set("text", "Added entry. Entries: " .. count_items(data))
  end
end

local function export_table()
  print("\n=== Exported: " .. current_demo .. " ===")
  print(serialize(data))
  print("=== End ===\n")
end

local function switch_demo(name, t)
  current_demo = name
  data = t
  item_meta = {}
  rebuild_tree(tree_node, data)
  if status_node then
    status_node:set("text", "Loaded: " .. name .. " | Entries: " .. count_items(data))
  end
  edit_id = nil
  if editor_key then editor_key:set("value", "") end
  if editor_val then editor_val:set("value", "") end
end

-- ============================================================
-- BUILD THE UI
-- ============================================================

local function build_ui()
  local root = bui.Node("column")
  root:set("padding", 6)

  -- === Toolbar ===
  local tb = bui.Node("row")
  tb:set("padding", 4)

  local title_lbl = bui.Node("label")
  title_lbl:set("text", "Lua Table Editor")
  tb:add(title_lbl)

  -- Demo selector dropdown
  local demo_dd = bui.Node("dropdown")
  local demo_names = {}
  for name in pairs(DEMOS) do demo_names[#demo_names + 1] = name end
  table.sort(demo_names)
  demo_dd:set_strs("items", demo_names)
  demo_dd:on(bui.EV_CHANGE, function()
    local idx = demo_dd:get("value")
    local name = demo_names[idx + 1]
    if name then switch_demo(name, DEMOS[name]) end
  end)
  tb:add(demo_dd)
  demo_dd:geom(bui.GEOM_PACK, {fill = 1, side = 1})

  local add_btn = bui.Node("button")
  add_btn:set("text", "+ Add")
  add_btn:on(bui.EV_CLICK, add_row)
  tb:add(add_btn)
  add_btn:geom(bui.GEOM_PACK, {side = 1})

  local del_btn = bui.Node("button")
  del_btn:set("text", "Delete")
  del_btn:on(bui.EV_CLICK, delete_selected)
  tb:add(del_btn)
  del_btn:geom(bui.GEOM_PACK, {side = 1})

  local exp_btn = bui.Node("button")
  exp_btn:set("text", "Export")
  exp_btn:on(bui.EV_CLICK, export_table)
  tb:add(exp_btn)
  exp_btn:geom(bui.GEOM_PACK, {side = 1})

  root:add(tb)

  -- === Main: tree + editor side panel ===
  local main_row = bui.Node("row")
  root:add(main_row)

  -- Tree view
  local tree = bui.Node("tree")
  tree:set_strs("headers", {"Key", "Type", "Value"})
  tree:set_strs("columns", {"0", "2", "1", "3"})
  tree:on(bui.EV_ACTIVATE, function() refresh_editor() end)
  tree:on(bui.EV_CHANGE,   function() refresh_editor() end)
  main_row:add(tree)
  tree_node = tree

  -- Editor side panel
  local ep = bui.Node("column")
  ep:set("padding", 6)

  local ep_header = bui.Node("label")
  ep_header:set("text", "Edit Entry")
  ep:add(ep_header)

  -- Key
  local kl = bui.Node("label")
  kl:set("text", "Key:")
  ep:add(kl)
  editor_key = bui.Node("text")
  editor_key:set("placeholder", "key name")
  ep:add(editor_key)

  -- Type
  local tl = bui.Node("label")
  tl:set("text", "Type:")
  ep:add(tl)
  editor_type = bui.Node("dropdown")
  editor_type:set_strs("items", {"string", "number", "boolean", "table", "array", "nil"})
  editor_type:on(bui.EV_CHANGE, function()
    -- Refresh the value widget when type changes
    refresh_editor()
  end)
  ep:add(editor_type)

  -- Value
  val_label = bui.Node("label")
  val_label:set("text", "Value:")
  ep:add(val_label)

  editor_val = bui.Node("text")
  editor_val:set("placeholder", "value")
  editor_val:on(bui.EV_SUBMIT, function()
    -- Press Enter in value field → apply
    apply_edit()
  end)
  ep:add(editor_val)

  -- Boolean toggle
  local bl = bui.Node("label")
  bl:set("text", "Toggle:")
  ep:add(bl)
  editor_bool = bui.Node("toggle")
  editor_bool:set("text", "Enabled")
  editor_bool:set("value", 0)
  ep:add(editor_bool)

  -- Apply
  local apply_btn = bui.Node("button")
  apply_btn:set("text", "Apply")
  apply_btn:on(bui.EV_CLICK, apply_edit)
  ep:add(apply_btn)
  apply_btn:geom(bui.GEOM_PACK, {fill = 1, pady0 = 4})

  local clear_btn = bui.Node("button")
  clear_btn:set("text", "Clear Editor")
  clear_btn:on(bui.EV_CLICK, function()
    edit_id = nil
    editor_key:set("value", "")
    editor_type:set("value", 0)
    editor_val:set("value", "")
    editor_bool:set("value", 0)
  end)
  ep:add(clear_btn)
  clear_btn:geom(bui.GEOM_PACK, {fill = 1, pady0 = 2})

  main_row:add(ep)

  -- === Status bar ===
  status_node = bui.Node("label")
  status_node:set("text", "Select a demo to begin")
  root:add(status_node)

  -- === Initial load ===
  local first_name = demo_names[1]
  switch_demo(first_name, DEMOS[first_name])
  if demo_dd then demo_dd:set("value", 0) end

  return root
end

-- ============================================================
-- MAIN
-- ============================================================

print("Lua Table Editor v0.1")
print("3 demos: Person Record, Project Config, Game State")
print("Select a demo from the dropdown, then edit entries.")
print("Export prints serialized Lua to stdout.\n")

local ui_root = build_ui()
local ctx = bui.Context(ui_root)
local win = bui.Window(ctx, "Lua Table Editor", 800, 550)

if win then
  win:show()
  bui.run()
  print("Editor closed.")
else
  print("No window system available. Printing demo tables:\n")
  for name, t in sorted_pairs(DEMOS) do
    print("=== " .. name .. " ===")
    print(serialize(t))
    print()
  end
end
