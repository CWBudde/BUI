"""Generate kit/intern/kit_theme_presets.c from Blender's interface theme presets.

    py -3 tools/gen_theme.py [BLENDER_SCRIPTS_PRESETS_DIR]

Blender ships its themes as XML presets (scripts/presets/interface_theme/
Blender_Dark.xml, Blender_Light.xml). Loading a preset first resets the theme
to the built-in defaults (userdef_default_theme.c, which is what Blender_Dark
.xml amounts to: it is empty) and then applies the XML on top, so this script
does the same: it starts from BUI's copy of the defaults (the dark table
below, identical to what kit_draw.c shipped before themes became switchable)
and overlays each preset's values onto the fields of BKTheme
(kit/intern/kit_intern.h). Stdlib only. Without an argument the presets are
looked up in ../geometry-nodes/geometry-nodes/blender/scripts/presets and,
failing that, the BLENDER_PRESETS environment variable.

SPDX-License-Identifier: GPL-2.0-or-later
"""
from __future__ import annotations

import os
import sys
import xml.etree.ElementTree as ET

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "kit", "intern", "kit_theme_presets.c")

# BKTheme's widget color sets, in struct order, and the ThemeUserInterface
# element each one is read from.
WCOLS = [
    ("regular", "wcol_regular"), ("tool", "wcol_tool"), ("toggle", "wcol_toggle"),
    ("radio", "wcol_radio"), ("option", "wcol_option"), ("textfield", "wcol_text"),
    ("num", "wcol_num"), ("numslider", "wcol_numslider"), ("menu", "wcol_menu"),
    ("pulldown", "wcol_pulldown"), ("menu_back", "wcol_menu_back"),
    ("menu_item", "wcol_menu_item"), ("list_item", "wcol_list_item"), ("box", "wcol_box"),
    ("scroll", "wcol_scroll"), ("progress", "wcol_progress"), ("tooltip", "wcol_tooltip"),
    ("tab", "wcol_tab"), ("pie_menu", "wcol_pie_menu"), ("curve", "wcol_curve"),
]
WCOL_COLORS = ["outline", "outline_sel", "inner", "inner_sel", "item", "text", "text_sel"]

