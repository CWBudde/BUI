"""tkinter-syntax demo: a tkinter-style application drawn entirely by BUI.

Laid out like a Blender properties editor: panels (LabelFrame) holding
"label | field" rows. Runs on the stdlib alone — no Qt, no pip packages.

    python examples/tk_syntax_demo.py

SPDX-License-Identifier: GPL-2.0-or-later
"""

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "bindings", "python"))

from bui.tksyntax import (Button, Canvas, Checkbutton, Entry, Frame, IntVar,
                          Label, LabelFrame, Listbox, Progressbar, Radiobutton,
                          Scale, Spinbox, StringVar, Tk)


def main() -> int:
    root = Tk()
    root.title("BUI — tkinter syntax")
    root.geometry("440x700")

    Label(root, text="tkinter syntax, BUI renderer", font=("", 15)).pack(pady=(10, 4))

    def row(panel, text):
        """A Blender-style split row: right-aligned label, field fills the rest."""
        f = Frame(panel)
        f.pack(fill="x", pady=2)
        Label(f, text=text, width=9, anchor="e").pack(side="left")
        return f

    # -- Object panel ---------------------------------------------------------
    obj = LabelFrame(root, text="Object")
    obj.pack(fill="x", padx=8, pady=4)

    name = StringVar()
    Entry(row(obj, "Name"), textvariable=name).pack(side="left", fill="x", expand=True)

    progress = None
    level = Scale(row(obj, "Level"), from_=0, to=100, orient="horizontal",
                  command=lambda v: progress.configure(value=float(v)))
    level.pack(side="left", fill="x", expand=True)

    count = Spinbox(row(obj, "Count"), from_=0, to=10)
    count.pack(side="left", fill="x", expand=True)

    progress = Progressbar(row(obj, "Progress"), maximum=100)
    progress.pack(side="left", fill="x", expand=True)

    echo = Label(obj, text="Type a name, then click.")
    echo.pack(pady=(6, 2))

    def on_click():
        n = getattr(on_click, "n", 0) + 1
        on_click.n = n
        echo.configure(text="{} — clicked {} time(s)!".format(name.get() or "BUI", n))

    Button(obj, text="Click me", command=on_click).pack(fill="x", pady=(2, 4))

    # -- Options panel --------------------------------------------------------
    opts = LabelFrame(root, text="Options")
    opts.pack(fill="x", padx=8, pady=4)
    Checkbutton(opts, text="Enable notifications").pack(anchor="w", padx=6)
    lvl = IntVar()
    for text, val in (("Beginner", 1), ("Expert", 2)):
        Radiobutton(opts, text=text, value=val, variable=lvl).pack(anchor="w", padx=6)

    # -- Items panel ----------------------------------------------------------
    items = LabelFrame(root, text="Items")
    items.pack(fill="x", padx=8, pady=4)
    lb = Listbox(items)
    for item in ("Cube", "Camera", "Light", "Suzanne"):
        lb.insert("end", item)
    lb.pack(fill="x", padx=4, pady=2)

    cv = Canvas(items, width=180, height=60, bg="#282828")
    cv.create_rectangle(10, 10, 80, 50, fill="#4772b3", outline="")
    cv.create_oval(100, 10, 170, 50, fill="#e6a23c", outline="")
    cv.create_text(45, 30, text="canvas", fill="#ffffff")
    cv.pack(pady=4)

    Button(root, text="Quit", command=root.destroy).pack(fill="x", padx=8, pady=8)

    root.mainloop()
    return 0


if __name__ == "__main__":
    sys.exit(main())
