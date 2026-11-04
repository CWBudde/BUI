"""Blender component library demo (bui.components).

    python examples/components_demo.py

Every component, laid out like Blender's properties editor. Try it like
Blender:

* drag number fields / sliders (cursor hides and never hits the edge;
  Shift = precise, Ctrl = snap, Alt = all of X/Y/Z), click to type (``2*pi``)
* Ctrl+Wheel over any value, Ctrl+C / Ctrl+V, Backspace resets, Minus negates,
  right-click any value for its context menu
* drag across checkboxes or panel headers; drag a panel's grip to reorder;
  Ctrl+click a header to solo it
* File/Edit menus: hover to switch, type to search, submenus; Q opens a pie
* color swatches open the HSV wheel (or square) with an eyedropper
* the side tabs on the left switch pages; hover them for names

SPDX-License-Identifier: GPL-2.0-or-later
"""

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "bindings", "python"))

from bui.components import (App, Box, Button, Checkbox, ColorRamp, ColorSwatch, Column,  # noqa: E402
                            CurveMapping, Decorator, Dropdown, EnumRow, Grid, GridView,
                            Histogram, HotkeyField, IconButton, IDSelector, Label, ListView,
                            MenuBar, MenuItem, NormalBall, NumberField, NumberSlider, Panel,
                            PathField, PieMenu, Popover, ProgressBar, Row, ScrollView,
                            SearchField, Separator, SideTabs, Split, Tabs, TextField, Toggle,
                            TreeItem, TreeView, VectorField)


def page(*children):
    """A properties-editor page: padded column of panels."""
    return Column(*children, padding=8, spacing=2)  # UI_PANEL_MARGIN_X / _Y: 0.4 / 0.1 units


def prop(label, component, decorate=True):
    """Blender property row: right-aligned label | value | keyframe dot."""
    right = Row(component, Decorator(weight=0)) if decorate else component
    return Split(Label(label, align="right"), right, factor=0.4)


# ---------------------------------------------------------------------------
# Pages


def object_page(status, progress):
    transform = Panel(
        "Transform",
        VectorField("Location", (0.0, 1.25, 0.0), suffix=" m",
                    description="Location of the object\nAlt+drag edits X, Y and Z together"),
        VectorField("Rotation", (0.0, 0.0, 90.0), suffix="°", step=5,
                    description="Rotation in degrees"),
        prop("Scale", NumberSlider("", 1.0, min=0.0, max=4.0, step=0.1,
                                   description="Uniform scale",
                                   on_change=lambda v: progress.set_value(v / 4))),
        prop("Mode", Dropdown(items=["Object Mode", "Edit Mode", "Sculpt Mode",
                                     "Vertex Paint", "Weight Paint", "Texture Paint"],
                              icons=["OBJECT_DATAMODE", "EDITMODE_HLT", "SCULPTMODE_HLT",
                                     "BRUSH_DATA", "BRUSH_DATA", "TEXTURE"],
                              description="Interaction mode")),
        icon="OBJECT_DATA",
    )
    visibility = Panel(
        "Visibility",
        Row(Toggle("", True, icon="RESTRICT_SELECT_OFF", description="Selectable"),
            Toggle("", True, icon="HIDE_OFF", description="Show in viewports"),
            Toggle("", True, icon="RESTRICT_RENDER_OFF", description="Show in renders"),
            align=True),
        Grid(Checkbox("Camera", True), Checkbox("Diffuse", True), Checkbox("Glossy", True),
             Checkbox("Transmission", True), Checkbox("Volume Scatter", False),
             Checkbox("Shadow", True), columns=2),
        Panel("Mask", Checkbox("Shadow Catcher"), Checkbox("Holdout")),
        Panel("Motion Blur", prop("Steps", NumberField("", 1, min=1, max=7)),
              header=[Checkbox("", True)]),
    )
    viewport = Panel(
        "Viewport Display",
        prop("Display As", EnumRow(items=["Bounds", "Wire", "Solid", "Textured"],
                                   icons=["MESH_CUBE", "SHADING_RENDERED", "MATERIAL",
                                          "TEXTURE"], icon_only=True, value="Textured")),
        prop("Color", ColorSwatch(value=(0.28, 0.45, 0.70, 1.0),
                                  description="Viewport display color")),
        prop("Shading", EnumRow(items=["Flat", "Smooth", "Auto"], value="Smooth")),
        Row(Toggle("X"), Toggle("Y", True), Toggle("Z"), align=True),
        collapsed=True,
    )
    return page(
        IDSelector(["Cube", "Camera", "Light", "Suzanne"], "Cube", icon="OBJECT_DATA"),
        transform, visibility, viewport,
    )


