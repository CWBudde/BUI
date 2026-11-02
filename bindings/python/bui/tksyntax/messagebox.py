"""tkinter.messagebox: Blender-styled modal message boxes.

SPDX-License-Identifier: GPL-2.0-or-later
"""

from .commondialog import Dialog

__all__ = ["showinfo", "showwarning", "showerror", "askquestion", "askokcancel", "askyesno",
           "askyesnocancel", "askretrycancel"]

# icons
ERROR = "error"
INFO = "info"
QUESTION = "question"
WARNING = "warning"

# types
ABORTRETRYIGNORE = "abortretryignore"
OK = "ok"
OKCANCEL = "okcancel"
RETRYCANCEL = "retrycancel"
YESNO = "yesno"
YESNOCANCEL = "yesnocancel"

# replies
ABORT = "abort"
RETRY = "retry"
IGNORE = "ignore"
OK = "ok"
CANCEL = "cancel"
YES = "yes"
NO = "no"

_BUTTONS = {ABORTRETRYIGNORE: (ABORT, RETRY, IGNORE), OK: (OK,), OKCANCEL: (OK, CANCEL),
            RETRYCANCEL: (RETRY, CANCEL), YESNO: (YES, NO), YESNOCANCEL: (YES, NO, CANCEL)}


class Message(Dialog):
    """A message box."""

    command = "tk_messageBox"

    def _show(self, options):
        from .simpledialog import SimpleDialog
        from . import _get_temp_root
        kind = options.get("type", OK)
        replies = _BUTTONS[kind]
        default = options.get("default", replies[0])
        cancel = CANCEL if CANCEL in replies else (NO if NO in replies else replies[-1])
        parent = options.get("parent") or self.master or _get_temp_root()
        d = SimpleDialog(parent, text=str(options.get("message", "")),
                         buttons=[r.capitalize() for r in replies],
                         default=replies.index(default) if default in replies else 0,
                         cancel=replies.index(cancel), title=options.get("title"),
                         _icon=options.get("icon", INFO), _detail=options.get("detail"))
        num = d.go()
        return replies[num] if num is not None else cancel


def _show(title=None, message=None, _icon=None, _type=None, **options):
    if _icon and "icon" not in options:
        options["icon"] = _icon
    if _type and "type" not in options:
        options["type"] = _type
    if title:
        options["title"] = title
    if message:
        options["message"] = message
    res = Message(**options).show()
    return str(res)


def showinfo(title=None, message=None, **options):
    """Show an info message"""
    return _show(title, message, INFO, OK, **options)


def showwarning(title=None, message=None, **options):
    """Show a warning message"""
    return _show(title, message, WARNING, OK, **options)


def showerror(title=None, message=None, **options):
    """Show an error message"""
    return _show(title, message, ERROR, OK, **options)


def askquestion(title=None, message=None, **options):
    """Ask a question"""
    return _show(title, message, QUESTION, YESNO, **options)


def askokcancel(title=None, message=None, **options):
    """Ask if operation should proceed; return true if the answer is ok"""
    return _show(title, message, QUESTION, OKCANCEL, **options) == OK


def askyesno(title=None, message=None, **options):
    """Ask a question; return true if the answer is yes"""
    return _show(title, message, QUESTION, YESNO, **options) == YES


def askyesnocancel(title=None, message=None, **options):
    """Ask a question; return true if the answer is yes, None if cancelled."""
    s = _show(title, message, QUESTION, YESNOCANCEL, **options)
    if s == CANCEL:
        return None
    return s == YES


def askretrycancel(title=None, message=None, **options):
    """Ask if operation should be retried; return true if the answer is yes"""
    return _show(title, message, WARNING, RETRYCANCEL, **options) == RETRY
