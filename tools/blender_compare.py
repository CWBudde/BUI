"""Side-by-side check against Blender: BUI's components rebuilding a piece of
Blender's default UI, rendered headlessly to a PNG, next to the same piece
captured from a real Blender.

    python tools/blender_compare.py OUT_DIR [--blender PATH]

Writes OUT_DIR/bui_properties.png, and with --blender also
OUT_DIR/blender_properties.png (Blender's Properties editor, factory
settings, captured with screen.screenshot_area).

SPDX-License-Identifier: GPL-2.0-or-later
"""

import os
import struct
import subprocess
import sys
import tempfile
import zlib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "bindings", "python"))
sys.path.insert(0, os.path.join(ROOT, "build", "bindings", "python"))

# Blender's Properties editor at factory settings, 1600x900 window, 1x scale.
SIZE = (282, 680)


def write_png(path, w, h, rgb):
    raw = b"".join(b"\x00" + rgb[y * w * 3:(y + 1) * w * 3] for y in range(h))

    def chunk(kind, data):
        c = struct.pack(">I", len(data)) + kind + data
        return c + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)

    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n")
        f.write(chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0)))
        f.write(chunk(b"IDAT", zlib.compress(raw, 9)))
        f.write(chunk(b"IEND", b""))


def bui_properties(out):
    from bui.components import (App, Column, Decorator, Dropdown, IDSelector, Label, NumberField,
                                Panel, Row, SideTabs, Split)

    # uiItemFullR with use_property_split and decorators: an aligned row of
    # [split(0.4): label | value] and the decorator column
    def prop(label, comp):
        return Row(Split(Label(label, align="right"), comp, factor=0.4), Decorator(weight=0),
                   spacing=0)

    def xyz(name, value, suffix=""):
        # labels "Location X", "Y", "Z" | an aligned column of fields | decorators
        labels = Column(*[Label(f"{name} {a}" if i == 0 else a, align="right")
                          for i, a in enumerate("XYZ")], spacing=0)
        fields = Column(*[NumberField("", value, suffix=suffix) for _ in "XYZ"], align=True)
        decos = Column(*[Decorator(weight=0) for _ in "XYZ"], spacing=0, weight=0)
        return Row(Split(labels, fields, factor=0.4), decos, spacing=0)

    transform = Panel(
        "Transform",
        xyz("Location", 0.0, " m"),
        xyz("Rotation", 0.0, "°"),
        prop("Mode", Dropdown(items=["XYZ Euler", "Quaternion (WXYZ)", "Axis Angle"])),
        xyz("Scale", 1.0),
        Panel("Delta Transform", collapsed=True),
    )
    names = ["Relations", "Collections", "Instancing", "Motion Paths", "Shading", "Visibility",
             "Viewport Display", "Line Art", "Animation", "Custom Properties"]
    page = Column(IDSelector(["Cube"], "Cube", icon="OBJECT_DATA"), transform,
                  *[Panel(n, collapsed=True) for n in names], padding=8, spacing=2)
    # Blender's default Properties tabs, Object selected
    icons = [("TOOL_SETTINGS", "Tool"), ("SCENE", "Render"), ("OUTPUT", "Output"),
             ("RENDERLAYERS", "View Layer"), ("SCENE_DATA", "Scene"), ("WORLD", "World"),
             ("OUTLINER_COLLECTION", "Collection"), ("OBJECT_DATA", "Object"),
             ("MODIFIER", "Modifiers"), ("PARTICLES", "Particles"), ("PHYSICS", "Physics"),
             ("CONSTRAINT", "Constraints"), ("MESH_DATA", "Data"), ("MATERIAL", "Material"),
             ("TEXTURE", "Texture")]
    entries = [(i, t, page if t == "Object" else Column()) for i, t in icons]
    # Blender's groups: Tool | Render..World | Collection | Object..Texture
    entries[1:1], entries[7:7], entries[9:9] = [None], [None], [None]
    tabs = SideTabs(entries, current=7, fill=True)
    app = App("Properties", SIZE, tabs, padding=0)
    raster = app.render_raster(*SIZE, full=True)
    ppm = raster.ppm(0, 0, *SIZE)
    header_end = ppm.index(b"255\n") + 4
    write_png(out, SIZE[0], SIZE[1], ppm[header_end:])


