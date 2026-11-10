"""Write THIRD_PARTY_NOTICES.md: the licenses of everything BUI's packages include.

    python tools/gen_notices.py            # (re)write the file
    python tools/gen_notices.py --check    # exit 1 when the file is out of date (CI, releases)

Three parts, all read from the repository so the file cannot drift from the code:

1. The C and C++ libraries and the assets vendored in the tree (Wuffs, libjpeg-turbo, libwebp,
   FreeType, Inter, Dear ImGui, ...): their own license files, copied verbatim.
2. What only the Go binding brings (wazero, xgb and X11's keysym tables, purego on macOS, and
   the wasi-sdk runtime inside bui_kit.wasm): the texts in bindings/go/licenses, copied there
   from the pinned sources.
3. The crates that the Rust crates, the Node addon and the QuickJS runtime are built from
   (`cargo metadata`, normal dependencies only): name, version, license, and each distinct
   license text found in the crate's source, once, with the crates that use it.

BUI itself is GPL-3.0-or-later (see LICENSE); the text of that license is not repeated here.
"""

import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RUST = ROOT / "bindings" / "rust"
OUT = ROOT / "THIRD_PARTY_NOTICES.md"
# the crates whose dependencies end up in what BUI ships
SHIPPED = ("blenderui", "blenderui-imgui", "blenderui-node", "blenderui-qjs")

MIT = """MIT License

Copyright (c) {holder}

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and
associated documentation files (the "Software"), to deal in the Software without restriction,
including without limitation the rights to use, copy, modify, merge, publish, distribute,
sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or
substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT
NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT
OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE."""

ZLIB = """zlib License

Copyright (c) {holder}

This software is provided 'as-is', without any express or implied warranty. In no event will the
authors be held liable for any damages arising from the use of this software.

Permission is granted to anyone to use this software for any purpose, including commercial
applications, and to alter it and redistribute it freely, subject to the following restrictions:

1. The origin of this software must not be misrepresented; you must not claim that you wrote the
   original software. If you use this software in a product, an acknowledgment in the product
   documentation would be appreciated but is not required.
2. Altered source versions must be plainly marked as such, and must not be misrepresented as being
   the original software.
3. This notice may not be removed or altered from any source distribution."""


def file_text(rel: str) -> str:
    return (ROOT / rel).read_text(encoding="utf-8", errors="replace").replace("\r\n", "\n").strip("\n")


def stb_text() -> str:
    """stb_truetype ends with its two license alternatives; keep them."""
    text = file_text("gpu/extern/stb_truetype.h")
    return text[text.index("ALTERNATIVE A - MIT License"):].strip()


