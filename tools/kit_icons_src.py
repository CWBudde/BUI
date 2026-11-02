"""Vector icons named after Blender's (``ADD``, ``HIDE_OFF``, ``MODIFIER`` ...).

Blender ships its icons as a bitmap sheet; these are resolution-independent
vector re-drawings so they stay crisp at any UI scale. Each icon is drawn
into a square ``(x, y, s)`` box in normalized 0..1 coordinates.

    icons.draw("HIDE_OFF", x, y, size)          # default tint
    icons.draw("ADD", x, y, size, color)        # explicit color
    icons.names()                               # every available icon

SPDX-License-Identifier: GPL-2.0-or-later
"""

from __future__ import annotations

import math

import buipy

from bui import theme as T

_ICONS = {}

# Blender colors some icons (properties-editor tabs, data types).
_ORANGE = (0.93, 0.58, 0.25, 1.0)
_BLUE = (0.40, 0.62, 0.96, 1.0)
_GREEN = (0.45, 0.78, 0.40, 1.0)
_RED = (0.90, 0.36, 0.36, 1.0)
_PURPLE = (0.70, 0.52, 0.95, 1.0)
_YELLOW = (0.95, 0.80, 0.30, 1.0)
TINT = {
    "OBJECT_DATA": _ORANGE, "MESH_CUBE": _ORANGE, "MODIFIER": _BLUE,
    "CONSTRAINT": _BLUE, "PHYSICS": _BLUE, "PARTICLES": _BLUE,
    "MATERIAL": _RED, "WORLD": _RED, "TEXTURE": _RED, "LIGHT": _GREEN,
    "CAMERA_DATA": _GREEN, "MESH_DATA": _GREEN, "SCENE_DATA": _PURPLE,
    "ERROR": _YELLOW, "DECORATE_KEYFRAME": _YELLOW, "DECORATE_ANIMATE": _YELLOW,
}


def names():
    return sorted(_ICONS)


def exists(name) -> bool:
    return name in _ICONS


def draw(name, x, y, s, color=None) -> None:
    """Draw icon ``name`` in the square at (x, y) of side ``s``."""
    fn = _ICONS.get(name)
    if fn is None:
        return
    if color is None:
        color = TINT.get(name, T.with_alpha(T.TEXT, 0.9))
    fn(_Pen(x, y, s, color))


def _icon(*names_):
    def reg(fn):
        for n in names_:
            _ICONS[n] = fn
        return fn
    return reg


class _Pen:
    """Normalized drawing in an icon box."""

    __slots__ = ("x", "y", "s", "c", "lw")

    def __init__(self, x, y, s, color):
        self.x, self.y, self.s, self.c = x, y, s, color
        self.lw = max(1.0, s * 0.085)

    def p(self, u, v):
        return (self.x + u * self.s, self.y + v * self.s)

    def line(self, *pts, w=1.0, color=None):
        pts = [self.p(u, v) for u, v in pts]
        for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
            buipy.draw_line(x0, y0, x1, y1, color or self.c, self.lw * w)

    def poly(self, *pts, color=None):
        buipy.draw_polygon([self.p(u, v) for u, v in pts], color or self.c)

    def disc(self, u, v, r, color=None):
        cx, cy = self.p(u, v)
        buipy.draw_circle(cx, cy, r * self.s, color or self.c)

    def ring(self, u, v, r, w=1.0, ry=None, color=None):
        cx, cy = self.p(u, v)
        buipy.draw_ellipse(cx, cy, r * self.s, (ry if ry is not None else r) * self.s,
                           color or self.c, 0, self.lw * w)

    def rect(self, u, v, w, h, color=None):
        x, y = self.p(u, v)
        buipy.draw_rect(x, y, w * self.s, h * self.s, color or self.c)

    def frame(self, u, v, w, h, r=0.08, color=None):
        x, y = self.p(u, v)
        buipy.draw_widget(x, y, w * self.s, h * self.s, r * self.s, None,
                          color or self.c, None, None, 15, self.lw)

    def box(self, u, v, w, h, r=0.08, color=None):
        x, y = self.p(u, v)
        buipy.draw_roundrect(x, y, w * self.s, h * self.s, r * self.s, color or self.c)

    def arc(self, u, v, r, a0, a1, w=1.0, n=14):
        """Arc in degrees (0 = right, 90 = up)."""
        pts = []
        for i in range(n + 1):
            a = math.radians(a0 + (a1 - a0) * i / n)
            pts.append((u + r * math.cos(a), v - r * math.sin(a)))
        self.line(*pts, w=w)

    def dim(self, k):
        r, g, b, a = self.c
        return (r, g, b, a * k)