# The built-in (dark) defaults as BUI draws them: outline, outline_sel, inner,
# inner_sel, item, text, text_sel, roundness. shaded/shadetop/shadedown are 0
# (Blender 5.x's default theme draws every widget flat).
DARK_WCOL = {
    "regular": (0x3d3d3dff, 0x3d3d3dff, 0x545454ff, 0x4772b3ff, 0x1d1d1d80, 0xe6e6e6ff, 0xffffffff, 0.2),
    "tool": (0x3d3d3dff, 0x3d3d3dff, 0x545454ff, 0x4772b3ff, 0xffffffff, 0xe6e6e6ff, 0xffffffff, 0.2),
    "toggle": (0x3d3d3dff, 0x3d3d3dff, 0x545454ff, 0x4772b3ff, 0x252525ff, 0xe6e6e6ff, 0xffffffff, 0.2),
    "radio": (0x3d3d3dff, 0x3d3d3dff, 0x545454ff, 0x4772b3ff, 0x252525ff, 0xe6e6e6ff, 0xffffffff, 0.2),
    "option": (0x3d3d3dff, 0x3d3d3dff, 0x545454ff, 0x4772b3ff, 0xffffffff, 0xe6e6e6ff, 0xffffffff, 0.2),
    "textfield": (0x3d3d3dff, 0x3d3d3dff, 0x1d1d1dff, 0x181818ff, 0xffffff33, 0xe6e6e6ff, 0xffffffff, 0.2),
    "num": (0x3d3d3dff, 0x3d3d3dff, 0x545454ff, 0x222222ff, 0x4772b3ff, 0xe6e6e6ff, 0xffffffff, 0.2),
    "numslider": (0x3d3d3dff, 0x3d3d3dff, 0x545454ff, 0x222222ff, 0x4772b3ff, 0xe6e6e6ff, 0xffffffff, 0.2),
    "menu": (0x3d3d3dff, 0x3d3d3dff, 0x282828ff, 0x4772b3b3, 0xd9d9d9ff, 0xe6e6e6ff, 0xffffffff, 0.2),
    "pulldown": (0x3d3d3d00, 0x3d3d3d00, 0x22222200, 0xffffff1a, 0xffffff8f, 0xd9d9d9ff, 0xffffffff, 0.2),
    "menu_back": (0x242424ff, 0x242424ff, 0x181818ff, 0x4772b3ff, 0xd9d9d9ff, 0x999999ff, 0xffffffff, 0.2),
    "menu_item": (0x3d3d3d00, 0x3d3d3d00, 0x18181800, 0x4772b3ff, 0xffffff8f, 0xddddddff, 0xffffffff, 0.2),
    "list_item": (0x2d2d2d00, 0x2d2d2dff, 0xffffff00, 0x4772b3ff, 0xffffff33, 0xccccccff, 0xffffffff, 0.2),
    "box": (0x3d3d3dff, 0x3d3d3dff, 0x1d1d1d80, 0x545454ff, 0x191919ff, 0xe6e6e6ff, 0xffffffff, 0.2),
    "scroll": (0x3d3d3dff, 0x3d3d3dff, 0x22222200, 0xffffffff, 0x545454ff, 0xe6e6e6ff, 0xffffffff, 0.5),
    "progress": (0x3d3d3dff, 0x3d3d3dff, 0x222222ff, 0x4772b3ff, 0x4772b3ff, 0xe6e6e6ff, 0xffffffff, 0.2),
    "tooltip": (0x242424ff, 0x242424ff, 0x1d1d1dff, 0x4772b3ff, 0xd9d9d9ff, 0xd9d9d9ff, 0xffffffff, 0.2),
    "tab": (0x1d1d1dff, 0x1d1d1dff, 0x1d1d1dff, 0x303030ff, 0x1d1d1dff, 0x989898ff, 0xffffffff, 0.2),
    "pie_menu": (0x242424ff, 0x242424ff, 0x181818ff, 0x4772b3ff, 0x545454ff, 0xd9d9d9ff, 0xffffffff, 0.2),
    "curve": (0x727272ff, 0x9a9a9aff, 0x545454ff, 0x000000ff, 0x00000059, 0x08080800, 0xc9c9c900, 0.2),
}

