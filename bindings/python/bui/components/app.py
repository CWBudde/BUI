"""App: a window hosting a component tree.

The window's whole runtime — layout, input state, drawing, damage tracking
— is the C kit (``buipy.KitContext``). :meth:`App.run` opens it in a
native window (Windows: input -> render -> present never leaves C) or a Tk
window elsewhere (:mod:`bui.host`). The input methods below drive the same
context headlessly (tests, tools).

SPDX-License-Identifier: GPL-2.0-or-later
"""

from __future__ import annotations

from typing import Callable, Optional

import buipy

from bui.components.containers import Column
from bui.components.core import CURSORS, Component, mods_bits  # noqa: F401

class App:
    def __init__(self, title: str = "BUI", size=(420, 640), *children: Component,
                 padding=None, spacing=None, header: Optional[Component] = None):
        self.title, self.size = title, tuple(size)
        u = buipy.kit_unit()
        self.root = Column(*children,
                           padding=round(0.4 * u) if padding is None else padding,
                           spacing=round(0.3 * u) if spacing is None else spacing)
        self.header = header
        self.ctx = buipy.KitContext(self.root)
        if header is not None:
            self.ctx.set_header(header)
        self.window = None
        self._raster = None
        self._timers = []          # (ms, id) when there is no host loop

    def add(self, *components: Component):
        return self.root.add(*components)

    def bind(self, combo: str, fn: Callable[[], None]) -> None:
        """Window hotkey: ``app.bind("ctrl+s", save)``."""
        self.ctx.bind(combo, fn)

    @property
    def cursor(self) -> Optional[str]:
        return CURSORS[self.ctx.cursor]

    # -- scene protocol (driven by TkSurface) ---------------------------------------
    def render(self, w: int, h: int, full: bool = False) -> int:
        """Render into the active raster; returns the number of redrawn regions
        (0 = nothing changed, nothing to present)."""
        return self.ctx.render(w, h, None, full)

    def press(self, x, y, mods=None):
        return self.ctx.press(x, y, mods_bits(mods))

    def release(self, x, y, mods=None):
        return self.ctx.release(x, y, mods_bits(mods))

    def motion(self, x, y, mods=None):
        return self.ctx.motion(x, y, mods_bits(mods))

    def context_menu(self, x, y, mods=None):
        return self.ctx.context_menu(x, y, mods_bits(mods))

    def key(self, char, keysym, keycode=0, mods=None):
        return self.ctx.key(char or "", keysym or "", mods_bits(mods))

    def wheel(self, x, y, delta, mods=None):
        return self.ctx.wheel(x, y, delta, mods_bits(mods))

    # -- running ---------------------------------------------------------------------
    def render_raster(self, w: int = None, h: int = None, full: bool = False) -> "buipy.Raster":
        """Render one frame headlessly (tests, screenshots). The raster is
        kept, so later calls redraw only what changed."""
        w, h = w or self.size[0], h or self.size[1]
        if self._raster is None or self._raster.size != (w, h):
            self._raster = buipy.Raster(w, h)
        self.ctx.render(w, h, self._raster, full)
        return self._raster

    def open(self, native: bool = None):
        """Create (and show) the window without running the loop."""
        from bui import host
        self.window = host.Window(self.ctx, self.title, self.size, native=native)
        self.window.show()
        return self.window

    def run(self, native: bool = None) -> None:
        """Open the window and run the event loop (native on Windows; Tk
        elsewhere or with ``BUI_HOST=tk``)."""
        from bui import host
        self.open(native)
        host.run()