# -- symbols --------------------------------------------------------------------


@_icon("ADD", "PLUS")
def _add(p):
    p.line((0.5, 0.18), (0.5, 0.82), w=1.2)
    p.line((0.18, 0.5), (0.82, 0.5), w=1.2)


@_icon("REMOVE", "MINUS")
def _remove(p):
    p.line((0.18, 0.5), (0.82, 0.5), w=1.2)


@_icon("X", "CANCEL", "PANEL_CLOSE")
def _x(p):
    p.line((0.24, 0.24), (0.76, 0.76), w=1.2)
    p.line((0.76, 0.24), (0.24, 0.76), w=1.2)


@_icon("CHECKMARK")
def _check(p):
    p.line((0.2, 0.52), (0.42, 0.74), (0.82, 0.28), w=1.3)


@_icon("SEARCH", "VIEWZOOM")
def _search(p):
    p.ring(0.43, 0.43, 0.24)
    p.line((0.6, 0.6), (0.84, 0.84), w=1.5)


@_icon("ZOOM_IN")
def _zoom_in(p):
    _search(p)
    p.line((0.43, 0.32), (0.43, 0.54))
    p.line((0.32, 0.43), (0.54, 0.43))


@_icon("ZOOM_OUT")
def _zoom_out(p):
    _search(p)
    p.line((0.32, 0.43), (0.54, 0.43))


@_icon("TRIA_DOWN")
def _tri_down(p):
    p.poly((0.22, 0.36), (0.78, 0.36), (0.5, 0.7))


@_icon("TRIA_UP")
def _tri_up(p):
    p.poly((0.22, 0.64), (0.78, 0.64), (0.5, 0.3))


@_icon("TRIA_RIGHT")
def _tri_right(p):
    p.poly((0.36, 0.22), (0.36, 0.78), (0.7, 0.5))


@_icon("TRIA_LEFT")
def _tri_left(p):
    p.poly((0.64, 0.22), (0.64, 0.78), (0.3, 0.5))


@_icon("DOWNARROW_HLT")
def _chev_down(p):
    p.line((0.25, 0.4), (0.5, 0.64), (0.75, 0.4), w=1.1)


@_icon("RIGHTARROW")
def _chev_right(p):
    p.line((0.4, 0.25), (0.64, 0.5), (0.4, 0.75), w=1.1)


@_icon("INFO")
def _info(p):
    p.ring(0.5, 0.5, 0.36)
    p.disc(0.5, 0.32, 0.055)
    p.line((0.5, 0.44), (0.5, 0.7), w=1.2)


@_icon("ERROR")
def _error(p):
    p.poly((0.5, 0.1), (0.92, 0.86), (0.08, 0.86))
    k = (0.1, 0.1, 0.1, 1.0)
    p.line((0.5, 0.36), (0.5, 0.62), w=1.3, color=k)
    p.disc(0.5, 0.74, 0.05, color=k)


@_icon("QUESTION")
def _question(p):
    p.ring(0.5, 0.5, 0.36)
    p.arc(0.5, 0.42, 0.11, 160, -60)
    p.line((0.55, 0.51), (0.5, 0.56), (0.5, 0.62))
    p.disc(0.5, 0.72, 0.045)


@_icon("DOT", "DECORATE", "LAYER_USED")
def _dot(p):
    p.disc(0.5, 0.5, 0.09)


