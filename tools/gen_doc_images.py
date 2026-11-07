"""Render the examples into images (headless, no window shown).

    python tools/gen_doc_images.py            # docs/images/*.png (README / docs)
    python tools/gen_doc_images.py --site     # website/public/images/shots/, dark + light,
                                              # every language (see INDEX.md there)
    python tools/gen_doc_images.py --site --only python      # one producer:
          python | rust | js | imgui | c   (--theme dark|light for one theme)

The site shots are 2x (BUI_UI_SCALE=2, crisp on high-DPI screens), in Blender's
dark and light themes (BUI_THEME), and losslessly recompressed (tools/pngopt.py).
Producers outside Python run as subprocesses and are skipped (with a note)
when their toolchain or build is missing:

* rust:  cargo run --release -j 1 -p blenderui --example <name> -- --screenshot
* js:    node examples/site/js/shoot.mjs (needs bindings/node's built addon)
* imgui: the C++ examples from the CMake build (BUI_BUILD_DIR, built with
         BUI_BUILD_EXAMPLES=ON BUI_BUILD_IMGUI=ON) and the Python bindings
* c:     bui_site_kit_app from the same build

Set BUIPY_BUILD_DIR to the build's bindings/python and BUI_BUILD_DIR to the
CMake build directory.

SPDX-License-Identifier: GPL-2.0-or-later
"""
import argparse
import os
import shutil
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DOCS_OUT = os.path.join(ROOT, "docs", "images")
SITE_OUT = os.path.join(ROOT, "website", "public", "images", "shots")
sys.path.insert(0, os.path.join(ROOT, "tools"))
THEMES = ("dark", "light")
SCALE = 2


# --------------------------------------------------------------------------------
# docs/images (1x, dark): the README and docs pages


def docs_images():
    sys.path.insert(0, os.path.join(ROOT, "bindings", "python"))
    sys.path.insert(0, os.path.join(ROOT, "examples"))
    import buipy
    from bui.tksyntax._images import _encode_png

    def save(raster, w, h, name):
        px = b"".join(bytes(raster.pixel(x, y)) + b"\xff" for y in range(h) for x in range(w))
        with open(os.path.join(DOCS_OUT, name), "wb") as fh:
            fh.write(_encode_png(w, h, px))
        print("saved", name, w, h)

    def ctx_shot(ctx, w, h, name):
        r = buipy.Raster(w, h)
        ctx.render(w, h, r, True)
        save(r, w, h, name)

    # 1. components demo
    import components_demo
    app = components_demo.build()
    r = app.render_raster(520, 690, True)
    save(r, 520, 690, "components.png")

    # 2. unchanged tkinter app (+ each tab)
    import bui.tksyntax as tks
    tks.install()
    import tkinter as tk
    import tk_unchanged_app
    for i, name in enumerate(("tkinter_items.png", "tkinter_notes.png", "tkinter_canvas.png")):
        root = tk_unchanged_app.build(tk.Tk())
        nb = [w for w in root.winfo_children() if w.winfo_class() == "TNotebook"][0]
        nb.select(i)
        root.update()
        ctx_shot(root._ctx, *root._window.size, name)
        root.destroy()

    # 3. messagebox
    from tkinter import messagebox
    from bui.tksyntax import _core
    root = tk.Tk()
    tk.Label(root, text=" ").pack()
    root.update()

    def shoot():
        top = [w for w in _core._windows() if w is not root][-1]
        ctx_shot(top._ctx, *top._window.size, "tkinter_messagebox.png")
        for d in top._descendants():
            if isinstance(d, tk.Button) and d.cget("text") == "No":
                d.invoke()
    root.after(150, shoot)
    messagebox.askyesno("Unsaved changes", "Save changes to the inventory before closing?",
                        detail="Your changes will be lost if you don't save them.")
    root.destroy()

    # 4. Qt demo
    import bui.qtsyntax as qt
    import qt_syntax_demo

    def fake_exec(self):
        w = [o for o in qt._custom_or_top()][-1]
        w._window.render_now()
        ctx_shot(w._ctx, w._window.size[0], 222, "qt.png")
        w.close()
        return 0
    qt.QApplication.exec = fake_exec
    qt_syntax_demo.main()


# --------------------------------------------------------------------------------
# website shots


def run(cmd, env=None, cwd=ROOT):
    print("+", " ".join(cmd), flush=True)
    return subprocess.run(cmd, env=env, cwd=cwd).returncode


def child_env(theme):
    env = dict(os.environ, BUI_THEME=theme, BUI_UI_SCALE=str(SCALE))
    env.setdefault("RUSTC_WRAPPER", "")
    return env


def site_python(theme):
    """Every Python shot, in this process (BUI_THEME / BUI_UI_SCALE already set)."""
    sys.path.insert(0, os.path.join(ROOT, "tools"))
    import site_shots_py
    site_shots_py.main(SITE_OUT, theme)


