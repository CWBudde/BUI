"""tkinter's classic widgets on the BUI kit (Blender look, tkinter API).

Label, Message, Button, Checkbutton, Radiobutton, Scale, Spinbox, Entry,
Listbox, Frame, LabelFrame, Scrollbar, Canvas, Text, Menu, Menubutton,
OptionMenu, PanedWindow.

SPDX-License-Identifier: GPL-2.0-or-later
"""

from __future__ import annotations

import math
import re

import buipy

from bui.components.core import EV_CHANGE, EV_CLICK, EV_MOTION, EV_PRESS, EV_SCROLL, EV_SUBMIT
from bui.components.menus import MenuItem

from ._core import (BaseWidget, Event, EventType, IntVar, StringVar, TclError, Variable, Widget,
                    XView, YView, _BindTable, _call, _chars, _command, _commands,
                    _font_scale, _get_default_root, _pads, _pixels, _setit, _set_quiet,
                    _text_width, _try_color, _var_value, _wrap, color, getboolean)
from ._images import Image, _images

_TEXT_FG = (0.9, 0.9, 0.9, 1.0)  # Blender's text color: default ink on dark canvases


def _image_of(value):
    if value in (None, ""):
        return None
    if isinstance(value, Image):
        return value
    img = _images.get(str(value))
    if img is None:
        raise TclError('image "%s" doesn\'t exist' % value)
    return img


_BITMAP_ICONS = {"error": "ERROR", "info": "INFO", "question": "QUESTION",
                 "warning": "ERROR", "hourglass": "TIME", "questhead": "QUESTION"}


# ==========================================================================
# Shared option groups


class _TextOptions:
    """text / textvariable / anchor / justify / wraplength / image / bitmap /
    width / height for text-showing widgets."""

    _TEXT_PROP = "text"     # kit property that shows the text
    _SIZE_IN_CHARS = True   # width in characters (pixels when showing an image)

    def _text_option(self, key, value) -> bool:
        if key in ("text", "wraplength"):
            self._show_text()
        elif key == "textvariable":
            var = value if isinstance(value, Variable) else None
            if var is not None:
                var.trace_add("write", lambda *a: self.winfo_exists() and self._show_text())
            self._show_text()
        elif key in ("anchor", "justify"):
            a = str(self._opts.get("anchor", "")) if "anchor" in self._opts else ""
            j = str(self._opts.get("justify", ""))
            align = "left" if "w" in a else "right" if "e" in a else \
                {"left": "left", "right": "right"}.get(j, "center")
            _set_quiet(self._node, "text_align", align)
        elif key == "image":
            self._show_image(_image_of(value))
        elif key == "bitmap":
            icon = _BITMAP_ICONS.get(str(value).lstrip("@"))
            if icon:
                _set_quiet(self._node, "icon", icon)
            elif isinstance(value, Image):
                self._show_image(value)
        elif key in ("width", "height"):
            self._apply_size()
        else:
            return False
        return True

    def _current_text(self):
        var = self._opts.get("textvariable")
        if var not in (None, ""):
            try:
                return str(_var_value(var))
            except KeyError:
                return ""
        return str(self._opts.get("text", ""))

    def _show_text(self):
        text = self._current_text()
        wl = self._opts.get("wraplength")
        if wl:
            text = _wrap(text, _pixels(wl))
        self._node.set(self._TEXT_PROP, text)

    def _show_image(self, img):
        if img is None:
            self._node.set_image()
        else:
            img._attach(self)
            self._image_changed(img)
        self._apply_size()

    def _image_changed(self, img):
        if self.winfo_exists():
            self._node.set_image(img.width(), img.height(), img.rgba())

    def _apply_size(self):
        img = self._opts.get("image")
        w, h = self._opts.get("width"), self._opts.get("height")
        params = {}
        if w not in (None, "", 0, "0"):
            params["reqwidth"] = int(_pixels(w)) if (img or not self._SIZE_IN_CHARS) else \
                _chars(int(w))
        else:
            params["reqwidth"] = 0
        if h not in (None, "", 0, "0"):
            unit = buipy.kit_unit()
            params["reqheight"] = int(_pixels(h)) if (img or not self._SIZE_IN_CHARS) else \
                max(unit, int(int(h) * unit * 0.8 + unit * 0.4))
        else:
            params["reqheight"] = 0
        self._node.geom(int(self._node.geom_get("manager")), **params)


class _Validation:
    """Entry/Spinbox validate + validatecommand (+ invalidcommand)."""

    def _validate(self, reason, new, old, action=-1, index=-1, change=""):
        mode = str(self._opts.get("validate", "none"))
        if mode == "none" or not self._opts.get("validatecommand"):
            return True
        wanted = {"key": ("key",), "focus": ("focusin", "focusout"), "focusin": ("focusin",),
                  "focusout": ("focusout",), "all": ("key", "focusin", "focusout", "forced")}
        if mode != "all" and reason not in wanted.get(mode, ()):
            return True
        subst = {"%d": str(action), "%i": str(index), "%P": new, "%s": old, "%S": change,
                 "%v": mode, "%V": reason, "%W": self._w}
        ok = self._run_vcmd(self._opts["validatecommand"], subst)
        if not ok:
            inv = self._opts.get("invalidcommand")
            if inv:
                self._run_vcmd(inv, subst)
        return ok

    @staticmethod
    def _run_vcmd(cmd, subst):
        if isinstance(cmd, (tuple, list)):
            fn = _commands.get(str(cmd[0])) or (cmd[0] if callable(cmd[0]) else None)
            args = [subst.get(str(a), a) for a in cmd[1:]]
        elif callable(cmd):
            fn, args = cmd, []
        else:
            parts = str(cmd).split()
            fn, args = _commands.get(parts[0]), [subst.get(a, a) for a in parts[1:]]
        if fn is None:
            return True
        r = fn(*args)
        return r is None or getboolean(r)


def _diff(old: str, new: str):
    """(index, removed, inserted) of a single edit."""
    a = 0
    while a < min(len(old), len(new)) and old[a] == new[a]:
        a += 1
    b = 0
    while b < min(len(old), len(new)) - a and old[-1 - b] == new[-1 - b]:
        b += 1
    return a, old[a:len(old) - b], new[a:len(new) - b]


# ==========================================================================
# Labels and buttons


class Label(_TextOptions, Widget):
    """Label widget which can display text and bitmaps."""

    _KIND = "label"
    _DEFAULTS = {"text": "", "anchor": "center", "justify": "center", "state": "normal",
                 "relief": "flat", "bd": 2, "padx": 1, "pady": 1, "width": 0, "height": 0,
                 "wraplength": 0, "compound": "none"}

    def __init__(self, master=None, cnf={}, **kw):  # noqa: B006
        Widget.__init__(self, master, "label", cnf, kw)

    def _setup(self):
        self._node.set("text_align", "center")

    def _apply(self, key, value):
        if not self._text_option(key, value):
            super()._apply(key, value)


class Message(Label):
    """Message widget: a label that wraps its text to `width` / `aspect`."""

    _DEFAULTS = dict(Label._DEFAULTS, aspect=150, width=0, justify="left")
    _SIZE_IN_CHARS = False

    def __init__(self, master=None, cnf={}, **kw):  # noqa: B006
        Widget.__init__(self, master, "message", cnf, kw)

    def _setup(self):
        self._node.set("text_align", "left")

    def _apply(self, key, value):
        if key in ("width", "aspect"):  # re-wrap whatever order options come in
            self._show_text()
        else:
            super()._apply(key, value)

    def _show_text(self):
        text = self._current_text()
        width = _pixels(self._opts["width"]) if self._opts.get("width") else 0
        if not width:  # aspect = 100 * width / height
            aspect = float(self._opts.get("aspect", 150)) / 100.0
            line_h = buipy.kit_unit() * 0.8
            width = max(40.0, math.sqrt(max(1.0, _text_width(text)) * line_h * aspect))
        self._node.set("text", _wrap(text, width))

    def _apply_size(self):
        pass


class Button(_TextOptions, Widget):
    """Button widget."""

    _KIND = "button"
    _TAKEFOCUS = True
    _DEFAULTS = {"text": "", "state": "normal", "default": "disabled", "relief": "raised",
                 "bd": 2, "width": 0, "height": 0, "command": "", "compound": "none"}

    def __init__(self, master=None, cnf={}, **kw):  # noqa: B006
        Widget.__init__(self, master, "button", cnf, kw)

    def _apply(self, key, value):
        if not self._text_option(key, value):
            super()._apply(key, value)

    def invoke(self):
        """Invoke the command associated with the button."""
        if str(self._opts.get("state", "normal")) == "disabled":
            return ""
        cmd = self._opts.get("command")
        return _command(cmd)() if cmd else ""

    def flash(self):
        """Flash the button (Blender buttons highlight on hover instead)."""

    def _event(self, kind, ref, a, b):
        if kind == EV_CLICK:
            _call(self, self.invoke)


class _Choice(_TextOptions):
    """Checkbutton / Radiobutton: text on the kit's 'label', optional
    toggle-button look (indicatoron=0)."""

    _TEXT_PROP = "label"
    _TAKEFOCUS = True

    def __init__(self, master, name, cnf, kw):
        kw = dict(cnf, **kw) if cnf else dict(kw)
        if str(kw.get("indicatoron", 1)) in ("0", "False", "false"):
            self._KIND = "toggle"  # Blender's toggle button
            self._TEXT_PROP = "text"
        Widget.__init__(self, master, name, {}, kw)

    def _apply(self, key, value):
        if key == "variable":
            if isinstance(value, Variable):
                value.trace_add("write", lambda *a: self.winfo_exists() and self._sync())
            self._sync()
        elif key in ("onvalue", "offvalue", "value"):
            self._sync()
        elif not self._text_option(key, value):
            super()._apply(key, value)

    def _var(self):
        var = self._opts.get("variable")
        if isinstance(var, str):
            from ._core import _variables
            var = _variables.get(var) or StringVar(name=var)
            self._opts["variable"] = var
        return var

    def flash(self):
        pass


class Checkbutton(_Choice, Widget):
    """Checkbutton widget which is either in on- or off-state."""

    _KIND = "checkbox"
    _DEFAULTS = {"text": "", "onvalue": 1, "offvalue": 0, "state": "normal",
                 "indicatoron": 1, "command": ""}

    def __init__(self, master=None, cnf={}, **kw):  # noqa: B006
        kw = dict(cnf, **kw) if cnf else dict(kw)
        _Choice.__init__(self, master, "checkbutton", {}, kw)
        if "variable" not in self._opts:  # Tk gives every checkbutton a variable
            self.configure(variable=IntVar(name=self._w))

    def _sync(self):
        var = self._var()
        if var is None:
            return
        on = self.cget("onvalue")
        try:
            value = var._raw()
        except AttributeError:
            return
        self._node.set("value", 1.0 if str(value) == str(on) or
                       (value is True and str(on) == "1") else 0.0)

    def _event(self, kind, ref, a, b):
        if kind == EV_CHANGE:
            on = self._node.get("value") != 0
            var = self._var()
            if var is not None:
                var.set(self.cget("onvalue") if on else self.cget("offvalue"))
            cmd = self._opts.get("command")
            if cmd:
                _call(self, _command(cmd))

    def select(self):
        """Put the button in on-state."""
        self._var().set(self.cget("onvalue"))

    def deselect(self):
        """Put the button in off-state."""
        self._var().set(self.cget("offvalue"))

    def toggle(self):
        """Toggle the button."""
        on = self._node.get("value") != 0
        self._var().set(self.cget("offvalue") if on else self.cget("onvalue"))

    def invoke(self):
        """Toggle the button and invoke a command if given as resource."""
        if str(self._opts.get("state", "normal")) == "disabled":
            return ""
        self.toggle()
        cmd = self._opts.get("command")
        return _command(cmd)() if cmd else ""


_selected_button = None


def _radio_default_var():
    global _selected_button
    if _selected_button is None:
        _selected_button = StringVar(name="selectedButton")
    return _selected_button


class Radiobutton(_Choice, Widget):
    """Radiobutton widget which shows only one of several buttons in on-state."""

    _KIND = "radio"
    _DEFAULTS = {"text": "", "state": "normal", "indicatoron": 1, "command": ""}

    def __init__(self, master=None, cnf={}, **kw):  # noqa: B006
        kw = dict(cnf, **kw) if cnf else dict(kw)
        _Choice.__init__(self, master, "radiobutton", {}, kw)
        if "variable" not in self._opts:  # Tk: one global variable for all
            self.configure(variable=_radio_default_var())
        if "value" not in self._opts:  # Tk: the value defaults to the name
            self.configure(value=self._name)

    def _sync(self):
        var = self._var()
        if var is None or "value" not in self._opts:
            return
        self._node.set("value", 1.0 if str(var._raw()) == str(self._opts["value"]) else 0.0)

    def _event(self, kind, ref, a, b):
        if kind == EV_CHANGE:
            if self._node.get("value") == 0:  # a radio can't be switched off by a click
                self._sync()
                return
            self._var().set(self._opts.get("value"))
            cmd = self._opts.get("command")
            if cmd:
                _call(self, _command(cmd))

    def select(self):
        """Put the button in on-state."""
        self._var().set(self._opts.get("value"))

    def deselect(self):
        """Put the button in off-state."""
        if str(self._var()._raw()) == str(self._opts.get("value")):
            self._var().set("")

    def invoke(self):
        """Select the button and invoke a command if given as resource."""
        if str(self._opts.get("state", "normal")) == "disabled":
            return ""
        self.select()
        cmd = self._opts.get("command")
        return _command(cmd)() if cmd else ""


# ==========================================================================
# Value widgets


def _resolution_precision(res) -> int:
    r = float(res)
    if r <= 0:
        return 2
    if r >= 1 and r == int(r):
        return 0
    return max(1, min(6, len(("%.6f" % r).rstrip("0").split(".")[1])))


