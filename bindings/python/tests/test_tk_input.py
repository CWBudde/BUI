"""tkinter-syntax widgets driven with real input through the kit (pointer,
keys), checking the C behaviour behind the tkinter API: scrollbars,
selection modes, sortable headings, tree open/close, raw bindings
(<Button-3>, <KeyRelease>, modifiers), password fields, <Configure>.

SPDX-License-Identifier: GPL-2.0-or-later
"""

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))

import bui  # noqa: E402,F401
import buipy  # noqa: E402
import bui.tksyntax as tk  # noqa: E402
from bui.tksyntax import ttk  # noqa: E402

CTRL, SHIFT, B1 = 2, 1, 8


class App:
    def __init__(self, w=360, h=400):
        self.root = tk.Tk()
        self.w, self.h = w, h
        self.raster = buipy.Raster(w, h)

    @property
    def ctx(self):
        return self.root._ctx

    def frame(self):
        self.ctx.render(self.w, self.h, self.raster, False)

    def at(self, widget, dx=None, dy=None):
        x, y, w, h = widget._node.rect
        return x + (w // 2 if dx is None else dx), y + (h // 2 if dy is None else dy)

    def click(self, widget, dx=None, dy=None, mods=0):
        x, y = self.at(widget, dx, dy)
        self.ctx.motion(x, y, mods)
        self.ctx.press(x, y, mods)
        self.ctx.release(x, y, mods)
        self.frame()

    def close(self):
        self.root.destroy()


def test_scrollbar_drives_listbox():
    app = App()
    frame = tk.Frame(app.root)
    frame.pack(fill="both", expand=True)
    sb = tk.Scrollbar(frame)
    sb.pack(side="right", fill="y")
    lb = tk.Listbox(frame, yscrollcommand=sb.set, height=5)
    lb.pack(side="left", fill="both", expand=True)
    sb.config(command=lb.yview)
    for i in range(40):
        lb.insert("end", "item %d" % i)
    app.frame()
    first, last = sb.get()
    assert first == 0.0 and 0 < last < 0.5, (first, last)  # yscrollcommand ran
    x, y = app.at(sb, dy=3)
    app.ctx.press(x, y, 0)  # grab the thumb at the top
    app.ctx.motion(x, y + sb.winfo_height() // 2, B1)
    app.ctx.release(x, y + sb.winfo_height() // 2, 0)
    app.frame()
    assert lb.yview()[0] > 0.3 and sb.get()[0] > 0.3, (lb.yview(), sb.get())
    x, y = app.at(lb)
    app.ctx.wheel(x, y, 3, 0)  # wheel on the list: the scrollbar follows
    app.frame()
    assert abs(sb.get()[0] - lb.yview()[0]) < 1e-6
    app.close()


def test_listbox_selection_modes():
    app = App()
    lb = tk.Listbox(app.root, selectmode="extended", height=8)
    lb.pack(fill="both", expand=True)
    for i in range(8):
        lb.insert("end", "row %d" % i)
    events = []
    lb.bind("<<ListboxSelect>>", lambda e: events.append(lb.curselection()))
    app.frame()
    row_h = buipy.kit_unit()
    pad = round(0.15 * row_h)

    def row(i, mods=0):
        app.click(lb, dx=20, dy=pad + i * row_h + row_h // 2, mods=mods)
    row(1)
    row(4, SHIFT)
    assert lb.curselection() == (1, 2, 3, 4), lb.curselection()
    row(2, CTRL)
    assert lb.curselection() == (1, 3, 4), lb.curselection()
    row(6)
    assert lb.curselection() == (6,) and events[-1] == (6,), (lb.curselection(), events)
    lb.selection_clear(0, "end")
    assert lb.curselection() == ()
    lb.config(selectmode="multiple")
    row(0)
    row(2)
    row(0)
    assert lb.curselection() == (2,), lb.curselection()
    app.close()


def test_treeview_heading_sort_and_open_events():
    app = App()
    tree = ttk.Treeview(app.root, columns=("size",), height=6)
    sorted_by = []
    tree.heading("#0", text="Name", command=lambda: sorted_by.append("#0"))
    tree.heading("size", text="Size", command=lambda: sorted_by.append("size"))
    tree.pack(fill="both", expand=True)
    parent = tree.insert("", "end", text="folder", values=("3",))
    tree.insert(parent, "end", text="a.txt", values=("1",))
    opened = []
    tree.bind("<<TreeviewOpen>>", lambda e: opened.append(tree.focus()))
    app.frame()
    u = buipy.kit_unit()
    pad = round(0.15 * u)
    app.click(tree, dx=tree.winfo_width() - 30, dy=pad + u // 2)  # the "Size" heading
    assert sorted_by == ["size"], sorted_by
    app.click(tree, dx=pad + u // 3, dy=pad + u + u // 2)  # the expander of row 0
    assert opened == [parent] and tree.item(parent, "open") == 1, (opened, tree.item(parent))
    app.close()


def test_raw_bindings():
    app = App()
    btn = tk.Button(app.root, text="target")
    btn.pack(fill="x")
    log = []
    btn.bind("<Button-3>", lambda e: log.append(("b3", e.num)))
    btn.bind("<Control-Button-1>", lambda e: log.append("ctrl-click"))
    btn.bind("<Button-1>", lambda e: log.append("click"))
    app.root.bind("<KeyRelease-a>", lambda e: log.append(("release", e.keysym)))
    sizes = []
    btn.bind("<Configure>", lambda e: sizes.append((e.width, e.height)))
    app.frame()
    x, y = app.at(btn)
    app.ctx.motion(x, y, 0)
    app.ctx.context_menu(x, y, 0)
    app.click(btn, mods=CTRL)
    app.click(btn)
    app.ctx.key_release("a", 0)
    assert log == [("b3", 3), "ctrl-click", "click", ("release", "a")], log
    assert sizes and sizes[-1] == (btn.winfo_width(), btn.winfo_height()), sizes
    app.w = 300
    app.raster = buipy.Raster(300, app.h)
    app.frame()
    assert sizes[-1][0] == btn.winfo_width() and len(sizes) >= 2, sizes
    app.close()


def test_entry_password_and_validation():
    app = App()
    pw = tk.Entry(app.root, show="*")
    pw.pack(fill="x")
    digits = tk.Entry(app.root, validate="key",
                      validatecommand=(app.root.register(str.isdigit), "%S"))
    digits.pack(fill="x")
    app.frame()
    app.click(pw)
    for ch in "secret":
        app.ctx.key(ch, ch, 0)
    app.frame()
    assert pw.get() == "secret"
    x, y, w, h = pw._node.rect
    shown = [app.raster.pixel(xx, y + h // 2) for xx in range(x, x + w)]
    assert len(set(shown)) > 2  # bullets are drawn
    app.ctx.key("", "Return", 0)
    app.click(digits)
    for ch in "1a2":
        app.ctx.key(ch, ch, 0)
    app.ctx.key("", "Return", 0)
    assert digits.get() == "12", digits.get()  # the letter was rejected while typing
    app.close()


def test_text_tag_click_and_font_scale():
    app = App()
    text = tk.Text(app.root, height=4, width=30)
    text.pack()
    text.insert("1.0", "click here please")
    text.tag_add("link", "1.6", "1.10")
    text.tag_configure("link", foreground="#4772b3", background="#202020")
    hits = []
    text.tag_bind("link", "<Button-1>", lambda e: hits.append(text.index("current")))
    big = tk.Label(app.root, text="Title\nsecond line", font=("Arial", 18, "bold"))
    big.pack()
    small = tk.Label(app.root, text="Title")
    small.pack()
    app.frame()
    char_w = buipy.measure_text("click h", buipy.kit_unit() * 0.55) or 40
    app.click(text, dx=int(buipy.kit_unit() * 0.4 + char_w), dy=buipy.kit_unit() // 2)
    assert hits and text.compare(hits[0], ">=", "1.6"), hits
    assert big.winfo_reqheight() > 2 * small.winfo_reqheight(), (big.winfo_reqheight(),
                                                                small.winfo_reqheight())
    app.close()


def test_panedwindow_sash_drag():
    app = App()
    pw = tk.PanedWindow(app.root, orient="horizontal")
    pw.pack(fill="both", expand=True)
    left, right = tk.Label(pw, text="left"), tk.Label(pw, text="right")
    pw.add(left, width=120)
    pw.add(right)
    app.frame()
    w0 = left.winfo_width()
    sx, sy = pw.sash_coord(0)
    x0, y0 = pw._node.rect[:2]
    app.ctx.press(x0 + sx + 1, y0 + sy + 5, 0)
    app.ctx.motion(x0 + sx + 41, y0 + sy + 5, B1)
    app.ctx.release(x0 + sx + 41, y0 + sy + 5, 0)
    app.frame()
    assert abs(left.winfo_width() - (w0 + 40)) <= 2, (w0, left.winfo_width())
    app.close()


def main() -> int:
    for name, fn in sorted(globals().items()):
        if name.startswith("test_"):
            fn()
            print("ok", name)
    print("tk input tests passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
