"""Window hosts for kit trees.

:class:`Window` shows a ``buipy.KitContext`` in a top-level window:

* **native** (Windows, Linux/X11, macOS/Cocoa): ``buipy.KitWindow`` — input,
  layout, rendering and presenting all stay in C; Python only runs callbacks.
  On macOS it needs the main thread (AppKit); elsewhere the Tk host is used.
* **tk** (elsewhere, or ``BUI_HOST=tk``): a stdlib Tk window with
  :class:`bui._surface.TkSurface` blitting the raster.

Both give the same API: ``after``, ``after_cancel``, ``show``, ``close``,
``title``, ``size``, ``on_close``, ``run`` (one loop for all windows),
``update`` (pump pending events).

SPDX-License-Identifier: GPL-2.0-or-later
"""

from __future__ import annotations

import os
import sys
from typing import Callable, Optional

import buipy

from bui import _realtk

from bui.components.core import CURSORS, mods_bits

TK_CURSORS = {None: "", "ew": "sb_h_double_arrow", "ns": "sb_v_double_arrow",
              "text": "xterm", "crosshair": "crosshair", "hand": "hand2", "move": "fleur",
              "none": "none"}


def native_available() -> bool:
    return buipy.kit_window_supported() and os.environ.get("BUI_HOST", "").lower() != "tk"


class _Scene:
    """TkSurface scene protocol over a KitContext."""

    def __init__(self, ctx):
        self.ctx = ctx

    def render(self, w, h):
        return self.ctx.render(w, h)

    def press(self, x, y, mods):
        return self.ctx.press(x, y, mods_bits(mods))

    def release(self, x, y, mods):
        return self.ctx.release(x, y, mods_bits(mods))

    def motion(self, x, y, mods):
        return self.ctx.motion(x, y, mods_bits(mods))

    def context_menu(self, x, y, mods):
        return self.ctx.context_menu(x, y, mods_bits(mods))

    def key(self, char, keysym, keycode, mods):
        return self.ctx.key(char or "", keysym or "", mods_bits(mods))

    def wheel(self, x, y, delta, mods):
        return self.ctx.wheel(x, y, delta, mods_bits(mods))

    def key_release(self, keysym, mods):
        return self.ctx.key_release(keysym or "", mods_bits(mods))

    def button(self, x, y, button, release, mods):
        return self.ctx.button(x, y, button, release, mods_bits(mods))


_tk_root = None  # one hidden Tk interpreter shared by all tk-hosted windows
_windows = []


def _tk():
    global _tk_root
    tk = _realtk.module()
    if _tk_root is None:
        _tk_root = tk.Tk()
        _tk_root.withdraw()
    return _tk_root


def _tk_owns_nsapp() -> None:
    """macOS: start the hidden Tk root before the first Cocoa window.

    Tk makes NSApp an instance of its own NSApplication subclass and fails if
    a plain NSApplication exists already, which the native host would create.
    BUI still uses the real Tk on the side (file dialogs, fonts, PhotoImage),
    so Tk goes first; the native host then reuses Tk's NSApp. Without tkinter
    nothing is needed. ``BUI_MAC_TK=0`` skips it.
    """
    if _tk_root is not None or os.environ.get("BUI_MAC_TK", "1") == "0":
        return
    try:
        _tk()
    except Exception:  # no tkinter / no Tcl: nothing can clash
        pass


