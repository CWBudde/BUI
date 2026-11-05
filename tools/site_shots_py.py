"""The website's Python screenshots (run by tools/gen_doc_images.py --site).

Each shot builds an example from examples/site/ headlessly, at the scale and in
the theme of the environment (BUI_UI_SCALE, BUI_THEME), and writes
<area>-<name>-<theme>.png.

SPDX-License-Identifier: GPL-2.0-or-later
"""
from __future__ import annotations

import os
import sys
from types import SimpleNamespace

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "bindings", "python"))
sys.path.insert(0, os.path.join(ROOT, "examples", "site"))
sys.path.insert(0, os.path.join(ROOT, "tools"))

import bui  # noqa: E402,F401
import buipy  # noqa: E402
import pngopt  # noqa: E402

OUT, THEME = ".", "dark"
S = float(os.environ.get("BUI_UI_SCALE", "2"))
buipy.kit_set_scale(S)  # off-screen: no window sets the resolution scale


def save_raster(r, name, x=0, y=0, w=None, h=None):
    rw, rh = r.size
    x, y = max(0, min(x, rw - 1)), max(0, min(y, rh - 1))
    w, h = min(w or rw - x, rw - x), min(h or rh - y, rh - y)
    ppm = r.ppm(x, y, w, h)
    # P6\n<w> <h>\n255\n<rgb>
    parts = ppm.split(b"\n", 3)
    rgb = parts[3]
    path = os.path.join(OUT, "%s-%s.png" % (name, THEME))
    n = pngopt.write_png(path, w, h, rgb)
    print("saved %s %dx%d %d bytes" % (os.path.basename(path), w, h, n), flush=True)


def mods(**kw):
    return SimpleNamespace(shift=kw.get("shift", False), ctrl=kw.get("ctrl", False),
                           alt=kw.get("alt", False), button1=kw.get("button1", False))


class Driver:
    """A components App driven headlessly (like tests/test_components.py)."""

    def __init__(self, app, size):
        self.app = app
        self.w, self.h = int(size[0] * S), int(size[1] * S)
        self.timers = []
        app.ctx.set_host(schedule=lambda ms, tid: self.timers.append(tid), cursor=lambda c: None,
                         warp=lambda x, y: None)
        self.frame()

    def frame(self):
        return self.app.render_raster(self.w, self.h)

    @staticmethod
    def center(c):
        x, y, w, h = c.rect
        return (x + w // 2, y + h // 2)

    def click(self, c=None, at=None):
        x, y = at or self.center(c)
        self.app.motion(x, y, mods())
        self.app.press(x, y, mods())
        self.app.release(x, y, mods())
        self.frame()

    def hover(self, c=None, at=None):
        self.app.motion(*(at or self.center(c)), mods())
        self.frame()

    def key(self, keysym, char=""):
        self.app.key(char, keysym, 0, mods())
        self.frame()

    def type(self, text):
        for ch in text:
            self.key(ch, ch)

    def fire_timers(self):
        while self.timers:
            self.app.ctx.timer(self.timers.pop(0))
        self.frame()

    def shot(self, name, crop=None):
        r = self.app.render_raster(self.w, self.h, True)
        if crop:
            x, y, w, h = (int(v * S) for v in crop)
            save_raster(r, name, x, y, w, h)
        else:
            save_raster(r, name)


# --------------------------------------------------------------------------------
# Blender components


def components():
    import render_props
    size = (500, 640)
    d = Driver(render_props.build((int(size[0] * S), int(size[1] * S))), size)
    d.shot("python-components")

    # the File menu, with "exp" typed: Blender's fuzzy menu search
    d = Driver(render_props.build((int(size[0] * S), int(size[1] * S))), size)
    d.click(d.app.menubar.buttons[0])
    d.type("exp")
    d.shot("feature-menu-search", crop=(0, 0, 500, 330))

    # a tooltip (hover, then the tooltip timer fires)
    d = Driver(render_props.build((int(size[0] * S), int(size[1] * S))), size)
    target = d.app.denoise
    d.hover(target)
    d.fire_timers()
    x, y, w, h = (v / S for v in target.rect)
    d.shot("feature-tooltip", crop=(40, y - 50, 460, 130))

    # the color picker of a swatch
    d = Driver(render_props.build((int(size[0] * S), int(size[1] * S))), size)
    d.click(d.app.background)
    x, y, w, h = (v / S for v in d.app.background.rect)
    d.shot("feature-color-picker", crop=(115, 250, 380, 330))


def nodes():
    import geometry_nodes
    size = (760, 400)
    d = Driver(geometry_nodes.build((int(size[0] * S), int(size[1] * S))), size)
    d.shot("feature-node-editor")
    d.hover(at=(int(400 * S), int(205 * S)))
    d.app.pie()
    d.frame()
    d.hover(at=(int(470 * S), int(195 * S)))  # toward "Rendered" (E)
    d.shot("feature-pie-menu", crop=(150, 26, 500, 344))


# --------------------------------------------------------------------------------
# tkinter / ttk / Qt syntax


def tk_shot(module, name, pad=0):
    import bui.tksyntax as tks
    tks.install()
    import tkinter as tk
    mod = __import__(module)
    root = mod.build(tk.Tk())
    root.update()
    w, h = root.winfo_reqwidth() + int(pad * S), root.winfo_reqheight()
    r = buipy.Raster(w, h)
    root._ctx.render(w, h, r, True)
    save_raster(r, name)
    root.destroy()


def qt_shot(module, name):
    import bui.qtsyntax as qt
    mod = __import__(module)
    app = qt.QApplication.instance() if hasattr(qt.QApplication, "instance") else None
    app = app or qt.QApplication([])
    w = mod.build()
    w.show()
    w._window.render_now()
    ww, hh = w._window.size
    r = buipy.Raster(ww, hh)
    w._ctx.render(ww, hh, r, True)
    save_raster(r, name)
    w.close()


def imgui_shots():
    import bui.imgui as imgui
    import imgui_inspector
    w, h = int(520 * S), int(490 * S)
    imgui.set_ui_scale(S)  # (the extension has its own kit: its own scale)
    for stock in (True, False):
        app = imgui.HeadlessApp(imgui_inspector.frame, (w, h), blender_style=not stock,
                                blender_font=True)
        imgui.set_blender_widgets(not stock and imgui.has_blender_widgets())
        if stock:
            (imgui.style_colors_light if THEME == "light" else imgui.style_colors_dark)()
        app.render(6)
        pw, ph, px = app.pixels()
        rgb = bytearray(pw * ph * 3)
        rgb[0::3], rgb[1::3], rgb[2::3] = px[2::4], px[1::4], px[0::4]
        path = os.path.join(OUT, "%s-%s.png" % ("imgui-python-" + ("stock" if stock else "blender"),
                                                THEME))
        n = pngopt.write_png(path, pw, ph, bytes(rgb))
        print("saved %s %dx%d %d bytes" % (os.path.basename(path), pw, ph, n), flush=True)
        app.close()


def main(out, theme):
    global OUT, THEME
    OUT, THEME = out, theme
    assert bui.theme_name() == theme, (bui.theme_name(), theme)
    only = os.environ.get("SITE_ONLY", "")
    for name, fn in (("components", components), ("nodes", nodes),
                     ("tk", lambda: tk_shot("tk_export", "python-tkinter")),
                     ("ttk", lambda: tk_shot("ttk_queue", "python-ttk")),
                     ("qt", lambda: qt_shot("qt_material", "python-qt")),
                     ("imgui", imgui_shots)):
        if not only or only == name:
            fn()


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else ".", os.environ.get("BUI_THEME", "dark"))