class Scale(Widget):
    """Scale widget which can display a numerical scale (a Blender slider)."""

    _KIND = "slider"
    _TAKEFOCUS = True
    _DEFAULTS = {"from": 0, "to": 100, "resolution": 1, "orient": "vertical", "length": 100,
                 "showvalue": 1, "digits": 0, "tickinterval": 0, "bigincrement": 0,
                 "state": "normal", "label": "", "command": ""}

    def __init__(self, master=None, cnf={}, **kw):  # noqa: B006
        Widget.__init__(self, master, "scale", cnf, kw)

    def _setup(self):
        n = self._node
        n.set("min", 0.0)
        n.set("max", 100.0)
        n.set("step", 1.0)
        n.set("integer", True)
        n.set("precision", 0)

    def _apply(self, key, value):
        n = self._node
        if key in ("from", "to"):
            lo, hi = float(self.cget("from")), float(self.cget("to"))
            n.set("min", min(lo, hi))
            n.set("max", max(lo, hi))
            n.set("value", min(max(n.get("value"), min(lo, hi)), max(lo, hi)))
        elif key == "resolution":
            r = float(value)
            n.set("step", r if r > 0 else 0.01)
            n.set("integer", r >= 1 and r == int(r))
            n.set("precision", _resolution_precision(r))
        elif key == "label":
            n.set("label", str(value))
        elif key == "variable":
            var = value
            if isinstance(var, Variable):
                var.trace_add("write", lambda *a: self.winfo_exists() and self._from_var())
            self._from_var()
        elif key == "length":
            if str(self.cget("orient")).startswith("h"):
                n.geom(int(n.geom_get("manager")), reqwidth=int(_pixels(value)))
        elif key == "orient" and "length" in self._opts:
            self._apply("length", self._opts["length"])
        else:
            super()._apply(key, value)

    def _from_var(self):
        var = self._opts.get("variable")
        if var not in (None, ""):
            try:
                self._node.set("value", float(_var_value(var)))
            except (TypeError, ValueError):
                pass

    def _format(self, v):
        prec = _resolution_precision(self.cget("resolution"))
        return str(int(round(v))) if prec == 0 else "%.*f" % (prec, v)

    def get(self):
        """Get the current value as integer or float."""
        v = self._node.get("value")
        return int(round(v)) if self._node.get("integer") else round(v, 6)

    def set(self, value):
        """Set the value to VALUE (runs the command, like Tk)."""
        self._node.set("set_value", float(value))

    def coords(self, value=None):
        """Coordinates of the point on the trough corresponding to VALUE."""
        v = self._node.get("value") if value is None else float(value)
        lo, hi = self._node.get("min"), self._node.get("max")
        frac = 0.0 if hi == lo else (v - lo) / (hi - lo)
        w, h = self.winfo_width(), self.winfo_height()
        return (int(frac * w), h // 2)

    def identify(self, x, y):
        """'slider', 'trough1' or 'trough2' at X, Y."""
        sx = self.coords()[0]
        if abs(int(x) - sx) <= 4:
            return "slider"
        return "trough1" if int(x) < sx else "trough2"

    def _event(self, kind, ref, a, b):
        if kind == EV_CHANGE:
            v = self.get()
            var = self._opts.get("variable")
            if var not in (None, ""):
                (var if isinstance(var, Variable) else Variable(name=str(var))).set(v)
            if self._opts.get("command") and not getattr(self, "_cmd_pending", False):
                self._cmd_pending = True  # Tk runs -command when idle (once per burst)
                self.after_idle(self._run_command)

    def _run_command(self):
        self._cmd_pending = False
        cmd = self._opts.get("command")
        if cmd and self.winfo_exists():
            _call(self, _command(cmd), self._format(self.get()))


class Entry(_Validation, Widget, XView):
    """Entry widget which allows displaying simple text."""

    _KIND = "text"
    _TAKEFOCUS = True
    _DEFAULTS = {"state": "normal", "show": "", "width": 20, "validate": "none",
                 "justify": "left", "exportselection": 1}

    def __init__(self, master=None, cnf={}, **kw):  # noqa: B006
        Widget.__init__(self, master, "entry", cnf, kw)

    def _setup(self):
        self._node.set("live", True)  # Tk entries change per keystroke
        self._last = ""
        self._cursor = 0
        self._anchor = 0
        self._sel = None
        self._node.geom(0, reqwidth=_chars(20))

    def _focus_node(self):
        return self._node

    def _apply(self, key, value):
        n = self._node
        if key == "textvariable":
            if isinstance(value, Variable):
                value.trace_add("write", lambda *a: self.winfo_exists() and self._from_var())
            self._from_var()
        elif key == "show":
            n.set("mask", str(value)[:1] if value else "")
        elif key == "width":
            n.geom(int(n.geom_get("manager")), reqwidth=_chars(int(value or 20)))
        elif key == "state":
            n.set("enabled", str(value) == "normal")
        elif key in ("placeholder",):
            n.set("placeholder", str(value))
        elif key == "validate":
            if str(value) in ("focus", "focusout", "all"):
                n.set("events", True)  # Return / Tab end the edit: SUBMIT
        else:
            super()._apply(key, value)

    def _from_var(self):
        var = self._opts.get("textvariable")
        if var in (None, ""):
            return
        text = str(_var_value(var))
        if text != self._node.get_str("value"):
            self._last = text
            self._node.set("value", text)

    def _set_text(self, text, reason="forced"):
        old = self.get()
        if text == old:
            return
        idx, removed, inserted = _diff(old, text)
        if not self._validate(reason, text, old, 1 if inserted else 0, idx, inserted or removed):
            return
        self._last = text
        self._node.set("value", text)
        self._to_var(text)

    def _to_var(self, text):
        var = self._opts.get("textvariable")
        if var not in (None, "") and str(_var_value(var)) != text:
            (var if isinstance(var, Variable) else StringVar(name=str(var))).set(text)

    def _event(self, kind, ref, a, b):
        if kind == EV_CHANGE:
            new = self._node.get_str("value")
            old = self._last
            idx, removed, inserted = _diff(old, new)
            if not self._validate("key", new, old, 1 if inserted else 0, idx,
                                  inserted or removed):
                self._node.set("value", old)  # rejected: the field shows the old text
                return
            self._last = new
            self._cursor = idx + len(inserted)
            self._to_var(new)
        elif kind == EV_SUBMIT:
            self._validate("focusout", self.get(), self.get())

    # -- tkinter API ---------------------------------------------------------------
    def get(self):
        """Return the text."""
        return self._node.get_str("value")

    def index(self, index):
        """Return position of cursor."""
        s = self.get()
        if isinstance(index, int):
            return max(0, min(len(s), index))
        index = str(index)
        if index == "end":
            return len(s)
        if index == "insert":
            return min(self._cursor, len(s))
        if index == "anchor":
            return min(self._anchor, len(s))
        if index in ("sel.first", "sel.last"):
            if not self._sel:
                raise TclError("selection isn't in widget %s" % self._w)
            return self._sel[0 if index == "sel.first" else 1]
        if index.startswith("@"):
            x = int(index[1:])
            pad = buipy.kit_unit() * 0.4
            for i in range(len(s) + 1):
                if _text_width(s[:i]) + pad >= x:
                    return i
            return len(s)
        try:
            return max(0, min(len(s), int(index)))
        except ValueError:
            raise TclError('bad entry index "%s"' % index) from None

    def insert(self, index, string):
        """Insert STRING at INDEX."""
        s = self.get()
        i = self.index(index)
        self._set_text(s[:i] + str(string) + s[i:], "key")  # Tk validates edits as keys
        self._cursor = i + len(str(string))

    def delete(self, first, last=None):
        """Delete text from FIRST to LAST (not included)."""
        s = self.get()
        a = self.index(first)
        b = a + 1 if last is None else self.index(last)
        if b > a:
            self._set_text(s[:a] + s[b:], "key")
            self._cursor = a

    def icursor(self, index):
        """Insert cursor at INDEX."""
        self._cursor = self.index(index)

    def bbox(self, index):
        """Return a tuple of (x,y,width,height) of the character at INDEX."""
        i = self.index(index)
        s = self.get()
        pad = int(buipy.kit_unit() * 0.4)
        x = pad + int(_text_width(s[:i]))
        return (x, 2, max(1, int(_text_width(s[i:i + 1] or " "))), self.winfo_height() - 4)

    def scan_mark(self, x):
        self._scan = int(x)

    def scan_dragto(self, x):
        pass  # the kit scrolls the field to keep the cursor visible

    def selection_adjust(self, index):
        i = self.index(index)
        a, b = self._sel or (i, i)
        self._sel = (min(a, i), max(b, i))

    select_adjust = selection_adjust

    def selection_clear(self):
        self._sel = None

    select_clear = selection_clear

    def selection_from(self, index):
        self._anchor = self.index(index)

    select_from = selection_from

    def selection_present(self):
        return bool(self._sel and self._sel[1] > self._sel[0])

    select_present = selection_present

    def selection_range(self, start, end):
        a, b = self.index(start), self.index(end)
        self._sel = (min(a, b), max(a, b)) if b != a else None

    select_range = selection_range

    def selection_to(self, index):
        i = self.index(index)
        self._sel = (min(self._anchor, i), max(self._anchor, i))

    select_to = selection_to

    def _selected_text(self):
        return self.get()[self._sel[0]:self._sel[1]] if self.selection_present() else ""

    def _view(self, axis):
        return (0.0, 1.0)  # the field scrolls itself around the cursor

    def _view_set(self, axis, op, args):
        pass


class Spinbox(_Validation, Widget, XView):
    """Spinbox widget: a Blender number field (or an enum dropdown with values=)."""

    _TAKEFOCUS = True
    _DEFAULTS = {"from": 0.0, "to": 0.0, "increment": 1.0, "state": "normal", "width": 20,
                 "wrap": 0, "format": "", "validate": "none", "command": ""}

    def __init__(self, master=None, cnf={}, **kw):  # noqa: B006
        kw = dict(cnf, **kw) if cnf else dict(kw)
        self._KIND = "dropdown" if kw.get("values") not in (None, "", ()) else "number"
        Widget.__init__(self, master, "spinbox", {}, kw)

    def _setup(self):
        n = self._node
        self._values = []
        self._last = ""
        if self._KIND == "number":
            n.set("integer", True)
            n.set("precision", 0)
            n.set("min", 0.0)
            n.set("max", 0.0)
            n.set("step", 1.0)

    def _focus_node(self):
        return self._node

    def _apply(self, key, value):
        n = self._node
        if key == "values":
            vals = list(value) if not isinstance(value, str) else value.split()
            self._values = [str(v) for v in vals]
            if self._KIND == "dropdown":
                n.set("items", self._values)
        elif key in ("from", "to") and self._KIND == "number":
            lo, hi = float(self.cget("from")), float(self.cget("to"))
            n.set("min", min(lo, hi))
            n.set("max", max(lo, hi))
            if not lo <= n.get("value") <= hi and lo <= hi:
                n.set("value", lo)
        elif key == "increment" and self._KIND == "number":
            inc = float(value)
            n.set("step", inc)
            self._set_precision()
        elif key == "format":
            self._set_precision()
        elif key == "textvariable":
            if isinstance(value, Variable):
                value.trace_add("write", lambda *a: self.winfo_exists() and self._from_var())
            self._from_var()
        elif key == "width":
            n.geom(int(n.geom_get("manager")), reqwidth=_chars(int(value or 20)))
        elif key == "state":
            n.set("enabled", str(value) == "normal")
        else:
            super()._apply(key, value)

    def _set_precision(self):
        fmt = str(self._opts.get("format", ""))
        m = re.search(r"\.(\d+)f", fmt)
        if m:
            prec = int(m.group(1))
        else:
            prec = _resolution_precision(self.cget("increment"))
        self._node.set("precision", prec)
        self._node.set("integer", prec == 0)

    def _from_var(self):
        var = self._opts.get("textvariable")
        if var not in (None, ""):
            self._show(str(_var_value(var)))

    def _show(self, text):
        n = self._node
        if self._KIND == "dropdown":
            if text in self._values:
                n.set("value", float(self._values.index(text)))
        else:
            try:
                n.set("value", float(text))
            except (TypeError, ValueError):
                pass
        self._last = self.get()

    def get(self):
        """Return the text."""
        n = self._node
        if self._KIND == "dropdown":
            i = int(n.get("value"))
            return self._values[i] if 0 <= i < len(self._values) else ""
        v = n.get("value")
        fmt = self._opts.get("format")
        if fmt:
            try:
                return str(fmt) % v
            except (TypeError, ValueError):
                pass
        return str(int(round(v))) if n.get("integer") else repr(round(v, 6))

    def _set_text(self, text, reason="forced"):
        old = self.get()
        if not self._validate(reason, text, old):
            return
        self._show(text)
        self._after_change()

    def _after_change(self):
        var = self._opts.get("textvariable")
        if var not in (None, ""):
            (var if isinstance(var, Variable) else StringVar(name=str(var))).set(self.get())

    def insert(self, index, s):
        cur = self.get()
        i = self.index(index)
        self._set_text(cur[:i] + str(s) + cur[i:])

    def delete(self, first, last=None):
        cur = self.get()
        a = self.index(first)
        b = a + 1 if last is None else self.index(last)
        self._set_text(cur[:a] + cur[b:])

    def index(self, index):
        s = self.get()
        if str(index) == "end":
            return len(s)
        if str(index) in ("insert", "anchor"):
            return len(s)
        return max(0, min(len(s), int(index)))

    def icursor(self, index):
        pass

    def bbox(self, index):
        return (0, 0, self.winfo_width(), self.winfo_height())

    def identify(self, x, y):
        """'entry', 'buttonup' or 'buttondown' at X, Y."""
        w = self.winfo_width()
        arrow = min(w // 3, int(self.winfo_height() * 0.7))
        if int(x) < arrow:
            return "buttondown"
        if int(x) >= w - arrow:
            return "buttonup"
        return "entry"

    def invoke(self, element):
        """Press 'buttonup' or 'buttondown'."""
        n = self._node
        if self._KIND == "dropdown":
            i = int(n.get("value")) + (1 if element == "buttonup" else -1)
            if self.cget("wrap") not in (0, "0", False):
                i %= max(1, len(self._values))
            n.set("set_value", float(max(0, min(len(self._values) - 1, i))))
        else:
            step = float(self.cget("increment")) * (1 if element == "buttonup" else -1)
            n.set("set_value", n.get("value") + step)
        return ""

    def scan(self, *args):
        return ""

    def scan_mark(self, x):
        pass

    def scan_dragto(self, x):
        pass

    def selection(self, *args):
        return ""

    def selection_adjust(self, index):
        pass

    def selection_clear(self):
        pass

    def selection_element(self, element=None):
        return "none"

    def selection_from(self, index):
        pass

    def selection_present(self):
        return False

    def selection_range(self, start, end):
        pass

    def selection_to(self, index):
        pass

    select_adjust, select_clear, select_from = selection_adjust, selection_clear, selection_from
    select_present, select_range, select_to = selection_present, selection_range, selection_to

    def _view(self, axis):
        return (0.0, 1.0)

    def _view_set(self, axis, op, args):
        pass

    def _event(self, kind, ref, a, b):
        if kind == EV_CHANGE:
            new, old = self.get(), self._last
            if not self._validate("key", new, old):
                self._show(old)
                return
            self._last = new
            self._after_change()
            cmd = self._opts.get("command")
            if cmd:
                _call(self, _command(cmd))


# ==========================================================================
# Listbox


_SELMODES = {"browse": 3, "single": 3, "multiple": 1, "extended": 2}


class Listbox(Widget, XView, YView):
    """Listbox widget which can display a list of strings."""

    _KIND = "list"
    _TAKEFOCUS = True
    _DEFAULTS = {"selectmode": "browse", "height": 10, "width": 20, "activestyle": "dotbox",
                 "state": "normal", "exportselection": 1}

    def __init__(self, master=None, cnf={}, **kw):  # noqa: B006
        Widget.__init__(self, master, "listbox", cnf, kw)

    def _setup(self):
        n = self._node
        self._items = []
        self._item_opts = {}
        self._anchor = 0
        self._last_sel = ()
        n.set("rows", 10)
        n.set("selmode", 3)
        n.geom(0, reqwidth=_chars(20))

    def _apply(self, key, value):
        n = self._node
        if key == "height":
            n.set("rows", int(value) or 10)
        elif key == "width":
            n.geom(int(n.geom_get("manager")), reqwidth=_chars(int(value or 20)))
        elif key == "selectmode":
            n.set("selmode", float(_SELMODES.get(str(value), 3)))
        elif key == "listvariable":
            if isinstance(value, Variable):
                value.trace_add("write", lambda *a: self.winfo_exists() and self._from_var())
            self._from_var()
        elif key in ("xscrollcommand", "yscrollcommand"):
            n.set("noscrollbar", bool(value) or bool(self._opts.get(
                "yscrollcommand" if key[0] == "x" else "xscrollcommand")))
            n.set("viewevents", True)
        elif key == "state":
            n.set("enabled", str(value) != "disabled")
        else:
            super()._apply(key, value)

    def _from_var(self):
        var = self._opts.get("listvariable")
        if var in (None, ""):
            return
        value = _var_value(var)
        if isinstance(value, str):
            value = re.findall(r"\{([^}]*)\}|(\S+)", value)
            value = [a or b for a, b in value]
        self._set_items([str(v) for v in value], sync_var=False)

    def _set_items(self, items, sync_var=True):
        marked = set(self.curselection())
        self._items = items
        self._node.set("items", items)
        self._node.set("clearmarks", 1)
        for i in marked:
            if i < len(items):
                self._node.set("mark%d" % i, 1)
        if sync_var:
            var = self._opts.get("listvariable")
            if isinstance(var, Variable):
                var._value = tuple(items)
                var._fire("write")

    def _index(self, index, end_is_size=True):
        if isinstance(index, int):
            return index
        s = str(index)
        if s == "end":
            return len(self._items) if end_is_size else max(0, len(self._items) - 1)
        if s == "active":
            return int(self._node.get("value"))
        if s == "anchor":
            return self._anchor
        if s.startswith("@"):
            _, _, y = s[1:].partition(",")
            return self.nearest(int(y))
        try:
            return int(s)
        except ValueError:
            raise TclError('bad listbox index "%s": must be active, anchor, end, @x,y, '
                           'or a number' % index) from None

    def _range(self, first, last):
        a = self._index(first, False)
        b = a if last is None else self._index(last, False)
        return a, b

    # -- items -------------------------------------------------------------------------
    def insert(self, index, *elements):
        """Insert ELEMENTS at INDEX."""
        i = min(self._index(index), len(self._items))
        marked = self.curselection()
        items = self._items[:i] + [str(e) for e in elements] + self._items[i:]
        self._set_items(items)
        self._node.set("clearmarks", 1)
        for m in marked:
            self._node.set("mark%d" % (m + len(elements) if m >= i else m), 1)

    def delete(self, first, last=None):
        """Delete items from FIRST to LAST (included)."""
        a, b = self._range(first, last)
        if b < a or not self._items:
            return
        marked = self.curselection()
        items = self._items[:a] + self._items[b + 1:]
        self._set_items(items)
        self._node.set("clearmarks", 1)
        for m in marked:
            if m < a:
                self._node.set("mark%d" % m, 1)
            elif m > b:
                self._node.set("mark%d" % (m - (b - a + 1)), 1)

    def get(self, first, last=None):
        """Get list of items from FIRST to LAST (included)."""
        a, b = self._range(first, last)
        if last is None:
            return self._items[a] if 0 <= a < len(self._items) else ""
        return tuple(self._items[a:b + 1])

    def size(self):
        """Return the number of elements in the listbox."""
        return len(self._items)

    def index(self, index):
        """Return index of item identified with INDEX."""
        i = self._index(index)
        return None if i is None else i

    def itemconfigure(self, index, cnf=None, **kw):
        i = self._index(index, False)
        if not kw and not cnf:
            opts = self._item_opts.get(i, {})
            return {k: (k, "", "", "", v) for k, v in opts.items()}
        self._item_opts.setdefault(i, {}).update(dict(cnf or {}, **kw))
        return None

    itemconfig = itemconfigure

    def itemcget(self, index, option):
        return self._item_opts.get(self._index(index, False), {}).get(option.lstrip("-"), "")

    # -- selection ------------------------------------------------------------------------
    def curselection(self):
        """Return the indices of currently selected item."""
        n = self._node
        return tuple(i for i in range(len(self._items)) if n.get("mark%d" % i))

    def selection_set(self, first, last=None):
        """Set the selection from FIRST to LAST (included) without changing
        the currently selected elements."""
        a, b = self._range(first, last)
        for i in range(max(0, a), min(len(self._items), b + 1)):
            self._node.set("mark%d" % i, 1)
        self._last_sel = self.curselection()

    select_set = selection_set

    def selection_clear(self, first, last=None):
        """Clear the selection from FIRST to LAST (included)."""
        a, b = self._range(first, last)
        for i in range(max(0, a), min(len(self._items), b + 1)):
            self._node.set("mark%d" % i, 0)
        self._last_sel = self.curselection()

    select_clear = selection_clear

    def selection_includes(self, index):
        """Return True if INDEX is part of the selection."""
        return bool(self._node.get("mark%d" % self._index(index, False)))

    select_includes = selection_includes

    def selection_anchor(self, index):
        """Set the fixed end of the selection to INDEX."""
        self._anchor = self._index(index, False)

    select_anchor = selection_anchor

    def activate(self, index):
        """Activate item identified by INDEX."""
        self._node.set("value", float(max(0, min(len(self._items) - 1, self._index(index,
                                                                                    False)))))

    def _selected_text(self):
        return "\n".join(self._items[i] for i in self.curselection())

    # -- geometry ---------------------------------------------------------------------------
    def _row_h(self):
        return buipy.kit_unit()

    def _first_row(self):
        return int(round(self._node.get("yfirst") * max(1, len(self._items))))

    def nearest(self, y):
        """Get index of item which is nearest to y coordinate Y."""
        if not self._items:
            return -1
        pad = round(0.15 * buipy.kit_unit())
        return max(0, min(len(self._items) - 1, self._first_row() +
                          (int(y) - pad) // self._row_h()))

    def bbox(self, index):
        """Return a tuple of X1,Y1,X2,Y2 coordinates for a rectangle which
        encloses the item identified by the given index."""
        i = self._index(index, False)
        first = self._first_row()
        vis = int(self._node.get("yvisible"))
        if not first <= i < first + vis:
            return None
        pad = round(0.15 * buipy.kit_unit())
        return (pad, pad + (i - first) * self._row_h(), self.winfo_width() - 2 * pad,
                self._row_h())

    def see(self, index):
        """Scroll such that INDEX is visible."""
        i = self._index(index, False)
        total = max(1, len(self._items))
        first = self._first_row()
        vis = max(1, int(self._node.get("yvisible")))
        if i < first:
            self._node.set("yview", i / total)
        elif i >= first + vis:
            self._node.set("yview", (i - vis + 1) / total)
        self._view_changed()

    def scan_mark(self, x, y):
        self._scan = (int(y), self._first_row())

    def scan_dragto(self, x, y):
        y0, first = getattr(self, "_scan", (int(y), self._first_row()))
        self._node.set("yview", (first - (int(y) - y0) * 10 // self._row_h()) /
                       max(1, len(self._items)))
        self._view_changed()

    def _event(self, kind, ref, a, b):
        if kind == EV_CHANGE:
            sel = self.curselection()
            if sel != self._last_sel:
                self._last_sel = sel
                self.event_generate("<<ListboxSelect>>")
            if self.cget("exportselection") not in (0, "0", False) and sel:
                self.selection_own()


# ==========================================================================
# Frames


class Frame(Widget):
    """Frame widget which may contain other widgets and can have a 3D border."""

    _KIND = "frame"
    _DEFAULTS = {"width": 0, "height": 0, "relief": "flat", "bd": 0, "padx": 0, "pady": 0}

    def __init__(self, master=None, cnf={}, **kw):  # noqa: B006
        Widget.__init__(self, master, "frame", cnf, kw)

    def _apply(self, key, value):
        if key in ("width", "height"):
            self._size_changed()
        elif key in ("padx", "pady", "padding"):
            pads = _pads(value)
            self._node.set("padding", int(max(pads)))
        else:
            super()._apply(key, value)

    def _size_changed(self):
        """Tk: width/height apply when the frame doesn't propagate its
        children's size (pack_propagate(False)) or has no managed children."""
        managed = any(c.winfo_manager() in ("pack", "grid", "place")
                      for c in self.winfo_children())
        own = not managed or bool(self._node.get("nopropagate"))
        w, h = self._opts.get("width", 0), self._opts.get("height", 0)
        self._node.geom(int(self._node.geom_get("manager")),
                        reqwidth=int(_pixels(w or 0)) if own else 0,
                        reqheight=int(_pixels(h or 0)) if own else 0)


class LabelFrame(Frame):
    """Labelframe widget: a Blender panel box with its title."""

    _DEFAULTS = dict(Frame._DEFAULTS, text="", labelanchor="nw")

    def __init__(self, master=None, cnf={}, **kw):  # noqa: B006
        Widget.__init__(self, master, "labelframe", cnf, kw)

    def _setup(self):
        self._node.set("padding", round(0.3 * buipy.kit_unit()))

    def _apply(self, key, value):
        if key == "text":
            self._node.set("text", str(value))
        elif key == "labelwidget" and value is not None:
            self._node.set("text", str(value.cget("text")))
        else:
            super()._apply(key, value)


# ==========================================================================
# Scrollbar


class Scrollbar(Widget):
    """Scrollbar widget which displays a slider at a certain position."""

    _KIND = "scrollbar"
    _DEFAULTS = {"orient": "vertical", "width": 0, "jump": 0, "command": ""}

    def __init__(self, master=None, cnf={}, **kw):  # noqa: B006
        Widget.__init__(self, master, "scrollbar", cnf, kw)

    def _apply(self, key, value):
        if key == "orient":
            self._node.set("vertical", not str(value).startswith("h"))
        elif key == "width" and value:
            vert = not str(self.cget("orient")).startswith("h")
            self._node.geom(int(self._node.geom_get("manager")),
                            **{"reqwidth" if vert else "reqheight": int(_pixels(value))})
        else:
            super()._apply(key, value)

    def set(self, first, last):
        """Set the fractional values of the slider position (upper and lower
        ends as value between 0 and 1)."""
        self._node.set("first", float(first))
        self._node.set("last", float(last))

    def get(self):
        """Return the current fractional values (upper and lower end) of the
        slider position."""
        return (round(self._node.get("first"), 6), round(self._node.get("last"), 6))

    def activate(self, index=None):
        if index is None:
            return getattr(self, "_active", None)
        self._active = index
        return None

    def _length(self):
        return self.winfo_height() if self._node.get("vertical") else self.winfo_width()

    def delta(self, deltax, deltay):
        """Return the fractional change of the scrollbar setting if it
        would be moved by DELTAX or DELTAY pixels."""
        d = deltay if self._node.get("vertical") else deltax
        return float(d) / max(1, self._length())

    def fraction(self, x, y):
        """Return the fractional value which corresponds to a slider
        position of X,Y."""
        v = y if self._node.get("vertical") else x
        return max(0.0, min(1.0, float(v) / max(1, self._length())))

    def identify(self, x, y):
        """Return the element under position X,Y as one of
        "arrow1","slider","arrow2" or ""."""
        f = self.fraction(x, y)
        first, last = self.get()
        if f < first:
            return "trough1"
        if f > last:
            return "trough2"
        return "slider"

    def _event(self, kind, ref, a, b):
        if kind == EV_SCROLL:
            cmd = self._opts.get("command")
            if not cmd:
                return
            fn = _command(cmd)
            if int(a) == 0:
                _call(self, fn, "moveto", repr(float(b)))
            else:
                _call(self, fn, "scroll", str(int(b)), "units" if int(a) == 1 else "pages")


# ==========================================================================
# Canvas


_CANVAS_KIT = {"rectangle": 0, "oval": 1, "line": 2, "polygon": 3, "text": 4, "arc": 5,
               "image": 6, "bitmap": 6}
_SUB = 1 << 24  # kit ids of an item's parts (line arrows)


class _Item:
    __slots__ = ("id", "type", "coords", "tags", "opts", "arrows")

    def __init__(self, iid, kind, coords, tags, opts):
        self.id, self.type, self.coords, self.tags, self.opts = iid, kind, coords, tags, opts
        self.arrows = 0  # bit k: arrowhead sub-item k exists in the kit


def _flat(coords):
    out = []
    for c in coords:
        if isinstance(c, (tuple, list)):
            out.extend(_flat(c))
        else:
            out.append(float(_pixels(c)))
    return out


def _spline(pts, steps):
    """Tk's smoothed line: quadratic B-spline through segment midpoints."""
    if len(pts) < 6:
        return pts
    xy = list(zip(pts[0::2], pts[1::2]))
    out = [xy[0]]
    for i in range(1, len(xy) - 1):
        p0 = xy[0] if i == 1 else ((xy[i - 1][0] + xy[i][0]) / 2, (xy[i - 1][1] + xy[i][1]) / 2)
        p2 = xy[-1] if i == len(xy) - 2 else ((xy[i][0] + xy[i + 1][0]) / 2,
                                               (xy[i][1] + xy[i + 1][1]) / 2)
        p1 = xy[i]
        for k in range(1, steps + 1):
            t = k / steps
            out.append(((1 - t) ** 2 * p0[0] + 2 * (1 - t) * t * p1[0] + t * t * p2[0],
                        (1 - t) ** 2 * p0[1] + 2 * (1 - t) * t * p1[1] + t * t * p2[1]))
    return [v for p in out for v in p]


def _seg_dist(px, py, x1, y1, x2, y2):
    dx, dy = x2 - x1, y2 - y1
    t = 0.0 if dx == dy == 0 else max(0.0, min(1.0, ((px - x1) * dx + (py - y1) * dy) /
                                               (dx * dx + dy * dy)))
    return math.hypot(px - (x1 + t * dx), py - (y1 + t * dy))


def _point_in_poly(x, y, pts):
    inside = False
    n = len(pts) // 2
    for i in range(n):
        x1, y1 = pts[2 * i], pts[2 * i + 1]
        x2, y2 = pts[2 * ((i + 1) % n)], pts[2 * ((i + 1) % n) + 1]
        if (y1 > y) != (y2 > y) and x < (x2 - x1) * (y - y1) / ((y2 - y1) or 1e-9) + x1:
            inside = not inside
    return inside


class Canvas(Widget, XView, YView):
    """Canvas widget to display graphical elements like lines or text."""

    _KIND = "canvas"
    _TAKEFOCUS = True
    _DEFAULTS = {"width": 378, "height": 265, "bg": "#282828", "confine": 1,
                 "closeenough": 1.0, "scrollregion": "", "xscrollincrement": 0,
                 "yscrollincrement": 0, "state": "normal"}

    def __init__(self, master=None, cnf={}, **kw):  # noqa: B006
        Widget.__init__(self, master, "canvas", cnf, kw)

    def _setup(self):
        self._items: dict = {}
        self._order: list = []
        self._next = 1
        self._item_binds: dict = {}
        self._current = None
        self._focus_item = None
        self._sel = None
        self._node.set("bg", color("#282828"))
        self._node.geom(0, reqwidth=378, reqheight=265)

    def _apply(self, key, value):
        n = self._node
        if key in ("width", "height"):
            n.geom(int(n.geom_get("manager")), **{"req" + key: int(_pixels(value))})
        elif key in ("scrollregion", "xscrollcommand", "yscrollcommand"):
            n.set("events", True)  # <Configure> keeps the scrollbars right
            if key == "scrollregion":
                self._confine()
            self._view_changed()
        elif key == "state":
            n.set("enabled", str(value) != "disabled")
        else:
            super()._apply(key, value)

    # -- item storage -------------------------------------------------------------------------
    def _create(self, kind, args, kw):
        coords = _flat(args)
        kw = dict(kw)
        if args and isinstance(args[-1], dict):
            kw = dict(args[-1], **kw)
            coords = _flat(args[:-1])
        tags = kw.pop("tags", ())
        tags = list(tags.split() if isinstance(tags, str) else tags)
        iid = self._next
        self._next += 1
        item = _Item(iid, kind, coords, tags, kw)
        self._items[iid] = item
        self._order.append(iid)
        self._push(item)
        return iid

    def _ink(self, item, key, default):
        v = item.opts.get(key, default)
        return None if v in ("", None) else _try_color(v)

    def _push(self, item):
        """Send one item to the kit display list."""
        o, kind = item.opts, item.type
        if kind == "window":
            return self._place_window(item)
        hidden = o.get("state", "") == "hidden"
        coords = item.coords
        common = dict(width=float(o.get("width", 1) or 1), hidden=hidden)
        if kind in ("rectangle", "oval", "arc"):
            fill = self._ink(item, "fill", "")
            outline = self._ink(item, "outline", "#e6e6e6")
            if kind == "arc" and str(o.get("style", "pieslice")) == "arc":
                fill = None
            self._node.canvas_set(item.id, _CANVAS_KIT[kind], coords, fill=fill, outline=outline,
                            start=float(o.get("start", 0)), extent=float(o.get("extent", 90)),
                            **common)
        elif kind in ("line", "polygon"):
            pts = coords
            if getboolean(o.get("smooth", False)) and o.get("smooth") not in ("", 0):
                pts = _spline(coords, int(o.get("splinesteps", 12)))
            ink = self._ink(item, "fill", "#e6e6e6")
            if kind == "line":
                self._node.canvas_set(item.id, 2, pts, outline=ink, **common)
                self._push_arrows(item, pts, ink, hidden)
            else:
                self._node.canvas_set(item.id, 3, pts, fill=ink,
                                outline=self._ink(item, "outline", ""), **common)
        elif kind == "text":
            text = str(o.get("text", ""))
            if o.get("width"):
                text = _wrap(text, _pixels(o["width"]))
            scale = _font_scale(o.get("font")) or 1.0
            self._node.canvas_set(item.id, 4, coords[:2], fill=self._ink(item, "fill", "#e6e6e6"),
                            text=text, anchor=_kit_anchor(o.get("anchor", "center")),
                            size=_font_px_kit(scale), hidden=hidden)
        elif kind in ("image", "bitmap"):
            img = _image_of(o.get("image")) if kind == "image" else self._bitmap(o)
            if img is None:
                self._node.canvas_delete(item.id)
                return None
            img._attach(self)
            self._node.canvas_set(item.id, 6, coords[:2], anchor=_kit_anchor(o.get("anchor",
                                                                            "center")),
                            image=(img.width(), img.height(), img.rgba()), hidden=hidden)
        return None

    def _bitmap(self, o):
        bm = o.get("bitmap")
        if isinstance(bm, Image):
            return bm
        return None

    def _push_arrows(self, item, pts, ink, hidden):
        arrow = str(item.opts.get("arrow", "none"))
        shape = item.opts.get("arrowshape", (8, 10, 3))
        a, b, c = (float(v) for v in (shape.split() if isinstance(shape, str) else shape))
        width = float(item.opts.get("width", 1) or 1)
        for k, (tip, base) in enumerate(((pts[0:2], pts[2:4]), (pts[-2:], pts[-4:-2]))):
            kid = item.id + _SUB * (k + 1)
            if arrow in (("first", "last")[k], "both") and len(pts) >= 4:
                dx, dy = tip[0] - base[0], tip[1] - base[1]
                d = math.hypot(dx, dy) or 1.0
                ux, uy = dx / d, dy / d
                bx, by = tip[0] - ux * b, tip[1] - uy * b
                wing = c + width / 2
                poly = [tip[0], tip[1], bx - uy * wing, by + ux * wing,
                        tip[0] - ux * a, tip[1] - uy * a, bx + uy * wing, by - ux * wing]
                self._node.canvas_set(kid, 3, poly, fill=ink, hidden=hidden)
                item.arrows |= 1 << k
            elif item.arrows & (1 << k):  # only delete what was created
                self._node.canvas_delete(kid)
                item.arrows &= ~(1 << k)

    def _place_window(self, item):
        w = item.opts.get("window")
        if w in (None, ""):
            return None
        w = w if isinstance(w, BaseWidget) else self.nametowidget(w)
        if w._node.parent is not self._node:
            self._node.add(w._node)
        params = {"reset_place": 1, "x": item.coords[0], "y": item.coords[1],
                  "anchor": _kit_anchor(item.opts.get("anchor", "center"))}
        for k in ("width", "height"):
            if item.opts.get(k):
                params[k] = float(_pixels(item.opts[k]))
        w._node.geom(3, **params)
        w._ginfo = {"manager": "canvas", "in": self}
        if item.opts.get("state") == "hidden":
            w._node.set("visible", False)
        return None

    def _image_changed(self, img):
        for item in self._items.values():
            if item.opts.get("image") is img or str(item.opts.get("image")) == str(img):
                self._push(item)

    # -- creation ------------------------------------------------------------------------
    def create_arc(self, *args, **kw):
        """Create arc shaped region with coordinates x1,y1,x2,y2."""
        return self._create("arc", args, kw)

    def create_bitmap(self, *args, **kw):
        """Create bitmap with coordinates x1,y1."""
        return self._create("bitmap", args, kw)

    def create_image(self, *args, **kw):
        """Create image item with coordinates x1,y1."""
        return self._create("image", args, kw)

    def create_line(self, *args, **kw):
        """Create line with coordinates x1,y1,...,xn,yn."""
        return self._create("line", args, kw)

    def create_oval(self, *args, **kw):
        """Create oval with coordinates x1,y1,x2,y2."""
        return self._create("oval", args, kw)

    def create_polygon(self, *args, **kw):
        """Create polygon with coordinates x1,y1,...,xn,yn."""
        return self._create("polygon", args, kw)

    def create_rectangle(self, *args, **kw):
        """Create rectangle with coordinates x1,y1,x2,y2."""
        return self._create("rectangle", args, kw)

    def create_text(self, *args, **kw):
        """Create text with coordinates x1,y1."""
        return self._create("text", args, kw)

    def create_window(self, *args, **kw):
        """Create window with coordinates x1,y1,x2,y2."""
        return self._create("window", args, kw)

    # -- tags and searches ----------------------------------------------------------------------
    def _ids(self, tag_or_id):
        """Item ids for an id, a tag or a tag expression ("a&&!b", "a||b")."""
        if isinstance(tag_or_id, int) or (isinstance(tag_or_id, str) and tag_or_id.isdigit()):
            i = int(tag_or_id)
            return [i] if i in self._items else []
        t = str(tag_or_id)
        if t == "all":
            return list(self._order)
        if t == "current":
            return [self._current] if self._current in self._items else []
        if any(op in t for op in ("&&", "||", "^", "!")):
            return [i for i in self._order if self._tag_expr(self._items[i], t)]
        return [i for i in self._order if t in self._items[i].tags]

    def _tag_expr(self, item, expr):
        tags = set(item.tags) | {"all"}
        for alt in expr.split("||"):
            ok = True
            for term in alt.split("&&"):
                term = term.strip().strip("()")
                neg = term.startswith("!")
                if (term.lstrip("!").strip() in tags) == neg:
                    ok = False
                    break
            if ok:
                return True
        return False

    def addtag(self, *args):
        """Internal function: addtag(newtag, 'withtag', tag) etc."""
        newtag, spec = args[0], args[1:]
        for i in self.find(*spec):
            if newtag not in self._items[i].tags:
                self._items[i].tags.append(newtag)

    def addtag_above(self, newtag, tagOrId):
        self.addtag(newtag, "above", tagOrId)

    def addtag_all(self, newtag):
        self.addtag(newtag, "all")

    def addtag_below(self, newtag, tagOrId):
        self.addtag(newtag, "below", tagOrId)

    def addtag_closest(self, newtag, x, y, halo=None, start=None):
        self.addtag(newtag, "closest", x, y, halo, start)

    def addtag_enclosed(self, newtag, x1, y1, x2, y2):
        self.addtag(newtag, "enclosed", x1, y1, x2, y2)

    def addtag_overlapping(self, newtag, x1, y1, x2, y2):
        self.addtag(newtag, "overlapping", x1, y1, x2, y2)

    def addtag_withtag(self, newtag, tagOrId):
        self.addtag(newtag, "withtag", tagOrId)

    def dtag(self, *args):
        """Delete tag or id given as last arguments in ARGS from items
        identified by first argument in ARGS."""
        target = args[0]
        tag = args[1] if len(args) > 1 else target
        for i in self._ids(target):
            if tag in self._items[i].tags:
                self._items[i].tags.remove(tag)

    def find(self, *args):
        """Internal function: find('withtag', tag) etc."""
        kind, rest = args[0], args[1:]
        if kind == "above":
            ids = self._ids(rest[0])
            if not ids:
                return ()
            k = self._order.index(ids[-1])
            return tuple(self._order[k + 1:k + 2])
        if kind == "below":
            ids = self._ids(rest[0])
            if not ids:
                return ()
            k = self._order.index(ids[0])
            return tuple(self._order[k - 1:k]) if k > 0 else ()
        if kind == "all":
            return tuple(self._order)
        if kind == "withtag":
            return tuple(self._ids(rest[0]))
        if kind == "closest":
            return self.find_closest(*rest)
        if kind == "enclosed":
            return self.find_enclosed(*rest)
        if kind == "overlapping":
            return self.find_overlapping(*rest)
        raise TclError('bad search command "%s"' % kind)

    def find_above(self, tagOrId):
        return self.find("above", tagOrId)

    def find_all(self):
        return tuple(self._order)

    def find_below(self, tagOrId):
        return self.find("below", tagOrId)

    def find_withtag(self, tagOrId):
        return tuple(self._ids(tagOrId))

    def find_enclosed(self, x1, y1, x2, y2):
        out = []
        for i in self._order:
            bb = self._item_bbox(self._items[i])
            if bb and bb[0] >= x1 and bb[1] >= y1 and bb[2] <= x2 and bb[3] <= y2:
                out.append(i)
        return tuple(out)

    def find_overlapping(self, x1, y1, x2, y2):
        out = []
        for i in self._order:
            bb = self._item_bbox(self._items[i])
            if bb and bb[0] <= x2 and bb[2] >= x1 and bb[1] <= y2 and bb[3] >= y1:
                out.append(i)
        return tuple(out)

    def find_closest(self, x, y, halo=None, start=None):
        x, y = float(x), float(y)
        best, best_d = None, None
        for i in reversed(self._order):
            item = self._items[i]
            if item.opts.get("state") == "hidden":
                continue
            d = self._distance(item, x, y)
            if best_d is None or d < best_d:
                best, best_d = i, d
            if d <= float(halo or 0):
                return (i,)
        return (best,) if best is not None else ()

    def gettags(self, tagOrId):
        ids = self._ids(tagOrId)
        return tuple(self._items[ids[0]].tags) if ids else ()

    def type(self, tagOrId):
        ids = self._ids(tagOrId)
        return self._items[ids[0]].type if ids else None

    # -- geometry of items --------------------------------------------------------------------
    def _item_bbox(self, item):
        c = item.coords
        if item.type in ("text", "image", "bitmap", "window") and len(c) >= 2:
            w, h = self._item_size(item)
            ax, ay = _anchor_offset(item.opts.get("anchor", "center"), w, h)
            x0, y0 = c[0] - ax, c[1] - ay
            return (x0, y0, x0 + w, y0 + h)
        if len(c) < 2:
            return None
        xs, ys = c[0::2], c[1::2]
        pad = float(item.opts.get("width", 1) or 1) / 2
        return (min(xs) - pad, min(ys) - pad, max(xs) + pad, max(ys) + pad)

    def _item_size(self, item):
        o = item.opts
        if item.type == "text":
            text = str(o.get("text", ""))
            if o.get("width"):
                text = _wrap(text, _pixels(o["width"]))
            lines = text.split("\n")
            scale = _font_scale(o.get("font")) or 1.0
            w = max(_text_width(line) for line in lines) * scale
            return w, len(lines) * buipy.kit_unit() * 0.8 * scale
        if item.type in ("image", "bitmap"):
            img = _image_of(o.get("image")) if item.type == "image" else self._bitmap(o)
            return (img.width(), img.height()) if img else (0, 0)
        w = o.get("window")
        if w:
            w = w if isinstance(w, BaseWidget) else self.nametowidget(w)
            return (float(_pixels(o["width"])) if o.get("width") else w.winfo_reqwidth(),
                    float(_pixels(o["height"])) if o.get("height") else w.winfo_reqheight())
        return 0, 0

    def _distance(self, item, x, y):
        c, t = item.coords, item.type
        width = float(item.opts.get("width", 1) or 1)
        if t == "line" and len(c) >= 4:
            return max(0.0, min(_seg_dist(x, y, c[i], c[i + 1], c[i + 2], c[i + 3])
                                for i in range(0, len(c) - 3, 2)) - width / 2)
        if t == "polygon" and len(c) >= 6:
            if _point_in_poly(x, y, c):
                return 0.0
            n = len(c) // 2
            return min(_seg_dist(x, y, c[2 * i], c[2 * i + 1], c[2 * ((i + 1) % n)],
                                 c[2 * ((i + 1) % n) + 1]) for i in range(n))
        if t == "oval" and len(c) >= 4:
            cx, cy = (c[0] + c[2]) / 2, (c[1] + c[3]) / 2
            rx, ry = abs(c[2] - c[0]) / 2 or 1, abs(c[3] - c[1]) / 2 or 1
            r = math.hypot((x - cx) / rx, (y - cy) / ry)
            filled = item.opts.get("fill") not in (None, "")
            if r <= 1 and filled:
                return 0.0
            return abs(r - 1) * min(rx, ry)
        bb = self._item_bbox(item)
        if not bb:
            return 1e9
        dx = max(bb[0] - x, 0, x - bb[2])
        dy = max(bb[1] - y, 0, y - bb[3])
        if t == "rectangle" and dx == 0 and dy == 0 and item.opts.get("fill") in (None, ""):
            return max(0.0, min(x - bb[0], bb[2] - x, y - bb[1], bb[3] - y) - width / 2)
        return math.hypot(dx, dy)

    def bbox(self, *args):
        """Return a tuple of X1,Y1,X2,Y2 coordinates for a rectangle which
        encloses all items with tags specified as arguments."""
        boxes = [self._item_bbox(self._items[i]) for t in (args or ("all",))
                 for i in self._ids(t)]
        boxes = [b for b in boxes if b]
        if not boxes:
            return None
        return (int(math.floor(min(b[0] for b in boxes))), int(math.floor(min(b[1] for b in
                                                                             boxes))),
                int(math.ceil(max(b[2] for b in boxes))), int(math.ceil(max(b[3] for b in
                                                                           boxes))))

    def coords(self, tagOrId, *args):
        """Return a list of coordinates for the item given in ARGS."""
        ids = self._ids(tagOrId)
        if not ids:
            return []
        if not args:
            return [float(v) for v in self._items[ids[0]].coords]
        flat = _flat(args)
        for i in ids[:1]:
            self._items[i].coords = flat
            self._push(self._items[i])
        return None

    def move(self, tagOrId, xAmount, yAmount):
        """Move an item TAGORID given in ARGS."""
        dx, dy = float(_pixels(xAmount)), float(_pixels(yAmount))
        for i in self._ids(tagOrId):
            item = self._items[i]
            item.coords = [v + (dx if k % 2 == 0 else dy) for k, v in enumerate(item.coords)]
            self._push(item)

    def moveto(self, tagOrId, x="", y=""):
        """Move the items given by TAGORID so the top-left of their bbox is at X, Y."""
        bb = self.bbox(tagOrId)
        if bb is None:
            return
        dx = float(x) - bb[0] if x != "" else 0
        dy = float(y) - bb[1] if y != "" else 0
        self.move(tagOrId, dx, dy)

    def scale(self, tagOrId, xOrigin, yOrigin, xScale, yScale):
        """Scale item TAGORID with XORIGIN, YORIGIN, XSCALE, YSCALE."""
        ox, oy, sx, sy = float(xOrigin), float(yOrigin), float(xScale), float(yScale)
        for i in self._ids(tagOrId):
            item = self._items[i]
            item.coords = [ox + (v - ox) * sx if k % 2 == 0 else oy + (v - oy) * sy
                           for k, v in enumerate(item.coords)]
            self._push(item)

    def delete(self, *args):
        """Delete items identified by all tag or ids contained in ARGS."""
        for t in args:
            for i in self._ids(t):
                item = self._items.pop(i)
                self._order.remove(i)
                if item.type == "window" and item.opts.get("window"):
                    w = item.opts["window"]
                    w = w if isinstance(w, BaseWidget) else self.nametowidget(w)
                    w._node.geom(0)
                    w._ginfo = {}
                else:
                    self._node.canvas_delete(i)
                    self._node.canvas_delete(i + _SUB)
                    self._node.canvas_delete(i + 2 * _SUB)

    def itemconfigure(self, tagOrId, cnf=None, **kw):
        """Configure resources of an item TAGORID."""
        kw = dict(cnf, **kw) if cnf else kw
        ids = self._ids(tagOrId)
        if not kw:
            if not ids:
                return {}
            return {k: (k, "", "", "", v) for k, v in self._items[ids[0]].opts.items()}
        for i in ids:
            item = self._items[i]
            if "tags" in kw:
                t = kw.pop("tags")
                item.tags = list(t.split() if isinstance(t, str) else t)
            item.opts.update(kw)
            self._push(item)
        return None

    itemconfig = itemconfigure

    def itemcget(self, tagOrId, option):
        """Return the resource value for an OPTION for item TAGORID."""
        ids = self._ids(tagOrId)
        if not ids:
            return ""
        item = self._items[ids[0]]
        option = option.lstrip("-")
        if option == "tags":
            return " ".join(item.tags)
        return item.opts.get(option, "")

    def tag_raise(self, *args):
        """Raise an item TAGORID given in ARGS (optional above another item)."""
        tag, above = args[0], (args[1] if len(args) > 1 else None)
        ids = self._ids(tag)
        if not ids:
            return
        for i in ids:
            self._order.remove(i)
        pos = len(self._order) if above is None else \
            self._order.index(self._ids(above)[-1]) + 1
        self._order[pos:pos] = ids
        self._resync_order()

    lift = tkraise = tag_raise

    def tag_lower(self, *args):
        """Lower an item TAGORID given in ARGS (optional below another item)."""
        tag, below = args[0], (args[1] if len(args) > 1 else None)
        ids = self._ids(tag)
        if not ids:
            return
        for i in ids:
            self._order.remove(i)
        pos = 0 if below is None else self._order.index(self._ids(below)[0])
        self._order[pos:pos] = ids
        self._resync_order()

    lower = tag_lower

    def _resync_order(self):
        for i in self._order:  # kit draw order = raise order
            self._node.canvas_raise(i, True)
            for k in (1, 2):
                self._node.canvas_raise(i + k * _SUB, True)

    # -- text items ---------------------------------------------------------------------------
    def _text_index(self, item, index):
        text = str(item.opts.get("text", ""))
        s = str(index)
        if s == "end":
            return len(text)
        if s == "insert":
            return int(item.opts.get("_icursor", len(text)))
        if s in ("sel.first", "sel.last"):
            if not self._sel or self._sel[0] != item.id:
                raise TclError("selection isn't in item")
            return self._sel[1 if s == "sel.first" else 2]
        if s.startswith("@"):
            x = float(s[1:].split(",")[0])
            bb = self._item_bbox(item)
            for k in range(len(text) + 1):
                if bb[0] + _text_width(text[:k]) >= x:
                    return k
            return len(text)
        return max(0, min(len(text), int(s)))

    def dchars(self, tagOrId, first, last=None):
        """Delete characters of text items identified by tag or id in ARGS."""
        for i in self._ids(tagOrId):
            item = self._items[i]
            if item.type != "text":
                continue
            a = self._text_index(item, first)
            b = a if last is None else self._text_index(item, last)
            t = str(item.opts.get("text", ""))
            item.opts["text"] = t[:a] + t[b + 1:]
            self._push(item)

    def insert(self, tagOrId, beforeThis, string):
        """Insert STRING in item TAGORID at position BEFORETHIS."""
        for i in self._ids(tagOrId):
            item = self._items[i]
            if item.type != "text":
                continue
            k = self._text_index(item, beforeThis)
            t = str(item.opts.get("text", ""))
            item.opts["text"] = t[:k] + str(string) + t[k:]
            item.opts["_icursor"] = k + len(str(string))
            self._push(item)

    def icursor(self, tagOrId, index):
        for i in self._ids(tagOrId):
            item = self._items[i]
            if item.type == "text":
                item.opts["_icursor"] = self._text_index(item, index)

    def index(self, tagOrId, index):
        ids = self._ids(tagOrId)
        if not ids:
            raise TclError("can't find an indexable item \"%s\"" % tagOrId)
        return self._text_index(self._items[ids[0]], index)

    def focus(self, tagOrId=None):
        if tagOrId is None:
            return self._focus_item or ""
        ids = self._ids(tagOrId)
        self._focus_item = ids[0] if ids else None
        return None

    def select_from(self, tagOrId, index):
        ids = self._ids(tagOrId)
        if ids:
            k = self._text_index(self._items[ids[0]], index)
            self._sel = (ids[0], k, k)

    def select_to(self, tagOrId, index):
        ids = self._ids(tagOrId)
        if ids and self._sel:
            k = self._text_index(self._items[ids[0]], index)
            self._sel = (ids[0], min(self._sel[1], k), max(self._sel[1], k))

    select_adjust = select_to

    def select_clear(self):
        self._sel = None

    def select_item(self):
        return self._sel[0] if self._sel else None

    def _selected_text(self):
        if not self._sel or self._sel[0] not in self._items:
            return ""
        return str(self._items[self._sel[0]].opts.get("text", ""))[self._sel[1]:self._sel[2] + 1]

    # -- view ------------------------------------------------------------------------------
    def _region(self):
        sr = self._opts.get("scrollregion")
        if sr:
            vals = [float(_pixels(v)) for v in (sr.split() if isinstance(sr, str) else sr)]
            return tuple(vals[:4])
        w, h = self.winfo_width(), self.winfo_height()
        bb = self.bbox("all") or (0, 0, w, h)
        return (min(0, bb[0]), min(0, bb[1]), max(w, bb[2]), max(h, bb[3]))

    def _view(self, axis):
        x0, y0, x1, y1 = self._region()
        k = 0 if axis == "x" else 1
        lo, hi = (x0, x1) if k == 0 else (y0, y1)
        size = self.winfo_width() if k == 0 else self.winfo_height()
        off = self._node.get("scroll_x" if k == 0 else "scroll")
        span = max(1.0, hi - lo)
        return (max(0.0, min(1.0, (off - lo) / span)),
                max(0.0, min(1.0, (off + size - lo) / span)))

    def _view_set(self, axis, op, args):
        x0, y0, x1, y1 = self._region()
        k = 0 if axis == "x" else 1
        lo, hi = (x0, x1) if k == 0 else (y0, y1)
        size = self.winfo_width() if k == 0 else self.winfo_height()
        key = "scroll_x" if k == 0 else "scroll"
        off = self._node.get(key)
        if op == "moveto":
            off = lo + float(args[0]) * (hi - lo)
        elif op == "scroll":
            n, what = int(float(args[0])), str(args[1])
            inc = float(_pixels(self._opts.get(axis + "scrollincrement", 0) or 0)) or size / 10
            off += n * (size * 0.9 if what.startswith("page") else inc)
        else:
            raise TclError('bad option "%s"' % op)
        self._node.set(key, float(self._confined(off, lo, hi, size)))
        self._view_changed()

    @staticmethod
    def _confined(off, lo, hi, size):
        return max(lo, min(off, hi - size)) if hi - lo > size else lo

    def _confine(self):
        if self.cget("confine") in (0, "0", False):
            return
        x0, y0, x1, y1 = self._region()
        self._node.set("scroll_x", float(self._confined(self._node.get("scroll_x"), x0, x1,
                                                         self.winfo_width())))
        self._node.set("scroll", float(self._confined(self._node.get("scroll"), y0, y1,
                                                       self.winfo_height())))

    def _configured(self, width, height):
        self._confine()
        self._view_changed()

    def canvasx(self, screenx, gridspacing=None):
        """Return the canvas x coordinate of pixel position SCREENX rounded
        to nearest multiple of GRIDSPACING units."""
        v = float(screenx) + self._node.get("scroll_x")
        return round(v / float(gridspacing)) * float(gridspacing) if gridspacing else v

    def canvasy(self, screeny, gridspacing=None):
        v = float(screeny) + self._node.get("scroll")
        return round(v / float(gridspacing)) * float(gridspacing) if gridspacing else v

    def scan_mark(self, x, y):
        self._scan = (int(x), int(y), self._node.get("scroll_x"), self._node.get("scroll"))

    def scan_dragto(self, x, y, gain=10):
        mx, my, sx, sy = getattr(self, "_scan", (int(x), int(y), 0, 0))
        x0, y0, x1, y1 = self._region()
        self._node.set("scroll_x", float(self._confined(sx - (int(x) - mx) * gain, x0, x1,
                                                         self.winfo_width())))
        self._node.set("scroll", float(self._confined(sy - (int(y) - my) * gain, y0, y1,
                                                       self.winfo_height())))
        self._view_changed()

    def postscript(self, cnf={}, **kw):  # noqa: B006
        """Return PostScript of the canvas (or write it to file=)."""
        kw = dict(cnf, **kw) if cnf else kw
        w, h = self.winfo_width(), self.winfo_height()
        ps = "%%!PS-Adobe-3.0 EPSF-3.0\n%%%%BoundingBox: 0 0 %d %d\n%%%%EndComments\n" \
            "showpage\n%%%%EOF\n" % (w, h)
        if kw.get("file"):
            with open(kw["file"], "w", encoding="ascii") as fh:
                fh.write(ps)
            return ""
        return ps

    # -- item bindings -----------------------------------------------------------------------
    def tag_bind(self, tagOrId, sequence=None, func=None, add=None):
        """Bind to all items with TAGORID at event SEQUENCE a call to function FUNC."""
        key = str(tagOrId)
        table = self._item_binds.setdefault(key, _BindTable())
        if sequence is None:
            return tuple(table.seqs)
        if func is None:
            return table.seqs.get(sequence, "")
        self._node.set("events", True)
        return table.bind(sequence, func, add)

    def tag_unbind(self, tagOrId, sequence, funcid=None):
        table = self._item_binds.get(str(tagOrId))
        if table is not None:
            table.unbind(sequence, funcid)

    def _item_at(self, x, y):
        halo = float(self.cget("closeenough") or 1.0)
        cx, cy = self.canvasx(x), self.canvasy(y)
        for i in reversed(self._order):
            item = self._items[i]
            if item.opts.get("state") in ("hidden", "disabled") or item.type == "window":
                continue
            if self._distance(item, cx, cy) <= halo:
                return i
        return None

    def _fire_item(self, iid, ev):
        if iid not in self._items:
            return
        item = self._items[iid]
        for key in ["all"] + item.tags + [str(iid)]:
            table = self._item_binds.get(key)
            if table is None:
                continue
            for fn in list(table.match(ev)):
                _call(self, fn, ev)

    def _raw_event(self, ev):
        if not self._item_binds:
            return
        t = ev.type
        if t in (EventType.Motion, EventType.Enter, EventType.Leave, EventType.ButtonPress):
            new = None if t == EventType.Leave else self._item_at(ev.x, ev.y)
            if new != self._current:
                old, self._current = self._current, new
                if old is not None:
                    self._fire_item(old, Event(type=EventType.Leave, widget=self, x=ev.x,
                                               y=ev.y, state=ev.state))
                if new is not None:
                    self._fire_item(new, Event(type=EventType.Enter, widget=self, x=ev.x,
                                               y=ev.y, state=ev.state))
        if t in (EventType.ButtonPress, EventType.ButtonRelease, EventType.Motion,
                 EventType.MouseWheel):
            if self._current is not None:
                self._fire_item(self._current, ev)
        elif t in (EventType.KeyPress, EventType.KeyRelease) and self._focus_item is not None:
            self._fire_item(self._focus_item, ev)


def _kit_anchor(anchor):
    a = str(anchor or "center").lower()
    if a == "center":
        return 0
    return (1 if "n" in a else 0) | (2 if "s" in a else 0) | (4 if "w" in a else 0) | \
        (8 if "e" in a else 0)


def _anchor_offset(anchor, w, h):
    a = str(anchor or "center").lower()
    ax = 0 if "w" in a else w if "e" in a else w / 2
    ay = 0 if "n" in a else h if "s" in a else h / 2
    if a == "center":
        ax, ay = w / 2, h / 2
    return ax, ay


def _font_px_kit(scale):
    return buipy.kit_unit() * 0.55 * scale


# ==========================================================================
# Text


_TAG_COLOR_KEYS = ("foreground", "fg", "background", "bg")


class Text(Widget, XView, YView):
    """Text widget which can display text in various forms (Blender text area)."""

    _KIND = "textarea"
    _TAKEFOCUS = True
    _DEFAULTS = {"width": 80, "height": 24, "wrap": "char", "state": "normal", "undo": 0,
                 "maxundo": 0, "autoseparators": 1, "tabs": "", "spacing1": 0}

    def __init__(self, master=None, cnf={}, **kw):  # noqa: B006
        Widget.__init__(self, master, "text", cnf, kw)

    def _setup(self):
        self._tags: dict = {}       # name -> {"ranges": [[a, b]], "opts": {}}
        self._tag_order: list = []  # priority, lowest first
        self._marks: dict = {}      # name -> [offset, gravity]
        self._undo: list = []
        self._redo: list = []
        self._last = ""
        self._modified = False
        self._peers: list = []
        self._tag_binds: dict = {}
        self._hover_tags = ()
        self._embedded: dict = {}
        self._node.set("rows", 24)
        self._node.geom(0, reqwidth=_chars(80))
        self._node.set("viewevents", True)

    def _focus_node(self):
        return self._node

    def _apply(self, key, value):
        n = self._node
        if key == "height":
            n.set("rows", int(value) or 1)
        elif key == "width":
            n.geom(int(n.geom_get("manager")), reqwidth=_chars(int(value or 80)))
        elif key == "state":
            n.set("enabled", str(value) != "disabled")
        elif key in ("xscrollcommand", "yscrollcommand"):
            n.set("noscrollbar", True)
        else:
            super()._apply(key, value)

    # -- text & indices ---------------------------------------------------------------------
    def _text(self):
        return self._node.get_str("value")

    def _byte(self, off):
        return len(self._text()[:off].encode("utf-8"))

    def _char(self, byte_off):
        return len(self._text().encode("utf-8")[:byte_off].decode("utf-8", "ignore"))

    def _line_col(self, off):
        t = self._text()
        before = t[:off]
        return before.count("\n") + 1, off - (before.rfind("\n") + 1)

    def _line_start(self, line):
        lines = self._text().split("\n")
        line = max(1, min(len(lines), line))
        return sum(len(lines[k]) + 1 for k in range(line - 1)), lines[line - 1]

    _MOD = re.compile(r"\s*([+-])\s*(\d+)\s*(?:(?:display|any)\s*)?([a-z]*)"
                      r"|\s*(linestart|lineend|wordstart|wordend)", re.I)

    def _off(self, index):
        """Tk text index -> character offset in [0, len] (the trailing
        newline Tk keeps is 'end')."""
        if isinstance(index, (int, float)) and not isinstance(index, bool):
            index = repr(float(index)) if isinstance(index, float) else "%d.0" % index
        s = str(index).strip()
        t = self._text()
        m = re.match(r"^(\d+)\.(\d+|end)|^(end)|^(insert|current|anchor)|^@(-?\d+),(-?\d+)|"
                     r"^([^\s+-]+)\.(first|last)|^([^\s+-]+)", s)
        if not m:
            raise TclError('bad text index "%s"' % index)
        if m.group(1):
            start, line = self._line_start(int(m.group(1)))
            if int(m.group(1)) > t.count("\n") + 1:
                off = len(t)
            else:
                off = start + (len(line) if m.group(2) == "end" else min(int(m.group(2)),
                                                                           len(line)))
        elif m.group(3):
            off = len(t)
        elif m.group(4):
            name = m.group(4)
            if name == "insert":
                off = self._char(self._node.ta_caret())
            elif name == "current":
                off = getattr(self, "_current_off", 0)
            else:
                off = self._marks.get("anchor", [0])[0]
        elif m.group(5) is not None:
            x, y = int(m.group(5)), int(m.group(6))
            r = self._node.rect
            off = self._char(self._node.ta_index_at(r[0] + x, r[1] + y))
        elif m.group(7):
            tag, which = m.group(7), m.group(8)
            ranges = self._ranges(tag)
            if not ranges:
                raise TclError('text doesn\'t contain any characters tagged with "%s"' % tag)
            off = ranges[0][0] if which == "first" else ranges[-1][1]
        else:
            name = m.group(9)
            if name not in self._marks:
                raise TclError('bad text index "%s"' % index)
            off = self._marks[name][0]
        rest = s[m.end():]
        while rest.strip():
            mm = self._MOD.match(rest)
            if not mm:
                raise TclError('bad text index "%s"' % index)
            rest = rest[mm.end():]
            if mm.group(4):
                off = self._modify(off, mm.group(4).lower())
                continue
            n = int(mm.group(2)) * (1 if mm.group(1) == "+" else -1)
            unit = (mm.group(3) or "c").lower()
            if unit.startswith("l"):
                line, col = self._line_col(off)
                start, text = self._line_start(line + n)
                off = start + min(col, len(text))
            else:
                off += n
            off = max(0, min(len(t), off))
        return max(0, min(len(t), off))

    def _modify(self, off, what):
        t = self._text()
        if what == "linestart":
            return t.rfind("\n", 0, off) + 1
        if what == "lineend":
            e = t.find("\n", off)
            return len(t) if e < 0 else e
        word = re.compile(r"\w")
        if what == "wordstart":
            while off > 0 and word.match(t[off - 1:off]):
                off -= 1
            return off
        while off < len(t) and word.match(t[off:off + 1]):
            off += 1
        return off

    def index(self, index):
        """Return the index in the form line.char for INDEX."""
        if str(index).strip() == "end":
            return "%d.0" % (self._text().count("\n") + 2)
        return "%d.%d" % self._line_col(self._off(index))

    def compare(self, index1, op, index2):
        """Return whether between index INDEX1 and index INDEX2 the
        relation OP is satisfied. OP is one of <, <=, ==, >=, >, or !=."""
        a, b = self._off(index1), self._off(index2)
        if str(index1).strip() == "end":
            a += 1
        if str(index2).strip() == "end":
            b += 1
        return {"<": a < b, "<=": a <= b, "==": a == b, ">=": a >= b, ">": a > b,
                "!=": a != b}[op]

    def count(self, index1, index2, *options, return_ints=False):
        """Counts the number of relevant things between the two indices."""
        a, b = self._off(index1), self._off(index2)
        t = self._text()
        out = []
        for opt in (options or ("indices",)):
            opt = opt.lstrip("-")
            if opt in ("chars", "indices", "displaychars", "displayindices"):
                out.append(b - a)
            elif opt in ("lines", "displaylines"):
                out.append(t[min(a, b):max(a, b)].count("\n") * (1 if b >= a else -1))
            elif opt == "xpixels":
                out.append(int(_text_width(t[a:b])))
            elif opt == "ypixels":
                out.append(t[min(a, b):max(a, b)].count("\n") * int(buipy.kit_unit() * 0.8))
            elif opt == "update":
                continue
        if return_ints or len(out) > 1:
            return tuple(out) if len(out) > 1 else out[0]
        return (out[0],) if out and out[0] else None

    # -- editing ------------------------------------------------------------------------
    def _snapshot(self):
        if self._undo_enabled():
            self._undo.append(self._text())
            self._redo.clear()

    def _undo_enabled(self):
        return self.cget("undo") not in (0, "0", False, "false")

    def _edited(self, off, removed, inserted):
        """Keep tag ranges and marks attached to their text (Tk gravity)."""
        n_ins, n_rem = len(inserted), len(removed)
        for tag in self._tags.values():
            new = []
            for a, b in tag["ranges"]:
                if n_rem:
                    e = off + n_rem
                    a = a - n_rem if a >= e else (off if a > off else a)
                    b = b - n_rem if b >= e else (off if b > off else b)
                if n_ins:
                    a = a + n_ins if a >= off else a
                    b = b + n_ins if b > off else b
                if b > a:
                    new.append([a, b])
            tag["ranges"] = new
        for name, mark in self._marks.items():
            pos, gravity = mark
            if n_rem:
                e = off + n_rem
                pos = pos - n_rem if pos >= e else (off if pos > off else pos)
            if n_ins and (pos > off or (pos == off and gravity == "right")):
                pos += n_ins
            mark[0] = pos
        self._last = self._text()
        self._set_modified(True)
        for peer in self._peers:
            if peer.winfo_exists() and peer._text() != self._last:
                peer._node.set("value", self._last)
                peer._last = self._last

    def insert(self, index, chars, *args):
        """Insert CHARS before the characters at INDEX. An additional tag can
        be given in ARGS. Additional CHARS and tags can follow in ARGS."""
        if str(self.cget("state")) == "disabled":
            return
        self._snapshot()
        off = self._off(index)
        pairs = [(chars, args[0] if args else None)] + \
            [(args[k], args[k + 1] if k + 1 < len(args) else None)
             for k in range(1, len(args), 2)]
        for text, tags in pairs:
            text = str(text)
            if not text:
                continue
            self._insert_chars(off, text)
            if tags not in (None, ""):
                names = tags.split() if isinstance(tags, str) else tags
                for name in names:
                    self._tag(name)["ranges"].append([off, off + len(text)])
            off += len(text)
        self._push_tags()

    def _insert_chars(self, off, text):
        before = self._text()
        self._node.ta_insert(self._byte(off), text)
        if self._text() != before:
            self._edited(off, "", text)

    def delete(self, index1, index2=None):
        """Delete the characters between INDEX1 and INDEX2 (not included)."""
        if str(self.cget("state")) == "disabled":
            return
        a = self._off(index1)
        b = a + 1 if index2 is None else self._off(index2)
        if b <= a:
            return
        self._snapshot()
        removed = self._text()[a:b]
        self._node.ta_delete(self._byte(a), self._byte(b))
        self._edited(a, removed, "")
        self._push_tags()

    def get(self, index1, index2=None):
        """Return the text from INDEX1 to INDEX2 (not included)."""
        t = self._text() + "\n"
        a = self._off(index1)
        if index2 is None:
            return t[a:a + 1] if str(index1).strip() != "end" else ""
        b = self._off(index2) + (1 if str(index2).strip() == "end" else 0)
        return t[a:b]

    def replace(self, index1, index2, chars, *args):
        """Replaces the range of characters between index1 and index2 with
        the given characters and tags specified by args."""
        a = self.index(index1)
        self.delete(index1, index2)
        self.insert(a, chars, *args)

    def see(self, index):
        """Scroll such that the character at INDEX is visible."""
        self._node.ta_see(self._byte(self._off(index)))
        self._view_changed()

    yview_pickplace = see

    def dlineinfo(self, index):
        """(x, y, width, height, baseline) of the display line holding INDEX."""
        line, _ = self._line_col(self._off(index))
        first = int(round(self._node.get("yfirst") * max(1, self._text().count("\n") + 1))) + 1
        lh = int(buipy.kit_unit() * 0.8)
        if not first <= line < first + int(self._node.get("yvisible")):
            return None
        _, text = self._line_start(line)
        pad = int(buipy.kit_unit() * 0.4)
        y = (line - first) * lh + pad // 2
        return (pad, y, int(_text_width(text)), lh, int(lh * 0.75))

    def bbox(self, index):
        """(x, y, width, height) of the character at INDEX."""
        off = self._off(index)
        info = self.dlineinfo(index)
        if info is None:
            return None
        line, col = self._line_col(off)
        _, text = self._line_start(line)
        return (info[0] + int(_text_width(text[:col])), info[1],
                max(1, int(_text_width(text[col:col + 1] or " "))), info[3])

    def search(self, pattern, index, stopindex=None, forwards=None, backwards=None,
               exact=None, regexp=None, nocase=None, count=None, elide=None, *, nolinestop=None,
               strictlimits=None):
        """Search PATTERN beginning from INDEX until STOPINDEX. Return the
        index of the first character of a match or an empty string."""
        t = self._text()
        start = self._off(index)
        stop = self._off(stopindex) if stopindex else None
        flags = (re.IGNORECASE if nocase else 0) | (re.DOTALL if nolinestop else 0)
        rx = re.compile(pattern if regexp else re.escape(pattern), flags)
        if backwards:
            limit = stop if stop is not None else 0
            m = None
            for cand in rx.finditer(t, limit, start):
                m = cand
            if m is None and stop is None:
                for cand in rx.finditer(t, start):
                    m = cand
        else:
            end = stop if stop is not None else len(t)
            m = rx.search(t, start, end) if end >= start else None
            if m is None and stop is None:
                m = rx.search(t, 0, start)
        if m is None:
            return ""
        if count is not None:
            count.set(len(m.group(0)))
        return self.index("1.0 + %d chars" % m.start())

    def dump(self, index1, index2=None, command=None, **kw):
        """Return (key, value, index) triples of text, tags and marks."""
        a = self._off(index1)
        b = a + 1 if index2 is None else self._off(index2)
        want_all = kw.get("all") or not any(kw.get(k) for k in ("text", "tag", "mark",
                                                                "image", "window"))
        out = []
        if want_all or kw.get("mark"):
            for name, (pos, _g) in self._marks.items():
                if a <= pos <= b:
                    out.append(("mark", name, self.index("1.0+%dc" % pos)))
        if want_all or kw.get("tag"):
            for name, tag in self._tags.items():
                for ra, rb in tag["ranges"]:
                    if a <= ra < b:
                        out.append(("tagon", name, self.index("1.0+%dc" % ra)))
                    if a < rb <= b:
                        out.append(("tagoff", name, self.index("1.0+%dc" % rb)))
        if want_all or kw.get("text"):
            text = self._text()[a:b]
            pos = a
            for line in text.split("\n"):
                chunk = line + ("\n" if pos + len(line) < b else "")
                if chunk:
                    out.append(("text", chunk, self.index("1.0+%dc" % pos)))
                pos += len(line) + 1
        out.sort(key=lambda e: self._off(e[2]))
        if command is not None:
            for e in out:
                command(*e)
            return None
        return out

    # -- undo -------------------------------------------------------------------------------
    def edit(self, *args):
        """Internal method: edit('undo'), edit('modified', flag) ..."""
        return getattr(self, "edit_" + args[0])(*args[1:])

    def edit_modified(self, arg=None):
        """Get or set the modified flag (changes fire <<Modified>>)."""
        if arg is None:
            return self._modified
        self._modified = getboolean(arg)
        self.event_generate("<<Modified>>")
        return None

    def _set_modified(self, flag):
        if flag and not self._modified:
            self._modified = True
            self.event_generate("<<Modified>>")

    def edit_redo(self):
        """Redo the last undone edit."""
        if not self._undo_enabled():
            return ""  # Tk: a no-op while -undo is off
        if not self._redo:
            raise TclError("nothing to redo")
        self._undo.append(self._text())
        self._set_all(self._redo.pop())

    def edit_reset(self):
        """Clears the undo and redo stacks."""
        self._undo.clear()
        self._redo.clear()

    def edit_separator(self):
        """Inserts a separator (boundary) on the undo stack."""
        self._snapshot()

    def edit_undo(self):
        """Undoes the last edit action."""
        if not self._undo_enabled():
            return ""  # Tk: a no-op while -undo is off
        if not self._undo:
            raise TclError("nothing to undo")
        self._redo.append(self._text())
        self._set_all(self._undo.pop())

    def _set_all(self, text):
        old = self._text()
        self._node.set("value", text)
        idx, removed, inserted = _diff(old, text)
        self._edited(idx, removed, inserted)
        self._push_tags()

    # -- marks -----------------------------------------------------------------------------
    def mark_set(self, markName, index):
        """Set mark MARKNAME before the character at INDEX."""
        off = self._off(index)
        if markName == "insert":
            self._node.ta_caret(self._byte(off))
        elif markName == "current":
            self._current_off = off
        else:
            self._marks.setdefault(markName, [off, "right"])[0] = off

    def mark_unset(self, *markNames):
        """Delete all marks in MARKNAMES."""
        for name in markNames:
            self._marks.pop(name, None)

    def mark_names(self):
        """Return all mark names."""
        return ("insert", "current") + tuple(self._marks)

    def mark_gravity(self, markName, direction=None):
        """Change the gravity of a mark MARKNAME to DIRECTION (LEFT or RIGHT)."""
        if markName not in self._marks:
            return "right"
        if direction is None:
            return self._marks[markName][1]
        self._marks[markName][1] = str(direction)
        return None

    def mark_next(self, index):
        """Return the name of the next mark after INDEX."""
        off = self._off(index)
        after = sorted((pos, name) for name, (pos, _) in self._marks.items() if pos >= off)
        return after[0][1] if after else ""

    def mark_previous(self, index):
        """Return the name of the previous mark before INDEX."""
        off = self._off(index)
        before = sorted((pos, name) for name, (pos, _) in self._marks.items() if pos < off)
        return before[-1][1] if before else ""

    # -- tags ---------------------------------------------------------------------------------
    def _tag(self, name):
        if name not in self._tags:
            self._tags[name] = {"ranges": [], "opts": {}}
            self._tag_order.append(name)
        return self._tags[name]

    def _ranges(self, name):
        if name == "sel":
            sel = self._node.ta_selection()
            if not sel or sel[0] == sel[1]:
                return []
            return [[self._char(sel[0]), self._char(sel[1])]]
        return sorted(self._tags.get(name, {"ranges": []})["ranges"])

    def tag_add(self, tagName, index1, *args):
        """Add tag TAGNAME to all characters between INDEX1 and index2 in ARGS."""
        idx = (index1,) + args
        for k in range(0, len(idx), 2):
            a = self._off(idx[k])
            b = a + 1 if k + 1 >= len(idx) else self._off(idx[k + 1])
            if b <= a:
                continue
            if tagName == "sel":
                self._node.ta_selection(self._byte(a), self._byte(b))
                continue
            ranges = self._tag(tagName)["ranges"]
            ranges.append([a, b])
            ranges.sort()
            merged = [ranges[0]]
            for r in ranges[1:]:
                if r[0] <= merged[-1][1]:
                    merged[-1][1] = max(merged[-1][1], r[1])
                else:
                    merged.append(r)
            self._tags[tagName]["ranges"] = merged
        self._push_tags()

    def tag_remove(self, tagName, index1, index2=None):
        """Remove tag TAGNAME from all characters between INDEX1 and INDEX2."""
        a = self._off(index1)
        b = a + 1 if index2 is None else self._off(index2)
        if tagName == "sel":
            self._node.ta_selection(-1, -1)
            return
        tag = self._tags.get(tagName)
        if tag:
            out = []
            for ra, rb in tag["ranges"]:
                if rb <= a or ra >= b:
                    out.append([ra, rb])
                else:
                    if ra < a:
                        out.append([ra, a])
                    if rb > b:
                        out.append([b, rb])
            tag["ranges"] = out
            self._push_tags()

    def tag_delete(self, *tagNames):
        """Delete all tags in TAGNAMES."""
        for n in tagNames:
            self._tags.pop(n, None)
            if n in self._tag_order:
                self._tag_order.remove(n)
        self._push_tags()

    def tag_configure(self, tagName, cnf=None, **kw):
        """Configure a tag TAGNAME."""
        kw = dict(cnf, **kw) if cnf else kw
        tag = self._tag(tagName)
        if not kw:
            return {k: (k, "", "", "", v) for k, v in tag["opts"].items()}
        tag["opts"].update(kw)
        self._push_tags()
        return None

    tag_config = tag_configure

    def tag_cget(self, tagName, option):
        """Return the value of OPTION for tag TAGNAME."""
        return self._tags.get(tagName, {"opts": {}})["opts"].get(option.lstrip("-"), "")

    def tag_names(self, index=None):
        """Return a list of all tag names (at INDEX)."""
        if index is None:
            return tuple(["sel"] + [n for n in self._tag_order])
        off = self._off(index)
        return tuple(n for n in ["sel"] + self._tag_order
                     if any(a <= off < b for a, b in self._ranges(n)))

    def tag_ranges(self, tagName):
        """Return a list of start and end index for the ranges of TAGNAME."""
        out = []
        for a, b in self._ranges(tagName):
            out.extend([self.index("1.0+%dc" % a), self.index("1.0+%dc" % b)])
        return tuple(out)

    def tag_nextrange(self, tagName, index1, index2=None):
        """First range of TAGNAME starting in [INDEX1, INDEX2)."""
        a = self._off(index1)
        b = len(self._text()) if index2 is None else self._off(index2)
        for ra, rb in self._ranges(tagName):
            if a <= ra < b or (ra < a < rb and False):
                return (self.index("1.0+%dc" % ra), self.index("1.0+%dc" % rb))
        return ()

    def tag_prevrange(self, tagName, index1, index2=None):
        """Last range of TAGNAME starting before INDEX1 (and at/after INDEX2)."""
        a = self._off(index1)
        b = 0 if index2 is None else self._off(index2)
        for ra, rb in reversed(self._ranges(tagName)):
            if b <= ra < a:
                return (self.index("1.0+%dc" % ra), self.index("1.0+%dc" % rb))
        return ()

    def tag_raise(self, tagName, aboveThis=None):
        """Change the priority of tag TAGNAME such that it is higher than
        the priority of ABOVETHIS."""
        self._tag(tagName)
        self._tag_order.remove(tagName)
        pos = len(self._tag_order) if aboveThis is None else \
            self._tag_order.index(aboveThis) + 1
        self._tag_order.insert(pos, tagName)
        self._push_tags()

    def tag_lower(self, tagName, belowThis=None):
        """Change the priority of tag TAGNAME such that it is lower than
        the priority of BELOWTHIS."""
        self._tag(tagName)
        self._tag_order.remove(tagName)
        pos = 0 if belowThis is None else self._tag_order.index(belowThis)
        self._tag_order.insert(pos, tagName)
        self._push_tags()

    def tag_bind(self, tagName, sequence, func, add=None):
        """Bind to all characters with TAGNAME at event SEQUENCE a call to FUNC."""
        table = self._tag_binds.setdefault(tagName, _BindTable())
        self._node.set("events", True)
        return table.bind(sequence, func, add)

    def tag_unbind(self, tagName, sequence, funcid=None):
        """Unbind for all characters with TAGNAME for event SEQUENCE."""
        table = self._tag_binds.get(tagName)
        if table is not None:
            table.unbind(sequence, funcid)

    def _push_tags(self):
        spans = []
        for name in self._tag_order:  # later = higher priority (drawn last)
            o = self._tags[name]["opts"]
            fg = _try_color(o.get("foreground", o.get("fg")))
            bg = _try_color(o.get("background", o.get("bg")))
            if fg is None and bg is None:
                continue
            for a, b in self._tags[name]["ranges"]:
                spans.append((self._byte(a), self._byte(b), fg, bg))
        self._node.ta_spans(spans)

    def _raw_event(self, ev):
        if ev.type in (EventType.Motion, EventType.ButtonPress, EventType.ButtonRelease,
                       EventType.Enter):
            r = self._node.rect
            self._current_off = self._char(self._node.ta_index_at(r[0] + ev.x, r[1] + ev.y))
        if not self._tag_binds:
            return
        tags = self.tag_names("current") if ev.type != EventType.Leave else ()
        if ev.type in (EventType.Motion, EventType.Enter, EventType.Leave):
            for name in self._hover_tags:
                if name not in tags:
                    self._fire_tag(name, Event(type=EventType.Leave, widget=self, x=ev.x,
                                               y=ev.y))
            for name in tags:
                if name not in self._hover_tags:
                    self._fire_tag(name, Event(type=EventType.Enter, widget=self, x=ev.x,
                                               y=ev.y))
            self._hover_tags = tags
        for name in tags:
            self._fire_tag(name, ev)

    def _fire_tag(self, name, ev):
        table = self._tag_binds.get(name)
        if table is not None:
            for fn in list(table.match(ev)):
                _call(self, fn, ev)

    # -- embedded images/windows: listed, not drawn by the kit text area -----------------------
    def image_create(self, index, cnf={}, **kw):  # noqa: B006
        name = kw.get("name") or "image%d" % (len(self._embedded) + 1)
        self._embedded[name] = ("image", self._off(index), dict(cnf, **kw))
        return name

    def image_cget(self, index, option):
        return ""

    def image_configure(self, index, cnf=None, **kw):
        return {}

    def image_names(self):
        return tuple(n for n, e in self._embedded.items() if e[0] == "image")

    def window_create(self, index, cnf={}, **kw):  # noqa: B006
        w = kw.get("window")
        self._embedded[str(w)] = ("window", self._off(index), dict(cnf, **kw))

    def window_cget(self, index, option):
        return ""

    def window_configure(self, index, cnf=None, **kw):
        return {}

    window_config = window_configure

    def window_names(self):
        return tuple(n for n, e in self._embedded.items() if e[0] == "window")

    # -- peers ----------------------------------------------------------------------------------
    def peer_create(self, newPathName, cnf={}, **kw):  # noqa: B006
        """Creates a peer text widget sharing this widget's text."""
        path = str(newPathName)
        parent_path, _, name = path.rpartition(".")
        master = self.nametowidget(parent_path or ".")
        peer = Text(master, cnf, name=name, **kw)
        peer._node.set("value", self._text())
        peer._last = self._text()
        peer._peers = [self] + self._peers
        for p in self._peers:
            p._peers.append(peer)
        self._peers.append(peer)
        return peer

    def peer_names(self):
        return tuple(p._w for p in self._peers if p.winfo_exists())

    def scan_mark(self, x, y):
        self._scan = (int(y), self._node.get("yfirst"))

    def scan_dragto(self, x, y):
        y0, first = getattr(self, "_scan", (int(y), self._node.get("yfirst")))
        lines = max(1, self._text().count("\n") + 1)
        self._node.set("yview", first - (int(y) - y0) * 10 / (buipy.kit_unit() * 0.8) / lines)
        self._view_changed()

    def debug(self, boolean=None):
        return False

    def _selected_text(self):
        r = self._ranges("sel")
        return self._text()[r[0][0]:r[0][1]] if r else ""

    def _event(self, kind, ref, a, b):
        if kind == EV_CHANGE:  # typed by the user: diff to keep tags and marks right
            new = self._text()
            if new != self._last:
                if self._undo_enabled():
                    self._undo.append(self._last)
                    self._redo.clear()
                idx, removed, inserted = _diff(self._last, new)
                self._edited(idx, removed, inserted)
                self._push_tags()


# ==========================================================================
# Menus


_MENU_TYPES = ("command", "cascade", "checkbutton", "radiobutton", "separator")


class Menu(Widget):
    """Menu widget: a Blender menu bar (root.config(menu=m)), a cascade, or
    a popup (post / tk_popup)."""

    _KIND = None
    _DEFAULTS = {"tearoff": 1, "type": "normal", "title": "", "postcommand": ""}

    def __init__(self, master=None, cnf={}, **kw):  # noqa: B006
        self._entries = []
        self._owners = []
        self._bar = None
        self._active = None
        Widget.__init__(self, master, "menu", cnf, kw)
        if "tearoff" not in self._opts:  # Tk's default: a tearoff entry at index 0
            self._apply("tearoff", 1)

    def _apply(self, key, value):
        if key == "tearoff":
            has = bool(self._entries) and self._entries[0]["type"] == "tearoff"
            if getboolean(value) and not has:
                self._entries.insert(0, {"type": "tearoff"})
            elif not getboolean(value) and has:
                self._entries.pop(0)

    # -- entries ----------------------------------------------------------------------------
    def add(self, itemType, cnf={}, **kw):  # noqa: B006
        """Internal function."""
        self.insert(len(self._entries), itemType, cnf, **kw)

    def add_cascade(self, cnf={}, **kw):  # noqa: B006
        """Add hierarchical menu item."""
        self.add("cascade", cnf or kw)

    def add_checkbutton(self, cnf={}, **kw):  # noqa: B006
        """Add checkbutton menu item."""
        self.add("checkbutton", cnf or kw)

    def add_command(self, cnf={}, **kw):  # noqa: B006
        """Add command menu item."""
        self.add("command", cnf or kw)

    def add_radiobutton(self, cnf={}, **kw):  # noqa: B006
        """Add radio menu item."""
        self.add("radiobutton", cnf or kw)

    def add_separator(self, cnf={}, **kw):  # noqa: B006
        """Add separator."""
        self.add("separator", cnf or kw)

    def insert(self, index, itemType, cnf={}, **kw):  # noqa: B006
        """Internal function."""
        if itemType not in _MENU_TYPES:
            raise TclError('bad menu entry type "%s"' % itemType)
        opts = dict(cnf, **kw)
        entry = dict(opts, type=itemType)
        if itemType == "checkbutton":
            entry.setdefault("onvalue", 1)
            entry.setdefault("offvalue", 0)
            if "variable" not in entry:
                entry["variable"] = IntVar()
        if itemType == "radiobutton":
            entry.setdefault("value", entry.get("label", ""))
            if "variable" not in entry:
                entry["variable"] = _radio_default_var()
        var = entry.get("variable")
        if isinstance(var, Variable):
            var.trace_add("write", lambda *a: self._changed())
        if isinstance(index, int):
            i = max(0, min(len(self._entries), index))
        elif str(index) in ("end", "last") or not self._entries:
            i = len(self._entries)
        else:
            i = self.index(index)
        self._entries.insert(i, entry)
        self._changed()

    def insert_cascade(self, index, cnf={}, **kw):  # noqa: B006
        self.insert(index, "cascade", cnf or kw)

    def insert_checkbutton(self, index, cnf={}, **kw):  # noqa: B006
        self.insert(index, "checkbutton", cnf or kw)

    def insert_command(self, index, cnf={}, **kw):  # noqa: B006
        self.insert(index, "command", cnf or kw)

    def insert_radiobutton(self, index, cnf={}, **kw):  # noqa: B006
        self.insert(index, "radiobutton", cnf or kw)

    def insert_separator(self, index, cnf={}, **kw):  # noqa: B006
        self.insert(index, "separator", cnf or kw)

    def delete(self, index1, index2=None):
        """Delete menu items between INDEX1 and INDEX2 (included)."""
        a = self.index(index1)
        b = a if index2 is None else self.index(index2)
        if a is None or b is None:
            return
        if self._entries and self._entries[0]["type"] == "tearoff":
            a = max(a, 1)
        del self._entries[a:b + 1]
        self._changed()

    def entrycget(self, index, option):
        """Return the resource value of a menu item for OPTION at INDEX."""
        e = self._entries[self.index(index)]
        return e.get(option.lstrip("-"), "")

    def entryconfigure(self, index, cnf=None, **kw):
        """Configure a menu item at INDEX."""
        e = self._entries[self.index(index)]
        kw = dict(cnf, **kw) if cnf else kw
        if not kw:
            return {k: (k, "", "", "", v) for k, v in e.items() if k != "type"}
        e.update(kw)
        self._changed()
        return None

    entryconfig = entryconfigure

    def index(self, index):
        """Return the index of a menu item identified by INDEX."""
        n = len(self._entries)
        if isinstance(index, int):
            return max(0, min(n - 1, index)) if n else None
        s = str(index)
        if s in ("end", "last"):
            return n - 1 if n else None
        if s == "none":
            return None
        if s == "active":
            return self._active
        if s.startswith("@"):
            y = int(s[1:].split(",")[-1])
            return max(0, min(n - 1, y // buipy.kit_unit()))
        if s.isdigit():
            return min(n - 1, int(s))
        for i, e in enumerate(self._entries):  # label pattern
            if e.get("label") is not None and re.fullmatch(
                    s.replace("*", ".*").replace("?", "."), str(e.get("label"))):
                return i
        raise TclError('bad menu entry index "%s"' % index)

    def type(self, index):
        """Return the type of the menu item at INDEX."""
        return self._entries[self.index(index)]["type"]

    def invoke(self, index):
        """Invoke a menu item identified by INDEX and execute the associated command."""
        e = self._entries[self.index(index)]
        if e.get("state") == "disabled":
            return ""
        return self._activate_entry(e)

    def _activate_entry(self, e):
        var = e.get("variable")
        if e["type"] == "checkbutton" and isinstance(var, Variable):
            var.set(e["offvalue"] if str(var._raw()) == str(e["onvalue"]) else e["onvalue"])
        elif e["type"] == "radiobutton" and isinstance(var, Variable):
            var.set(e.get("value"))
        cmd = e.get("command")
        return _call(self, _command(cmd)) if cmd else ""

    def activate(self, index):
        """Activate entry at INDEX."""
        self._active = self.index(index)

    def xposition(self, index):
        return 0

    def yposition(self, index):
        """Return the y-position of the topmost pixel of the menu item at INDEX."""
        return (self.index(index) or 0) * buipy.kit_unit()

    # -- kit menus ------------------------------------------------------------------------
    def _items(self):
        out = []
        for e in self._entries:
            t = e["type"]
            if t == "tearoff":
                continue
            if t == "separator":
                out.append("-")
            elif t == "cascade":
                sub = e.get("menu")
                sub = sub if isinstance(sub, Menu) or sub in (None, "") else \
                    self.nametowidget(sub)
                if isinstance(sub, Menu) and self not in sub._owners:
                    sub._owners.append(self)
                out.append(MenuItem(str(e.get("label", "")),
                                    submenu=sub._items() if isinstance(sub, Menu) else []))
            else:
                checked = None
                var = e.get("variable")
                if t == "checkbutton" and isinstance(var, Variable):
                    checked = str(var._raw()) == str(e["onvalue"])
                elif t == "radiobutton" and isinstance(var, Variable):
                    checked = str(var._raw()) == str(e.get("value"))
                out.append(MenuItem(str(e.get("label", "")),
                                    (lambda e=e: self._activate_entry(e)),
                                    shortcut=e.get("accelerator") or None, checked=checked,
                                    enabled=e.get("state", "normal") != "disabled"))
        return out

    def _bar_node(self):
        """The window header: a Blender menu bar of this menu's cascades."""
        if self._bar is None:
            self._bar = buipy.KitNode("menubar")
        self._rebuild_bar()
        return self._bar

    def _rebuild_bar(self):
        bar = self._bar
        for c in bar.children:
            bar.remove(c)
        for e in self._entries:
            if e["type"] in ("cascade", "command"):
                b = buipy.KitNode("menubutton")
                b.set("text", str(e.get("label", "")))
                sub = e.get("menu")
                if isinstance(sub, Menu):
                    if self not in sub._owners:
                        sub._owners.append(self)
                    b.set_menu(sub._items())
                elif e["type"] == "command":  # a command directly on the bar
                    b.set_menu([MenuItem(str(e.get("label", "")),
                                         (lambda e=e: self._activate_entry(e)))])
                bar.add(b)

    def _changed(self):
        if self._bar is not None:
            self._rebuild_bar()
        for owner in list(self._owners):
            owner._menu_changed()

    def _menu_changed(self):
        self._changed()

    def post(self, x, y):
        """Display a menu at position X,Y (screen coordinates)."""
        post = self.cget("postcommand")
        if post:
            _call(self, _command(post))
        top = self.master._toplevel_window() if self.master is not None else \
            _get_default_root()
        pos = top._window.wm_get("pos") or (0, 0)
        top._ctx.open_menu(None, self._items(), self.cget("title") or None,
                           (int(x) - int(pos[0]), int(y) - int(pos[1])))

    def tk_popup(self, x, y, entry=""):
        """Post the menu at position X,Y with entry ENTRY."""
        self.post(x, y)

    def unpost(self):
        """Unmap a menu."""
        self._toplevel_window()._ctx.close_popups()


class Menubutton(_TextOptions, Widget):
    """Menubutton widget, obsolete since Tk8.0 (a Blender header menu)."""

    _KIND = "menubutton"
    _DEFAULTS = {"text": "", "direction": "below", "indicatoron": 0, "state": "normal"}

    def __init__(self, master=None, cnf={}, **kw):  # noqa: B006
        Widget.__init__(self, master, "menubutton", cnf, kw)

    def _apply(self, key, value):
        if key == "menu":
            menu = value if isinstance(value, Menu) or value in (None, "") else \
                self.nametowidget(value)
            if isinstance(menu, Menu):
                if self not in menu._owners:
                    menu._owners.append(self)
                self._opts["menu"] = menu
                self._menu_changed()
        elif not self._text_option(key, value):
            super()._apply(key, value)

    def _menu_changed(self):
        menu = self._opts.get("menu")
        if isinstance(menu, Menu) and self.winfo_exists():
            self._node.set_menu(menu._items())


class OptionMenu(Menubutton):
    """OptionMenu which allows the user to select a value from a menu (a
    Blender enum dropdown)."""

    _KIND = "dropdown"

    def __init__(self, master, variable, value, *values, **kwargs):
        """Construct an optionmenu widget with the parent MASTER, with the
        resource textvariable set to VARIABLE, the initially selected value
        VALUE, the other menu values VALUES and an additional keyword
        argument command."""
        command = kwargs.pop("command", None)
        if kwargs:
            raise TclError("unknown option -" + next(iter(kwargs)))
        Widget.__init__(self, master, "menubutton", {}, {})
        self._variable = variable
        self.__menu = Menu(self, name="menu", tearoff=0)
        self._opts["menu"] = self.__menu
        self._opts["textvariable"] = variable
        self.__menu._owners.append(self)
        for v in (value,) + values:
            self.__menu.add_command(label=v, command=_setit(variable, v, command))
        variable.set(value)
        variable.trace_add("write", lambda *a: self.winfo_exists() and self._from_var())
        self._menu_changed()

    def _labels(self):
        return [str(e.get("label", "")) for e in self.__menu._entries
                if e["type"] in ("command", "radiobutton", "checkbutton")]

    def _menu_changed(self):
        if self.winfo_exists():
            self._node.set("items", self._labels())
            self._from_var()

    def _from_var(self):
        labels = self._labels()
        v = str(self._variable._raw())
        if v in labels:
            self._node.set("value", float(labels.index(v)))

    def _event(self, kind, ref, a, b):
        if kind == EV_CHANGE:
            entries = [e for e in self.__menu._entries
                       if e["type"] in ("command", "radiobutton", "checkbutton")]
            i = int(self._node.get("value"))
            if 0 <= i < len(entries):
                self.__menu._activate_entry(entries[i])

    def __getitem__(self, name):
        if name == "menu":
            return self.__menu
        return Widget.__getitem__(self, name)

    def destroy(self):
        """Destroy this widget and the associated menu."""
        Menubutton.destroy(self)
        self.__menu = None


# ==========================================================================
# PanedWindow


class _Sash(buipy.KitNode):
    """The divider between two panes (Blender's region edge)."""

    def __init__(self, paned, index):
        super().__init__("frame")
        self.paned, self.index = paned, index
        self.set("events", True)
        self.set("bg", (0.11, 0.11, 0.11, 1.0))

    def _on_event(self, kind, ref, a, b):
        p = self.paned
        horizontal = p._horizontal()
        if kind == EV_PRESS:
            pane = p._panes[self.index]
            r = pane._node.rect
            self._start = (r[2] if horizontal else r[3], self.rect[0] + int(a),
                           self.rect[1] + int(b))
        elif kind == EV_MOTION and getattr(self, "_start", None):
            size, x0, y0 = self._start
            pos = self.rect[0] + int(a) if horizontal else self.rect[1] + int(b)
            p._set_pane_size(self.index, size + pos - (x0 if horizontal else y0))
        elif kind == 9:  # release
            self._start = None


class PanedWindow(Widget):
    """panedwindow widget: panes side by side with draggable sashes."""

    _KIND = "frame"
    _DEFAULTS = {"orient": "horizontal", "sashwidth": 4, "showhandle": 0, "handlesize": 8,
                 "opaqueresize": 1, "sashrelief": "flat"}

    def __init__(self, master=None, cnf={}, **kw):  # noqa: B006
        Widget.__init__(self, master, "panedwindow", cnf, kw)

    def _setup(self):
        self._panes = []
        self._sashes = []
        self._pane_opts = {}

    def _horizontal(self):
        return str(self.cget("orient")).startswith("h")

    def _apply(self, key, value):
        if key in ("orient", "sashwidth"):
            self._relayout()
        elif key in ("width", "height"):
            self._node.geom(int(self._node.geom_get("manager")),
                            **{"req" + key: int(_pixels(value))})
        else:
            super()._apply(key, value)

    def add(self, child, **kw):
        """Add a child widget to the panedwindow in a new pane."""
        if child in self._panes:
            self.paneconfigure(child, **kw)
            return
        self._panes.append(child)
        self._pane_opts[child] = dict(kw)
        self._relayout()

    def insert(self, pos, child, **kw):
        if child in self._panes:
            self._panes.remove(child)
        i = len(self._panes) if str(pos) == "end" else (
            pos if isinstance(pos, int) else self._panes.index(pos))
        self._panes.insert(i, child)
        self._pane_opts.setdefault(child, {}).update(kw)
        self._relayout()

    def remove(self, child):
        """Remove the pane containing child from the panedwindow."""
        if child in self._panes:
            self._panes.remove(child)
            self._pane_opts.pop(child, None)
            child._node.geom(0)
            child._ginfo = {}
            self._relayout()

    forget = remove

    def panes(self):
        """Returns an ordered list of the child panes."""
        return tuple(self._panes)

    def _weight(self, child):
        o = self._pane_opts.get(child, {})
        if "weight" in o:
            return float(o["weight"])
        stretch = str(o.get("stretch", "last"))
        i = self._panes.index(child)
        last = len(self._panes) - 1
        return 1.0 if (stretch == "always" or (stretch == "last" and i == last) or
                       (stretch == "first" and i == 0) or
                       (stretch == "middle" and 0 < i < last)) else 0.0

    def _relayout(self):
        node = self._node
        for s in self._sashes:
            node.remove(s)
        self._sashes = []
        h = self._horizontal()
        side = 2 if h else 0
        sw = int(_pixels(self.cget("sashwidth")))
        for i, child in enumerate(self._panes):
            if i:
                sash = _Sash(self, i - 1)
                sash.set("cursor", 1.0 if h else 2.0)
                node.add(sash)
                sash.geom(1, side=side, fill=2 if h else 1,
                          **{"reqwidth" if h else "reqheight": sw})
                self._sashes.append(sash)
            node.add(child._node)
            o = self._pane_opts.get(child, {})
            px, py = _pads(o.get("padx", 0)), _pads(o.get("pady", 0))
            size = o.get("width" if h else "height")
            params = {"reqwidth" if h else "reqheight": int(_pixels(size)) if size else 0}
            child._node.geom(1, side=side, fill=3, expand=1 if self._weight(child) else 0,
                             padx0=px[0], padx1=px[1], pady0=py[0], pady1=py[1], **params)
            child._ginfo = {"manager": "panedwindow", "in": self}

    def _set_pane_size(self, index, size):
        child = self._panes[index]
        o = self._pane_opts.setdefault(child, {})
        minsize = int(_pixels(o.get("minsize", 0) or 0))
        size = max(minsize, int(size))
        o["width" if self._horizontal() else "height"] = size
        child._node.geom(1, **{"reqwidth" if self._horizontal() else "reqheight": size})

    def paneconfigure(self, tagOrId, cnf=None, **kw):
        """Query or modify the configuration options of the pane."""
        o = self._pane_opts.setdefault(tagOrId, {})
        kw = dict(cnf, **kw) if cnf else kw
        if not kw:
            return {k: (k, "", "", "", v) for k, v in o.items()}
        o.update(kw)
        self._relayout()
        return None

    paneconfig = paneconfigure

    def panecget(self, child, option):
        """Query a management option for window."""
        return self._pane_opts.get(child, {}).get(option.lstrip("-"), "")

    def sash_coord(self, index):
        """Return the current x and y pair for the sash given by index."""
        r = self._sashes[int(index)].rect
        own = self._node.rect
        return (r[0] - own[0], r[1] - own[1])

    def sash_mark(self, index):
        self._mark = self.sash_coord(index)
        return self._mark

    def sash_place(self, index, x, y):
        """Place the sash given by index at the given coordinates."""
        child = self._panes[int(index)]
        own = self._node.rect
        r = child._node.rect
        if self._horizontal():
            self._set_pane_size(int(index), int(x) - (r[0] - own[0]))
        else:
            self._set_pane_size(int(index), int(y) - (r[1] - own[1]))

    def sash(self, *args):
        return getattr(self, "sash_" + args[0])(*args[1:])

    def proxy_coord(self):
        return getattr(self, "_proxy", (0, 0))

    def proxy_forget(self):
        self._proxy = (0, 0)

    def proxy_place(self, x, y):
        self._proxy = (int(x), int(y))

    def proxy(self, *args):
        return getattr(self, "proxy_" + args[0])(*args[1:])

    def identify(self, x, y):
        """Identify the panedwindow component at point x, y."""
        own = self._node.rect
        for i, s in enumerate(self._sashes):
            r = s.rect
            if r[0] - own[0] <= int(x) < r[0] - own[0] + r[2] and \
                    r[1] - own[1] <= int(y) < r[1] - own[1] + r[3]:
                return (i, "sash")
        return ""
