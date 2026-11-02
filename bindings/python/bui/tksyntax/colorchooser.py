"""tkinter.colorchooser: askcolor with Blender's color picker.

The dialog shows a Blender color swatch; clicking it opens the picker (hue
wheel, value slider, hex field).

SPDX-License-Identifier: GPL-2.0-or-later
"""

from __future__ import annotations

from .commondialog import Dialog

__all__ = ["Chooser", "askcolor"]


class Chooser(Dialog):
    """Create a dialog for the tk_chooseColor command."""

    command = "tk_chooseColor"

    def _fixoptions(self):
        """Ensure initialcolor is a tk color string."""
        try:
            color = self.options["initialcolor"]
            if isinstance(color, tuple):
                self.options["initialcolor"] = "#%02x%02x%02x" % color
        except KeyError:
            pass

    def _fixresult(self, widget, result):
        """((r, g, b), '#rrggbb') or (None, None) when cancelled."""
        if not result:
            return None, None
        r, g, b = (int(result[i:i + 2], 16) for i in (1, 3, 5))
        return (r, g, b), result

    def _show(self, options):
        from . import (Button, Frame, Label, Toplevel, _get_temp_root, color)
        from ._core import Widget, _hex
        from .simpledialog import _place_window

        class _Swatch(Widget):
            _KIND = "swatch"

            def __init__(self, master, rgba):
                Widget.__init__(self, master, "swatch", {}, {})
                self._node.set("value", rgba)

        parent = options.get("parent") or self.master or _get_temp_root()
        top = Toplevel(parent)
        top.title(options.get("title", "Color"))
        top.resizable(False, False)
        result = {"value": None}
        body = Frame(top, padx=12, pady=10)
        body.pack(fill="both", expand=True)
        Label(body, text="Click the color to pick", anchor="w").pack(fill="x")
        initial = color(options.get("initialcolor") or "#808080")
        sw = _Swatch(body, initial)
        sw.pack(fill="x", pady=6, ipady=12)
        buttons = Frame(top, padx=10, pady=8)
        buttons.pack(fill="x")

        def done(ok):
            if ok:
                result["value"] = _hex(sw._node.get_color("value"))
            top.grab_release()
            top.destroy()
        Button(buttons, text="OK", command=lambda: done(True)).pack(side="left", fill="x",
                                                                   expand=True, padx=3)
        Button(buttons, text="Cancel", command=lambda: done(False)).pack(side="left",
                                                                         fill="x", expand=True,
                                                                         padx=3)
        top.bind("<Return>", lambda e: done(True))
        top.bind("<Escape>", lambda e: done(False))
        top.protocol("WM_DELETE_WINDOW", lambda: done(False))
        top.transient(parent)
        _place_window(top, parent)
        top.grab_set()
        top.wait_window()
        return result["value"]


def askcolor(color=None, **options):
    """Display dialog window for selection of a color.

    Convenience wrapper for the Chooser class. Displays the color
    chooser dialog with color as the initial value."""
    if color:
        options = options.copy()
        options["initialcolor"] = color
    return Chooser(**options).show()
