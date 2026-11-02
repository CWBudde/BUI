"""tkinter.commondialog: base class of the standard dialogs.

SPDX-License-Identifier: GPL-2.0-or-later
"""

__all__ = ["Dialog"]


class Dialog:
    command = None

    def __init__(self, master=None, **options):
        if master is None:
            master = options.get("parent")
        self.master = master
        self.options = options

    def _fixoptions(self):
        pass

    def _fixresult(self, widget, result):
        return result

    def _show(self, options):
        raise NotImplementedError

    def show(self, **options):
        for k, v in options.items():
            self.options[k] = v
        self._fixoptions()
        return self._fixresult(self.master, self._show(self.options))
