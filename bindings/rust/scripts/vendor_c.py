"""Copy the C/C++/Objective-C sources blenderui-sys builds into the crate (`csrc/`),
so the published crate is self-contained. Run before `cargo package` / `cargo publish`:

    python bindings/rust/scripts/vendor_c.py

`csrc/` is git-ignored; inside the repo, build.rs uses the repo itself.

What build.rs configures (BUI_BUILD_TESTS / EXAMPLES / PYTHON_BINDINGS / UI / QT_BACKEND
off) needs: the top CMakeLists.txt (+ the version in bindings/python/bui/_version.py),
core, types, gpu (with FreeType and the embedded Inter font), compat, kit (every window
host: Win32, X11, Cocoa .m, null; Blender's C++ in kit/intern/blender, the theme presets,
the file browser), api, ui/CMakeLists.txt (an interface target when BUI_BUILD_UI is off)
and, for the `imgui` feature, bindings/imgui (Dear ImGui, its C API, the Blender widgets
and tests/bui_png.h, which the C API includes). Test directories, generators and caches
are left out. The script checks that every source the CMake files name is present.
"""

import re
import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
DEST = ROOT / "bindings" / "rust" / "blenderui-sys" / "csrc"
DIRS = ["core", "types", "gpu", "compat", "kit", "api", "bindings/imgui"]
FILES = ["CMakeLists.txt", "features.json", "features.cmake", "LICENSE", "bindings/python/bui/_version.py", "ui/CMakeLists.txt"]
# relative directories (from ROOT) never needed by the crate's CMake configuration
SKIP_DIRS = {"core/tests", "gpu/tests", "bindings/imgui/gen"}
SKIP_NAMES = shutil.ignore_patterns("__pycache__", "*.pyc", "*.py", "build*", "*.o", "*.obj")
# header names in CMake files that are no paths: <bui_ftoption.h> includes, find_path
NOT_FILES = {"bui_ftoption.h", "bui_ftmodule.h", "X11/Xlib.h"}


def ignore(src: str, names: list[str]) -> set[str]:
    rel = Path(src).resolve().relative_to(ROOT).as_posix()
    skipped = set(SKIP_NAMES(src, names))
    skipped |= {n for n in names if f"{rel}/{n}" in SKIP_DIRS}
    return skipped


def check_sources() -> list[str]:
    """Every path a vendored CMakeLists.txt names (intern/x.c, ${FT}/src/..., ...) exists."""
    missing = []
    for cm in DEST.rglob("CMakeLists.txt"):
        text = cm.read_text(encoding="utf-8", errors="replace")
        text = re.sub(r"#[^\n]*", "", text)
        ft = cm.parent / "extern" / "freetype"
        imgui = cm.parent / "extern" / "imgui"
        for tok in re.findall(r"[\w${}/.+-]+\.(?:c|cc|cpp|m|mm|h|hh)\b", text):
            if "CMAKE_SOURCE_DIR" in tok or tok.startswith(("tests/", "bench/")) and cm.parent == DEST:
                continue  # examples/, tests/, bench/ of the top directory: only with tests/examples on
            path = (tok.replace("${CMAKE_CURRENT_SOURCE_DIR}/", "")
                       .replace("${FT}", str(ft)).replace("${IMGUI_DIR}", str(imgui)))
            if "$" in path or tok.startswith(("/FI", "-include")) or tok in NOT_FILES:
                continue
            p = Path(path) if Path(path).is_absolute() else cm.parent / path
            # ui/: its sources only build with BUI_BUILD_UI (off for the crate)
            if not p.exists() and cm.parent != DEST / "ui":
                missing.append(str(p.relative_to(DEST)))
    return missing


def main() -> None:
    if DEST.exists():
        shutil.rmtree(DEST)
    for d in DIRS:
        shutil.copytree(ROOT / d, DEST / d, ignore=ignore)
    for f in FILES:
        (DEST / f).parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(ROOT / f, DEST / f)
    size = sum(p.stat().st_size for p in DEST.rglob("*") if p.is_file())
    print(f"vendored {size / 1e6:.1f} MB of C sources into {DEST}")
    missing = check_sources()
    if missing:
        sys.exit("missing from csrc/ (add them to vendor_c.py):\n  " + "\n  ".join(missing))


if __name__ == "__main__":
    main()
