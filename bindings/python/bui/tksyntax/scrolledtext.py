"""A ScrolledText widget feels like a text widget but also has a vertical
scroll bar on its right (tkinter.scrolledtext).

SPDX-License-Identifier: GPL-2.0-or-later
"""

from . import Frame, Grid, Pack, Place, Scrollbar, Text
from .constants import BOTH, LEFT, RIGHT, Y

__all__ = ["ScrolledText"]


class ScrolledText(Text):
    def __init__(self, master=None, **kw):
        self.frame = Frame(master)
        self.vbar = Scrollbar(self.frame)
        self.vbar.pack(side=RIGHT, fill=Y)
        kw["yscrollcommand"] = self.vbar.set
        Text.__init__(self, self.frame, **kw)
        self.pack(side=LEFT, fill=BOTH, expand=True)
        self.vbar["command"] = self.yview
        # Copy geometry methods of self.frame without overriding Text methods
        text_meths = vars(Text).keys()
        methods = vars(Pack).keys() | vars(Grid).keys() | vars(Place).keys()
        methods = methods.difference(text_meths)
        for m in methods:
            if m[0] != "_" and m != "config" and m != "configure":
                setattr(self, m, getattr(self.frame, m))

    def __str__(self):
        return str(self.frame)
