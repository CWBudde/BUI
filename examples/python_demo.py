"""BUI Python example: render the demo scene to an SVG file.

Works headless (CI-friendly). For a live window, see bui/qt.py or run:

    python -c "import sys; from PySide6.QtWidgets import QApplication; \
               import buipy; from bui.qt import BuiCanvas; \
               app=QApplication(sys.argv); c=BuiCanvas(); c.resize(800,600); \
               c.show(); sys.exit(app.exec())"

SPDX-License-Identifier: GPL-2.0-or-later
"""

import glob
import os
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
# The buipy extension lives in the build tree; the 'bui' package in the source
# tree. BUIPY_BUILD_DIR overrides the auto-detection of build*/bindings/python.
_candidates = [os.environ.get("BUIPY_BUILD_DIR", "")]
_candidates += sorted(
    glob.glob(os.path.join(_HERE, "..", "build*", "bindings", "python")), reverse=True
)
_seen = set()
for _p in _candidates:
    if _p and os.path.isdir(_p) and _p not in _seen:
        _seen.add(_p)
        sys.path.insert(0, _p)
sys.path.insert(0, os.path.join(_HERE, "..", "bindings", "python"))

import buipy  # noqa: E402
from bui.svg import SvgSink  # noqa: E402


def main() -> int:
    sink = SvgSink(640, 480)
    buipy.set_backend(sink)
    try:
        buipy.demo_scene(640, 480)
    finally:
        buipy.set_backend(None)

    out = sys.argv[1] if len(sys.argv) > 1 else "bui_python_demo.svg"
    sink.save(out)
    print("SVG written to", out)
    return 0


if __name__ == "__main__":
    sys.exit(main())