def site_rust(theme, only_example=None):
    rust = os.path.join(ROOT, "bindings", "rust")
    if not shutil.which("cargo"):
        print("rust: cargo not found, skipped")
        return
    shots = [  # (example, output name)
        ("site_dashboard", "rust-rsx-dashboard"),
        ("site_dashboard_builder", "rust-builder-dashboard"),
        ("site_plain_html", "rust-html-plain"),
        ("site_styled", "rust-html-styled"),
        ("site_components", "rust-components"),
        ("site_effects", "feature-html-effects"),
        ("site_grid", "feature-css-grid"),
        ("site_animation", "feature-animation"),
        ("site_data_table", "data-table"),
        ("site_chat", "chat-list"),
        ("site_shadcn_table", "shadcn-data-table"),
        ("site_shadcn_list", "shadcn-virtual-list"),
    ]
    for ex, name in shots:
        if only_example and ex != only_example:
            continue
        out = os.path.join(SITE_OUT, "%s-%s.png" % (name, theme))
        run(["cargo", "run", "--release", "-j", "1", "-q", "-p", "blenderui", "--example", ex,
             "--", "--screenshot", out], env=child_env(theme), cwd=rust)


def site_js(theme):
    node = shutil.which("node")
    addon = [f for f in os.listdir(os.path.join(ROOT, "bindings", "node"))
             if f.endswith(".node")]
    if not node or not addon:
        print("js: node or the built addon (bindings/node: npm run build) missing, skipped")
        return
    run([node, os.path.join(ROOT, "examples", "site", "js", "shoot.mjs"), SITE_OUT, theme],
        env=child_env(theme))


def site_native(theme, which):
    """C / C++ examples from the CMake build (BUI_BUILD_DIR)."""
    build = os.environ.get("BUI_BUILD_DIR", os.path.join(ROOT, "build"))
    exes = {
        "imgui": [("bui_site_imgui", "imgui-cpp-stock", ["--stock"]),
                  ("bui_site_imgui", "imgui-cpp-blender", [])],
        "c": [("bui_site_kit_app", "c-kit-app", [])],
    }[which]
    for exe, name, args in exes:
        path = None
        for sub in ("examples/Release", "examples", "bindings/imgui/Release", "bindings/imgui"):
            p = os.path.join(build, sub, exe + (".exe" if os.name == "nt" else ""))
            if os.path.exists(p):
                path = p
                break
        if not path:
            print("%s: %s not built in %s, skipped" % (which, exe, build))
            continue
        out = os.path.join(SITE_OUT, "%s-%s.png" % (name, theme))
        run([path, "--screenshot", out] + args, env=child_env(theme))


def site_lua(theme):
    """Lua shots: run bui-lua headless to render the table editor."""
    build = os.environ.get("BUI_BUILD_DIR", os.path.join(ROOT, "build"))
    exe = os.path.join(build, "bindings", "lua", "bui-lua" + (".exe" if os.name == "nt" else ""))
    if not os.path.exists(exe):
        print("lua: bui-lua not built in %s, skipped" % build)
        return
    script = os.path.join(ROOT, "bindings", "lua", "examples", "editor_screenshot.lua")
    import subprocess, tempfile
    for name in ("lua-editor",):
        out_ppm = os.path.join(SITE_OUT, "%s-%s.ppm" % (name, theme))
        run([exe, script, out_ppm], env=child_env(theme))
        if os.path.exists(out_ppm):
            import pngopt
            with open(out_ppm, "rb") as f:
                header = f.read(15)
            assert header[:3] == b"P6\n"
            dims = header[3:].split(b" " if os.name == "nt" else b" ")
            w, h = int(dims[0]), int(dims[1].split(b"\n")[0])
            rgb = f.read()
            out_png = os.path.join(SITE_OUT, "%s-%s.png" % (name, theme))
            pngopt.write_png(out_png, w, h, rgb)
            os.remove(out_ppm)
            print("  ->", out_png)


def site(only, themes, example=None):
    os.makedirs(SITE_OUT, exist_ok=True)
    before = set(os.listdir(SITE_OUT))
    for theme in themes:
        if only in (None, "python"):
            # a fresh interpreter per theme: the theme and scale are set before
            # the first window exists
            run([sys.executable, os.path.abspath(__file__), "--site-python", theme],
                env=child_env(theme))
        if only in (None, "rust"):
            site_rust(theme, example)
        if only in (None, "js"):
            site_js(theme)
        if only in (None, "imgui"):
            site_native(theme, "imgui")
        if only in (None, "c"):
            site_native(theme, "c")
        if only in (None, "lua"):
            site_lua(theme)
    # lossless recompression of what other toolchains wrote
    import pngopt
    for f in sorted(os.listdir(SITE_OUT)):
        if f.endswith(".png") and any(f.endswith("-%s.png" % t) for t in themes):
            p = os.path.join(SITE_OUT, f)
            n = pngopt.recompress(p)
            flag = "  (over 300 KB)" if n > 300 * 1024 else ""
            print("%-48s %7d bytes%s" % (f, n, flag))
    new = sorted(set(os.listdir(SITE_OUT)) - before)
    if new:
        print("new files:", ", ".join(new))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--site", action="store_true")
    ap.add_argument("--only", choices=("python", "rust", "js", "imgui", "c", "lua"))
    ap.add_argument("--theme", choices=THEMES)
    ap.add_argument("--example", help="with --only rust: just this example (e.g. site_data_table)")
    ap.add_argument("--site-python", choices=THEMES, help=argparse.SUPPRESS)
    a = ap.parse_args()
    if a.site_python:
        site_python(a.site_python)
    elif a.site:
        site(a.only, [a.theme] if a.theme else list(THEMES), a.example)
    else:
        docs_images()


if __name__ == "__main__":
    main()
