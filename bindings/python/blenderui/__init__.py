"""blenderui: the PyPI name of BUI (``pip install blenderui``).

``blenderui`` and ``bui`` are the same package: ``blenderui.X`` *is*
``bui.X`` (the same module objects), so either spelling works::

    import blenderui.tksyntax as tk
    from blenderui.components import App, Panel

Prefer ``blenderui`` in code you publish: another, unrelated project owns the
name ``bui`` on PyPI.

SPDX-License-Identifier: GPL-2.0-or-later
"""

from __future__ import annotations

import importlib
import importlib.abc
import importlib.util
import sys

import bui as _bui
from bui import *  # noqa: F401,F403
from bui._version import __version__  # noqa: F401

_PREFIX = __name__ + "."


class _AliasLoader(importlib.abc.Loader):
    def __init__(self, target):
        self.target = target

    def create_module(self, spec):
        return importlib.import_module(self.target)

    def exec_module(self, module):
        pass  # already executed as bui.*


class _AliasFinder(importlib.abc.MetaPathFinder):
    """Resolve ``blenderui.<path>`` to ``bui.<path>``."""

    def find_spec(self, fullname, path=None, target=None):
        if not fullname.startswith(_PREFIX):
            return None
        real = "bui." + fullname[len(_PREFIX):]
        if importlib.util.find_spec(real) is None:
            return None
        return importlib.util.spec_from_loader(fullname, _AliasLoader(real))


if not any(isinstance(f, _AliasFinder) for f in sys.meta_path):
    sys.meta_path.insert(0, _AliasFinder())

__path__ = []  # a package, so `import blenderui.x` consults the finder
__all__ = getattr(_bui, "__all__", [n for n in dir(_bui) if not n.startswith("_")])
