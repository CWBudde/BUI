"""Menus (C: ``kit/intern/kit_menus.c``): items, menu buttons, the header
menu bar and pie menus.

Items are :class:`MenuItem` objects, plain strings or ``"-"``::

    MenuBar(("File", [MenuItem("New", new, icon="FILE", shortcut="Ctrl N"),
                      MenuItem("Open Recent", submenu=[...]),
                      "-",
                      MenuItem("Quit", quit)]))

As in Blender: press-drag-release picks, hover switches between menu-bar
menus and opens submenus, typing searches every item.

SPDX-License-Identifier: GPL-2.0-or-later
"""

from __future__ import annotations

from typing import Callable, Optional, Sequence

from bui.components.core import Component

SEPARATOR = "-"


class MenuItem:
    """One menu entry. ``checked`` (True/False) shows a checkbox; ``submenu``
    (a list of items) opens another menu."""

    def __init__(self, text: str, on_click: Optional[Callable[[], None]] = None, *,
                 icon: str = None, shortcut: str = None, checked=None, submenu=None,
                 enabled: bool = True, description: str = ""):
        self.text, self.on_click, self.icon = text, on_click, icon
        self.shortcut, self.checked, self.submenu = shortcut, checked, submenu
        self.enabled, self.description = enabled, description

    def __repr__(self):
        return "MenuItem(%r)" % self.text


def menu_items(items, on_pick: Optional[Callable[[str], None]] = None):
    """Normalize items; plain strings call ``on_pick(text)``."""
    out = []
    for it in items:
        if it == SEPARATOR or it is None:
            out.append(SEPARATOR)
        elif isinstance(it, MenuItem):
            if it.submenu:
                it.submenu = menu_items(it.submenu, on_pick)
            out.append(it)
        else:
            text = str(it)
            out.append(MenuItem(text, (lambda t=text: on_pick(t)) if on_pick else None))
    return out


class MenuButton(Component):
    """Header pulldown (``File``, ``Edit`` ...)."""

    TYPE = "menubutton"

    def __init__(self, text: str, items: Sequence, *, icon: str = None, **kw):
        super().__init__(**kw)
        self.set("text", text)
        if icon:
            self.set("icon", icon)
        self.items = menu_items(items)
        self.set_menu(self.items)


class MenuBar(Component):
    """Blender's top bar: ``MenuBar(("File", [...]), ("Edit", [...]),
    right=[components])``."""

    TYPE = "menubar"

    def __init__(self, *menus, right: Sequence[Component] = (), **kw):
        super().__init__(**kw)
        self.buttons = [m if isinstance(m, Component) else MenuButton(m[0], m[1])
                        for m in menus]
        self.right = list(right)
        for c in self.right:
            c.weight = 0
        self.add(*self.buttons, *self.right)


class Menu:
    """Open a popup menu from code: ``Menu.show(ctx, owner, items, at=(x, y))``."""

    @staticmethod
    def show(ctx, owner, items, title: str = None, at=None, on_pick=None) -> None:
        ctx.open_menu(owner, menu_items(items, on_pick), title, at)


class PieMenu:
    """Radial menu around the pointer: ``PieMenu.show(ctx, items, "Mode")``.
    Up to 8 items in Blender's order W, E, S, N, NW, NE, SW, SE; move toward
    an item and click, or press 1-8."""

    @staticmethod
    def show(ctx, items, title: str = "") -> None:
        ctx.open_pie(menu_items(items), title or None)