@_icon("DECORATE_KEYFRAME", "KEYFRAME_HLT")
def _key_filled(p):
    p.poly((0.5, 0.2), (0.8, 0.5), (0.5, 0.8), (0.2, 0.5))


@_icon("DECORATE_ANIMATE", "KEYFRAME")
def _key_open(p):
    p.line((0.5, 0.2), (0.8, 0.5), (0.5, 0.8), (0.2, 0.5), (0.5, 0.2))


@_icon("GRIP", "OPTIONS")
def _grip(p):
    for u in (0.36, 0.64):
        for v in (0.25, 0.5, 0.75):
            p.disc(u, v, 0.06)


@_icon("COLLAPSEMENU", "MENU_PANEL")
def _hamburger(p):
    for v in (0.3, 0.5, 0.7):
        p.line((0.2, v), (0.8, v), w=1.1)


# -- visibility / state ---------------------------------------------------------------


@_icon("HIDE_OFF", "RESTRICT_VIEW_OFF")
def _eye(p):
    p.arc(0.5, 0.86, 0.5, 135, 45, w=1.1)
    p.arc(0.5, 0.14, 0.5, -135, -45, w=1.1)
    p.disc(0.5, 0.5, 0.14)


@_icon("HIDE_ON", "RESTRICT_VIEW_ON")
def _eye_closed(p):
    p.arc(0.5, 0.14, 0.5, -140, -40, w=1.1)
    for u, v, du, dv in ((0.5, 0.64, 0.0, 0.14), (0.3, 0.6, -0.07, 0.12),
                         (0.7, 0.6, 0.07, 0.12)):
        p.line((u, v), (u + du, v + dv))


@_icon("RESTRICT_SELECT_OFF")
def _pointer(p):
    p.poly((0.28, 0.12), (0.28, 0.82), (0.45, 0.66), (0.58, 0.9), (0.68, 0.85),
           (0.56, 0.62), (0.78, 0.6))


@_icon("RESTRICT_SELECT_ON")
def _pointer_off(p):
    p.c = p.dim(0.35)
    _pointer(p)


@_icon("RESTRICT_RENDER_OFF", "RENDER_STILL", "CAMERA_DATA", "SCENE")
def _camera(p):
    p.box(0.12, 0.34, 0.56, 0.42, 0.08)
    p.poly((0.68, 0.5), (0.9, 0.36), (0.9, 0.76), (0.68, 0.62))
    p.ring(0.32, 0.24, 0.1)
    p.ring(0.52, 0.24, 0.1)


@_icon("RESTRICT_RENDER_ON")
def _camera_off(p):
    p.c = p.dim(0.35)
    _camera(p)


@_icon("LOCKED")
def _locked(p):
    p.box(0.22, 0.46, 0.56, 0.42, 0.08)
    p.arc(0.5, 0.46, 0.18, 0, 180)


@_icon("UNLOCKED")
def _unlocked(p):
    p.box(0.22, 0.46, 0.56, 0.42, 0.08)
    p.arc(0.68, 0.38, 0.16, 0, 180)


@_icon("PINNED")
def _pinned(p):
    p.line((0.5, 0.56), (0.5, 0.9), w=1.1)
    p.box(0.3, 0.12, 0.4, 0.3, 0.06)
    p.rect(0.2, 0.44, 0.6, 0.12)


@_icon("UNPINNED")
def _unpinned(p):
    p.line((0.44, 0.56), (0.18, 0.86), w=1.1)
    p.poly((0.5, 0.1), (0.9, 0.5), (0.6, 0.6), (0.4, 0.4))


@_icon("FAKE_USER_OFF")
def _shield_off(p):
    p.line((0.5, 0.12), (0.82, 0.24), (0.78, 0.6), (0.5, 0.88), (0.22, 0.6),
           (0.18, 0.24), (0.5, 0.12))


@_icon("FAKE_USER_ON")
def _shield_on(p):
    p.poly((0.5, 0.12), (0.82, 0.24), (0.78, 0.6), (0.5, 0.88), (0.22, 0.6), (0.18, 0.24))


