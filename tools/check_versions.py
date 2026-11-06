"""Check that every published package agrees with a release tag before anything is published.

    python tools/check_versions.py v0.2.0

The Python package, the Rust workspace and the Rust crates' path dependencies share one
version (the tag). The npm packages are versioned on their own (the native addon can get a
patch release the Rust crates don't), so they are only listed, not compared; the release
workflow skips an npm version that is already published.
"""

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RUST = ROOT / "bindings" / "rust"


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def npm_packages() -> list[str]:
    """The npm packages, in publish order: the workspaces of bindings/package.json (native first)."""
    return json.loads(read(ROOT / "bindings" / "package.json"))["workspaces"]


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__)
        return 2
    want = sys.argv[1].removeprefix("v")
    found: dict[str, str] = {}

    found["bindings/python/bui/_version.py"] = re.search(
        r'__version__\s*=\s*"([^"]+)"', read(ROOT / "bindings/python/bui/_version.py")).group(1)
    found["bindings/rust/Cargo.toml [workspace.package]"] = re.search(
        r'^version\s*=\s*"([^"]+)"', read(RUST / "Cargo.toml"), re.M).group(1)

    # the published crates name their siblings with an exact version
    for crate in ("blenderui", "blenderui-macros", "blenderui-imgui", "blenderui-table", "blenderui-vlist", "blenderui-paint"):
        text = read(RUST / crate / "Cargo.toml")
        for dep, ver in re.findall(r'^(blenderui[\w-]*)\s*=\s*\{[^}]*version\s*=\s*"([^"]+)"', text, re.M):
            found[f"bindings/rust/{crate}/Cargo.toml: {dep}"] = ver

    bad = {k: v for k, v in found.items() if v != want}
    for k, v in found.items():
        print(f"{'ok ' if v == want else 'BAD'} {v:8} {k}")
    for pkg in npm_packages():
        meta = json.loads(read(ROOT / "bindings" / pkg / "package.json"))
        print(f"npm {meta['version']:8} {meta['name']} (own version; published only if new)")
    if bad:
        print(f"\nThe tag says {want}, but {len(bad)} place(s) disagree.")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
