"""Templates: Blender's composite property layouts, built from C nodes.

* :class:`IDSelector` — ``template_ID``: browse menu, name, fake user,
  duplicate, unlink.
* :class:`PathField` — a path with a folder button.

SPDX-License-Identifier: GPL-2.0-or-later
"""

from __future__ import annotations

from typing import Callable, Optional, Sequence

from bui.components.containers import Row
from bui.components.widgets import Dropdown, IconButton, TextField, Toggle


class IDSelector(Row):
    """Data-block selector: ``IDSelector(["Metal", "Glass"], "Metal",
    icon="MATERIAL")``."""

    def __init__(self, items: Sequence[str] = (), value: str = None, *,
                 icon: str = "MESH_DATA", fake_user: bool = False,
                 on_change: Optional[Callable[[str], None]] = None,
                 on_new: Optional[Callable[[str], None]] = None,
                 on_unlink: Optional[Callable[[], None]] = None, **kw):
        self.icon = icon
        self.on_change, self.on_new, self.on_unlink = on_change, on_new, on_unlink
        items = list(items)
        value = value if value is not None else (items[0] if items else "")
        self.browse = Dropdown("", items, value or None, icons=[icon] * len(items),
                               icon_only=True, on_change=self._picked, weight=0,
                               description="Browse data-blocks")
        self.name = TextField("", value, on_change=self._renamed, weight=6,
                              description="Data-block name")
        self.fake_user = Toggle("", fake_user, icon="FAKE_USER_OFF", icon_on="FAKE_USER_ON",
                                weight=0, description="Save this data-block even if it "
                                "has no users")
        self.new = IconButton("DUPLICATE", self._duplicate, emboss=True, weight=0,
                              description="Duplicate the data-block")
        self.unlink = IconButton("X", self._unlink, emboss=True, weight=0,
                                 description="Unlink the data-block")
        super().__init__(self.browse, self.name, self.fake_user, self.new, self.unlink,
                         align=True, **kw)

    @property
    def items(self):
        return self.browse.items

    @property
    def value(self) -> str:
        return self.name.value

    def set_value(self, name: str, notify: bool = True) -> None:
        self.name.set_value(name, notify=False)
        if name in self.items:
            self.browse.set_value(name, notify=False)
        if notify and self.on_change is not None:
            self.on_change(name)

    def _set_items(self, items):
        self.browse.items = items
        self.browse.set("icons", [self.icon] * len(items))

    def _picked(self, name):
        self.set_value(name)

    def _renamed(self, new):
        items, old = self.items, self.browse.value
        if old in items and new and new not in items:
            items[items.index(old)] = new
            self._set_items(items)
            self.browse.set_value(new, notify=False)
        if self.on_change is not None:
            self.on_change(new)

    def _duplicate(self):
        base, items = self.value or "Data", self.items
        n = 1
        while "%s.%03d" % (base, n) in items:
            n += 1
        name = "%s.%03d" % (base, n)
        self._set_items(items + [name])
        self.set_value(name)
        if self.on_new is not None:
            self.on_new(name)

    def _unlink(self):
        self.name.set_value("", notify=False)
        if self.on_unlink is not None:
            self.on_unlink()


class PathField(Row):
    """File path with a folder button (``directory=True`` picks folders).
    ``on_browse()`` may return a path; without it a native dialog opens."""

    def __init__(self, label: str = "", value: str = "", *, directory: bool = False,
                 on_browse: Optional[Callable[[], Optional[str]]] = None,
                 on_change: Optional[Callable[[str], None]] = None, **kw):
        self.directory, self.on_browse = directory, on_browse
        self.field = TextField(label, value, on_change=on_change, weight=8)
        self.button = IconButton("FILE_FOLDER", self.browse, emboss=True, weight=0,
                                 description="Open the file browser")
        super().__init__(self.field, self.button, align=True, **kw)

    @property
    def value(self) -> str:
        return self.field.value

    def set_value(self, path: str, notify: bool = True) -> None:
        self.field.set_value(path, notify)

    def browse(self) -> None:
        path = None
        if self.on_browse is not None:
            path = self.on_browse()
        else:
            try:
                from tkinter import filedialog
                path = (filedialog.askdirectory() if self.directory
                        else filedialog.askopenfilename())
            except Exception:  # headless / no tk
                path = None
        if path:
            self.set_value(path)