# Plain colors: (BKTheme field, default, XML source, opaque). The source is a
# path of element tags below <Theme> ending in an attribute; None derives the
# value (see derived()). `opaque` drops the alpha (Blender ignores it for
# editor backgrounds).
COLORS = [
    ("region_bg", 0x303030ff, "properties/ThemeProperties/space/ThemeSpaceGeneric@back", True),
    ("header_bg", 0x181818ff, None, True),  # the top bar: header over back
    ("tabs_bg", 0x181818ff, "regions/ThemeRegions/sidebars/ThemeRegionsSidebars@tab_back", True),
    ("panel_header", 0x3d3d3dff, "user_interface/ThemeUserInterface@panel_header", False),
    ("panel_back", 0x3d3d3dff, "user_interface/ThemeUserInterface@panel_back", False),
    ("panel_sub_back", 0x0000001f, "user_interface/ThemeUserInterface@panel_sub_back", False),
    ("panel_outline", 0xffffff11, "user_interface/ThemeUserInterface@panel_outline", False),
    ("text", 0xe6e6e6ff, "properties/ThemeProperties/space/ThemeSpaceGeneric@text", False),
    ("text_hi", 0xffffffff, "properties/ThemeProperties/space/ThemeSpaceGeneric@text_hi", False),
    ("emboss", 0x00000026, "user_interface/ThemeUserInterface@widget_emboss", False),
    ("cursor", 0x71a8ffff, "user_interface/ThemeUserInterface@widget_text_cursor", False),
    ("selection", 0x4772b380, None, False),  # wcol_regular.inner_sel at 50%
]
SCALARS = [
    ("menu_shadow_fac", 0.2, "user_interface/ThemeUserInterface@menu_shadow_fac", "f"),
    ("menu_shadow_width", 6, "user_interface/ThemeUserInterface@menu_shadow_width", "i"),
    ("panel_roundness", 0.4, "user_interface/ThemeUserInterface@panel_roundness", "f"),
]
FILE_COLORS = [
    ("file_back", 0x282828ff, "file_browser/ThemeFileBrowser/space/ThemeSpaceGeneric@back", True),
    ("file_text", 0xe6e6e6ff, "file_browser/ThemeFileBrowser/space/ThemeSpaceGeneric@text", False),
    ("file_header", 0x303030b3, "file_browser/ThemeFileBrowser/space/ThemeSpaceGeneric@header", False),
    ("file_row_alternate", 0xffffff04, "file_browser/ThemeFileBrowser@row_alternate", False),
]
NODE = "node_editor/ThemeNodeEditor"
NODE_COLORS = [
    ("node_back", 0x1a1a1aff, NODE + "/space/ThemeSpaceGeneric@back", True),
    ("node_grid", 0x303030ff, NODE + "@grid", True),
    ("node_wire", 0x1a1a1aff, NODE + "@wire", False),
    ("node_body", 0x303030ff, NODE + "@node_backdrop", False),
    ("node_outline", 0xffffff26, NODE + "@node_outline", False),
    ("node_selected", 0xed5700ff, NODE + "@node_selected", False),
    ("node_active", 0xffffffff, NODE + "@node_active", False),
]
TOPBAR = "topbar/ThemeTopBar/space/ThemeSpaceGeneric"
PROPS = "properties/ThemeProperties/space/ThemeSpaceGeneric"
# derived: (field, default); computed in build()
EXTRA = [("area_header", 0x303030ff)]  # editor headers: space header over the region back


def parse_color(s: str, default: int) -> int:
    s = s.strip().lstrip("#")
    if len(s) == 6:  # RGB-only properties keep the default's alpha
        return (int(s, 16) << 8) | (default & 0xff)
    return int(s[:8], 16)


def lookup(theme, path: str):
    elem_path, attr = path.split("@")
    e = theme.find(elem_path)
    return None if e is None else e.get(attr)


def over(top: int, back: int) -> int:
    a = (top & 0xff) / 255.0
    out = 0
    for sh in (24, 16, 8):
        t, b = (top >> sh) & 0xff, (back >> sh) & 0xff
        out |= int(round(t * a + b * (1.0 - a))) << sh
    return out | 0xff


def build(theme) -> dict:
    """Defaults + the preset's overrides (theme: the <Theme> element or None)."""
    t = {"wcol": {}}
    for name, xml_name in WCOLS:
        d = DARK_WCOL[name]
        w = {k: d[i] for i, k in enumerate(WCOL_COLORS)}
        w.update(shaded=0, shadetop=0, shadedown=0, roundness=d[7])
        e = theme.find("user_interface/ThemeUserInterface/%s/ThemeWidgetColors" % xml_name) \
            if theme is not None else None
        if e is not None:
            for k in WCOL_COLORS:
                if e.get(k) is not None:
                    w[k] = parse_color(e.get(k), w[k])
            if e.get("show_shaded") is not None:
                w["shaded"] = 1 if e.get("show_shaded").upper() == "TRUE" else 0
            for k in ("shadetop", "shadedown"):
                if e.get(k) is not None:
                    w[k] = int(e.get(k))
            if e.get("roundness") is not None:
                w["roundness"] = float(e.get("roundness"))
        t["wcol"][name] = w
    for field, default, path, opaque in COLORS + FILE_COLORS + NODE_COLORS:
        v = default
        if path and theme is not None and lookup(theme, path) is not None:
            v = parse_color(lookup(theme, path), default)
        t[field] = (v | 0xff) if opaque else v
    for field, default, path, kind in SCALARS:
        v = default
        if theme is not None and lookup(theme, path) is not None:
            v = (float if kind == "f" else int)(lookup(theme, path))
        t[field] = v
    if theme is not None and lookup(theme, TOPBAR + "@header") is not None:
        back = parse_color(lookup(theme, TOPBAR + "@back"), 0xff)
        t["header_bg"] = over(parse_color(lookup(theme, TOPBAR + "@header"), 0xff), back | 0xff)
    t["selection"] = (t["wcol"]["regular"]["inner_sel"] & 0xffffff00) | 0x80
    t["area_header"] = 0x303030ff
    if theme is not None and lookup(theme, PROPS + "@header") is not None:
        t["area_header"] = over(parse_color(lookup(theme, PROPS + "@header"), 0xff),
                                t["region_bg"])
    return t


