"""The stdlib tkinter, pinned.

:func:`bui.tksyntax.install` makes ``import tkinter`` return BUI's tkinter.
BUI itself still needs the real one in a few places (the Tk fallback window
host, OS file dialogs, decoding PNG/GIF), so it always goes through here.

SPDX-License-Identifier: GPL-2.0-or-later
"""

from __future__ import annotations

import importlib
import sys

_pinned: dict = {}

_NAMES = ("tkinter", "tkinter.font", "tkinter.filedialog", "tkinter.colorchooser")


def pin() -> None:
    """Import and remember the real modules (before anything aliases them)."""
    for name in _NAMES:
        if name not in _pinned:
            mod = sys.modules.get(name)
            if mod is None or getattr(mod, "__bui__", False):
                saved = {n: sys.modules.pop(n) for n in list(sys.modules)
                         if n == "tkinter" or n.startswith("tkinter.")}
                try:
                    mod = importlib.import_module(name)
                finally:
                    for n, m in saved.items():
                        sys.modules[n] = m
            _pinned[name] = mod


def module(name: str = "tkinter"):
    """The real ``tkinter`` (or ``tkinter.font`` ...)."""
    if name not in _pinned:
        pin()
    return _pinned[name]
