"""Blender components: a Geometry Nodes editor (scatter rocks on a terrain).

    python examples/site/geometry_nodes.py

Drag sockets to link, Ctrl+right-drag to cut, middle-drag to pan, wheel to zoom.

SPDX-License-Identifier: GPL-2.0-or-later
"""

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..",
                                "bindings", "python"))

from bui.components import (App, Area, Column, Label, MenuButton, MenuItem, Node,  # noqa: E402
                            NodeEditor, PieMenu, Spacer)


def build(size=(760, 400)) -> App:
    status = Label("Geometry Nodes  ·  7 nodes, 8 links", align="left", icon="GEOMETRY_NODES")
    grp_in = Node("Group Input", outputs=[("Geometry", "geometry"), ("Density", "float")],
                  location=(-560, 40), kind="group")
    distribute = Node("Distribute Points on Faces",
                      inputs=[("Mesh", "geometry"), ("Density", "float", 10.0),
                              ("Seed", "int", 7)],
                      outputs=[("Points", "geometry"), ("Normal", "vector"),
                               ("Rotation", "rotation")],
                      location=(-330, 40), kind="geometry", width=190)
    rock = Node("Object Info", inputs=[("As Instance", "bool")],
                outputs=[("Location", "vector"), ("Geometry", "geometry")],
                location=(-330, -170), kind="input")
    rand = Node("Random Value", inputs=[("Min", "float", 0.4), ("Max", "float", 1.6)],
                outputs=[("Value", "float")], location=(-90, -190), kind="converter")
    inst = Node("Instance on Points",
                inputs=[("Points", "geometry"), ("Instance", "geometry"),
                        ("Rotation", "rotation"), ("Scale", "vector")],
                outputs=[("Instances", "geometry")], location=(-60, 40), kind="geometry",
                width=170)
    join = Node("Join Geometry", inputs=[("Geometry", "geometry")],
                outputs=[("Geometry", "geometry")], location=(160, 40), kind="geometry")
    grp_out = Node("Group Output", inputs=[("Geometry", "geometry")], location=(350, 40),
                   kind="group")
    ed = NodeEditor([grp_in, distribute, rock, rand, inst, join, grp_out],
                    links=[(grp_in, 0, distribute, 0), (grp_in, 1, distribute, 1),
                           (distribute, 0, inst, 0), (distribute, 2, inst, 2),
                           (rock, 1, inst, 1), (rand, 0, inst, 3), (inst, 0, join, 0),
                           (join, 0, grp_out, 0)])
    area = Area("Geometry Node Editor", ed,
                header=[MenuButton("View", ["Frame All", "Frame Selected"]),
                        MenuButton("Select", ["All", "None", "Invert"]),
                        MenuButton("Add", ["Input", "Output", "Geometry", "Instances",
                                           "Mesh", "Point", "Utilities"]),
                        MenuButton("Node", ["Group", "Ungroup", "Mute"]), Spacer()])
    app = App("Geometry Nodes", size, area, Column(status, padding=6),
              padding=0, spacing=0)
    app.editor = ed

    def pie():
        PieMenu.show(app.ctx, [
            MenuItem("Wireframe", icon="SHADING_WIRE"),
            MenuItem("Rendered", icon="SHADING_RENDERED"),
            MenuItem("Solid", icon="SHADING_SOLID"),
            MenuItem("Material Preview", icon="SHADING_TEXTURE"),
            MenuItem("Toggle X-Ray", icon="XRAY"),
            MenuItem("Toggle Overlays", icon="OVERLAY"),
        ], "Shading")

    app.pie = pie
    app.bind("z", pie)
    return app


if __name__ == "__main__":
    build().run()