def bui_sidebar(out, size):
    from bui.components import (App, Column, Decorator, Dropdown, Label, NumberField, Panel, Row,
                                Sidebar, Split)

    def xyz(name, value, suffix=""):
        labels = Column(*[Label(f"{name} {a}" if i == 0 else a, align="right")
                          for i, a in enumerate("XYZ")], spacing=0)
        fields = Column(*[NumberField("", value, suffix=suffix) for _ in "XYZ"], align=True)
        return Split(labels, fields, factor=0.4)

    transform = Panel("Transform", xyz("Location", 0.0, " m"), xyz("Rotation", 0.0, "°"),
                      Split(Label("Mode", align="right"), Dropdown(items=["XYZ Euler"]), factor=0.4),
                      xyz("Scale", 1.0), xyz("Dimensions", 2.0, " m"), category="Item")
    side = Sidebar(transform, Panel("Options", category="Tool"), Panel("View", category="View"),
                   fill=True)
    app = App("Sidebar", size, side, padding=0)
    raster = app.render_raster(*size, full=True)
    ppm = raster.ppm(0, 0, *size)
    write_png(out, size[0], size[1], ppm[ppm.index(b"255\n") + 4:])


BLENDER_SIDEBAR = r"""
import bpy, sys
out = sys.argv[sys.argv.index('--') + 1]
def shot():
    win = bpy.context.window_manager.windows[0]
    area = next(a for a in win.screen.areas if a.type == 'VIEW_3D')
    area.spaces[0].show_region_ui = True
    def later():
        region = next(r for r in area.regions if r.type == 'UI')
        print("SIDEBAR", region.width, region.height)
        with bpy.context.temp_override(window=win, screen=win.screen, area=area, region=region):
            bpy.ops.screen.screenshot_area(filepath=out)
        bpy.ops.wm.quit_blender()
    bpy.app.timers.register(later, first_interval=1.0)
bpy.app.timers.register(shot, first_interval=2.5)
"""


BLENDER_SCRIPT = r"""
import bpy, sys
out = sys.argv[sys.argv.index('--') + 1]
def shot():
    win = bpy.context.window_manager.windows[0]
    area = next(a for a in win.screen.areas if a.type == 'PROPERTIES')
    region = next(r for r in area.regions if r.type == 'WINDOW')
    with bpy.context.temp_override(window=win, screen=win.screen, area=area, region=region):
        bpy.ops.screen.screenshot_area(filepath=out)
    bpy.ops.wm.quit_blender()
bpy.app.timers.register(shot, first_interval=2.5)
"""


def blender_properties(blender, out):
    with tempfile.NamedTemporaryFile("w", suffix=".py", delete=False) as f:
        f.write(BLENDER_SCRIPT)
    try:
        subprocess.run([blender, "--factory-startup", "--window-geometry", "0", "0", "1600", "900",
                        "--python", f.name, "--", out], check=False, capture_output=True, timeout=120)
    finally:
        os.unlink(f.name)


def blender_run(blender, script, out):
    with tempfile.NamedTemporaryFile("w", suffix=".py", delete=False) as f:
        f.write(script)
    try:
        r = subprocess.run([blender, "--factory-startup", "--window-geometry", "0", "0", "1600", "900",
                            "--python", f.name, "--", out], check=False, capture_output=True,
                           text=True, timeout=120)
        return r.stdout
    finally:
        os.unlink(f.name)


def main():
    out_dir = sys.argv[1] if len(sys.argv) > 1 else "."
    os.makedirs(out_dir, exist_ok=True)
    bui_properties(os.path.join(out_dir, "bui_properties.png"))
    bui_sidebar(os.path.join(out_dir, "bui_sidebar.png"), (280, 560))
    if "--blender" in sys.argv:
        log = blender_run(sys.argv[sys.argv.index("--blender") + 1], BLENDER_SIDEBAR,
                          os.path.join(out_dir, "blender_view3d.png"))
        print([l for l in log.splitlines() if l.startswith("SIDEBAR")])
        blender_properties(sys.argv[sys.argv.index("--blender") + 1],
                           os.path.join(out_dir, "blender_properties.png"))
    print("wrote", out_dir)


if __name__ == "__main__":
    main()
