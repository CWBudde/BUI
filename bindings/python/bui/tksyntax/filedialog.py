"""tkinter.filedialog: the operating system's file dialogs.

Like Tk on Windows and macOS, these are native dialogs (shown through the
stdlib Tk); the option names and return values are tkinter's.

SPDX-License-Identifier: GPL-2.0-or-later
"""

from __future__ import annotations

import os

from bui import _realtk
from .commondialog import Dialog

__all__ = ["FileDialog", "LoadFileDialog", "SaveFileDialog", "Open", "SaveAs", "Directory",
           "askopenfilename", "asksaveasfilename", "askopenfilenames", "askopenfile",
           "askopenfiles", "asksaveasfile", "askdirectory"]


def _native(kind, options):
    """Call the real tkinter.filedialog function `kind` with our options."""
    from bui import host
    real = _realtk.module("tkinter.filedialog")
    opts = {k: v for k, v in options.items() if k != "parent"}
    opts["parent"] = host._tk()
    return getattr(real, kind)(**opts)


class _Dialog(Dialog):
    _KIND = "askopenfilename"

    def _show(self, options):
        return _native(self._KIND, options)

    def _fixresult(self, widget, result):
        if result and not isinstance(result, (tuple, list)):
            self.options["initialdir"], self.options["initialfile"] = os.path.split(result)
        return result


class Open(_Dialog):
    """Ask for a filename to open."""

    command = "tk_getOpenFile"

    def _show(self, options):
        kind = "askopenfilenames" if options.get("multiple") else "askopenfilename"
        opts = {k: v for k, v in options.items() if k != "multiple"}
        return _native(kind, opts)


class SaveAs(_Dialog):
    """Ask for a filename to save as."""

    command = "tk_getSaveFile"
    _KIND = "asksaveasfilename"


class Directory(Dialog):
    """Ask for a directory."""

    command = "tk_chooseDirectory"

    def _show(self, options):
        return _native("askdirectory", options)


def askopenfilename(**options):
    """Ask for a filename to open"""
    return Open(**options).show()


def asksaveasfilename(**options):
    """Ask for a filename to save as"""
    return SaveAs(**options).show()


def askopenfilenames(**options):
    """Ask for multiple filenames to open (a tuple, () if cancelled)."""
    options["multiple"] = 1
    return Open(**options).show()


def askopenfile(mode="r", **options):
    """Ask for a filename to open, and returned the opened file"""
    filename = Open(**options).show()
    return open(filename, mode) if filename else None


def askopenfiles(mode="r", **options):
    """Ask for multiple filenames and return the open file objects."""
    files = askopenfilenames(**options)
    return [open(f, mode) for f in files] if files else []


def asksaveasfile(mode="w", **options):
    """Ask for a filename to save as, and returned the opened file"""
    filename = SaveAs(**options).show()
    return open(filename, mode) if filename else None


def askdirectory(**options):
    """Ask for a directory, and return the file name"""
    return Directory(**options).show()


class FileDialog:
    """Standard file selection dialog (go() returns the chosen path or None)."""

    title = "File Selection Dialog"
    _SAVE = False

    def __init__(self, master, title=None):
        self.master = master
        if title is not None:
            self.title = title

    def go(self, dir_or_file=os.curdir, pattern="*", default="", key=None):
        directory, file = (dir_or_file, default) if os.path.isdir(dir_or_file) else \
            os.path.split(dir_or_file)
        opts = {"title": self.title, "initialdir": directory or os.curdir,
                "initialfile": file or default, "filetypes": [("Files", pattern),
                                                              ("All files", "*")]}
        result = asksaveasfilename(**opts) if self._SAVE else askopenfilename(**opts)
        return result or None

    def quit(self, how=None):
        pass


class LoadFileDialog(FileDialog):
    """File selection dialog which checks that the file exists."""

    title = "Load File Selection Dialog"


class SaveFileDialog(FileDialog):
    """File selection dialog which checks that the file may be created."""

    title = "Save File Selection Dialog"
    _SAVE = True
