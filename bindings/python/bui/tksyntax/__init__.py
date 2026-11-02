"""BUI with tkinter syntax: tkinter's API, Blender's look, the C kit underneath.

Write plain tkinter; widgets are BUI kit nodes (pack / grid / place, input
and drawing run in C), windows are native on Windows::

    import bui.tksyntax as tk
    from bui.tksyntax import ttk, messagebox

    root = tk.Tk()
    tk.Button(root, text="Hello", command=root.destroy).pack()
    root.mainloop()

Existing tkinter programs run unchanged with ``python -m bui.tksyntax
script.py``, or after :func:`install` (``import tkinter`` then returns this
package).

SPDX-License-Identifier: GPL-2.0-or-later
"""

from __future__ import annotations

import importlib
import sys

from .constants import *  # noqa: F401,F403
from ._core import (BaseWidget, BooleanVar, CallWrapper, DoubleVar, Event, EventType,  # noqa: F401
                    EXCEPTION, Grid, IntVar, Misc, NoDefaultRoot, Pack, Place, READABLE,
                    StringVar, TclError, TclVersion, Tk, TkVersion, Toplevel, Variable,
                    WRITABLE, Widget, Wm, XView, YView, _get_default_root, _get_temp_root,
                    _setit, color, getboolean, getdouble, getint, mainloop, wantobjects)
from ._images import BitmapImage, Image, PhotoImage, image_names, image_types  # noqa: F401
from ._widgets import (Button, Canvas, Checkbutton, Entry, Frame, Label, LabelFrame,  # noqa: F401
                       Listbox, Menu, Menubutton, Message, OptionMenu, PanedWindow,
                       Radiobutton, Scale, Scrollbar, Spinbox, Text)

__bui__ = True  # marks this package where it stands in for tkinter

__all__ = [name for name, obj in globals().items()
           if not name.startswith("_") and not isinstance(obj, type(sys))
           and name not in ("TYPE_CHECKING", "annotations", "importlib", "sys")]


def Tcl(screenName=None, baseName=None, className="Tk", useTk=False):  # noqa: N802
    """An interpreter without a window (it opens only if used like a Tk)."""
    return Tk(screenName, baseName, className, useTk)


Studbutton = Button
Tributton = Button

# ttk widgets were reachable from bui.tksyntax before the ttk module existed
_TTK_COMPAT = ("Progressbar", "Combobox", "Treeview", "Notebook", "Separator", "Sizegrip",
               "Style", "LabeledScale")

_SUBMODULES = ("constants", "ttk", "font", "messagebox", "filedialog", "simpledialog",
               "colorchooser", "commondialog", "dialog", "scrolledtext")


def __getattr__(name):
    if name == "_default_root":  # tkinter._default_root is read by some programs
        from . import _core
        return _core._default_root
    if name in _TTK_COMPAT:
        from . import ttk
        return getattr(ttk, name)
    if name in _SUBMODULES:
        return importlib.import_module(__name__ + "." + name)
    raise AttributeError("module %r has no attribute %r" % (__name__, name))


def install() -> None:
    """Make ``import tkinter`` (and ``tkinter.ttk``, ``tkinter.messagebox``
    ...) return BUI's tkinter for the rest of the process."""
    from bui import _realtk
    _realtk.pin()  # BUI keeps using the real Tk for OS dialogs / GIF decoding
    pkg = sys.modules[__name__]
    sys.modules["tkinter"] = pkg
    for sub in _SUBMODULES:
        sys.modules["tkinter." + sub] = importlib.import_module(__name__ + "." + sub)