@_icon("LINKED")
def _linked(p):
    p.ring(0.36, 0.5, 0.18, ry=0.12)
    p.ring(0.64, 0.5, 0.18, ry=0.12)


@_icon("UNLINKED")
def _unlinked(p):
    p.ring(0.28, 0.5, 0.16, ry=0.11)
    p.ring(0.72, 0.5, 0.16, ry=0.11)


# -- files / edit --------------------------------------------------------------------------


@_icon("FILE_FOLDER", "FILEBROWSER", "FOLDER_REDIRECT")
def _folder(p):
    p.poly((0.1, 0.24), (0.4, 0.24), (0.48, 0.34), (0.9, 0.34), (0.9, 0.82), (0.1, 0.82))


@_icon("FILE", "FILE_BLANK", "NEW")
def _file(p):
    p.line((0.24, 0.1), (0.6, 0.1), (0.78, 0.28), (0.78, 0.9), (0.24, 0.9), (0.24, 0.1))
    p.line((0.6, 0.1), (0.6, 0.28), (0.78, 0.28))


@_icon("DUPLICATE", "COPYDOWN")
def _duplicate(p):
    p.frame(0.14, 0.14, 0.5, 0.5, 0.06)
    p.box(0.38, 0.38, 0.5, 0.5, 0.06)


@_icon("PASTEDOWN")
def _paste(p):
    p.frame(0.18, 0.18, 0.64, 0.7, 0.06)
    p.box(0.34, 0.1, 0.32, 0.16, 0.04)


@_icon("TRASH")
def _trash(p):
    p.line((0.16, 0.24), (0.84, 0.24), w=1.2)
    p.line((0.38, 0.24), (0.42, 0.12), (0.58, 0.12), (0.62, 0.24))
    p.line((0.24, 0.3), (0.3, 0.88), (0.7, 0.88), (0.76, 0.3))
    p.line((0.5, 0.38), (0.5, 0.78))


@_icon("EYEDROPPER")
def _eyedropper(p):
    p.line((0.16, 0.84), (0.56, 0.44), w=1.4)
    p.poly((0.5, 0.36), (0.72, 0.14), (0.86, 0.28), (0.64, 0.5))
    p.line((0.44, 0.32), (0.68, 0.56), w=1.2)


@_icon("EXPORT")
def _export(p):
    p.line((0.2, 0.55), (0.2, 0.86), (0.8, 0.86), (0.8, 0.55))
    p.line((0.5, 0.66), (0.5, 0.14), w=1.2)
    p.line((0.32, 0.32), (0.5, 0.14), (0.68, 0.32), w=1.2)


@_icon("IMPORT")
def _import(p):
    p.line((0.2, 0.55), (0.2, 0.86), (0.8, 0.86), (0.8, 0.55))
    p.line((0.5, 0.12), (0.5, 0.66), w=1.2)
    p.line((0.32, 0.48), (0.5, 0.66), (0.68, 0.48), w=1.2)


@_icon("LOOP_BACK", "UNDO")
def _undo(p):
    p.arc(0.54, 0.54, 0.28, 150, -110, w=1.1)
    p.poly((0.12, 0.3), (0.36, 0.24), (0.26, 0.48))


@_icon("LOOP_FORWARDS", "REDO")
def _redo(p):
    p.arc(0.46, 0.54, 0.28, 30, 290, w=1.1)
    p.poly((0.88, 0.3), (0.64, 0.24), (0.74, 0.48))


@_icon("SORTALPHA")
def _sort(p):
    for i, w in enumerate((0.3, 0.45, 0.6)):
        p.line((0.2, 0.3 + i * 0.2), (0.2 + w, 0.3 + i * 0.2), w=1.1)


@_icon("FILTER")
def _filter(p):
    p.poly((0.12, 0.18), (0.88, 0.18), (0.58, 0.52), (0.58, 0.84), (0.42, 0.76), (0.42, 0.52))