# (name and version, where it is used, license, source, license text)
VENDORED = [
    ("Wuffs 0.4", "PNG, GIF, BMP, TGA and QOI decoding (kit/extern/wuffs)",
     "MIT or Apache-2.0 (used as MIT)", "https://github.com/google/wuffs",
     lambda: file_text("kit/extern/wuffs/LICENSE")),
    ("libjpeg-turbo 3.1.2", "JPEG decoding (kit/extern/libjpeg-turbo)",
     "IJG, BSD-3-Clause and zlib", "https://libjpeg-turbo.org",
     lambda: file_text("kit/extern/libjpeg-turbo/LICENSE.md")),
    ("libwebp 1.6.0", "WebP decoding, including animation (kit/extern/libwebp)",
     "BSD-3-Clause", "https://chromium.googlesource.com/webm/libwebp",
     lambda: file_text("kit/extern/libwebp/COPYING")),
    ("FreeType 2.13.3", "Glyph loading, hinting and rendering (gpu/extern/freetype)",
     "FreeType License (FTL) or GPL-2.0-or-later; BUI uses it under the FTL, which is compatible with GPL-3.0",
     "https://freetype.org",
     lambda: "Portions of this software are copyright (c) 2002-2024 The FreeType Project (www.freetype.org). "
             "All rights reserved.\n\nThis software is based in part on the work of the FreeType Team. The FreeType "
             "License is at https://freetype.org/license.html.\n\n" + file_text("gpu/extern/freetype/LICENSE.TXT")),
    ("Inter (font)", "The embedded UI font, as in Blender (gpu/extern)",
     "SIL Open Font License 1.1", "https://rsms.me/inter",
     lambda: file_text("gpu/extern/Inter-LICENSE.txt")),
    ("Dear ImGui 1.93.1", "The immediate-mode GUI that runs on BUI (bindings/imgui/extern/imgui)",
     "MIT", "https://github.com/ocornut/imgui",
     lambda: file_text("bindings/imgui/extern/imgui/LICENSE.txt")),
    ("stb_truetype 1.26", "Font metrics and character maps, and the fallback rasterizer (gpu/extern)",
     "MIT or public domain", "https://github.com/nothings/stb", stb_text),
    ("NanoSVG, as edited by Blender", "Rasterizing Blender's SVG icons (kit/extern/nanosvg)",
     "zlib", "https://github.com/memononen/nanosvg",
     lambda: ZLIB.format(holder="2013-14 Mikko Mononen <memon@inside.org>")),
    ("Slug reference shaders", "GPU glyph coverage (gpu/intern/gpu_gl.c, gpu_d3d11.c)",
     "MIT", "https://github.com/EricLengyel/Slug",
     lambda: MIT.format(holder="2017 by Eric Lengyel")),
    ("Tailwind CSS v4 theme values", "The default theme behind the Tailwind support (blenderui-tailwind)",
     "MIT", "https://tailwindcss.com",
     lambda: MIT.format(holder="Tailwind Labs, Inc.")),
    ("Lua 5.4.7", "The Lua runtime embedded in bui-lua (bindings/lua, fetched from lua.org)",
     "MIT", "https://www.lua.org",
     lambda: MIT.format(holder="1994-2024 Lua.org, PUC-Rio")),
    ("linenoise 1.0", "REPL line editing (bindings/lua/extern/linenoise)",
     "BSD-2-Clause", "https://github.com/antirez/linenoise",
     lambda: (lambda t: t[:t.index("*/")].strip())(file_text("bindings/lua/extern/linenoise/linenoise.h"))),
]


def go_texts(*names: str) -> str:
    """Texts from bindings/go/licenses, each under a heading with its file name when there are several."""
    texts = [file_text(f"bindings/go/licenses/{n}") for n in names]
    if len(texts) == 1:
        return texts[0]
    return "\n\n".join(f"=== {n} ===\n\n{t}" for n, t in zip(names, texts))


LLVM_LICENSE = "Apache-2.0 WITH LLVM-exception, and older NCSA and MIT"
LLVM_SOURCE = "https://github.com/llvm/llvm-project"

