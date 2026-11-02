"""Tests for the high-level syntax layers (Qt syntax, tkinter syntax).

Both facades are thin Python over the C kit: widgets are kit nodes, layout is
the kit's C geometry managers. Input is simulated through the kit context —
the same entry points the native window calls.

SPDX-License-Identifier: GPL-2.0-or-later
"""

import os
import sys

_here = os.path.dirname(os.path.abspath(__file__))
for p in (os.environ.get("BUIPY_BUILD_DIR", ""), os.path.dirname(_here)):
    if p and p not in sys.path:
        sys.path.insert(0, p)

import bui  # noqa: E402,F401  (auto-locates buipy)
import buipy  # noqa: E402



def pos(w):
    """Window coordinates of a widget (Qt widgets are kit nodes; tk widgets wrap one)."""
    return w.absolutePos() if hasattr(w, "absolutePos") else tuple(w._node.rect[:2])

def click(ctx, w, dx=5, dy=5):
    x, y = pos(w)
    ctx.press(x + dx, y + dy, 0)
    ctx.release(x + dx, y + dy, 0)


def test_qt_syntax():
    from bui.qtsyntax import (QApplication, QLabel, QLineEdit, QProgressBar, QPushButton,
                              QRadioButton, QSlider, QSpinBox, QVBoxLayout, QWidget)

    QApplication([])
    window = QWidget()
    layout = QVBoxLayout()
    label = QLabel("Hello, BUI!")
    button = QPushButton("Click me")
    button.clicked.connect(lambda: label.setText("Clicked!"))
    entry = QLineEdit()
    edits = []
    entry.textChanged.connect(edits.append)
    slider = QSlider()
    slider.setRange(0, 10)
    spin = QSpinBox()
    spin.setRange(0, 5)
    bar = QProgressBar()
    bar.setValue(4)
    radio1, radio2 = QRadioButton("one"), QRadioButton("two")
    for w in (label, button, entry, slider, spin, bar, radio1, radio2):
        layout.addWidget(w)
    window.setLayout(layout)

    ctx = buipy.KitContext(window)
    r = buipy.Raster(320, 420)
    ctx.render(320, 420, r, True)
    for w in (label, button, entry, slider, bar):
        assert w.width() > 200 and w.height() > 0, (type(w).__name__, w.geometry().width())

    click(ctx, button)
    assert label.text() == "Clicked!", label.text()

    click(ctx, entry)                 # click-to-edit, then type
    for ch in "ok":
        ctx.key(ch, ch, 0)
    ctx.key("", "Return", 0)
    assert entry.text() == "ok", entry.text()
    assert edits and edits[-1] == "ok", edits

    sx, sy = pos(slider)
    ctx.press(sx + slider.width() - 3, sy + 5, 0)
    ctx.motion(sx + slider.width() + 3000, sy + 5, 8)
    ctx.release(sx + slider.width() + 3000, sy + 5, 0)
    assert slider.value() == 10, slider.value()

    spin.stepUp()
    assert spin.value() == 1
    radio1.setChecked(True)
    click(ctx, radio2)
    assert radio2.isChecked() and not radio1.isChecked()
    assert bar.value() == 4 and bar.get_str("text") == "4%"


def test_qt_layout_semantics():
    from bui.qtsyntax import (QGridLayout, QHBoxLayout, QLabel, QLineEdit, QPushButton,
                              QVBoxLayout, QWidget)
    win = QWidget()
    root = QVBoxLayout()
    grid = QGridLayout()
    name, edit = QLabel("Name:"), QLineEdit()
    grid.addWidget(name, 0, 0)
    grid.addWidget(edit, 0, 1)
    root.addLayout(grid)
    row = QHBoxLayout()
    a, b = QPushButton("A"), QPushButton("B")
    row.addWidget(a)
    row.addWidget(b)
    root.addLayout(row)
    win.setLayout(root)
    buipy.KitContext(win).render(400, 200, buipy.Raster(400, 200), True)
    for w in (name, edit, a, b):
        assert w.parentWidget() is win, "widget in nested layout not parented"
        assert w.width() > 0 and w.height() > 0
    assert edit.width() > name.width(), "expanding line edit did not take the space"
    assert abs(a.width() - b.width()) <= 1, (a.width(), b.width())  # row shared evenly
    assert a.x() < b.x() and a.y() == b.y()
    assert a.x() == name.x(), "nested layouts must not add margins"


def test_qt_painter_and_signals():
    from bui.qtsyntax import QColor, QPainter, QVBoxLayout, QWidget, Signal

    class Swatch(QWidget):
        def paintEvent(self, event):  # noqa: N802
            p = QPainter(self)
            p.setBrush(QColor("#ff0000"))
            p.setPen(None)
            p.drawRect(0, 0, 30, 20)
            p.end()

    win = QWidget()
    lay = QVBoxLayout(win)
    sw = Swatch()
    sw.setFixedHeight(40)
    lay.addWidget(sw)
    ctx = buipy.KitContext(win)
    r = buipy.Raster(200, 100)
    ctx.render(200, 100, r, True)
    sw.update()
    ctx.render(200, 100, r, False)
    x, y = pos(sw)
    assert sw.height() == 40, sw.height()
    assert r.pixel(x + 10, y + 10) == (255, 0, 0), r.pixel(x + 10, y + 10)

    class Emitter(QWidget):
        valueChanged = Signal(int)

    e, got = Emitter(), []
    e.valueChanged.connect(got.append)
    e.valueChanged.emit(42)
    e.valueChanged.connect(lambda: got.append("no-arg slot"))
    e.valueChanged.emit(1)
    assert got == [42, 1, "no-arg slot"], got


