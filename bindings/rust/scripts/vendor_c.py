"""Copy the C sources blenderui-sys builds into the crate (`csrc/`), so the
published crate is self-contained. Run before `cargo package` / `cargo publish`:

    python bindings/rust/scripts/vendor_c.py

`csrc/` is git-ignored; inside the repo, build.rs uses the repo itself.
"""

import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
DEST = ROOT / "bindings" / "rust" / "blenderui-sys" / "csrc"
# bindings/imgui: Dear ImGui on BUI + its C API (blenderui-sys's `imgui` feature)
DIRS = ["core", "types", "gpu", "compat", "ui", "kit", "api", "bindings/imgui"]
FILES = ["CMakeLists.txt", "LICENSE", "bindings/python/bui/_version.py"]
SKIP = shutil.ignore_patterns("__pycache__", "*.pyc", "build*", "*.o", "*.obj")


def main() -> None:
    if DEST.exists():
        shutil.rmtree(DEST)
    for d in DIRS:
        shutil.copytree(ROOT / d, DEST / d, ignore=SKIP)
    for f in FILES:
        (DEST / f).parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(ROOT / f, DEST / f)
    size = sum(p.stat().st_size for p in DEST.rglob("*") if p.is_file())
    print(f"vendored {size / 1e6:.1f} MB of C sources into {DEST}")


if __name__ == "__main__":
    main()
