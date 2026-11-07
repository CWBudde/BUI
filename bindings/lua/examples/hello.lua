-- Hello BUI from Lua
local bui = require("bui")

local root = bui.Node("frame")
root:set("padding", 8)

local btn = bui.Node("button")
btn:set("text", "Click Me!")
btn:on(bui.EV_CLICK, function(self)
    print("Button clicked!")
end)
root:add(btn)
btn:geom(bui.GEOM_PACK, {fill = 1})

local ctx = bui.Context(root)

if not bui.Window then
    print("Window type not available (headless mode)")
    return
end

local win = bui.Window(ctx, "Hello from Lua", 320, 120)
if win then
    win:show()
    bui.run()
    print("Window closed, goodbye!")
else
    local px, _, w, h = ctx:render(320, 120, 1)
    print("No window system. Headless render: " .. w .. "x" .. h .. " " .. #px .. " bytes")
end
