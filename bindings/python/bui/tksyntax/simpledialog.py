"""tkinter.simpledialog: Dialog, SimpleDialog, askstring / askinteger /
askfloat — Blender-styled modal windows.

SPDX-License-Identifier: GPL-2.0-or-later
"""

from __future__ import annotations

from . import (ACTIVE, Button, Entry, Frame, Label, Message, TclError, Toplevel,
               _get_temp_root, getdouble, getint)
from .constants import BOTH, END, LEFT, W, X

__all__ = ["SimpleDialog", "Dialog", "askinteger", "askfloat", "askstring"]


def _place_window(w, parent=None):
    """Center `w` over its parent (or the screen) and show it."""
    w.wm_withdraw()
    w.update_idletasks()
    rw, rh = w._node.natural()
    w.geometry("%dx%d" % (max(rw, 240), max(rh, 60)))
    if parent is not None and parent.winfo_ismapped():
        px, py = parent.winfo_rootx(), parent.winfo_rooty()
        pw, ph = parent.winfo_width(), parent.winfo_height()
        x, y = px + (pw - max(rw, 240)) // 2, py + (ph - max(rh, 60)) // 2
    else:
        x = (w.winfo_screenwidth() - max(rw, 240)) // 2
        y = (w.winfo_screenheight() - max(rh, 60)) // 3
    w.geometry("+%d+%d" % (max(0, x), max(0, y)))
    w.wm_deiconify()


class SimpleDialog:
    """A message with a row of buttons; go() returns the chosen index."""

    def __init__(self, master, text="", buttons=[], default=None, cancel=None,  # noqa: B006
                 title=None, class_=None, *, _icon=None, _detail=None):
        self.root = Toplevel(master, class_=class_) if class_ else Toplevel(master)
        if title:
            self.root.title(title)
            self.root.iconname(title)
        self.root.resizable(False, False)
        body = Frame(self.root, padx=12, pady=10)
        body.pack(fill=BOTH, expand=True)
        if _icon:
            Label(body, bitmap=_icon).pack(side=LEFT, anchor="n", padx=(0, 10))
        texts = Frame(body)
        texts.pack(side=LEFT, fill=BOTH, expand=True)
        self.message = Message(texts, text=text, aspect=400, width=320)
        self.message.pack(anchor=W, fill=X)
        if _detail:
            Message(texts, text=_detail, width=320).pack(anchor=W, fill=X, pady=(6, 0))
        self.frame = Frame(self.root, padx=10, pady=8)
        self.frame.pack(fill=X)
        self.num = default
        self.cancel = cancel
        self.default = default
        self.root.bind("<Return>", self.return_event)
        for num, s in enumerate(buttons):
            b = Button(self.frame, text=s, command=(lambda self=self, num=num: self.done(num)))
            if num == default:
                b.config(default=ACTIVE)
            b.pack(side=LEFT, fill=X, expand=True, padx=3)
        self.root.protocol("WM_DELETE_WINDOW", self.wm_delete_window)
        self.root.transient(master)
        _place_window(self.root, master)

    def go(self):
        self.root.wait_visibility()
        self.root.grab_set()
        self.root.wait_window()
        return self.num

    def return_event(self, event):
        if self.default is None:
            self.root.bell()
        else:
            self.done(self.default)

    def wm_delete_window(self):
        if self.cancel is None:
            self.root.bell()
        else:
            self.done(self.cancel)

    def done(self, num):
        self.num = num
        self.root.grab_release()
        self.root.destroy()


