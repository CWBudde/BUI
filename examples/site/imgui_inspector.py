"""Dear ImGui on BUI from Python: a scene inspector (the same app as
examples/site/imgui_inspector.cpp).

    python examples/site/imgui_inspector.py            # Blender widgets + style
    python examples/site/imgui_inspector.py --stock    # Dear ImGui's own look

SPDX-License-Identifier: GPL-2.0-or-later
"""

import math
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..",
                                "bindings", "python"))

import bui.imgui as imgui  # noqa: E402

OBJECTS = [("Camera", "CAMERA", 1), ("Key Light", "LIGHT", 1), ("Robot Arm", "MESH", 48210),
           ("Floor", "MESH", 2), ("Backdrop", "MESH", 512)]
FRAME_MS = [16.6 + 2.5 * math.sin(i * 0.45) + (6 if i % 17 == 0 else 0) for i in range(90)]

state = {"loc": (0.0, 1.25, 0.4), "rot": (0.0, 0.0, 90.0), "color": (0.80, 0.33, 0.18),
         "rough": 0.35, "metal": 0.9, "engine": 0, "samples": 128, "denoise": True,
         "selected": 2, "shadows": True}


def frame(width=520, height=600):
    s = imgui.get_io().display_size
    imgui.set_next_window_pos((0, 0))
    imgui.set_next_window_size((s[0], s[1]))
    imgui.begin("Scene Inspector", None, imgui.WINDOW_NO_COLLAPSE | imgui.WINDOW_NO_MOVE |
                imgui.WINDOW_NO_RESIZE)
    if imgui.begin_table("objects", 3, imgui.TABLE_BORDERS_INNER_V | imgui.TABLE_ROW_BG |
                         imgui.TABLE_BORDERS_OUTER):
        imgui.table_setup_column("Object")
        imgui.table_setup_column("Type")
        imgui.table_setup_column("Faces")
        imgui.table_headers_row()
        for i, (name, kind, faces) in enumerate(OBJECTS):
            imgui.table_next_row()
            imgui.table_next_column()
            if imgui.selectable(name, state["selected"] == i,
                                imgui.SELECTABLE_SPAN_ALL_COLUMNS):
                state["selected"] = i
            imgui.table_next_column()
            imgui.text(kind)
            imgui.table_next_column()
            imgui.text("{:,}".format(faces))
        imgui.end_table()
    imgui.spacing()
    if imgui.collapsing_header("Transform", imgui.TREE_NODE_DEFAULT_OPEN):
        _, state["loc"] = imgui.drag_float3("Location", state["loc"], 0.01)
        _, state["rot"] = imgui.drag_float3("Rotation", state["rot"], 1.0)
    if imgui.collapsing_header("Material", imgui.TREE_NODE_DEFAULT_OPEN):
        _, state["color"] = imgui.color_edit3("Base Color", state["color"])
        _, state["metal"] = imgui.slider_float("Metallic", state["metal"], 0.0, 1.0)
        _, state["rough"] = imgui.slider_float("Roughness", state["rough"], 0.0, 1.0)
    if imgui.collapsing_header("Render", imgui.TREE_NODE_DEFAULT_OPEN):
        _, state["engine"] = imgui.combo("Engine", state["engine"], ["Cycles", "EEVEE",
                                                                     "Workbench"])
        _, state["samples"] = imgui.slider_int("Samples", state["samples"], 1, 4096)
        _, state["denoise"] = imgui.checkbox("Denoise", state["denoise"])
        imgui.same_line()
        _, state["shadows"] = imgui.checkbox("Soft Shadows", state["shadows"])
        imgui.progress_bar(0.62, (-1, 0), "Rendering 62%")
        imgui.plot_lines("##ms", FRAME_MS, 0, "Frame time (ms)", 0.0, 30.0, (-1, 60))
        imgui.button("Render Image")
        imgui.same_line()
        imgui.button("Cancel")
    imgui.end()


def main():
    stock = "--stock" in sys.argv
    imgui.run(frame, title="Scene Inspector", size=(520, 600), blender_style=not stock)


if __name__ == "__main__":
    main()