class Window:
    def __init__(self, ctx, title: str = "BUI", size=(640, 480), native: bool = None,
                 gpu: bool = None):
        self.ctx = ctx
        self._title, self._size = title, tuple(size)
        self.native = native_available() if native is None else native
        self.on_close: Optional[Callable[[], object]] = None
        self._closed = False
        self._win = None
        self._top = self._surface = None
        if self.native:
            if sys.platform == "darwin":
                _tk_owns_nsapp()
            self._win = buipy.KitWindow(ctx, title, *self._size)
            self._win.on_close = self._close_requested
            if gpu:  # None: buipy.set_gpu() / BUI_GPU decide
                self._win.gpu = True
        else:
            self._make_tk()
        _windows.append(self)

    # -- tk host ----------------------------------------------------------------
    def _make_tk(self):
        tk = _realtk.module()

        from bui._surface import TkSurface
        top = tk.Toplevel(_tk())
        top.title(self._title)
        top.geometry("%dx%d" % self._size)
        top.configure(bg="#303030")
        top.protocol("WM_DELETE_WINDOW", self._tk_close)
        top.withdraw()
        surface = TkSurface(top, _Scene(self.ctx), width=self._size[0], height=self._size[1])
        surface.size_hint = self._size
        widget = surface.widget

        def set_cursor(cid):
            try:
                widget.configure(cursor=TK_CURSORS.get(CURSORS[cid], ""))
            except tk.TclError:
                pass

        def warp(x, y):
            try:
                widget.event_generate("<Motion>", warp=True, x=x, y=y)
            except tk.TclError:
                pass

        def set_clip(text):
            top.clipboard_clear()
            top.clipboard_append(text)

        def schedule(ms, timer_id):
            def fire():
                self.ctx.timer(timer_id)
                surface.request()
            top.after(ms, fire)

        self.ctx.set_host(request=surface.request, cursor=set_cursor, warp=warp,
                          clipboard_get=top.clipboard_get, clipboard_set=set_clip,
                          schedule=schedule)
        self._top, self._surface = top, surface

    def _tk_close(self):
        if self._close_requested():
            self.close()

    def _close_requested(self):
        if self.on_close is not None:
            return bool(self.on_close())
        return True

    # -- API --------------------------------------------------------------------------
    def show(self) -> None:
        if self.native:
            self._win.show()
        else:
            self._top.deiconify()
            self._surface.request()

    def close(self) -> None:
        if self._closed:
            return
        self._closed = True
        if self.native:
            self._win.close()
        elif self._top is not None:
            try:
                self._top.destroy()
            except Exception:
                pass
        if self in _windows:
            _windows.remove(self)
        if not self.native and not _windows and _tk_root is not None:
            _tk_root.quit()

    @property
    def is_open(self) -> bool:
        if self.native:
            return self._win.is_open()
        return not self._closed

    def title(self, text: str = None):
        if text is None:
            return self._title
        self._title = text
        if self.native:
            self._win.set_title(text)
        else:
            self._top.title(text)

    def set_size(self, w: int, h: int) -> None:
        self._size = (w, h)
        if self.native:
            self._win.set_size(w, h)
        else:
            self._top.geometry("%dx%d" % (w, h))
            self._surface.size_hint = (w, h)

    @property
    def size(self):
        if self.native:
            s = self._win.size
            return s if s[0] > 1 else self._size
        return self._surface.size()

    def after(self, ms: int, fn: Callable[[], None]):
        if self.native:
            return self._win.after(int(ms), fn)
        return self._top.after(int(ms), fn)

    def after_cancel(self, ident) -> None:
        if self.native:
            self._win.after_cancel(ident)
        else:
            self._top.after_cancel(ident)

    def request(self) -> None:
        """Ask for a redraw (the kit also does this on every change)."""
        if not self.native and self._surface is not None:
            self._surface.request()

    def render_now(self) -> None:
        if self.native:
            self._win.render_now()
        else:
            self._surface.render()

    @property
    def frames(self) -> int:
        return self._win.frames if self.native else self._surface.frames

    def pixel(self, x: int, y: int):
        if self.native:
            return self._win.pixel(x, y)
        return self._surface.raster.pixel(x, y)

    # -- window manager (Tk's wm) ---------------------------------------------------
    _TK_STATES = ("normal", "iconic", "withdrawn", "zoomed")

    def wm(self, key: str, a: float = 0.0, b: float = 0.0) -> None:
        """Window-manager request: pos, minsize, maxsize, resizable, topmost,
        alpha, override, state (0 normal, 1 iconic, 2 withdrawn, 3 zoomed),
        lift, lower, focus."""
        if self.native:
            self._win.wm(key, float(a), float(b))
            return
        t = self._top
        actions = {
            "pos": lambda: t.geometry("+%d+%d" % (a, b)),
            "minsize": lambda: t.minsize(int(a), int(b)),
            "maxsize": lambda: t.maxsize(int(a) or 100000, int(b) or 100000),
            "resizable": lambda: t.resizable(bool(a), bool(b)),
            "topmost": lambda: t.attributes("-topmost", bool(a)),
            "alpha": lambda: t.attributes("-alpha", float(a)),
            "override": lambda: t.overrideredirect(bool(a)),
            "state": lambda: t.state(self._TK_STATES[int(a)]),
            "lift": t.lift, "lower": t.lower, "focus": t.focus_force,
        }
        try:
            actions[key]()
        except Exception:  # e.g. "zoomed" on X11
            pass

    def wm_get(self, key: str):
        """(a, b) for pos, state, screen, pointer, scale; None if unknown."""
        if self.native:
            return self._win.wm_get(key)
        t = self._top
        values = {
            "pos": lambda: (t.winfo_rootx(), t.winfo_rooty()),
            "state": lambda: (self._TK_STATES.index(t.state()), 0),
            "screen": lambda: (t.winfo_screenwidth(), t.winfo_screenheight()),
            "pointer": lambda: t.winfo_pointerxy(),
            "scale": lambda: (1.0, 0.0),
        }
        try:
            return tuple(float(v) for v in values[key]())
        except Exception:
            return None

    @property
    def handle(self):
        return self._win.hwnd if self.native else self._top.winfo_id()


def run() -> None:
    """Run the event loop until every window is closed."""
    if any(w.native for w in _windows):
        buipy.kit_run()
    if any(not w.native for w in _windows) and _tk_root is not None:
        _tk_root.mainloop()


def update() -> None:
    """Handle pending events and redraws once (like tkinter's update())."""
    if any(w.native for w in _windows):
        buipy.kit_pump()
    if _tk_root is not None:
        _tk_root.update()


def quit() -> None:  # noqa: A001 - tkinter-style name
    buipy.kit_quit()
    if _tk_root is not None:
        _tk_root.quit()
