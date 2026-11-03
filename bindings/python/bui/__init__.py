"""bui — Python companion package for the BUI UI toolkit.

High-level syntax layers (the recommended way to write apps):

* :mod:`bui.qtsyntax` — Qt syntax: ``QApplication``, ``QWidget``,
  ``QPushButton``, ``QVBoxLayout``, ``clicked.connect(...)``.
* :mod:`bui.tksyntax` — tkinter syntax: ``Tk()``, ``Button(command=...)``,
  ``pack()``, ``mainloop()``.

Lower-level pieces:

* :class:`bui.svg.SvgSink` — record a frame to an SVG file (no dependencies).
* :class:`bui.qt.BuiCanvas` — embed BUI in a real PySide6/PyQt6 widget.

Everything renders through the BUI engine (buipy, a small extension module).
If ``import buipy`` fails, this package looks for the compiled extension in
the CMake build tree automatically; you can point it at a build dir with the
``BUIPY_BUILD_DIR`` environment variable.

SPDX-License-Identifier: GPL-2.0-or-later
"""

from __future__ import annotations

import glob
import os
import sys

class Backend:
    """Protocol BUI calls with everything the toolkit draws.

    All coordinates are 2D pixels. Colors are RGBA floats in 0..1.
    Override only what you need; unimplemented methods are skipped.
    """

    def begin_frame(self, width: int, height: int) -> None:
        """Called before a batch of primitives."""

    def scissor(self, x: int, y: int, w: int, h: int) -> None:
        """Clip rect; ``w``/``h`` <= 0 removes the clip."""

    def triangles(self, coords, colors) -> None:
        """Non-indexed triangles: ``coords``/``colors`` are per vertex."""

    def lines(self, coords, colors) -> None:
        """Line list: vertices come in endpoint pairs."""

    def points(self, coords, colors, size) -> None:
        """Points at ``size`` pixels."""

    def text(self, x, y, text, size, color) -> None:
        """Text run at baseline position."""

    def end_frame(self) -> None:
        """Called after a batch of primitives."""


def _ensure_buipy_importable() -> None:
    """Make `import buipy` work when running from the source tree.

    The compiled extension lives in the CMake build tree
    (build*/bindings/python). Locating it here means apps can simply
    `import bui` — no PYTHONPATH juggling. Set BUIPY_BUILD_DIR to override.
    """
    try:
        import buipy  # noqa: F401

        return
    except ImportError:
        pass

    # __file__ = <repo>/bindings/python/bui/__init__.py → repo root is up 3 dirs.
    pkg_dir = os.path.dirname(os.path.abspath(__file__))
    repo = os.path.abspath(os.path.join(pkg_dir, "..", "..", ".."))
    import importlib.machinery

    # Only extensions built for THIS interpreter (e.g. .cp314-win_amd64.pyd).
    suffixes = importlib.machinery.EXTENSION_SUFFIXES

    def _ext_in(d):
        found = []
        for suf in suffixes:
            found += glob.glob(os.path.join(d, "buipy" + suf))
        return found

    override = os.environ.get("BUIPY_BUILD_DIR", "")
    if override and _ext_in(override):
        best_dir = override
    else:
        # Several build trees may exist (build, build-ui, ...): use the most
        # recently built extension so a stale tree never shadows a fresh one.
        best_dir, best_mtime = None, -1.0
        for cand in glob.glob(os.path.join(repo, "build*", "bindings", "python")):
            for ext in _ext_in(cand):
                mtime = os.path.getmtime(ext)
                if mtime > best_mtime:
                    best_dir, best_mtime = cand, mtime
    if best_dir and best_dir not in sys.path:
        sys.path.insert(0, best_dir)


_ensure_buipy_importable()

try:
    import buipy  # noqa: F401
except ImportError as _e:  # pragma: no cover
    raise ImportError(
        "buipy (the BUI engine extension) was not found. Build it with CMake "
        "(cmake --build build --target buipy) and/or set BUIPY_BUILD_DIR to the "
        "directory containing the built buipy.pyd. Note: the extension must be "
        "built for the Python interpreter you are running."
    ) from _e


from bui._version import __version__  # noqa: E402


def version() -> str:
    """BUI library version as a string."""
    return buipy.version()


def set_backend(backend) -> None:
    """Route BUI drawing into ``backend`` (or None to disable rendering)."""
    buipy.set_backend(backend)


def set_gpu(on: bool = True) -> None:
    """Draw new native windows on the GPU (Direct3D 11): the same pixels as
    the software raster, with rasterization off the CPU. Falls back to the
    software raster where no device exists. ``BUI_GPU=1``/``0`` overrides.
    Per window: ``Tk(gpu=True)``."""
    buipy.set_gpu(bool(on))


def set_slug(on: bool = True) -> None:
    """How the GPU renderer draws text. ``True`` (the default): Slug, glyph
    outlines evaluated in the pixel shader, sharp at any size. ``False``: a
    per-glyph bitmap atlas like Blender's, with less GPU work. Takes effect
    on the next frame. ``BUI_SLUG=1``/``0`` overrides."""
    buipy.set_slug(bool(on))
