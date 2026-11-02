"""Embed BUI in a Qt application with PySide6 or PyQt6 — pure Python.

The widget installs a QPainter backend into buipy, so everything the BUI
toolkit draws is rendered by Qt. No C++ compilation or shiboken tooling needed.

    import sys
    from PySide6.QtWidgets import QApplication
    import buipy
    from bui.qt import BuiCanvas

    app = QApplication(sys.argv)
    canvas = BuiCanvas()
    canvas.resize(800, 600)
    canvas.show()
    sys.exit(app.exec())

SPDX-License-Identifier: GPL-2.0-or-later
"""

from __future__ import annotations

import buipy
from bui import Backend

try:
    from PySide6.QtCore import QPointF, QRectF, Qt, QTimer
    from PySide6.QtGui import QColor, QFont, QImage, QPainter, QPen, QPolygonF
    from PySide6.QtWidgets import QApplication, QWidget

    QT_BACKEND = "PySide6"
except ImportError:  # pragma: no cover - PyQt6 fallback
    from PyQt6.QtCore import QPointF, QRectF, Qt, QTimer
    from PyQt6.QtGui import QColor, QFont, QImage, QPainter, QPen, QPolygonF
    from PyQt6.QtWidgets import QApplication, QWidget

    QT_BACKEND = "PyQt6"


def _qcolor(color) -> QColor:
    r, g, b, a = (int(max(0.0, min(1.0, c)) * 255 + 0.5) for c in color)
    return QColor(r, g, b, a)


class QtPainterBackend(Backend):
    """Draws BUI primitives with a live QPainter.

    Per-vertex colors are reduced to the first vertex color of each primitive;
    toolkit primitives are solid-color per shape in practice.
    """

    def __init__(self, painter: QPainter):
        self._p = painter

    def triangles(self, coords, colors) -> None:
        painter = self._p
        painter.setPen(Qt.PenStyle.NoPen)
        for i in range(0, len(coords), 3):
            painter.setBrush(_qcolor(colors[i]))
            painter.drawPolygon(
                QPolygonF([QPointF(*coords[i]), QPointF(*coords[i + 1]), QPointF(*coords[i + 2])])
            )

    def lines(self, coords, colors) -> None:
        painter = self._p
        for i in range(0, len(coords), 2):
            painter.setPen(QPen(_qcolor(colors[i]), 1))
            painter.drawLine(int(coords[i][0]), int(coords[i][1]),
                             int(coords[i + 1][0]), int(coords[i + 1][1]))

    def points(self, coords, colors, size) -> None:
        painter = self._p
        painter.setPen(Qt.PenStyle.NoPen)
        d = max(1.0, size)
        for (x, y), color in zip(coords, colors):
            painter.setBrush(_qcolor(color))
            painter.drawEllipse(QRectF(x - d / 2, y - d / 2, d, d))

    def text(self, x, y, text, size, color) -> None:
        painter = self._p
        painter.setPen(QPen(_qcolor(color), 1))
        f = painter.font()
        f.setPixelSize(max(1, int(size)))
        painter.setFont(f)
        painter.drawText(int(x), int(y), text)


class BuiCanvas(QWidget):
    """A QWidget that renders BUI through the native raster.

    Each paint renders the whole UI into one ``buipy.Raster`` frame in C and
    draws it with a single ``QPainter.drawImage`` — the per-primitive
    QPainter path (``QtPainterBackend``) remains available for custom use.

    Subclass (or set ``render_func``) to emit BUI draw calls; the default
    renders the toolkit's demo scene. Event hooks (``on_press``,
    ``on_release``, ``on_motion``, ``on_key``) let hosts forward input into
    the widget model — bui.qtsyntax wires these automatically.
    """

    def __init__(self, parent=None):
        super().__init__(parent)
        self.render_func = None  # Optional[Callable[[], None]]
        self.on_press = None     # Optional[Callable[[int, int], None]]
        self.on_release = None   # Optional[Callable[[int, int], None]]
        self.on_motion = None    # Optional[Callable[[int, int(, bool)], None]]
        self.on_key = None       # Optional[Callable[[str, int], None]]
        self.setMinimumSize(200, 150)
        self.setFocusPolicy(Qt.FocusPolicy.StrongFocus)
        self.setMouseTracking(True)
        # We paint every pixel ourselves: skip Qt's background fill.
        self.setAttribute(Qt.WidgetAttribute.WA_OpaquePaintEvent, True)
        self._raster = buipy.Raster(1, 1)
        self._image_bytes = None  # keeps QImage's buffer alive while painting

    # -- Qt events ---------------------------------------------------------
    def paintEvent(self, event):  # noqa: N802 (Qt naming)
        w, h = max(1, self.width()), max(1, self.height())
        buipy.set_backend(self._raster)
        try:
            buipy.begin_frame(w, h)  # sizes the raster even if render_func doesn't
            if self.render_func is not None:
                self.render_func()
            else:
                buipy.demo_scene(w, h)
            buipy.end_frame()
        finally:
            buipy.set_backend(None)
        rw, rh = self._raster.size
        self._image_bytes = self._raster.rgb32()
        image = QImage(self._image_bytes, rw, rh, rw * 4, QImage.Format.Format_RGB32)
        painter = QPainter(self)
        try:
            painter.drawImage(0, 0, image)
            runs = self._raster.text_runs()
            if runs:  # platforms without a native text engine
                for x, y, text, size, rgba, family, flags in runs:
                    font = QFont(family) if family else QFont()
                    font.setPixelSize(max(1, int(size)))
                    font.setBold(bool(flags & 1))
                    font.setItalic(bool(flags & 2))
                    font.setUnderline(bool(flags & 4))
                    painter.setFont(font)
                    painter.setPen(QPen(_qcolor(rgba), 1))
                    painter.drawText(QPointF(x, y), text)
        finally:
            painter.end()

    def resizeEvent(self, event):  # noqa: N802 (Qt naming)
        buipy.set_viewport(event.size().width(), event.size().height())
        super().resizeEvent(event)

    def mousePressEvent(self, event):  # noqa: N802 (Qt naming)
        if self.on_press:
            self.on_press(event.position().x(), event.position().y())
        super().mousePressEvent(event)

    def mouseReleaseEvent(self, event):  # noqa: N802 (Qt naming)
        if self.on_release:
            self.on_release(event.position().x(), event.position().y())
        super().mouseReleaseEvent(event)

    def mouseMoveEvent(self, event):  # noqa: N802 (Qt naming)
        if self.on_motion:
            held = bool(event.buttons() & Qt.MouseButton.LeftButton)
            try:
                self.on_motion(event.position().x(), event.position().y(), held)
            except TypeError:  # legacy 2-argument hook
                self.on_motion(event.position().x(), event.position().y())
        super().mouseMoveEvent(event)

    def keyPressEvent(self, event):  # noqa: N802 (Qt naming)
        if self.on_key:
            self.on_key(event.text(), event.key())
        super().keyPressEvent(event)

    def request_update(self) -> None:
        """Schedule a repaint; Qt coalesces repeated requests into one."""
        self.update()
