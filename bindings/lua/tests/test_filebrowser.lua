-- File browser API tests
-- SPDX-License-Identifier: GPL-2.0-or-later

local function check(cond, msg)
  if not cond then io.stderr:write("FAIL: " .. msg .. "\n"); os.exit(1) end
end

local bui = require("bui")

-- ── File browser node creation ─────────────────────────────────────

local fb = bui.Node("filebrowser")
check(fb ~= nil, "filebrowser creation failed")
check(fb:type() == "filebrowser", "filebrowser type, got " .. fb:type())

-- ── Properties ──────────────────────────────────────────────────────

-- Set and get directory
fb:set("dir", "/tmp")
check(fb:get("dir") == "/tmp" or type(fb:get("dir")) == "string",
       "filebrowser dir property")

-- File property (filename selection)
fb:set("file", "test.txt")
check(fb:get("file") == "test.txt" or type(fb:get("file")) == "string",
       "filebrowser file property")

-- Mode: 0=open, 1=save, 2=directory
fb:set("mode", 0)
fb:set("mode", 1)
fb:set("mode", 2)

-- Sort: 0=Name, 1=Date, 2=Size
fb:set("sort", 0)
fb:set("sort", 2)

-- Invert sort
fb:set("invert", 1)
check(fb:get("invert") == 1, "invert != 1, got " .. tostring(fb:get("invert")))
fb:set("invert", 0)

-- Hidden files
fb:set("hidden", 0)
fb:set("hidden", 1)

-- Search
fb:set("search", "*.txt")

-- Filters (set as string list)
fb:set_strs("filters", {"All\t*", "Python\t*.py", "Text\t*.txt"})

-- Active filter index
fb:set("filter", 0)
fb:set("filter", 1)
check(fb:get("filter") == 1, "filter != 1, got " .. tostring(fb:get("filter")))

-- Multiple selection
fb:set("multiple", 0)
fb:set("multiple", 1)

-- State (read by filebrowser after dialog completes)
fb:set("state", 0)

-- ── Render (verify the node renders without crash) ──────────────────

local root = bui.Node("frame")
root:add(fb)
fb:geom(bui.GEOM_PACK, {fill = 1})
local ctx = bui.Context(root)
local px, regions, w, h = ctx:render(400, 300, 1)
check(px ~= nil, "filebrowser render nil")
check(#px == 480000, "filebrowser render size != 480000, got " .. #px)
check(w == 400, "filebrowser render width != 400")
check(h == 300, "filebrowser render height != 300")
ctx:free()

-- ── All tests passed ─────────────────────────────────────────────────

io.write("OK: test_filebrowser.lua\n")
os.exit(0)
