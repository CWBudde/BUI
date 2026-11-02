"""SVG recorder backend for BUI — zero dependencies.

Useful for tests, CI snapshots, and documentation figures:

    import buipy, bui
    from bui.svg import SvgSink

    sink = SvgSink(640, 480)
    buipy.set_backend(sink)
    buipy.demo_scene(640, 480)
    open("demo.svg", "w", encoding="utf-8").write(sink.to_svg())

SPDX-License-Identifier: GPL-2.0-or-later
"""

from __future__ import annotations

from bui import Backend


def _hex_color(color) -> str:
    r, g, b, a = (max(0.0, min(1.0, c)) for c in color)
    return "#{:02x}{:02x}{:02x}".format(int(r * 255 + 0.5), int(g * 255 + 0.5), int(b * 255 + 0.5))


class SvgSink(Backend):
    """Collects BUI primitives and renders them as an SVG document."""

    def __init__(self, width: int, height: int):
        self.width = width
        self.height = height
        self._shapes: list[str] = []
        self._clip: tuple[int, int, int, int] | None = None

    # -- Backend protocol ------------------------------------------------
    def begin_frame(self, width: int, height: int) -> None:
        self.width = width
        self.height = height
        self._shapes = []

    def scissor(self, x: int, y: int, w: int, h: int) -> None:
        self._clip = (x, y, w, h) if w > 0 and h > 0 else None

    def triangles(self, coords, colors) -> None:
        for i in range(0, len(coords), 3):
            tri = coords[i : i + 3]
            pts = " ".join("{:.1f},{:.1f}".format(x, y) for x, y in tri)
            self._shapes.append(
                '<polygon points="{}" fill="{}"/>'.format(pts, _hex_color(colors[i]))
            )

    def lines(self, coords, colors) -> None:
        for i in range(0, len(coords), 2):
            (x1, y1), (x2, y2) = coords[i], coords[i + 1]
            self._shapes.append(
                '<line x1="{:.1f}" y1="{:.1f}" x2="{:.1f}" y2="{:.1f}" '
                'stroke="{}" stroke-width="1"/>'.format(x1, y1, x2, y2, _hex_color(colors[i]))
            )

    def points(self, coords, colors, size) -> None:
        for (x, y), color in zip(coords, colors):
            self._shapes.append(
                '<circle cx="{:.1f}" cy="{:.1f}" r="{:.1f}" fill="{}"/>'.format(
                    x, y, size / 2.0, _hex_color(color)
                )
            )

    def text(self, x, y, text, size, color) -> None:
        self._shapes.append(
            '<text x="{:.1f}" y="{:.1f}" font-size="{:.0f}" fill="{}">{}</text>'.format(
                x, y, size, _hex_color(color), text
            )
        )

    # -- Output -----------------------------------------------------------
    def to_svg(self) -> str:
        body = "\n  ".join(self._shapes)
        clip = ""
        if self._clip:
            x, y, w, h = self._clip
            clip = '<clipPath id="clip"><rect x="{}" y="{}" width="{}" height="{}"/></clipPath>'.format(
                x, y, w, h
            )
            return (
                '<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" '
                'viewBox="0 0 {} {}">\n  {}<g clip-path="url(#clip)">\n  {}\n  </g>\n</svg>\n'.format(
                    self.width, self.height, self.width, self.height, clip, body
                )
            )
        return (
            '<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" '
            'viewBox="0 0 {} {}">\n  {}\n</svg>\n'.format(
                self.width, self.height, self.width, self.height, body
            )
        )

    def save(self, path: str) -> None:
        with open(path, "w", encoding="utf-8") as f:
            f.write(self.to_svg())
