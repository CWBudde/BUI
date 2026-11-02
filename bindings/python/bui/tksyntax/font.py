"""tkinter.font: Font objects and named fonts.

Text is drawn in Blender's UI font; a font's size scales it (``font_scale``
on the kit node) and "bold" selects the bold face. ``measure`` uses the
native text engine.

SPDX-License-Identifier: GPL-2.0-or-later
"""

from __future__ import annotations

import itertools

import buipy

NORMAL = "normal"
ROMAN = "roman"
BOLD = "bold"
ITALIC = "italic"

_counter = itertools.count(1)
_named: dict = {}
_DEFAULT = {"family": "Segoe UI", "size": 9, "weight": NORMAL, "slant": ROMAN,
            "underline": 0, "overstrike": 0}


def nametofont(name, root=None):
    """Given the name of a tk named font, returns a Font representation."""
    _ensure_named()
    if str(name) not in _named:
        raise RuntimeError('named font %s does not already exist' % name)
    return _named[str(name)]


def families(root=None, displayof=None):
    """Get font families (as a tuple)."""
    try:
        from bui import _realtk, host
        return tuple(_realtk.module("tkinter.font").families(host._tk()))
    except Exception:
        return ("Segoe UI", "Arial", "Courier New", "Times New Roman", "TkDefaultFont")


def names(root=None):
    """Get names of defined fonts (as a tuple)."""
    _ensure_named()
    return tuple(_named)


def _parse(font):
    from ._core import _font_parts
    family, size, styles = _font_parts(font)
    return {"family": family or _DEFAULT["family"], "size": int(size) if size else
            _DEFAULT["size"], "weight": BOLD if "bold" in styles else NORMAL,
            "slant": ITALIC if "italic" in styles else ROMAN,
            "underline": 1 if "underline" in styles else 0,
            "overstrike": 1 if "overstrike" in styles else 0}


class Font:
    """Represents a named font."""

    def __init__(self, root=None, font=None, name=None, exists=False, **options):
        _ensure_named()
        self.name = str(name) if name else "font%d" % next(_counter)
        if exists:
            if self.name not in _named:
                raise RuntimeError('named font %s does not already exist' % self.name)
            self._opts = _named[self.name]._opts
            return
        self._opts = dict(_DEFAULT)
        if font is not None:
            self._opts.update(_parse(font))
        self._opts.update({k: v for k, v in options.items() if v is not None})
        if name is not None or exists is False:
            _named.setdefault(self.name, self)
        self.delete_font = True

    def __str__(self):
        return self.name

    def __repr__(self):
        return "<%s.%s object %r>" % (self.__class__.__module__, self.__class__.__qualname__,
                                     self.name)

    def __eq__(self, other):
        return isinstance(other, Font) and self.name == other.name

    def __hash__(self):
        return hash(self.name)

    def __getitem__(self, key):
        return self.cget(key)

    def __setitem__(self, key, value):
        self.configure(**{key: value})

    def copy(self):
        """Return a distinct copy of the current font."""
        return Font(**self.actual())

    def actual(self, option=None, displayof=None):
        """Return actual font attributes."""
        if option:
            return self._opts.get(option.lstrip("-"))
        return dict(self._opts)

    def cget(self, option):
        """Get font attribute."""
        return self._opts.get(option.lstrip("-"))

    def config(self, **options):
        """Modify font attributes."""
        if not options:
            return dict(self._opts)
        self._opts.update(options)
        return None

    configure = config

    def _px(self):
        size = float(self._opts.get("size", 9))
        points = size if size > 0 else -size * 0.75
        return buipy.kit_unit() * 0.55 * (points / 9.0)

    def measure(self, text, displayof=None):
        """Return text width."""
        text = str(text)
        w = buipy.measure_text(text, self._px())
        return int(round(w if w is not None else len(text) * self._px() * 0.55))

    def metrics(self, *options, **kw):
        """Return font metrics: ascent, descent, linespace, fixed."""
        px = self._px()
        m = {"ascent": int(round(px * 0.92)), "descent": int(round(px * 0.26)),
             "linespace": int(round(px * 1.45)),
             "fixed": 1 if "courier" in str(self._opts.get("family", "")).lower() or
             "mono" in str(self._opts.get("family", "")).lower() else 0}
        if options:
            return m[options[0].lstrip("-")] if len(options) == 1 else \
                {o.lstrip("-"): m[o.lstrip("-")] for o in options}
        return m

    def delete(self):
        _named.pop(self.name, None)


_NAMED_DEFAULTS = {"TkDefaultFont": 9, "TkTextFont": 9, "TkFixedFont": 10, "TkMenuFont": 9,
                   "TkHeadingFont": 9, "TkCaptionFont": 9, "TkSmallCaptionFont": 8,
                   "TkIconFont": 9, "TkTooltipFont": 8}


def _ensure_named():
    if _named:
        return
    for name, size in _NAMED_DEFAULTS.items():
        f = Font.__new__(Font)
        f.name = name
        f._opts = dict(_DEFAULT, size=size)
        if name == "TkFixedFont":
            f._opts["family"] = "Courier New"
        if name == "TkHeadingFont":
            f._opts["weight"] = BOLD
        _named[name] = f