def modifiers_page():
    ramp = ColorRamp(((0.0, (0.05, 0.05, 0.2, 1.0)), (0.45, (0.9, 0.3, 0.1, 1.0)),
                      (1.0, (1.0, 0.95, 0.6, 1.0))), interpolation="Ease")
    curves = CurveMapping(((0.0, 0.0), (0.3, 0.18), (0.7, 0.85), (1.0, 1.0)),
                          channels=("C", "R", "G", "B"))
    subdiv = Panel(
        "Subdivision",
        EnumRow(items=["Catmull-Clark", "Simple"]),
        prop("Levels Viewport", NumberField("", 1, min=0, max=6)),
        prop("Render", NumberField("", 2, min=0, max=6)),
        Checkbox("Optimal Display", True),
        icon="MODIFIER",
    )
    return page(
        Row(Button("Add Modifier", icon="ADD"),
            IconButton("PREFERENCES", description="Modifier options", weight=0)),
        subdiv,
        Panel("Color Ramp", ramp, icon="TEXTURE"),
        Panel("RGB Curves", curves, icon="CURVE_DATA"),
    )


def material_page():
    bins = []
    for i in range(48):
        t = i / 47
        bins.append((0.25 + 0.7 * t * (1 - t) * 4 * abs(__import__("math").sin(t * 7)) * 0.5,
                     0.2 + 0.6 * (1 - abs(t - 0.45) * 2) ** 2,
                     0.15 + 0.5 * t ** 2))
    tiles = GridView([("Metal", (0.7, 0.72, 0.75, 1)), ("Gold", (0.95, 0.75, 0.3, 1)),
                      ("Glass", (0.6, 0.85, 0.95, 1)), ("Rubber", (0.15, 0.15, 0.15, 1)),
                      ("Emission", (1.0, 0.5, 0.2, 1)), ("Wood", (0.5, 0.3, 0.15, 1)),
                      ("World", "WORLD"), ("Node", "NODE")], tile=3.6, rows=2)
    surface = Panel(
        "Surface",
        prop("Base Color", ColorSwatch(value=(0.8, 0.2, 0.15, 1.0))),
        prop("Emission", ColorSwatch(value=(1.0, 0.6, 0.2, 0.5), picker="square",
                                     description="Square picker (SV + Hue)")),
        prop("Metallic", NumberSlider("", 0.0, min=0, max=1, step=0.05)),
        prop("Roughness", NumberSlider("", 0.5, min=0, max=1, step=0.05)),
        prop("IOR", NumberField("", 1.45, min=1.0, max=4.0, step=0.01)),
        icon="MATERIAL",
    )
    return page(
        IDSelector(["Material", "Metal", "Glass"], "Material", icon="MATERIAL"),
        Panel("Preview", tiles),
        surface,
        Panel("Normal", Row(NormalBall("", (0.3, 0.5, 0.8)),
                            Column(Label("Drag the ball to aim"), Label("the light direction."),
                                   weight=2))),
        Panel("Histogram", Histogram(bins)),
    )


def outliner_page(status):
    scene = TreeItem("Scene Collection", icon="SCENE_DATA")
    props = TreeItem("Props", icon="OUTLINER_COLLECTION",
                     toggles={"hide": False, "render": False})
    for name, icon in (("Cube", "MESH_CUBE"), ("Suzanne", "MESH_CUBE"),
                       ("Plane", "MESH_CUBE")):
        props.add(TreeItem(name, TreeItem(name + " Mesh", icon="MESH_DATA"),
                           TreeItem("Material", icon="MATERIAL"), icon="OBJECT_DATA",
                           expanded=False))
    scene.add(TreeItem("Camera", icon="CAMERA_DATA"), TreeItem("Light", icon="LIGHT"), props)
    tree = TreeView([scene], rows=9, columns=("hide", "select", "render"),
                    on_change=lambda it: status.show("Selected " + it.label if it else ""))

    names = ["Cube", "Camera", "Light", "Suzanne", "Plane", "Cylinder", "Empty", "Torus"]
    lst = ListView(names, rows=5, icons=["OBJECT_DATA"] * len(names))

    def filt(text):
        tree.roots = [scene] if not text else [
            it for it in _walk(scene) if text.lower() in it.label.lower()]

    return page(
        SearchField(on_change=filt, description="Filter the tree"),
        tree,
        Panel("UI List", lst, Row(
            IconButton("ADD", lambda: setattr(lst, "items", lst.items + [
                "Object.%03d" % len(lst.items)]), emboss=True, weight=0),
            IconButton("REMOVE", lambda: setattr(lst, "items", lst.items[:-1]),
                       emboss=True, weight=0))),
    )


def _walk(item):
    for c in item.children:
        yield c
        yield from _walk(c)


