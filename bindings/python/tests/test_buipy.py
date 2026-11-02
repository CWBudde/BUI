"""End-to-end test: BUI -> sink dispatch -> Python backend -> SVG output.

Run standalone (PYTHONPATH must contain the built buipy extension and the
bindings/python source dir) or via ctest.

SPDX-License-Identifier: GPL-2.0-or-later
"""

import os
import sys
import tempfile

# Make both the built extension and this package importable when run by hand.
_here = os.path.dirname(os.path.abspath(__file__))
_build = os.environ.get("BUIPY_BUILD_DIR", os.path.join(os.path.dirname(_here), "..", "..", "build", "bindings", "python"))
for p in (_build, os.path.dirname(_here)):
    if os.path.isdir(p) and p not in sys.path:
        sys.path.insert(0, p)

import buipy  # noqa: E402
from bui.svg import SvgSink  # noqa: E402


def main() -> int:
    assert buipy.version().startswith("0."), buipy.version()

    sink = SvgSink(640, 480)
    buipy.set_backend(sink)
    try:
        buipy.demo_scene(640, 480)
    finally:
        buipy.set_backend(None)

    svg = sink.to_svg()
    assert "<polygon" in svg, "expected triangles in SVG output"
    assert "<line" in svg, "expected lines in SVG output"
    assert "<circle" in svg, "expected points in SVG output"

    # A clean double-install and clear round-trip must not crash.
    buipy.set_backend(SvgSink(320, 240))
    buipy.demo_scene(320, 240)
    buipy.set_backend(None)

    out = os.path.join(tempfile.gettempdir(), "bui_py_test.svg")
    with open(out, "w", encoding="utf-8") as f:
        f.write(svg)

    print("buipy test OK; SVG written to", out)
    return 0


if __name__ == "__main__":
    sys.exit(main())
