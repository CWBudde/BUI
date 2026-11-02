"""Run a tkinter program on BUI, unchanged: ``python -m bui.tksyntax app.py [args]``.

SPDX-License-Identifier: GPL-2.0-or-later
"""

import os
import runpy
import sys

from bui import tksyntax


def main() -> int:
    if len(sys.argv) < 2:
        sys.stderr.write("usage: python -m bui.tksyntax script.py [args...]\n")
        return 2
    script = sys.argv[1]
    sys.argv = sys.argv[1:]
    sys.path.insert(0, os.path.dirname(os.path.abspath(script)))
    tksyntax.install()
    runpy.run_path(script, run_name="__main__")
    return 0


if __name__ == "__main__":
    sys.exit(main())
