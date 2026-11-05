#!/usr/bin/env python3
"""Keep blender/imgui_widgets_blender.cpp in step with Dear ImGui's imgui_widgets.cpp.

The copy is upstream's imgui_widgets.cpp plus BUI's edits, every one marked "BUI:". To move it
onto a newer Dear ImGui:

  1. copy the new upstream files into extern/imgui (as bindings/imgui/README.md says), but do
     not commit yet: git still holds the old imgui_widgets.cpp, the base of the copy;
  2. py -3 bindings/imgui/blender/rebase.py merge
     a three-way merge (git merge-file): old upstream -> new upstream, applied to the copy.
     Conflicts are left as <<<<<<< markers in the copy; resolve them keeping the BUI: edits;
  3. py -3 bindings/imgui/blender/rebase.py check
     every changed hunk against the (new) upstream file must mention "BUI";
  4. build with -DBUI_IMGUI_BLENDER_WIDGETS=ON and run ctest (imgui_backend,
     imgui_c_api, imgui_blender_widgets_match_kit, the Python test).

Other commands:
  rebase.py diff            the BUI edits: upstream (extern/imgui) vs the copy, unified diff
  rebase.py merge --base F  use file F as the old upstream instead of git's HEAD version

Line endings: the copy keeps upstream's CRLF; comparisons ignore them.
SPDX-License-Identifier: GPL-2.0-or-later
"""

import argparse
import difflib
import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
UPSTREAM = os.path.join(HERE, "..", "extern", "imgui", "imgui_widgets.cpp")
UPSTREAM_GIT = "bindings/imgui/extern/imgui/imgui_widgets.cpp"
COPY = os.path.join(HERE, "imgui_widgets_blender.cpp")


def read_lines(path):
    with open(path, encoding="utf-8", newline="") as f:
        return [l.rstrip("\r\n") for l in f.read().split("\n")]


def cmd_diff(_args):
    a, b = read_lines(UPSTREAM), read_lines(COPY)
    sys.stdout.writelines(l + "\n" for l in difflib.unified_diff(a, b, "imgui_widgets.cpp", "imgui_widgets_blender.cpp", n=2, lineterm=""))
    return 0


def cmd_check(_args):
    a, b = read_lines(UPSTREAM), read_lines(COPY)
    sm = difflib.SequenceMatcher(None, a, b, autojunk=False)
    bad = 0
    hunks = 0
    for tag, i1, i2, j1, j2 in sm.get_opcodes():
        if tag == "equal":
            continue
        hunks += 1
        # the hunk, plus the line before and after it (a "// BUI" may sit on the line above)
        ctx = b[max(0, j1 - 2):min(len(b), j2 + 1)] if j2 > j1 else b[max(0, j1 - 2):min(len(b), j1 + 1)]
        if not any("BUI" in l for l in ctx):
            bad += 1
            print("unmarked change at copy line %d (upstream %d-%d):" % (j1 + 1, i1 + 1, i2))
            for l in a[i1:i2]:
                print("  - " + l)
            for l in b[j1:j2]:
                print("  + " + l)
    print("%d changed hunks, %d without a BUI: mark" % (hunks, bad))
    return 1 if bad else 0


def cmd_merge(args):
    if args.base:
        base = args.base
    else:
        old = subprocess.run(["git", "-C", ROOT, "show", "HEAD:" + UPSTREAM_GIT], capture_output=True, check=True).stdout
        fd, base = tempfile.mkstemp(suffix="_imgui_widgets_base.cpp")
        with os.fdopen(fd, "wb") as f:
            f.write(old)
    r = subprocess.run(["git", "merge-file", "-L", "imgui_widgets_blender.cpp", "-L", "old upstream", "-L", "new upstream", COPY, base, UPSTREAM])
    if r.returncode == 0:
        print("merged cleanly; now: rebase.py check, build with BUI_IMGUI_BLENDER_WIDGETS=ON, ctest")
    elif r.returncode > 0:
        print("%d conflict(s) left as <<<<<<< markers in %s" % (r.returncode, COPY))
    return 0 if r.returncode == 0 else 1


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = p.add_subparsers(dest="cmd", required=True)
    sub.add_parser("diff")
    sub.add_parser("check")
    m = sub.add_parser("merge")
    m.add_argument("--base", help="the old upstream imgui_widgets.cpp (default: git HEAD's)")
    args = p.parse_args()
    return {"diff": cmd_diff, "check": cmd_check, "merge": cmd_merge}[args.cmd](args)


if __name__ == "__main__":
    sys.exit(main())
