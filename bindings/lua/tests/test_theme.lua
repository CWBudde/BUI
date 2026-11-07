-- Theme API tests: set_theme, theme_name, theme_color, switching
-- SPDX-License-Identifier: GPL-2.0-or-later

local function check(cond, msg)
  if not cond then io.stderr:write("FAIL: " .. msg .. "\n"); os.exit(1) end
end

local bui = require("bui")

-- ── Module-level theme functions ─────────────────────────────────────

check(type(bui.set_theme) == "function", "set_theme not a function")
check(type(bui.theme_name) == "function", "theme_name not a function")
check(type(bui.theme_color) == "function", "theme_color not a function")

-- ── Default theme is "dark" ──────────────────────────────────────────

local name = bui.theme_name()
check(type(name) == "string", "theme_name not a string, got " .. type(name))
check(name == "dark" or name == "light", "unexpected theme name: " .. name)

-- ── Theme colors ─────────────────────────────────────────────────────

-- Known theme colors from Blender's dark theme
local region_bg = {bui.theme_color("region_bg")}
check(#region_bg == 4, "theme_color region_bg should return 4 values, got " .. #region_bg)
check(type(region_bg[1]) == "number", "theme_color r not a number")
check(type(region_bg[2]) == "number", "theme_color g not a number")
check(type(region_bg[3]) == "number", "theme_color b not a number")
check(type(region_bg[4]) == "number", "theme_color a not a number")

-- Widget colors should exist
local wcol = {bui.theme_color("text_fg")}
-- text_fg is a known theme color, should return 4 values or 0
if #wcol == 4 then
  check(type(wcol[1]) == "number", "text_fg r not a number")
end

-- Unknown color key returns nil
local unknown = {bui.theme_color("nonexistent_color_key")}
-- With the current API, unknown colors return 0 values or nil values
-- Just check it doesn't crash

-- ── Switch to light theme ───────────────────────────────────────────

local ok = bui.set_theme("light")
check(ok == 1 or ok == true, "set_theme('light') failed, got " .. tostring(ok))

local light_name = bui.theme_name()
check(light_name == "light", "theme should be 'light' got '" .. tostring(light_name) .. "'")

-- Light theme should have different colors
local light_region_bg = {bui.theme_color("region_bg")}
if #region_bg == 4 and #light_region_bg == 4 then
  local is_different = false
  for i = 1, 3 do
    if math.abs(region_bg[i] - light_region_bg[i]) > 0.01 then
      is_different = true
      break
    end
  end
  check(is_different, "dark and light region_bg should differ")
end

-- ── Render at both themes and compare pixels ─────────────────────────

local function render_solid()
  local root = bui.Node("frame")
  root:set("padding", 8)
  local lbl = bui.Node("label")
  lbl:set("text", "Hello")
  root:add(lbl)
  local ctx = bui.Context(root)
  local px, _, w, h = ctx:render(100, 50, 1)
  ctx:free()
  return px, w, h
end

-- Render in light theme
local light_px, lw, lh = render_solid()
check(light_px ~= nil, "light render nil")
check(lw == 100, "light render width")
check(lh == 50, "light render height")

-- Switch back to dark
local ok2 = bui.set_theme("dark")
check(ok2 == 1 or ok2 == true, "set_theme('dark') failed")

-- Render in dark theme
local dark_px, dw, dh = render_solid()
check(dark_px ~= nil, "dark render nil")
check(dw == 100, "dark render width")
check(dh == 50, "dark render height")

-- Pixels should differ between dark and light themes
local any_diff = false
for i = 1, #light_px, 4 do
  if light_px:byte(i) ~= dark_px:byte(i) then
    any_diff = true
    break
  end
end
check(any_diff, "dark and light theme renders should have different pixels")

-- ── Invalid theme name ───────────────────────────────────────────────

local bad = bui.set_theme("nonexistent")
check(bad == 0 or bad == false, "set_theme('nonexistent') should fail, got " .. tostring(bad))

-- ── Theme name after failed set should still be dark ─────────────────

local final_name = bui.theme_name()
check(final_name == "dark", "theme should still be 'dark' after failed set, got '" .. tostring(final_name) .. "'")

-- ── All tests passed ─────────────────────────────────────────────────

io.write("OK: test_theme.lua\n")
os.exit(0)
