"""Render the examples into docs/images/ (headless, no window shown).

    python tools/gen_doc_images.py

SPDX-License-Identifier: GPL-2.0-or-later
"""
import os, sys
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "docs", "images")
sys.path.insert(0, os.path.join(ROOT, "bindings", "python"))
sys.path.insert(0, os.path.join(ROOT, "examples"))
import bui, buipy
from bui.tksyntax._images import _encode_png

def save(raster, w, h, name):
    px = b"".join(bytes(raster.pixel(x, y)) + b"\xff" for y in range(h) for x in range(w))
    with open(os.path.join(OUT, name), "wb") as fh:
        fh.write(_encode_png(w, h, px))
    print("saved", name, w, h)

def ctx_shot(ctx, w, h, name):
    r = buipy.Raster(w, h)
    ctx.render(w, h, r, True)
    save(r, w, h, name)

# 1. components demo
import components_demo
app = components_demo.build()
r = app.render_raster(520, 690, True)
save(r, 520, 690, "components.png")

# 2. unchanged tkinter app (+ each tab)
import bui.tksyntax as tks
tks.install()
import tkinter as tk
import tk_unchanged_app
for i, name in enumerate(("tkinter_items.png", "tkinter_notes.png", "tkinter_canvas.png")):
    root = tk_unchanged_app.build(tk.Tk())
    nb = [w for w in root.winfo_children() if w.winfo_class() == "TNotebook"][0]
    nb.select(i)
    root.update()
    ctx_shot(root._ctx, *root._window.size, name)
    root.destroy()

# 3. messagebox
from tkinter import messagebox
from bui.tksyntax import _core
root = tk.Tk(); tk.Label(root, text=" ").pack(); root.update()
def shoot():
    top = [w for w in _core._windows() if w is not root][-1]
    ctx_shot(top._ctx, *top._window.size, "tkinter_messagebox.png")
    for d in top._descendants():
        if isinstance(d, tk.Button) and d.cget("text") == "No":
            d.invoke()
root.after(150, shoot)
messagebox.askyesno("Unsaved changes", "Save changes to the inventory before closing?",
                    detail="Your changes will be lost if you don't save them.")
root.destroy()

# 4. Qt demo
import bui.qtsyntax as qt
import qt_syntax_demo
def fake_exec(self):
    w = [o for o in qt._custom_or_top()][-1]
    w._window.render_now()
    ctx_shot(w._ctx, w._window.size[0], 222, "qt.png")
    w.close()
    return 0
qt.QApplication.exec = fake_exec
qt_syntax_demo.main()