# What a program built with the Go binding (bindings/go) contains beyond the kit and the tree's
# parts above: (name and version, where it ends up, license, source, license text)
GO_BINDING = [
    ("wazero 1.12.0", "Runs bui_kit.wasm: the program, as a Go module", "Apache-2.0",
     "https://github.com/tetratelabs/wazero", lambda: go_texts("wazero.txt", "wazero-NOTICE.txt")),
    ("golang.org/x/sys 0.44.0", "CPU features and system calls for wazero: the program, as a Go module",
     "BSD-3-Clause", "https://pkg.go.dev/golang.org/x/sys", lambda: go_texts("x-sys.txt")),
    ("xgb 1.3.1", "The X protocol for the X11 host, on Linux with the wasm engine: the program, as a Go module", "BSD-3-Clause",
     "https://github.com/jezek/xgb", lambda: go_texts("xgb.txt")),
    ("X11's keysym tables", "Keysym names and characters in bindings/go/internal/host/x11/keysyms_gen.go, "
     "generated from keysymdef.h, XF86keysym.h and Sunkeysym.h: the program, on Linux with the wasm engine", "MIT-style (X11)",
     "https://gitlab.freedesktop.org/xorg/proto/xorgproto", lambda: go_texts("x11-keysyms.txt")),
    ("purego 0.11.3", "Calls AppKit for the Cocoa window host: the program on macOS, as a Go module",
     "Apache-2.0, and BSD-3-Clause for the parts from Go's runtime", "https://github.com/ebitengine/purego",
     lambda: go_texts("purego.txt")),
    ("wasi-libc, from wasi-sdk 34.0", "The C library in bui_kit.wasm",
     "Apache-2.0 WITH LLVM-exception, Apache-2.0 or MIT", "https://github.com/WebAssembly/wasi-libc",
     lambda: go_texts("wasi-libc.txt", "wasi-libc-LICENSE-APACHE-LLVM.txt", "wasi-libc-LICENSE-APACHE.txt",
                      "wasi-libc-LICENSE-MIT.txt")),
    ("musl, in wasi-libc", "The C library in bui_kit.wasm", "MIT", "https://musl.libc.org",
     lambda: go_texts("wasi-libc-musl.txt")),
    ("cloudlibc, in wasi-libc", "The C library in bui_kit.wasm", "BSD-2-Clause",
     "https://github.com/NuxiNL/cloudlibc", lambda: go_texts("wasi-libc-cloudlibc.txt")),
    ("dlmalloc, in wasi-libc", "malloc in bui_kit.wasm", "CC0, by the notice in dlmalloc/src/malloc.c (lines 1-5)",
     "https://github.com/WebAssembly/wasi-libc/tree/main/dlmalloc", lambda: go_texts("wasi-libc-dlmalloc.txt")),
    ("libc++, from wasi-sdk 34.0", "The C++ standard library in bui_kit.wasm", LLVM_LICENSE, LLVM_SOURCE,
     lambda: go_texts("llvm-libcxx.txt")),
    ("libc++abi, from wasi-sdk 34.0", "C++ exceptions and RTTI in bui_kit.wasm", LLVM_LICENSE, LLVM_SOURCE,
     lambda: go_texts("llvm-libcxxabi.txt")),
    ("libunwind, from wasi-sdk 34.0", "Unwinding for C++ exceptions in bui_kit.wasm", LLVM_LICENSE, LLVM_SOURCE,
     lambda: go_texts("llvm-libunwind.txt")),
    ("compiler-rt builtins, from wasi-sdk 34.0", "Compiler helper functions in bui_kit.wasm", LLVM_LICENSE,
     LLVM_SOURCE, lambda: go_texts("llvm-compiler-rt.txt")),
    ("Wayland's xdg-shell and xdg-decoration protocols", "Code wayland-scanner generates into libbui_kit.a, "
     "which the cgo engine (tag bui_cgo) links", "MIT", "https://gitlab.freedesktop.org/wayland/wayland-protocols",
     lambda: go_texts("wayland-protocols-xdg-shell.txt", "wayland-protocols-xdg-decoration.txt")),
    ("Notices inside BUI's own sources", "The kit, on either engine (the last item above says which parts)", "Various, see the text",
     "This repository", lambda: go_texts("bui-source-notices.txt")),
]

