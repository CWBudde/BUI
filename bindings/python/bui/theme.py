"""Blender's default UI theme and metrics, for the BUI widget model.

Values follow Blender 5.x's default theme (``userdef_default_theme.c``:
``wcol_regular``, ``wcol_text``, ``wcol_numslider`` ...) and its interface
metrics (``U.widget_unit`` = 20 px at scale 1, 11 pt UI text, roundness as a
fraction of the widget unit). Widgets read these at paint time, so changing
:data:`SCALE` or a color set restyles everything.

SPDX-License-Identifier: GPL-2.0-or-later
"""

from __future__ import annotations

import glob
import os
import sys
from dataclasses import dataclass


def _rgba(hex_rgba: int):
    return (((hex_rgba >> 24) & 255) / 255.0, ((hex_rgba >> 16) & 255) / 255.0,
            ((hex_rgba >> 8) & 255) / 255.0, (hex_rgba & 255) / 255.0)


@dataclass
class WidgetColors:
    """uiWidgetColors: one Blender widget color set."""
    outline: tuple
    inner: tuple
    inner_sel: tuple
    item: tuple
    text: tuple
    text_sel: tuple
    roundness: float = 0.2
    shadetop: int = 0
    shadedown: int = 0


def _wcol(outline, inner, inner_sel, item, text, text_sel, roundness=0.2,
          shadetop=0, shadedown=0) -> WidgetColors:
    return WidgetColors(_rgba(outline), _rgba(inner), _rgba(inner_sel), _rgba(item),
                        _rgba(text), _rgba(text_sel), roundness, shadetop, shadedown)


# --- color sets (Blender 5.x default theme: userdef_default_theme.c) ----------
WCOL = {
    "regular": _wcol(0x3d3d3dff, 0x545454ff, 0x4772b3ff, 0x1d1d1d80, 0xe6e6e6ff, 0xffffffff, roundness=0.2),
    "tool": _wcol(0x3d3d3dff, 0x545454ff, 0x4772b3ff, 0xffffffff, 0xe6e6e6ff, 0xffffffff, roundness=0.2),
    "toggle": _wcol(0x3d3d3dff, 0x545454ff, 0x4772b3ff, 0x252525ff, 0xe6e6e6ff, 0xffffffff, roundness=0.2),
    "radio": _wcol(0x3d3d3dff, 0x545454ff, 0x4772b3ff, 0x252525ff, 0xe6e6e6ff, 0xffffffff, roundness=0.2),
    "option": _wcol(0x3d3d3dff, 0x545454ff, 0x4772b3ff, 0xffffffff, 0xe6e6e6ff, 0xffffffff, roundness=0.2),
    "text": _wcol(0x3d3d3dff, 0x1d1d1dff, 0x181818ff, 0xffffff33, 0xe6e6e6ff, 0xffffffff, roundness=0.2),
    "num": _wcol(0x3d3d3dff, 0x545454ff, 0x222222ff, 0x4772b3ff, 0xe6e6e6ff, 0xffffffff, roundness=0.2),
    "numslider": _wcol(0x3d3d3dff, 0x545454ff, 0x222222ff, 0x4772b3ff, 0xe6e6e6ff, 0xffffffff, roundness=0.2),
    "menu": _wcol(0x3d3d3dff, 0x282828ff, 0x4772b3b3, 0xd9d9d9ff, 0xe6e6e6ff, 0xffffffff, roundness=0.2),
    "pulldown": _wcol(0x3d3d3d00, 0x22222200, 0xffffff1a, 0xffffff8f, 0xd9d9d9ff, 0xffffffff, roundness=0.2),
    "menu_back": _wcol(0x242424ff, 0x181818ff, 0x4772b3ff, 0xd9d9d9ff, 0x999999ff, 0xffffffff, roundness=0.2),
    "menu_item": _wcol(0x3d3d3d00, 0x18181800, 0x4772b3ff, 0xffffff8f, 0xddddddff, 0xffffffff, roundness=0.2),
    "list_item": _wcol(0x2d2d2d00, 0xffffff00, 0x4772b3ff, 0xffffff33, 0xccccccff, 0xffffffff, roundness=0.2),
    "box": _wcol(0x3d3d3dff, 0x1d1d1d80, 0x545454ff, 0x191919ff, 0xe6e6e6ff, 0xffffffff, roundness=0.2),
    "progress": _wcol(0x3d3d3dff, 0x222222ff, 0x4772b3ff, 0x4772b3ff, 0xe6e6e6ff, 0xffffffff, roundness=0.2),
    "scroll": _wcol(0x3d3d3dff, 0x22222200, 0xffffffff, 0x545454ff, 0xe6e6e6ff, 0xffffffff, roundness=0.5),
    "tooltip": _wcol(0x242424ff, 0x1d1d1dff, 0x4772b3ff, 0xd9d9d9ff, 0xd9d9d9ff, 0xffffffff, roundness=0.2),
    "tab": _wcol(0x1d1d1dff, 0x1d1d1dff, 0x303030ff, 0x1d1d1dff, 0x989898ff, 0xffffffff, roundness=0.2),
    "pie_menu": _wcol(0x242424ff, 0x181818ff, 0x4772b3ff, 0x545454ff, 0xd9d9d9ff, 0xffffffff, roundness=0.2),
    "curve": _wcol(0x727272ff, 0x545454ff, 0x000000ff, 0x00000059, 0x08080800, 0xc9c9c900, roundness=0.2),
}

