"""The "filebrowser" node (kit/intern/kit_filebrowser.c) and bui.filebrowser, headless.

SPDX-License-Identifier: GPL-2.0-or-later
"""
import os
import shutil
import sys
import tempfile

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

TMP = tempfile.mkdtemp(prefix="bui_fb_")
os.environ["BUI_BOOKMARKS"] = os.path.join(TMP, "config", "bookmarks.txt")

import bui  # noqa: E402,F401
import buipy  # noqa: E402
from bui import filebrowser  # noqa: E402

W, H = 1000, 600
CTRL = 2
ROOT = os.path.join(TMP, "files")


def setup_tree():
    os.makedirs(os.path.join(ROOT, "Projects"))
    os.makedirs(os.path.join(ROOT, "renders"))
    sizes = {"b_small.txt": 10, "a_big.py": 5000, "c_mid.blend": 300, ".hidden": 1}
    for name, size in sizes.items():
        with open(os.path.join(ROOT, name), "wb") as fh:
            fh.write(b"x" * size)


class Browser:
    def __init__(self, **props):
        self.node = buipy.KitNode("filebrowser")
        for k, v in props.items():
            self.node.set(k, v)
        self.ctx = buipy.KitContext(self.node)
        self.raster = buipy.Raster(W, H)
        self.render(True)
        self.list = next(c for c in self.node.children if c.type == "filelist")

    def render(self, full=False):
        self.ctx.render(W, H, self.raster, full)

    def over_list(self):
        x, y, w, h = self.list.rect
        self.ctx.motion(x + w // 3, y + h // 2, 0)

    def key(self, keysym, mods=0, text=""):
        self.over_list()
        self.ctx.key(text, keysym, mods)
        self.render()

    def click_header(self, frac):
        x, y, w, h = self.list.rect
        px, py = x + int(w * frac), y + 4
        self.ctx.motion(px, py, 0)
        self.ctx.press(px, py, 0)
        self.ctx.release(px, py, 0)
        self.render()


def test_listing_and_sort():
    b = Browser(dir=ROOT)
    assert b.node.get_str("dir") == ROOT + os.sep
    assert b.node.get("count") == 6  # "..", 2 folders, 3 files (.hidden not shown)
    b.click_header(0.95)  # Size column: largest first, folders still on top
    assert b.node.get("sort") == 2
    b.key("End")
    assert b.node.get_str("file") == "b_small.txt"
    b.click_header(0.05)  # back to Name
    assert b.node.get("sort") == 0 and not b.node.get("invert")
    b.key("End")
    assert b.node.get_str("file") == "c_mid.blend"


def test_hidden_search_filters():
    b = Browser(dir=ROOT)
    b.key("h", CTRL)
    assert b.node.get("count") == 7 and b.node.get("hidden") == 1
    b.key("h", CTRL)
    b.node.set("search", "MID")  # case-insensitive "*mid*"
    assert b.node.get("count") == 2  # ".." + c_mid.blend
    b.node.set("search", "")
    b.node.set("filters", ["Python\t.py", "Text\t*.txt *.blend", "All\t*"])
    assert b.node.get("count") == 4  # "..", folders, a_big.py
    b.node.set("filter", 1)
    assert b.node.get("count") == 5


def test_navigation_history():
    b = Browser(dir=ROOT)
    b.key("Home")
    b.key("Down")  # Projects
    b.key("Return")
    assert b.node.get_str("dir").endswith("Projects" + os.sep)
    b.key("BackSpace")
    assert b.node.get_str("dir") == ROOT + os.sep
    b.key("Left", 4)  # Alt+Left: back into Projects
    assert b.node.get_str("dir").endswith("Projects" + os.sep)
    b.key("Right", 4)
    assert b.node.get_str("dir") == ROOT + os.sep


def test_open_save_directory_cancel():
    b = Browser(dir=ROOT)
    b.node.set("file", "a_big.py")
    b.node.set("execute", 1)
    assert b.node.get("state") == 1
    assert b.node.get_str("path") == os.path.join(ROOT, "a_big.py")

    b = Browser(dir=ROOT, multiple=1)
    b.key("End")
    b.key("Up", 1)  # Shift: extend
    b.key("Return")
    assert sorted(os.path.basename(p) for p in b.node.strs("result")) == ["b_small.txt",
                                                                          "c_mid.blend"]

    b = Browser(mode=1, dir=ROOT, file="a_big.py")
    exec_button = [c for c in b.node.children if c.type == "button"][-1]
    assert exec_button.get_str("text") == "Overwrite"
    b.node.set("file", "new.txt")
    assert exec_button.get_str("text") == "Save"

    b = Browser(mode=2, dir=ROOT)
    assert b.node.get("count") == 3  # folders only
    b.node.set("execute", 1)
    assert b.node.get_str("path") == ROOT

    b = Browser(dir=ROOT)
    b.key("Escape")
    assert b.node.get("state") == -1 and b.node.strs("result") == []


def test_typed_paths():
    b = Browser(dir=ROOT)
    b.node.set("file", "")
    fname = [c for c in b.node.children if c.type == "text"][-1]
    fname.set("set_value", os.path.join(ROOT, "Projects"))  # an absolute folder: go there
    assert b.node.get_str("dir") == os.path.join(ROOT, "Projects") + os.sep
    fname.set("set_value", "..")  # relative to the current folder
    assert b.node.get_str("dir") == ROOT + os.sep
    b.node.set("file", os.path.join(ROOT, "renders", "out.png"))  # a path moves the folder
    assert b.node.get_str("dir") == os.path.join(ROOT, "renders") + os.sep
    assert b.node.get_str("file") == "out.png"


def test_new_folder_rename_bookmarks():
    b = Browser(dir=ROOT)
    b.key("i")
    for ch in "made":
        b.ctx.key(ch, ch, 0)
    b.ctx.key("", "Return", 0)
    assert os.path.isdir(os.path.join(ROOT, "made"))
    os.rmdir(os.path.join(ROOT, "made"))
    b.key("b", CTRL)  # bookmark the folder
    with open(os.environ["BUI_BOOKMARKS"]) as fh:
        assert ROOT + os.sep in fh.read().split("\n")
    b.node.set("file", "a_big.py")
    b.node.set("execute", 1)  # chosen folders go to Recent
    with open(os.environ["BUI_BOOKMARKS"]) as fh:
        text = fh.read()
    assert text.index("[Recent]") < text.rindex(ROOT + os.sep)


def test_filter_helpers():
    assert filebrowser.tk_filters([("Images", ("*.png", "*.jpg")), ("Text", ".txt")]) == [
        "Images\t*.png *.jpg", "Text\t.txt"]
    assert filebrowser.qt_filters("Images (*.png *.jpg);;All (*)") == ["Images\t*.png *.jpg",
                                                                       "All\t*"]


def main():
    setup_tree()
    try:
        for name, fn in list(globals().items()):
            if name.startswith("test_"):
                fn()
                print("ok", name)
    finally:
        shutil.rmtree(TMP, ignore_errors=True)


if __name__ == "__main__":
    main()
