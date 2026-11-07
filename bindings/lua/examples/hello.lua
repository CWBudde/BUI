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
if not win then
    print("No window system available (display not found)")
    return
end

win:show()
-- Note: script blocks here until window closes
bui.run()
print("Window closed, goodbye!")