# --- region / panel colors ----------------------------------------------------
REGION_BG = _rgba(0x303030ff)       # editor region background
TOPBAR_BG = _rgba(0x181818ff)       # top bar / menubar header
PANEL_HEADER = _rgba(0x3d3d3dff)
PANEL_BACK = _rgba(0x3d3d3dff)
PANEL_OUTLINE = _rgba(0xffffff11)
TEXT = _rgba(0xe6e6e6ff)            # labels
TEXT_DISABLED_FAC = 0.5             # Blender draws inactive widgets at 50% alpha
WIDGET_EMBOSS = _rgba(0x00000026)
TEXT_CURSOR = _rgba(0x71a8ffff)
TEXT_SELECTION = _rgba(0x4772b380)
MENU_SHADOW_FAC = 0.2
MENU_SHADOW_WIDTH = 6
HOVER_LIGHTEN = 15 / 255.0          # widget_active_color(): inner + 15


# --- metrics ------------------------------------------------------------------
def _detect_scale() -> float:
    env = os.environ.get("BUI_UI_SCALE")
    if env:
        try:
            return max(0.5, min(4.0, float(env)))
        except ValueError:
            pass
    if sys.platform == "win32":
        try:
            import ctypes
            dpi = ctypes.windll.user32.GetDpiForSystem()
            return max(1.0, dpi / 96.0)
        except Exception:  # pragma: no cover - older Windows
            return 1.0
    return 1.0


def _detect_font() -> str:
    """Blender 4.x ships Inter; use it when installed, else the OS UI font."""
    if sys.platform == "win32":
        dirs = [os.path.join(os.environ.get("WINDIR", r"C:\Windows"), "Fonts"),
                os.path.join(os.environ.get("LOCALAPPDATA", ""), "Microsoft", "Windows",
                             "Fonts")]
        for d in dirs:
            if d and glob.glob(os.path.join(d, "Inter*")):
                return "Inter"
        return "Segoe UI"
    for d in ("/usr/share/fonts", "/usr/local/share/fonts",
              os.path.expanduser("~/.fonts"), "/Library/Fonts",
              os.path.expanduser("~/Library/Fonts")):
        if glob.glob(os.path.join(d, "**", "Inter*"), recursive=True):
            return "Inter"
    return ""  # host default UI font


SCALE = _detect_scale()
FONT_FAMILY = _detect_font()


# Scale-derived metrics, recomputed only when the scale changes (they are
# read hundreds of times per frame).
_UNIT = _FONT = _PAD = 0


def _update_metrics() -> None:
    global _UNIT, _FONT, _PAD
    _UNIT = int(round(20 * SCALE))
    _FONT = round(11 * SCALE)
    _PAD = int(round(0.4 * _UNIT))


def unit() -> int:
    """U.widget_unit: standard widget height in pixels."""
    return _UNIT


def font_size() -> float:
    """UI text size in pixels (11 pt at Blender's 72-dpi base)."""
    return _FONT


def pad() -> int:
    """UI_TEXT_MARGIN_X: horizontal text inset inside widgets."""
    return _PAD


def radius(wcol: WidgetColors) -> float:
    return wcol.roundness * unit()


def set_scale(scale: float) -> None:
    """Change the UI scale (like Blender's Resolution Scale)."""
    global SCALE
    SCALE = max(0.5, min(4.0, float(scale)))
    _update_metrics()


# --- color helpers --------------------------------------------------------------
def lighten(c, amount=HOVER_LIGHTEN):
    return (min(1.0, c[0] + amount), min(1.0, c[1] + amount), min(1.0, c[2] + amount), c[3])


def shade(c, delta: int):
    """Blender's shadetop/shadedown: add `delta` (-100..100) in 0..255 units."""
    d = delta / 255.0
    return (max(0.0, min(1.0, c[0] + d)), max(0.0, min(1.0, c[1] + d)),
            max(0.0, min(1.0, c[2] + d)), c[3])


def with_alpha(c, a):
    return (c[0], c[1], c[2], c[3] * a)


_update_metrics()