@_icon("HOME")
def _home(p):
    p.poly((0.5, 0.12), (0.88, 0.48), (0.12, 0.48))
    p.rect(0.24, 0.48, 0.52, 0.38)


@_icon("FULLSCREEN_ENTER")
def _fullscreen(p):
    for (u, v), (du, dv) in (((0.16, 0.16), (1, 1)), ((0.84, 0.16), (-1, 1)),
                             ((0.16, 0.84), (1, -1)), ((0.84, 0.84), (-1, -1))):
        p.line((u, v + 0.24 * dv), (u, v), (u + 0.24 * du, v), w=1.1)


# -- playback ------------------------------------------------------------------------------


@_icon("PLAY")
def _play(p):
    p.poly((0.3, 0.16), (0.3, 0.84), (0.82, 0.5))


@_icon("PAUSE")
def _pause(p):
    p.rect(0.26, 0.18, 0.16, 0.64)
    p.rect(0.58, 0.18, 0.16, 0.64)


@_icon("REW")
def _rew(p):
    p.rect(0.16, 0.2, 0.1, 0.6)
    p.poly((0.82, 0.2), (0.82, 0.8), (0.3, 0.5))


@_icon("FF")
def _ff(p):
    p.rect(0.74, 0.2, 0.1, 0.6)
    p.poly((0.18, 0.2), (0.18, 0.8), (0.7, 0.5))


@_icon("REC")
def _rec(p):
    p.disc(0.5, 0.5, 0.3, color=(0.9, 0.25, 0.25, 1.0))


# -- data / editors ------------------------------------------------------------------------


@_icon("PREFERENCES", "SETTINGS", "TOOL_SETTINGS")
def _gear(p):
    for i in range(8):
        a = math.radians(i * 45)
        p.line((0.5 + 0.22 * math.cos(a), 0.5 + 0.22 * math.sin(a)),
               (0.5 + 0.4 * math.cos(a), 0.5 + 0.4 * math.sin(a)), w=1.8)
    p.ring(0.5, 0.5, 0.24, w=1.6)


@_icon("MODIFIER", "PROPERTIES")
def _wrench(p):
    p.line((0.22, 0.8), (0.58, 0.44), w=2.0)
    p.ring(0.66, 0.34, 0.16, w=1.6)


@_icon("OBJECT_DATA", "OBJECT_DATAMODE")
def _object(p):
    p.poly((0.5, 0.12), (0.86, 0.32), (0.86, 0.7), (0.5, 0.9), (0.14, 0.7), (0.14, 0.32))
    k = (0.0, 0.0, 0.0, 0.45)
    p.line((0.14, 0.32), (0.5, 0.52), (0.86, 0.32), color=k)
    p.line((0.5, 0.52), (0.5, 0.9), color=k)


@_icon("MESH_CUBE", "CUBE", "MESH_DATA", "EDITMODE_HLT")
def _cube(p):
    p.line((0.5, 0.12), (0.86, 0.32), (0.86, 0.7), (0.5, 0.9), (0.14, 0.7), (0.14, 0.32),
           (0.5, 0.12))
    p.line((0.14, 0.32), (0.5, 0.52), (0.86, 0.32))
    p.line((0.5, 0.52), (0.5, 0.9))


@_icon("LIGHT", "LIGHT_DATA", "OUTLINER_OB_LIGHT")
def _light(p):
    p.disc(0.5, 0.42, 0.22)
    p.rect(0.4, 0.66, 0.2, 0.18)


@_icon("MATERIAL", "SHADING_RENDERED", "NODE_MATERIAL")
def _material(p):
    p.disc(0.5, 0.5, 0.36)
    p.disc(0.4, 0.4, 0.1, color=(1.0, 1.0, 1.0, 0.7))


@_icon("WORLD", "WORLD_DATA")
def _world(p):
    p.ring(0.5, 0.5, 0.36)
    p.ring(0.5, 0.5, 0.16, ry=0.36)
    p.line((0.14, 0.5), (0.86, 0.5))