class Dialog(Toplevel):
    """Class to open dialogs (override body(), validate(), apply())."""

    def __init__(self, parent, title=None):
        """Initialize a dialog."""
        master = parent
        if master is None:
            master = _get_temp_root()
        Toplevel.__init__(self, master)
        self.withdraw()
        if parent is not None and parent.winfo_viewable():
            self.transient(parent)
        if title:
            self.title(title)
        self.parent = parent
        self.result = None
        body = Frame(self)
        self.initial_focus = self.body(body)
        body.pack(padx=8, pady=8, fill=BOTH, expand=True)
        self.buttonbox()
        if self.initial_focus is None:
            self.initial_focus = self
        self.protocol("WM_DELETE_WINDOW", self.cancel)
        _place_window(self, parent)
        self.initial_focus.focus_set()
        self.wait_visibility()
        self.grab_set()
        self.wait_window(self)

    def destroy(self):
        """Destroy the window."""
        self.initial_focus = None
        Toplevel.destroy(self)

    def body(self, master):
        """Create dialog body. Return widget that should have initial focus."""

    def buttonbox(self):
        """Add standard button box (override if you do not want the standard buttons)."""
        box = Frame(self)
        w = Button(box, text="OK", width=10, command=self.ok, default=ACTIVE)
        w.pack(side=LEFT, padx=5, pady=5)
        w = Button(box, text="Cancel", width=10, command=self.cancel)
        w.pack(side=LEFT, padx=5, pady=5)
        self.bind("<Return>", self.ok)
        self.bind("<Escape>", self.cancel)
        box.pack()

    def ok(self, event=None):
        if not self.validate():
            self.initial_focus.focus_set()  # put focus back
            return
        self.withdraw()
        self.update_idletasks()
        try:
            self.apply()
        finally:
            self.cancel()

    def cancel(self, event=None):
        if self.parent is not None:
            self.parent.focus_set()
        self.grab_release()
        self.destroy()

    def validate(self):
        """Validate the data (override)."""
        return 1

    def apply(self):
        """Process the data (override)."""


class _QueryDialog(Dialog):
    def __init__(self, title, prompt, initialvalue=None, minvalue=None, maxvalue=None,
                 parent=None):
        self.prompt = prompt
        self.minvalue = minvalue
        self.maxvalue = maxvalue
        self.initialvalue = initialvalue
        Dialog.__init__(self, parent, title)

    def destroy(self):
        self.entry = None
        Dialog.destroy(self)

    def body(self, master):
        w = Label(master, text=self.prompt, justify=LEFT, anchor=W)
        w.grid(row=0, padx=5, sticky=W)
        self.entry = Entry(master, name="entry")
        self.entry.grid(row=1, padx=5, sticky="we")
        if self.initialvalue is not None:
            self.entry.insert(0, self.initialvalue)
            self.entry.select_range(0, END)
        return self.entry

    def validate(self):
        try:
            result = self.getresult()
        except (ValueError, TclError):
            from . import messagebox
            messagebox.showwarning("Illegal value", self.errormessage + "\nPlease try again",
                                   parent=self)
            return 0
        if self.minvalue is not None and result < self.minvalue:
            from . import messagebox
            messagebox.showwarning("Too small", "The allowed minimum value is %s. "
                                   "Please try again." % self.minvalue, parent=self)
            return 0
        if self.maxvalue is not None and result > self.maxvalue:
            from . import messagebox
            messagebox.showwarning("Too large", "The allowed maximum value is %s. "
                                   "Please try again." % self.maxvalue, parent=self)
            return 0
        self.result = result
        return 1


class _QueryInteger(_QueryDialog):
    errormessage = "Not an integer."

    def getresult(self):
        return getint(self.entry.get())


class _QueryFloat(_QueryDialog):
    errormessage = "Not a floating-point value."

    def getresult(self):
        return getdouble(self.entry.get())


class _QueryString(_QueryDialog):
    def __init__(self, *args, **kw):
        self.__show = kw.pop("show", None)
        _QueryDialog.__init__(self, *args, **kw)

    def body(self, master):
        entry = _QueryDialog.body(self, master)
        if self.__show is not None:
            entry.configure(show=self.__show)
        return entry

    def getresult(self):
        return self.entry.get()


def askinteger(title, prompt, **kw):
    """Get an integer from the user (None if cancelled)."""
    d = _QueryInteger(title, prompt, **kw)
    return d.result


def askfloat(title, prompt, **kw):
    """Get a float from the user (None if cancelled)."""
    d = _QueryFloat(title, prompt, **kw)
    return d.result


def askstring(title, prompt, **kw):
    """Get a string from the user (None if cancelled)."""
    d = _QueryString(title, prompt, **kw)
    return d.result
