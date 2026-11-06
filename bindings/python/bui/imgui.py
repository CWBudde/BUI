"""Dear ImGui on BUI for Python: ``import blenderui.imgui as imgui`` (or ``bui.imgui``).

The real Dear ImGui (the version vendored in bindings/imgui) drawn by BUI, in Blender's
look. The API is generated from imgui.h, pyimgui-style:

- functions in snake_case: ``imgui.begin("Window")``, ``imgui.slider_float(...)``;
- in/out values are arguments and come back after the result:
  ``changed, value = imgui.slider_float("Value", value, 0.0, 1.0)``,
  ``expanded, opened = imgui.begin("Window", opened)``;
- ImVec2 / ImVec4 are tuples (results are ``imgui.Vec2`` / ``imgui.Vec4`` named tuples);
- ``imgui.text("...")`` and the other printf-style functions take one string;
- enums are constants, both ``imgui.WINDOW_NO_TITLE_BAR`` and the C++ name
  ``imgui.ImGuiWindowFlags_NoTitleBar``;
- structs are objects with snake_case fields: ``imgui.get_io().display_size``,
  ``imgui.get_style().colors[imgui.COL_TEXT] = (1, 1, 1, 1)``,
  ``imgui.get_window_draw_list().add_line(p1, p2, col)``.

Run a window::

    import blenderui.imgui as imgui

    state = {"value": 0.5}

    def frame():
        imgui.show_demo_window()
        imgui.begin("Hello")
        _, state["value"] = imgui.slider_float("Value", state["value"], 0.0, 1.0)
        imgui.end()

    imgui.run(frame, title="Hello", size=(1280, 800))

Headless (tests, screenshots): ``with imgui.HeadlessApp(frame) as app: app.render(3)``.

SPDX-License-Identifier: GPL-2.0-or-later
"""

from __future__ import annotations

import ctypes

import buipy

try:
    # Dear ImGui lives inside buipy (one kit per process); this is its `_buiimgui` module
    _buiimgui = buipy.imgui_module()
except AttributeError:
    raise ImportError(
        "this buipy was built without Dear ImGui (CMake option BUI_BUILD_IMGUI)") from None

from _buiimgui import *  # noqa: E402,F401,F403  (functions, types, constants)
from _buiimgui import App  # noqa: E402

__all__ = [n for n in dir(_buiimgui) if not n.startswith("_")] + [
    "run", "HeadlessApp", "im_col32", "payload_data", "BK_SHIFT", "BK_CTRL", "BK_ALT"]

# modifier bits of HeadlessApp.key()
BK_SHIFT, BK_CTRL, BK_ALT = 1, 2, 4


def im_col32(r: int, g: int, b: int, a: int = 255) -> int:
    """IM_COL32(r, g, b, a): a packed ImU32 color."""
    return ((a & 0xFF) << 24) | ((b & 0xFF) << 16) | ((g & 0xFF) << 8) | (r & 0xFF)


def payload_data(payload) -> bytes:
    """The bytes of an ImGuiPayload (accept_drag_drop_payload() / get_drag_drop_payload())."""
    if payload is None or not payload.data:
        return b""
    return ctypes.string_at(payload.data, payload.data_size)


class _Context:
    """Creates an ImGui context when none is current; destroys it afterwards."""

    def __init__(self, ini_file):
        self.ini_file = ini_file
        self.ctx = None

    def __enter__(self):
        if _buiimgui.get_current_context() is None:
            self.ctx = _buiimgui.create_context()
            io = _buiimgui.get_io()
            io.ini_filename = self.ini_file
        return self

    def __exit__(self, *exc):
        if self.ctx is not None:
            _buiimgui.destroy_context(self.ctx)
            self.ctx = None


def run(frame, title: str = "Dear ImGui on BUI", size=(1280, 800), *, blender_style: bool = True,
        blender_font: bool = True, font_size: float = 0.0, ini_file: str | None = None) -> None:
    """Open a BUI window and call ``frame()`` inside each ImGui frame until it closes.

    Creates the ImGui context if none is current. Without a window system the call
    returns at once (use HeadlessApp). An exception raised by ``frame`` closes the window
    and propagates from here.
    """
    with _Context(ini_file):
        app = App(frame, title, int(size[0]), int(size[1]), False, blender_style, blender_font, font_size)
        try:
            app.run()
        finally:
            app.destroy()


class HeadlessApp:
    """ImGui on BUI without a window: frames render into an offscreen raster.

    ``render(n)`` runs n BUI render passes (ImGui frames run when something asks for one),
    ``click(x, y)``, ``mouse_move``, ``mouse_down`` / ``mouse_up``, ``scroll``,
    ``key("Return")``, ``type_text("abc")`` feed input like a window would;
    ``pixels()`` / ``save_png(path)`` read the result.
    """

    def __init__(self, frame, size=(1280, 800), *, blender_style: bool = True, blender_font: bool = True,
                 font_size: float = 0.0, ini_file: str | None = None):
        self._ctx = _Context(ini_file)
        self._ctx.__enter__()
        self.app = App(frame, "", int(size[0]), int(size[1]), True, blender_style, blender_font, font_size)
        self.size = (int(size[0]), int(size[1]))

    def __getattr__(self, name):
        return getattr(self.app, name)

    def close(self) -> None:
        if self.app is not None:
            self.app.destroy()
            self.app = None
            self._ctx.__exit__()

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        self.close()