def prefs_page(status):
    many = ScrollView(Column(*[Checkbox("Add-on %02d" % i, i % 3 == 0) for i in range(24)]),
                      rows=5)
    return page(
        Panel("Input",
              prop("Search", HotkeyField("", "F3"), decorate=False),
              prop("Render", HotkeyField("", "F12"), decorate=False),
              prop("Emulate", EnumRow(items=["Off", "3 Button", "Numpad"], vertical=True),
                   decorate=False),
              icon="PREFERENCES"),
        Panel("File Paths",
              prop("Fonts", PathField("", "//fonts/", directory=True), decorate=False),
              prop("Textures", PathField("", "//textures/", directory=True), decorate=False),
              prop("Temp", TextField(placeholder="Temporary directory"), decorate=False)),
        Panel("Add-ons", SearchField(), many),
        Panel("Interface",
              Popover("Viewport Options",
                      Checkbox("Show Gizmos", True), Checkbox("Show Overlays", True),
                      NumberSlider("Opacity", 0.8), icon="PREFERENCES"),
              Tabs([("Resolution", Box(prop("Scale", NumberSlider("", 1.0, min=0.5, max=2),
                                            decorate=False))),
                    ("Text", Box(prop("Hinting", Dropdown(items=["Auto", "None", "Slight",
                                                                 "Full"]), decorate=False)))]),
              Row(Button("Save Preferences", icon="FILE",
                         on_click=lambda: status.show("Preferences saved")),
                  Button("Disabled", enabled=False))),
    )


# ---------------------------------------------------------------------------
# Window


class Status(Label):
    def show(self, text):
        self.text = text


def build() -> App:
    status = Status("Q: pie menu · right-click values · drag panel grips", align="left",
                    icon="INFO")
    progress = ProgressBar(0.25, "Rendering 25%")

    def say(text):
        return lambda: status.show(text)

    recent = [MenuItem("/projects/%s.blend" % n, say("Open " + n)) for n in
              ("robot", "forest", "studio")]
    menubar = MenuBar(
        ("File", [MenuItem("New", say("New file"), icon="FILE", shortcut="Ctrl N"),
                  MenuItem("Open...", say("Open"), icon="FILE_FOLDER", shortcut="Ctrl O"),
                  MenuItem("Open Recent", submenu=recent, shortcut="Shift Ctrl O"),
                  "-",
                  MenuItem("Save", say("Saved"), icon="FILE", shortcut="Ctrl S"),
                  MenuItem("Save As...", say("Save as"), shortcut="Shift Ctrl S"),
                  MenuItem("Auto Save", checked=True),
                  "-",
                  MenuItem("Import", submenu=[MenuItem("glTF 2.0"), MenuItem("FBX"),
                                              MenuItem("Wavefront (.obj)")], icon="IMPORT"),
                  MenuItem("Export", submenu=[MenuItem("glTF 2.0"), MenuItem("USD")],
                           icon="EXPORT"),
                  "-",
                  MenuItem("Quit", say("Quit"), shortcut="Ctrl Q")]),
        ("Edit", [MenuItem("Undo", say("Undo"), icon="LOOP_BACK", shortcut="Ctrl Z"),
                  MenuItem("Redo", say("Redo"), icon="LOOP_FORWARDS", shortcut="Shift Ctrl Z"),
                  "-",
                  MenuItem("Lock Object Modes", checked=False),
                  MenuItem("Preferences...", say("Preferences"), icon="PREFERENCES")]),
        ("Render", [MenuItem("Render Image", say("Render"), icon="RENDER_STILL",
                             shortcut="F12"),
                    MenuItem("Render Animation", say("Animation"), shortcut="Ctrl F12"),
                    MenuItem("View Render", enabled=False, shortcut="F11")]),
        ("Help", [MenuItem("Manual", icon="INFO"), MenuItem("Report a Bug", icon="ERROR")]),
        right=[IconButton("PLAY", say("Play"), description="Play animation"),
               IconButton("SEARCH", say("Search"), description="Search (F3)")],
    )

    pages = SideTabs([
        ("OBJECT_DATA", "Object", object_page(status, progress)),
        ("MODIFIER", "Modifiers", modifiers_page()),
        ("MATERIAL", "Material", material_page()),
        ("OUTLINER", "Outliner", outliner_page(status)),
        ("PREFERENCES", "Preferences", prefs_page(status)),
    ])

    app = App("BUI — Blender components", (520, 900), pages,
              Column(progress, status, padding=8, spacing=4),
              header=menubar, padding=0, spacing=0)

    def pie():
        PieMenu.show(app.ctx, [
            MenuItem("Object Mode", say("Object Mode"), icon="OBJECT_DATAMODE"),
            MenuItem("Edit Mode", say("Edit Mode"), icon="EDITMODE_HLT"),
            MenuItem("Sculpt Mode", say("Sculpt Mode"), icon="SCULPTMODE_HLT"),
            MenuItem("Vertex Paint", say("Vertex Paint"), icon="BRUSH_DATA"),
            MenuItem("Weight Paint", say("Weight Paint"), icon="BRUSH_DATA"),
            MenuItem("Texture Paint", say("Texture Paint"), icon="TEXTURE"),
        ], "Mode")

    app.bind("q", pie)
    app.bind("ctrl+s", say("Saved"))
    return app


if __name__ == "__main__":
    build().run()