GO_NOTES = """A program built with the Go binding (`bindings/go`) contains BUI's kit, and with it the
tree's Wuffs, libjpeg-turbo, libwebp, FreeType, Inter, stb_truetype and NanoSVG above. With the
default wasm engine the kit is `bindings/go/internal/engine/wasmengine/bui_kit.wasm`, built from the
commit in `bindings/go/BUI_COMMIT` with `BUI_GPU=OFF`, so without the Slug shaders; with the cgo
engine (the tag `bui_cgo`) it is the libraries of the tree's own CMake build, which hold them when
`BUI_GPU` is on. It contains none of the Rust crates below, and neither Dear ImGui, Tailwind's theme,
Lua nor linenoise. The table lists what the binding adds.

What is not known, or not covered here:

- wasi-sdk's archive carries none of its license texts, so they come from the sources it was
  built from. The tag `wasi-sdk-34` pins them as submodules, and the archive's own `VERSION` file
  names the same commits: wasi-libc `2e6fb9d8ee0cdf9e431fbcabe8af3115de000a13` and llvm-project
  `895aa2c896ada719451be2e3673c83da8ddf1141`.
- wasi-libc also holds `fts` and `emmalloc`, under their own licenses. Neither is in
  `bui_kit.wasm`, whose name section shows `dlmalloc` and no `fts_open`.
- The Wayland protocol code is generated at build time from the build machine's
  wayland-protocols. The texts here are from wayland-protocols 1.49, which need not be the version
  a given build used. Every Linux build of the kit with `BUI_WAYLAND` contains this code, not only
  the Go binding's.
- The X11 host's keysym tables are generated from the X11 headers of the machine that ran
  `internal/host/x11/gen`. The committed file regenerates byte for byte from xorgproto 2023.2's.
  The text here holds the notices of `keysymdef.h` and of the first notice in `Sunkeysym.h`
  (Oracle); `Sunkeysym.h`'s second notice, The Open Group's of 1991 and 1998, worded as the one
  of `keysymdef.h`, is missing from it, and `XF86keysym.h` carries none.
- purego includes code from Go's own runtime (`runtime/cgo`), under Go's BSD-3-Clause license, as
  its README lists: on macOS without cgo its `internal/fakecgo` stands in for `runtime/cgo`. Go's
  `LICENSE` is the same text as golang.org/x/sys's above. Every macOS program built with the
  binding contains purego, also one that only renders headless.
- A cgo program also links the C and C++ runtime of the machine it is built on
  (`-lstdc++ -ldl -lm -lpthread`), and the kit opens libX11 with `dlopen` at run time, and for a
  Wayland window libwayland-client, libwayland-cursor and libxkbcommon. None of these is in this
  repository.
- The Slug entry above is the standard MIT text with the name from the comment in
  `gpu/intern/gpu_gl.c`. That comment names no upstream revision, so the revision BUI ported is
  not known.
- In BUI's own sources (the last entry): the list merge sort in `core/intern/list_sort_impl.h` has
  MIT wording and no copyright line; the dual quaternion conversion in
  `core/intern/math_rotation_c.c` carries only the SPDX identifier Zlib; and
  `core/include/BLI_build_config.h`, based on Chromium's `build_config.h`, names "a BSD-style
  license" whose text is not in the tree. Of the code these notices cover, `bui_kit.wasm` holds the
  atomic operations from jemalloc, `wcwidth` and FreeType's hash table, while the name section shows
  no symbol from the merge sort, the quaternion conversion or MurmurHash; the cgo engine's libraries
  hold all of them.
"""


def run_metadata() -> dict:
    env = dict(os.environ, RUSTC_WRAPPER="")
    r = subprocess.run(["cargo", "metadata", "--format-version", "1", "--locked"],
                       cwd=RUST, capture_output=True, text=True, env=env)
    if r.returncode != 0:
        sys.exit("cargo metadata failed:\n" + r.stderr[-800:])
    return json.loads(r.stdout)


def shipped_crates(meta: dict) -> list[dict]:
    """The packages (not BUI's own) that the shipped crates depend on, normal dependencies only."""
    by_id = {p["id"]: p for p in meta["packages"]}
    members = set(meta["workspace_members"])
    nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
    start = [i for i in members if by_id[i]["name"] in SHIPPED]
    seen, stack = set(), list(start)
    while stack:
        i = stack.pop()
        if i in seen:
            continue
        seen.add(i)
        for d in nodes[i]["deps"]:
            if any(k["kind"] is None for k in d["dep_kinds"]):  # normal dependency (not build, not dev)
                stack.append(d["pkg"])
    return sorted((by_id[i] for i in seen if i not in members), key=lambda p: (p["name"], p["version"]))


LICENSE_NAMES = ("license", "licence", "copying", "unlicense", "notice")


def license_files(pkg: dict) -> list[Path]:
    root = Path(pkg["manifest_path"]).parent
    found = []
    for base in (root, *[d for d in sorted(root.iterdir()) if d.is_dir() and d.name not in ("target", "tests", "benches", "examples", "src")]):
        for f in sorted(base.iterdir()):
            if f.is_file() and f.stem.lower().split("-")[0] in LICENSE_NAMES and f.suffix.lower() in ("", ".md", ".txt"):
                found.append(f)
    return found