def c_float(v: float) -> str:
    s = repr(float(v))
    return s + "f"


def emit(name: str, t: dict, const: bool) -> str:
    out = ["%sBKTheme %s = {" % ("const " if const else "", name)]
    for wname, _ in WCOLS:
        w = t["wcol"][wname]
        cols = ", ".join("RGBA(0x%08xu)" % w[k] for k in WCOL_COLORS)
        out.append("    /* %s */ {%s, %d, %d, %d, %s}," % (
            wname, cols, w["shaded"], w["shadetop"], w["shadedown"], c_float(w["roundness"])))
    for field, *_ in COLORS:
        out.append("    RGBA(0x%08xu), /* %s */" % (t[field], field))
    out.append("    %s, /* menu_shadow_fac */" % c_float(t["menu_shadow_fac"]))
    out.append("    %d, /* menu_shadow_width */" % t["menu_shadow_width"])
    out.append("    %s, /* panel_roundness */" % c_float(t["panel_roundness"]))
    for field, *_ in FILE_COLORS + NODE_COLORS + EXTRA:
        out.append("    RGBA(0x%08xu), /* %s */" % (t[field], field))
    out.append("};")
    return "\n".join(out)


def find_presets(argv) -> str:
    cands = argv[1:2] + [
        os.environ.get("BLENDER_PRESETS", ""),
        os.path.join(ROOT, "..", "geometry-nodes", "geometry-nodes", "blender", "scripts",
                     "presets"),
    ]
    for c in cands:
        if c and os.path.isdir(os.path.join(c, "interface_theme")):
            return os.path.join(c, "interface_theme")
        if c and os.path.isfile(os.path.join(c, "Blender_Light.xml")):
            return c
    sys.exit("gen_theme.py: Blender's scripts/presets directory not found (pass it)")


def load(path: str):
    root = ET.parse(path).getroot()
    return root.find("Theme")


def main(argv) -> None:
    d = find_presets(argv)
    themes = [("dark", load(os.path.join(d, "Blender_Dark.xml"))),
              ("light", load(os.path.join(d, "Blender_Light.xml")))]
    built = {name: build(th) for name, th in themes}
    parts = [
        "/* Generated by tools/gen_theme.py from Blender's interface theme presets",
        " * (scripts/presets/interface_theme/Blender_Dark.xml, Blender_Light.xml) over",
        " * the built-in defaults (userdef_default_theme.c). Do not edit.",
        " *",
        " * SPDX-License-Identifier: GPL-2.0-or-later */",
        "",
        '#include "kit_intern.h"',
        "",
        "#define RGBA(v) {(uchar)((v) >> 24), (uchar)((v) >> 16), (uchar)((v) >> 8), (uchar)(v)}",
        "",
        "/* The active theme (starts as Blender's default, dark). */",
        emit("bk_theme", built["dark"], False),
        "",
    ]
    for name, _ in themes:
        parts += [emit("bk_theme_%s" % name, built[name], True), ""]
    text = "\n".join(parts)
    with open(OUT, "w", newline="\n") as fh:
        fh.write(text)
    print("wrote", os.path.relpath(OUT, ROOT))


if __name__ == "__main__":
    main(sys.argv)
