"""Tests for the advanced surface: Text, Treeview/Table, Notebook, Menu and
menubars, PhotoImage, QPainter custom painting, timers and animation.

SPDX-License-Identifier: GPL-2.0-or-later
"""

import os
import sys
import time

_here = os.path.dirname(os.path.abspath(__file__))
for p in (os.environ.get("BUIPY_BUILD_DIR", ""), os.path.dirname(_here)):
    if p and p not in sys.path:
        sys.path.insert(0, p)

import bui  # noqa: E402,F401  (auto-locates buipy)
import buipy  # noqa: E402
from bui import host  # noqa: E402



def pos(w):
    """Window coordinates of a widget (Qt widgets are kit nodes; tk widgets wrap one)."""
    return w.absolutePos() if hasattr(w, "absolutePos") else tuple(w._node.rect[:2])

def test_tk_text_treeview_notebook_menu():
    from bui.tksyntax import Frame, Menu, Notebook, Text, Tk, Treeview

    root = Tk()
    t = Text(root, undo=True)
    t.insert("1.0", "hello\nworld")
    assert t.index("insert") == "2.5", t.index("insert")
    t.tag_add("warn", "1.1", "1.3")
    t.tag_configure("warn", foreground="#ffcc00")
    assert t.tag_names("1.2") == ("warn",)
    t.delete("1.0", "1.1")
    assert t.get("1.0", "1.end") == "ello"
    t.edit_undo()
    assert t.get("1.0", "1.end") == "hello"
    assert t.search("world", "1.0") == "2.0"
    assert t.get("1.0", "end") == "hello\nworld\n"

    tv = Treeview(root, columns=("size", "mod"))
    tv.heading("#0", text="Name")
    i1 = tv.insert("", "end", text="src", values=("4 KB", "today"))
    i2 = tv.insert(i1, "end", text="main.py")
    tv.selection_set(i1)
    assert tv.selection() == (i1,) and tv.get_children() == (i1,)
    assert tv.exists(i2) and tv.depth(i2) == 1 and tv.parent(i2) == i1
    tv.item(i1, open=True)
    assert tv.item(i1)["values"] == ["4 KB", "today"]

    nb = Notebook(root)
    p1, p2 = Frame(nb), Frame(nb)
    nb.add(p1, text="Tab One")
    nb.add(p2, text="Tab Two")
    nb.select(1)
    assert nb.index(p2) == 1 and nb.tabs() == (str(p1), str(p2)) and nb.index("current") == 1

    calls = []
    bar = Menu(root)
    fm = Menu(bar, tearoff=0)
    fm.add_command(label="Open", command=lambda: calls.append("open"))
    fm.add_separator()
    fm.add_command(label="Save", command=lambda: calls.append("save"))
    bar.add_cascade(label="File", menu=fm)
    root.config(menu=bar)
    fm.invoke(2)
    assert calls == ["save"], calls

    # open the menubar dropdown and pick the first entry with the pointer
    for w in (t, tv, nb):
        w.pack(fill="x")
    ctx = root._ctx
    ctx.render(400, 600, buipy.Raster(400, 600), True)
    mb = root._menubar._bar
    fx, fy, fw, fh = mb.children[0].rect
    ctx.press(fx + 5, fy + 5, 0)
    ctx.release(fx + 5, fy + 5, 0)
    assert ctx.popup_count() == 1, "menubar dropdown did not open"
    px, py, pw, ph = ctx.popup(0).rect
    ctx.motion(px + 20, py + 12, 0)
    ctx.press(px + 20, py + 12, 0)
    ctx.release(px + 20, py + 12, 0)
    assert calls == ["save", "open"], calls
    root.destroy()


def test_photoimage():
    from bui.tksyntax import PhotoImage

    ppm = b"P3\n2 2\n255\n255 0 0  0 255 0  0 0 255  255 255 0\n"
    photo = PhotoImage(data=ppm.decode())
    assert photo.width() == 2 and photo.height() == 2
    assert photo.get(0, 0) == (255, 0, 0)
    assert photo.get(1, 1) == (255, 255, 0)
    photo.put("#00ff00", (0, 0))
    assert photo.get(0, 0) == (0, 255, 0)