def render() -> str:
    out = ["""# Third-party notices

BUI is licensed under the GNU General Public License, version 3 or later (see `LICENSE`), like
Blender, which it is derived from. This file lists the other software and assets that BUI's
packages (the Python wheels, the npm packages, the Rust crates and the prebuilt runtimes) and
programs built with its Go binding include, and their licenses. It is generated by
`tools/gen_notices.py`: do not edit it by hand.

## Libraries and assets in the BUI tree

| Part | Used for | License | Source |
| --- | --- | --- | --- |"""]
    for name, use, lic, src, _ in VENDORED:
        out.append(f"| {name} | {use} | {lic} | {src} |")
    out.append("\nThe parts of Blender that BUI is derived from are GPL-2.0-or-later and are covered by `LICENSE`.\n")
    for name, _, lic, _, text in VENDORED:
        out.append(f"### {name}\n\nLicense: {lic}\n\n````text\n{text()}\n````\n")

    out.append("""## The Go binding

""" + GO_NOTES + """
| Part | Where it ends up | License | Source |
| --- | --- | --- | --- |""")
    for name, use, lic, src, _ in GO_BINDING:
        out.append(f"| {name} | {use} | {lic} | {src} |")
    out.append("")
    for name, _, lic, _, text in GO_BINDING:
        out.append(f"### {name}\n\nLicense: {lic}\n\n````text\n{text()}\n````\n")

    meta = run_metadata()
    crates = shipped_crates(meta)
    out.append("""## Rust crates

The crates that the Rust crates, the Node.js addon and the QuickJS runtime are built from (normal
dependencies; build-time and test-only dependencies are left out). The license of each is listed
here; every distinct license text found in the crates' sources follows, with the crates that use it.

| Crate | Version | License | Source |
| --- | --- | --- | --- |""")
    texts: dict[str, tuple[str, list[str]]] = {}
    for p in crates:
        out.append(f"| {p['name']} | {p['version']} | {p.get('license') or 'see the crate'} | {p.get('repository') or 'https://crates.io/crates/' + p['name']} |")
        files = license_files(p)
        for f in files:
            body = f.read_text(encoding="utf-8", errors="replace").replace("\r\n", "\n").strip("\n")
            key = hashlib.sha256(body.encode()).hexdigest()
            texts.setdefault(key, (body, []))[1].append(f"{p['name']} {p['version']}")
        if not files and "MIT" in (p.get("license") or ""):
            # the published crate carries no license file: the standard MIT text, with the crate's authors
            holder = ", ".join(a.split("<")[0].strip() for a in p.get("authors") or []) or f"the authors of {p['name']}"
            body = ("(The crate declares the MIT license and does not include the text; this is the standard MIT "
                    "text with the crate's authors.)\n\n" + MIT.format(holder=holder))
            key = hashlib.sha256(body.encode()).hexdigest()
            texts.setdefault(key, (body, []))[1].append(f"{p['name']} {p['version']}")
    out.append("")
    for body, users in sorted(texts.values(), key=lambda t: (t[1][0], len(t[0]))):
        out.append(f"### Used by {', '.join(users)}\n\n````text\n{body}\n````\n")
    return "\n".join(out).rstrip("\n") + "\n"


def main() -> int:
    text = render()
    if "--check" in sys.argv:
        current = OUT.read_text(encoding="utf-8").replace("\r\n", "\n") if OUT.exists() else ""
        if current != text:
            print("THIRD_PARTY_NOTICES.md is out of date: run `python tools/gen_notices.py`")
            return 1
        print("THIRD_PARTY_NOTICES.md is up to date")
        return 0
    OUT.write_text(text, encoding="utf-8", newline="\n")
    print(f"wrote {OUT.name}: {len(VENDORED)} vendored parts, {len(GO_BINDING)} Go binding parts, {text.count('### Used by')} distinct crate license texts, {len(text) // 1024} KB")
    return 0


if __name__ == "__main__":
    sys.exit(main())
