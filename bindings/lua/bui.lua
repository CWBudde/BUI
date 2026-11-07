--- BUI Lua binding — idiomatic wrapper on top of the C module.
--- Re-exports everything from `_bui` (the C module) and adds table-based
--- constructors like `bui.Button{ text = "...", on_click = fn }`.
---
--- SPDX-License-Identifier: GPL-2.0-or-later
-- When loaded inside the bui-lua executable, the C module is registered as
-- "bui" directly (no loadable module). Fall back gracefully.
local ok, _bui = pcall(require, "_bui")
if not ok then _bui = require("bui") end

-- Collect the C keys first to avoid modifying the table during iteration
local c_keys = {}
for k in pairs(_bui) do c_keys[#c_keys + 1] = k end
for _, k in ipairs(c_keys) do _bui[k] = _bui[k] end

-- Widget constructor table: bui.Button{ text="...", on_click=fn, ... }
-- Maps widget type names to the opts they accept.
local WIDGET_OPTS = {
  button    = {"text", "enabled", "icon"},
  toggle    = {"text", "value", "enabled"},
  checkbox  = {"text", "value", "enabled"},
  radio     = {"text", "value", "enabled"},
  label     = {"text", "value", "icon"},
  text      = {"value", "placeholder", "enabled", "password"},
  search    = {"value", "placeholder"},
  hotkey    = {"placeholder"},
  number    = {"value", "min", "max", "step", "label", "enabled"},
  slider    = {"value", "min", "max", "step", "label", "enabled"},
  dropdown  = {"items", "value", "enabled"},
  enum      = {"items", "value"},
  swatch    = {"color", "enabled"},
  progress  = {"value"},
  meter     = {"value"},
  decorator = {"text"},
  panel     = {"text", "collapsed"},
  tabs      = {"items"},
  scroll    = {"padding"},
  separator = {},
  frame     = {"padding"},
  column    = {"padding"},
  row       = {"padding"},
  box       = {"padding"},
  image     = {"source", "fit"},
  list      = {"items", "rows"},
  tree      = {"headers", "columns"},
}

-- Event names that map to bui.EV_* constants
local EVENTS = {
  on_change    = "EV_CHANGE",
  on_click     = "EV_CLICK",
  on_press     = "EV_PRESS",
  on_release   = "EV_RELEASE",
  on_key       = "EV_KEY",
  on_submit    = "EV_SUBMIT",
  on_activate  = "EV_ACTIVATE",
  on_toggle    = "EV_TOGGLE",
  on_scroll    = "EV_SCROLL",
  on_wheel     = "EV_WHEEL",
  on_enter     = "EV_ENTER",
  on_leave     = "EV_LEAVE",
  on_motion    = "EV_MOTION",
  on_browse    = "EV_BROWSE",
}

-- Metatable for wrapper constructors
local WidgetMt = {}
function WidgetMt.__call(cls, opts)
  opts = opts or {}
  local node = _bui.Node(cls._type)
  if not node then return nil end

  -- Set known properties from opts
  for _, key in ipairs(cls._opts or WIDGET_OPTS[cls._type] or {}) do
    if opts[key] ~= nil then
      if type(opts[key]) == "table" and key ~= "color" then
        node:set_strs(key, opts[key])
      else
        node:set(key, opts[key])
      end
    end
  end

  -- Attach event handlers
  for opt_key, ev_name in pairs(EVENTS) do
    if opts[opt_key] then
      node:on(_bui[ev_name], opts[opt_key])
    end
  end

  -- Children: add each as a child
  if opts.children then
    for _, child in ipairs(opts.children) do
      if type(child) == "userdata" then
        node:add(child)
      end
    end
  end

  return node
end

-- Build factory functions for each widget type
local function add_widget(name)
  local cls = {_type = name, _opts = WIDGET_OPTS[name] or {}}
  setmetatable(cls, WidgetMt)
  _bui[name:gsub("^%l", string.upper)] = cls
end

for name in pairs(WIDGET_OPTS) do
  add_widget(name)
end

-- Special: Context + Window constructor
function _bui.open(opts)
  opts = opts or {}
  local root = opts.root or opts.node
  if not root then return nil, "no root node" end
  local ctx = _bui.Context(root)
  if not ctx then return nil, "context creation failed" end
  if opts.bind then
    for combo, fn in pairs(opts.bind) do
      ctx:bind(combo, fn)
    end
  end
  local title = opts.title or "BUI"
  local w, h = opts.width or 640, opts.height or 480
  local win = _bui.Window(ctx, title, w, h)
  if not win then return nil, "no window system" end
  win:show()
  if opts.on_frame then
    win:on_frame(opts.on_frame)
  end
  if not opts.no_run then
    _bui.run()
  end
  return win, ctx
end

return _bui
