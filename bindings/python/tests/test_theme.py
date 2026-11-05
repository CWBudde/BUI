"""Themes: bui.set_theme / BUI_THEME switch Blender's Dark and Light presets.

SPDX-License-Identifier: GPL-2.0-or-later
"""

import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, ".."))

import bui  # noqa: E402  (locates buipy)
import buipy  # noqa: E402
from bui import theme  # noqa: E402
from bui.components import App, Button, Checkbox, NumberSlider, Panel  # noqa: E402


def gray(r, x, y):
    return tuple(r.pixel(x, y)[:3])


def build():
    return App("theme", (260, 200),
               Panel("Render", NumberSlider("Samples", 64, min=1, max=512),
                     Checkbox("Denoise", True), Button("Render Image")))


def test_default_dark():
    assert bui.theme_name() == "dark"
    assert all(abs(a - b) < 1e-6 for a, b in zip(bui.theme_color("region_bg"), (48 / 255,) * 3 + (1.0,)))
    assert len(bui.theme_color("regular.inner")) == 4
    try:
        bui.theme_color("no_such_color")
    except KeyError:
        pass
    else:
        raise AssertionError("unknown key")
    try:
        bui.set_theme("solarized")
    except ValueError:
        pass
    else:
        raise AssertionError("unknown theme")
    assert bui.theme_name() == "dark"


def test_switch_redraws_and_dark_is_unchanged():
    app = build()
    w, h = app.size
    r = app.render_raster(w, h, True)
    dark = [r.pixel(x, y) for y in range(0, h, 3) for x in range(0, w, 3)]
    assert gray(r, w - 2, h - 2) == (48, 48, 48)
    bui.set_theme("light")
    assert bui.theme_name() == "light"
    assert theme.REGION_BG == bui.theme_color("region_bg")
    r = app.render_raster(w, h)  # the switch damaged the whole frame
    assert gray(r, w - 2, h - 2) == (179, 179, 179), gray(r, w - 2, h - 2)
    light = [r.pixel(x, y) for y in range(0, h, 3) for x in range(0, w, 3)]
    assert sum(a != b for a, b in zip(dark, light)) > len(dark) // 2
    bui.set_theme("dark")
    r = app.render_raster(w, h)
    assert [r.pixel(x, y) for y in range(0, h, 3) for x in range(0, w, 3)] == dark
    assert theme.REGION_BG == bui.theme_color("region_bg")


def test_tk_root_follows_theme():
    import bui.tksyntax as tk
    bui.set_theme("light")
    try:
        root = tk.Tk()
        tk.Label(root, text="hi").pack()
        root.update()
        w, h = root._window.size
        r = buipy.Raster(w, h)
        root._ctx.render(w, h, r, True)
        assert gray(r, w - 2, h - 2) == (179, 179, 179), gray(r, w - 2, h - 2)
        root.destroy()
    finally:
        bui.set_theme("dark")


def test_env_var():
    code = "import bui; print(bui.theme_name(), bui.theme_color('region_bg')[0])"
    env = dict(os.environ, BUI_THEME="light")
    out = subprocess.run([sys.executable, "-c", code], env=env, capture_output=True, text=True,
                         cwd=os.path.join(HERE, ".."))
    assert out.returncode == 0, out.stderr
    name, r = out.stdout.split()
    assert name == "light" and abs(float(r) - 179 / 255) < 1e-6, out.stdout


def main() -> int:
    tests = [v for k, v in sorted(globals().items()) if k.startswith("test_")]
    for t in tests:
        t()
        print("ok", t.__name__)
    print("all theme tests passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
