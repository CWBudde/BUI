"""An unchanged tkinter program (classic Tk widgets): a mesh export dialog.

    python -m bui.tksyntax examples/site/tk_export.py   # Blender look
    python examples/site/tk_export.py                   # stdlib Tk

SPDX-License-Identifier: GPL-2.0-or-later
"""

import tkinter as tk


def build(root):
    root.title("Export glTF 2.0")

    top = tk.Frame(root)
    top.pack(fill="x", padx=10, pady=(10, 4))
    tk.Label(top, text="File").pack(side="left")
    path = tk.Entry(top)
    path.insert(0, "//renders/robot_arm.glb")
    path.pack(side="left", fill="x", expand=True, padx=6)
    tk.Button(top, text="Browse...").pack(side="left")

    body = tk.Frame(root)
    body.pack(fill="both", expand=True, padx=10)

    # left: what to include
    include = tk.LabelFrame(body, text="Include", padx=6, pady=4)
    include.pack(side="left", fill="both", expand=True)
    for label, on in (("Selected Objects", True), ("Visible Objects", False),
                      ("Active Collection", False), ("Custom Properties", True),
                      ("Cameras", True), ("Punctual Lights", False)):
        v = tk.BooleanVar(value=on)
        tk.Checkbutton(include, text=label, variable=v, anchor="w").pack(fill="x")
    tk.Label(include, text="Format", anchor="w").pack(fill="x", pady=(6, 0))
    fmt = tk.StringVar(value="glb")
    for text, value in (("glTF Binary (.glb)", "glb"), ("glTF Separate (.gltf)", "gltf"),
                        ("glTF Embedded", "embedded")):
        tk.Radiobutton(include, text=text, value=value, variable=fmt, anchor="w").pack(fill="x")

    # right: mesh options
    mesh = tk.LabelFrame(body, text="Mesh", padx=6, pady=4)
    mesh.pack(side="left", fill="both", expand=True, padx=(8, 0))
    mesh.columnconfigure(1, weight=1)
    tk.Label(mesh, text="Scale").grid(row=0, column=0, sticky="e", padx=(0, 6))
    scale = tk.Spinbox(mesh, from_=0.01, to=100, increment=0.1, format="%.2f",
                       textvariable=tk.StringVar(root, "1.00"))
    scale.grid(row=0, column=1, sticky="we", pady=2)
    tk.Label(mesh, text="Forward").grid(row=1, column=0, sticky="e", padx=(0, 6))
    axis = tk.StringVar(value="-Z Forward")
    tk.OptionMenu(mesh, axis, "-Z Forward", "X Forward", "Y Forward", "Z Forward").grid(
        row=1, column=1, sticky="we", pady=2)
    tk.Label(mesh, text="Compression").grid(row=2, column=0, sticky="e", padx=(0, 6))
    level = tk.Scale(mesh, from_=0, to=10, orient="horizontal", showvalue=True)
    level.set(6)
    level.grid(row=2, column=1, sticky="we")
    tk.Label(mesh, text="UV Maps").grid(row=3, column=0, sticky="ne", padx=(0, 6), pady=2)
    uvs = tk.Listbox(mesh, height=4, exportselection=False)
    for name in ("UVMap", "Lightmap", "Decals", "Detail"):
        uvs.insert("end", name)
    uvs.selection_set(0)
    uvs.grid(row=3, column=1, sticky="we", pady=2)
    for i, (label, on) in enumerate((("Apply Modifiers", True), ("Tangents", False),
                                     ("Vertex Colors", True))):
        v = tk.BooleanVar(value=on)
        tk.Checkbutton(mesh, text=label, variable=v).grid(row=4 + i, column=1, sticky="w")

    bottom = tk.Frame(root)
    bottom.pack(fill="x", padx=10, pady=10)
    tk.Label(bottom, text="3 objects, 48,210 triangles").pack(side="left")
    tk.Button(bottom, text="Export", width=10, default="active").pack(side="right")
    tk.Button(bottom, text="Cancel", width=10).pack(side="right", padx=6)
    return root


if __name__ == "__main__":
    build(tk.Tk()).mainloop()
