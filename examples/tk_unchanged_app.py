"""A plain tkinter program - no BUI imports.

Run it on BUI (Blender look, native window) without changing a line:

    python -m bui.tksyntax examples/tk_unchanged_app.py

or on the stdlib Tk for comparison:

    python examples/tk_unchanged_app.py

SPDX-License-Identifier: GPL-2.0-or-later
"""

import tkinter as tk
from tkinter import filedialog, messagebox, ttk


def build(root):
    root.title("Inventory")

    def open_file():
        path = filedialog.askopenfilename(parent=root, title="Open",
                                          filetypes=[("Text", "*.txt"), ("All files", "*")])
        if path:
            root.title("Inventory - " + path)
    menubar = tk.Menu(root)
    filemenu = tk.Menu(menubar, tearoff=0)
    filemenu.add_command(label="New", accelerator="Ctrl+N")
    filemenu.add_command(label="Open...", accelerator="Ctrl+O", command=open_file)
    filemenu.add_separator()
    filemenu.add_command(label="Quit", command=root.destroy)
    menubar.add_cascade(label="File", menu=filemenu)
    editmenu = tk.Menu(menubar, tearoff=0)
    editmenu.add_command(label="Undo", accelerator="Ctrl+Z")
    menubar.add_cascade(label="Edit", menu=editmenu)
    root.config(menu=menubar)

    nb = ttk.Notebook(root)
    nb.pack(fill="both", expand=True, padx=6, pady=6)

    # -- Items tab: a form and a table -------------------------------------------
    items = ttk.Frame(nb, padding=6)
    nb.add(items, text="Items")
    form = ttk.LabelFrame(items, text="New item")
    form.pack(fill="x")
    name = tk.StringVar(value="Cube")
    ttk.Label(form, text="Name").grid(row=0, column=0, sticky="e", padx=4, pady=2)
    ttk.Entry(form, textvariable=name).grid(row=0, column=1, sticky="we", pady=2)
    ttk.Label(form, text="Kind").grid(row=1, column=0, sticky="e", padx=4, pady=2)
    kind = ttk.Combobox(form, values=("Mesh", "Light", "Camera"), state="readonly")
    kind.current(0)
    kind.grid(row=1, column=1, sticky="we", pady=2)
    ttk.Label(form, text="Count").grid(row=2, column=0, sticky="e", padx=4, pady=2)
    count = tk.Spinbox(form, from_=1, to=99)
    count.grid(row=2, column=1, sticky="we", pady=2)
    ttk.Label(form, text="Scale").grid(row=3, column=0, sticky="e", padx=4, pady=2)
    scale = tk.Scale(form, from_=0, to=100, orient="horizontal")
    scale.set(40)
    scale.grid(row=3, column=1, sticky="we", pady=2)
    visible = tk.BooleanVar(value=True)
    ttk.Checkbutton(form, text="Visible", variable=visible).grid(row=4, column=1, sticky="w")
    form.columnconfigure(1, weight=1)

    table = ttk.Frame(items)
    table.pack(fill="both", expand=True, pady=(6, 0))
    tree = ttk.Treeview(table, columns=("name", "kind", "count"), show="headings", height=6)
    tree.heading("name", text="Name")
    tree.heading("kind", text="Kind")
    tree.heading("count", text="Count")
    for row in (("Cube", "Mesh", 2), ("Sun", "Light", 1), ("Cam", "Camera", 1)):
        tree.insert("", "end", values=row)
    sb = ttk.Scrollbar(table, orient="vertical", command=tree.yview)
    tree.configure(yscrollcommand=sb.set)
    tree.pack(side="left", fill="both", expand=True)
    sb.pack(side="right", fill="y")

    def add():
        tree.insert("", "end", values=(name.get(), kind.get(), count.get()))
    buttons = ttk.Frame(items)
    buttons.pack(fill="x", pady=(6, 0))
    ttk.Button(buttons, text="Add", command=add).pack(side="left", expand=True, fill="x")
    ttk.Button(buttons, text="About", command=lambda: messagebox.showinfo(
        "About", "A plain tkinter program.")).pack(side="left", expand=True, fill="x", padx=4)

    # -- Notes tab: text with tags and a scrollbar ---------------------------------------
    notes = ttk.Frame(nb, padding=6)
    nb.add(notes, text="Notes")
    text = tk.Text(notes, height=10, width=40)
    tsb = ttk.Scrollbar(notes, command=text.yview)
    text.configure(yscrollcommand=tsb.set)
    text.pack(side="left", fill="both", expand=True)
    tsb.pack(side="right", fill="y")
    text.insert("end", "TODO: ", "todo")
    text.insert("end", "light the scene\n")
    text.tag_configure("todo", foreground="#e6a23c")

    # -- Canvas tab -----------------------------------------------------------------------
    draw = ttk.Frame(nb, padding=6)
    nb.add(draw, text="Canvas")
    canvas = tk.Canvas(draw, width=300, height=180)
    canvas.pack(fill="both", expand=True)
    canvas.create_rectangle(20, 20, 120, 80, fill="#4772b3", outline="")
    canvas.create_oval(150, 30, 230, 110, fill="#e6a23c", outline="")
    canvas.create_line(20, 150, 280, 120, width=2, arrow="last")
    canvas.create_text(70, 50, text="canvas", fill="white")
    return root


if __name__ == "__main__":
    build(tk.Tk()).mainloop()