def test_tk_syntax():
    from bui.tksyntax import (Button, Canvas, Checkbutton, Entry, IntVar, Label, Listbox,
                              Radiobutton, Scale, Spinbox, StringVar, Tk)

    root = Tk()
    root.geometry("420x520")
    name = StringVar()
    entry = Entry(root, textvariable=name)
    entry.pack(fill="x", pady=4)
    mirror = Label(root, textvariable=name)
    mirror.pack()
    out = Label(root, text="")
    out.pack()
    btn = Button(root, text="Go", command=lambda: out.configure(text=name.get() + "!"))
    btn.pack()
    lb = Listbox(root)
    lb.insert("end", "one", "two", "three")
    lb.pack(fill="x")
    cv = Canvas(root, width=100, height=50)
    tag_hits = []
    cv.create_rectangle(0, 0, 40, 40, fill="red", tags=("hot",))
    cv.tag_bind("hot", "<Button-1>", lambda ev: tag_hits.append(1))
    cv.pack()
    sc = Scale(root, from_=0, to=10, orient="horizontal")
    sc.pack(fill="x")
    sp = Spinbox(root, from_=0, to=5)
    sp.pack()
    cb = Checkbutton(root, text="opt")
    cb.pack()
    var2 = IntVar()
    rb = Radiobutton(root, text="r1", value=1, variable=var2)
    rb.pack()
    ctx = root._ctx
    r = buipy.Raster(420, 520)
    ctx.render(420, 520, r, True)

    bx, by = pos(btn)
    assert r.pixel(bx + btn.winfo_width() // 2, by + 3) != r.pixel(2, 2), "button not drawn"

    click(ctx, entry)
    for ch in "hi":
        ctx.key(ch, ch, 0)
    assert name.get() == "hi", name.get()      # Entry is live, like Tk
    assert mirror.cget("text") == "hi"
    ctx.key("", "Return", 0)

    click(ctx, btn)
    assert out.cget("text") == "hi!", out.cget("text")

    click(ctx, lb, 10, 10)
    assert lb.curselection() == (0,), lb.curselection()

    sx, sy = pos(sc)
    ctx.press(sx + sc.winfo_width() - 2, sy + 5, 0)
    ctx.motion(sx + sc.winfo_width() + 3000, sy + 5, 8)
    ctx.release(sx + sc.winfo_width() + 3000, sy + 5, 0)
    assert sc.get() == 10, sc.get()

    click(ctx, rb)
    assert var2.get() == 1

    calls = []
    btn.bind("<Button-1>", lambda e: calls.append((e.x, e.y)))
    click(ctx, btn)
    assert calls, "bind() did not fire"

    click(ctx, cv, 10, 10)
    assert tag_hits == [1], tag_hits

    cb.invoke()
    assert root.getvar(str(cb.cget("variable"))) == 1
    root.destroy()


def test_tk_geometry_managers():
    from bui.tksyntax import Button, Frame, Label, Tk

    root = Tk()
    fr = Frame(root)
    fr.pack(fill="both", expand=True)
    inner = Label(fr, text="inside")
    inner.pack()
    Button(root, text="bottom").pack()
    ctx = root._ctx
    ctx.render(300, 300, buipy.Raster(300, 300), True)
    fx, fy = pos(fr)
    ix, iy = pos(inner)
    assert fx <= ix < fx + fr.winfo_width() and fy <= iy < fy + fr.winfo_height()
    big = fr.winfo_height()
    ctx.render(300, 150, buipy.Raster(300, 150), True)
    assert fr.winfo_height() < big, "expanded size stuck after shrinking"

    g = Frame(root)
    g.pack(fill="x")
    cells = [Label(g, text="c%d" % i) for i in range(3)]
    for i, c in enumerate(cells):
        c.grid(row=0, column=i, sticky="ew")
    g.columnconfigure(1, weight=1)
    p = Label(root, text="placed")
    p.place(x=7, y=9)
    ctx.render(300, 300, buipy.Raster(300, 300), True)
    assert cells[1].winfo_width() > cells[0].winfo_width() * 2, "grid weight ignored"
    assert cells[0].winfo_x() < cells[1].winfo_x() < cells[2].winfo_x()
    assert (p.winfo_x(), p.winfo_y()) == (7, 9)
    assert g.grid_slaves() and inner.pack_info()["side"] == "top"
    root.destroy()


def main() -> int:
    for k, v in sorted(globals().items()):
        if k.startswith("test_"):
            v()
            print("ok", k)
    print("syntax layer tests OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