def test_qt_advanced():
    from bui.qtsyntax import (QApplication, QColor, QFont, QMenuBar, QPainter, QPen,
                              QPropertyAnimation, QTableWidget, QTableWidgetItem,
                              QTimer, QTreeWidget, QTreeWidgetItem, QVBoxLayout, QWidget)

    QApplication([])

    class CustomWidget(QWidget):
        painted = []

        def paintEvent(self, event):  # noqa: N802 (Qt naming)
            p = QPainter(self)
            p.setBrush(QColor("#224488"))
            p.setPen(QPen(QColor("#ffcc00"), 2))
            p.drawRect(4, 4, 60, 40)
            p.drawRoundedRect(70, 4, 40, 40, 8, 8)
            p.setPen(QColor("#ffffff"))
            p.setFont(QFont("Segoe UI", 12))
            p.drawText(10, 60, "custom painted!")
            CustomWidget.painted.append(1)

    win = QWidget()
    lay = QVBoxLayout(win)
    cw = CustomWidget()
    cw.setFixedHeight(80)
    lay.addWidget(cw)
    tree = QTreeWidget()
    tree.setHeaderLabels(["Name", "Size"])
    top = QTreeWidgetItem(["src", "4 KB"])
    tree.addTopLevelItem(top)
    QTreeWidgetItem(top, ["main.py", "1 KB"])
    tree.addTopLevelItem(QTreeWidgetItem(["docs", "8 KB"]))
    tree.expandAll()
    lay.addWidget(tree)
    table = QTableWidget(2, 2)
    table.setHorizontalHeaderLabels(["A", "B"])
    table.setItem(0, 0, QTableWidgetItem("cell-a"))
    lay.addWidget(table)
    mb = QMenuBar()
    menu_hits = []
    mb.addMenu("File").addAction("Open", lambda: menu_hits.append("open"))
    win.setMenuBar(mb)

    ctx = buipy.KitContext(win)
    win._ctx = ctx
    ctx.set_header(mb._node)
    r = buipy.Raster(360, 420)
    ctx.render(360, 420, r, True)
    cw.update()
    ctx.render(360, 420, r, False)
    assert CustomWidget.painted, "custom paintEvent not called"
    x, y = pos(cw)
    assert r.pixel(x + 30, y + 20) == (0x22, 0x44, 0x88), r.pixel(x + 30, y + 20)
    assert tree.topLevelItemCount() == 2 and top.childCount() == 1 and top.isExpanded()

    table.setItem(0, 1, QTableWidgetItem("cell-b2"))
    assert table.item(0, 1).text() == "cell-b2"
    table.setCurrentCell(1, 0)
    assert table.currentRow() == 1
    tree.setCurrentItem(top)
    assert tree.currentItem() is top

    bx, by, bw, bh = mb._node.children[0].rect
    ctx.press(bx + 5, by + 5, 0)
    ctx.release(bx + 5, by + 5, 0)
    assert ctx.popup_count() == 1, "menubar dropdown did not open"
    px, py, pw, ph = ctx.popup(0).rect
    ctx.motion(px + 20, py + 12, 0)
    ctx.press(px + 20, py + 12, 0)
    ctx.release(px + 20, py + 12, 0)
    assert menu_hits == ["open"], menu_hits

    # timers + property animation run on a real window's clock
    w2 = QWidget()
    w2.resize(120, 60)
    w2.show()
    ticks = []
    timer = QTimer()
    timer.timeout.connect(lambda: ticks.append(1))
    timer.start(5)
    target = type("T", (), {"value": 0})()
    anim = QPropertyAnimation(target, b"value")
    anim.setStartValue(0)
    anim.setEndValue(100)
    anim.setDuration(60)
    anim.start()
    deadline = time.time() + 2
    while (anim.state() or len(ticks) < 3) and time.time() < deadline:
        host.update()
        time.sleep(0.005)
    timer.stop()
    assert target.value == 100, target.value
    assert len(ticks) >= 3, ticks
    w2.close()
    host.update()


def main() -> int:
    for k, v in sorted(globals().items()):
        if k.startswith("test_"):
            v()
            print("ok", k)
    print("advanced surface tests OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
