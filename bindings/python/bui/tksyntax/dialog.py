"""tkinter.dialog: the classic tk_dialog (a message with buttons).

SPDX-License-Identifier: GPL-2.0-or-later
"""

from . import Widget, _get_default_root

__all__ = ["Dialog", "DIALOG_ICON"]

DIALOG_ICON = "questhead"


class Dialog(Widget):
    def __init__(self, master=None, cnf={}, **kw):  # noqa: B006
        from .simpledialog import SimpleDialog
        cnf = dict(cnf, **kw)
        master = master or _get_default_root()
        d = SimpleDialog(master, text=cnf.get("text", ""), buttons=list(cnf.get("strings",
                                                                                ())),
                         default=cnf.get("default"), title=cnf.get("title"),
                         _icon=cnf.get("bitmap"))
        self.num = d.go()

    def destroy(self):
        pass
