"""An unchanged tkinter program (ttk widgets): a render farm queue.

    python -m bui.tksyntax examples/site/ttk_queue.py   # Blender look
    python examples/site/ttk_queue.py                   # stdlib Tk

SPDX-License-Identifier: GPL-2.0-or-later
"""

import tkinter as tk
from tkinter import ttk

JOBS = [
    ("robot_arm.blend", "Cycles", "1-240", "Active", 62),
    ("forest_flyby.blend", "EEVEE", "1-1200", "Queued", 0),
    ("product_shot.blend", "Cycles", "1", "Done", 100),
    ("studio_lighting.blend", "Cycles", "1-48", "Done", 100),
    ("ocean_sim.blend", "EEVEE", "1-500", "Paused", 35),
    ("title_card.blend", "EEVEE", "1-90", "Queued", 0),
]


def build(root):
    root.title("Render Queue")

    bar = ttk.Frame(root, padding=(8, 8, 8, 4))
    bar.pack(fill="x")
    ttk.Label(bar, text="Farm").pack(side="left")
    farm = ttk.Combobox(bar, values=("Studio (8 nodes)", "Local GPU", "Cloud"), width=16,
                        state="readonly")
    farm.current(0)
    farm.pack(side="left", padx=(6, 12))
    ttk.Label(bar, text="Priority").pack(side="left")
    prio = ttk.Spinbox(bar, from_=1, to=10, width=4)
    prio.set(5)
    prio.pack(side="left", padx=6)
    ttk.Button(bar, text="Add Job...").pack(side="right")

    nb = ttk.Notebook(root)
    nb.pack(fill="both", expand=True, padx=8, pady=4)
    jobs = ttk.Frame(nb, padding=6)
    nb.add(jobs, text="Jobs")
    nb.add(ttk.Frame(nb), text="Nodes")
    nb.add(ttk.Frame(nb), text="Log")

    tree = ttk.Treeview(jobs, columns=("engine", "frames", "status", "done"),
                        show="tree headings", height=6)
    tree.heading("#0", text="File")
    tree.heading("engine", text="Engine")
    tree.heading("frames", text="Frames")
    tree.heading("status", text="Status")
    tree.heading("done", text="Done")
    tree.column("#0", width=170)
    for c, w in (("engine", 90), ("frames", 70), ("status", 95), ("done", 50)):
        tree.column(c, width=w, anchor="center" if c != "engine" else "w")
    first = None
    for name, engine, frames, status, pct in JOBS:
        iid = tree.insert("", "end", text=name, values=(engine, frames, status, "%d%%" % pct))
        first = first or iid
    tree.selection_set(first)
    tree.pack(fill="both", expand=True)

    detail = ttk.LabelFrame(jobs, text="robot_arm.blend", padding=6)
    detail.pack(fill="x", pady=(6, 0))
    detail.columnconfigure(1, weight=1)
    ttk.Label(detail, text="Frame 149 / 240").grid(row=0, column=0, sticky="w")
    pb = ttk.Progressbar(detail, maximum=100, value=62)
    pb.grid(row=0, column=1, sticky="we", padx=8)
    ttk.Label(detail, text="ETA 00:18:40").grid(row=0, column=2, sticky="e")
    opts = ttk.Frame(detail)
    opts.grid(row=1, column=0, columnspan=3, sticky="we", pady=(6, 0))
    ttk.Checkbutton(opts, text="Notify when done", variable=tk.BooleanVar(root, True)).pack(
        side="left")
    ttk.Checkbutton(opts, text="Shut down nodes", variable=tk.BooleanVar(root, False)).pack(
        side="left", padx=10)
    ttk.Button(opts, text="Cancel").pack(side="right")
    ttk.Button(opts, text="Pause").pack(side="right", padx=6)

    status = ttk.Frame(root, padding=(8, 2, 8, 8))
    status.pack(fill="x")
    ttk.Label(status, text="6 jobs  ·  2 done  ·  8 of 8 nodes online").pack(side="left")
    ttk.Sizegrip(status).pack(side="right")
    return root


if __name__ == "__main__":
    build(tk.Tk()).mainloop()