@_icon("TEXTURE", "IMAGE_DATA", "IMAGE")
def _texture(p):
    for i in range(3):
        for j in range(3):
            if (i + j) % 2 == 0:
                p.rect(0.16 + i * 0.23, 0.16 + j * 0.23, 0.23, 0.23)
    p.frame(0.16, 0.16, 0.69, 0.69, 0.04)


@_icon("SCENE_DATA")
def _scene(p):
    p.disc(0.3, 0.66, 0.14)
    p.poly((0.5, 0.18), (0.82, 0.82), (0.5, 0.82))
    p.rect(0.12, 0.84, 0.76, 0.06)


@_icon("COLLECTION", "OUTLINER_COLLECTION", "GROUP")
def _collection(p):
    p.frame(0.16, 0.24, 0.68, 0.56, 0.1)
    p.line((0.16, 0.42), (0.84, 0.42))


@_icon("OUTLINER")
def _outliner(p):
    for i, d in enumerate((0, 0.14, 0.14, 0)):
        v = 0.22 + i * 0.19
        p.rect(0.16 + d, v - 0.05, 0.1, 0.1)
        p.line((0.32 + d, v), (0.84, v))


@_icon("CONSTRAINT")
def _constraint(p):
    p.ring(0.32, 0.5, 0.18, ry=0.12)
    p.ring(0.68, 0.5, 0.18, ry=0.12)
    p.line((0.4, 0.5), (0.6, 0.5), w=1.3)


@_icon("PHYSICS")
def _physics(p):
    p.ring(0.5, 0.5, 0.38, ry=0.14)
    p.disc(0.5, 0.5, 0.14)


@_icon("PARTICLES")
def _particles(p):
    for u, v, r in ((0.3, 0.3, 0.1), (0.7, 0.36, 0.08), (0.42, 0.7, 0.12), (0.76, 0.74, 0.07)):
        p.disc(u, v, r)


@_icon("CURVE_DATA", "CURVE_BEZCURVE")
def _curve(p):
    pts = []
    for i in range(13):
        t = i / 12
        pts.append((0.14 + 0.72 * t, 0.7 - 0.5 * math.sin(math.pi * t) * (1 - 0.3 * t)))
    p.line(*pts, w=1.1)


@_icon("NODE", "NODETREE")
def _node(p):
    p.box(0.1, 0.16, 0.34, 0.3, 0.06)
    p.box(0.56, 0.54, 0.34, 0.3, 0.06)
    p.line((0.44, 0.31), (0.5, 0.31), (0.5, 0.69), (0.56, 0.69))


@_icon("BRUSH_DATA", "SCULPTMODE_HLT")
def _brush(p):
    p.line((0.8, 0.14), (0.42, 0.56), w=1.6)
    p.poly((0.42, 0.5), (0.5, 0.6), (0.3, 0.86), (0.14, 0.86), (0.18, 0.7))


@_icon("SNAP_ON")
def _magnet(p):
    p.arc(0.5, 0.48, 0.26, 180, 360, w=2.2)
    p.line((0.24, 0.48), (0.24, 0.18), w=2.2)
    p.line((0.76, 0.48), (0.76, 0.18), w=2.2)


@_icon("PROP_ON")
def _proportional(p):
    p.ring(0.5, 0.5, 0.36)
    p.ring(0.5, 0.5, 0.2)
    p.disc(0.5, 0.5, 0.07)


@_icon("VIEW_CAMERA")
def _view_camera(p):
    p.frame(0.14, 0.24, 0.72, 0.52, 0.06)
    p.poly((0.4, 0.38), (0.4, 0.62), (0.62, 0.5))


@_icon("TIME", "PREVIEW_RANGE")
def _clock(p):
    p.ring(0.5, 0.5, 0.36)
    p.line((0.5, 0.28), (0.5, 0.5), (0.66, 0.6))


@_icon("TEXT", "FONT_DATA")
def _text(p):
    p.line((0.2, 0.2), (0.8, 0.2), w=1.3)
    p.line((0.5, 0.2), (0.5, 0.84), w=1.3)
