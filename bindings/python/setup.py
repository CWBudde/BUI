"""pip packaging for bui + buipy.

Builds the native extension with CMake first, then packages the `bui` package
together with the compiled buipy extension so `pip install` just works:

    cmake -S . -B build -G "MinGW Makefiles"
    cmake --build build --target buipy
    python -m pip install bindings/python

The extension is interpreter-specific; build it with the same interpreter you
pip-install into (see INTEGRATION.md, "Testing the Python bindings").

SPDX-License-Identifier: GPL-2.0-or-later
"""

import glob
import os
import shutil
import sys

from setuptools import setup

HERE = os.path.dirname(os.path.abspath(__file__))


def locate_extension() -> str:
    """Find the built buipy extension matching THIS interpreter's ABI tag
    (e.g. buipy.cp312-win_amd64.pyd), from BUIPY_BUILD_DIR or ../build*."""
    tag = "cp%d%d" % sys.version_info[:2]
    candidates = [os.environ.get("BUIPY_BUILD_DIR", "")]
    candidates += sorted(
        glob.glob(os.path.join(HERE, "..", "..", "build*", "bindings", "python")),
        reverse=True,
    )
    for cand in candidates:
        if not cand:
            continue
        for ext in ("*.pyd", "*.so"):
            # Prefer an exact ABI match; fall back to any buipy extension.
            hits = [h for h in glob.glob(os.path.join(cand, "buipy" + ext)) if tag in h]
            hits = hits or glob.glob(os.path.join(cand, "buipy" + ext))
            if hits:
                return hits[0]
    return ""


def stage_extension() -> str:
    src = locate_extension()
    if not src:
        print(
            "error: no built buipy extension found. Build it first:\n"
            "  cmake -S . -B build && cmake --build build --target buipy\n"
            "or set BUIPY_BUILD_DIR.",
            file=sys.stderr,
        )
        sys.exit(1)
    dest = os.path.join(HERE, "bui", os.path.basename(src))
    shutil.copy2(src, dest)
    print("staged native extension:", dest)
    return os.path.basename(src)


ext_name = stage_extension()

setup(
    name="bui",
    version="0.1.0",
    description="BUI — Blender-derived UI toolkit with Qt/tkinter-style Python syntax",
    packages=["bui"],
    package_data={"bui": [ext_name]},
    include_package_data=True,
    python_requires=">=3.10",
    license="GPL-2.0-or-later",
)
