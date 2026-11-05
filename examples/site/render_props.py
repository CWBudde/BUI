"""Blender components: a render-settings editor (Blender's Properties editor).

    python examples/site/render_props.py

Also provides the feature shots for the website (tools/gen_doc_images.py
--site): the File menu with a search typed, the mode pie menu, a tooltip and
the color picker all open on top of this window.

SPDX-License-Identifier: GPL-2.0-or-later
"""

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..",
                                "bindings", "python"))

from bui.components import (App, Button, Checkbox, ColorSwatch, Column, Decorator,  # noqa: E402
                            Dropdown, EnumRow, IconButton, Label, MenuBar, MenuItem,
                            NumberField, NumberSlider, Panel, PieMenu, ProgressBar, Row,
                            SideTabs, Split, Toggle, VectorField)


def prop(label, component, decorate=True):
    """A Blender property row: right-aligned label | value | keyframe dot."""
    right = Row(component, Decorator(weight=0)) if decorate else component
    return Split(Label(label, align="right"), right, factor=0.4)


def page(*children):
    return Column(*children, padding=8, spacing=2)


def render_page(status):
    denoise = Dropdown(items=["OpenImageDenoise", "OptiX"],
                       description="Denoise the final render with AI denoising")
    status.denoise = denoise
    sampling = Panel(
        "Sampling",
        Panel("Viewport",
              prop("Noise Threshold", NumberSlider("", 0.1, min=0, max=1, step=0.01)),
              prop("Max Samples", NumberField("", 1024, min=1, max=65536)),
              header=[Checkbox("", True)]),
        Panel("Render",
              prop("Noise Threshold", NumberSlider("", 0.01, min=0, max=1, step=0.001)),
              prop("Max Samples", NumberField("", 4096, min=1, max=65536)),
              prop("Time Limit", NumberField("", 0, suffix=" s", min=0)),
              prop("Denoise", denoise),
              header=[Checkbox("", True)]),
        icon="RENDER_STILL",
    )
    light_paths = Panel(
        "Light Paths",
        prop("Total", NumberField("", 12, min=0, max=1024)),
        prop("Diffuse", NumberField("", 4, min=0, max=1024)),
        prop("Glossy", NumberField("", 4, min=0, max=1024)),
        prop("Transmission", NumberField("", 12, min=0, max=1024)),
        Row(Checkbox("Caustics Reflective", True), Checkbox("Refractive", True)),
        collapsed=False,
    )
    background = ColorSwatch(value=(0.16, 0.38, 0.62, 1.0), description="World background color")
    status.background = background
    color = Panel(
        "Color Management",
        prop("View", Dropdown(items=["AgX", "Filmic", "Standard", "Khronos PBR Neutral"])),
        prop("Look", Dropdown(items=["Medium High Contrast", "None", "Punchy"])),
        prop("Exposure", NumberSlider("", 0.35, min=-10, max=10, step=0.05)),
        prop("Background", background),
        collapsed=False,
    )
    return page(
        Split(Label("Render Engine", align="right"),
              Dropdown(items=["Cycles", "EEVEE", "Workbench"],
                       icons=["SHADING_RENDERED", "SHADING_RENDERED", "SHADING_SOLID"]),
              factor=0.4),
        prop("Device", EnumRow(items=["CPU", "GPU Compute"], value="GPU Compute"), False),
        sampling, light_paths, color,
    )


def output_page():
    return page(
        Panel("Format",
              VectorField("Resolution", (1920, 1080), axes="XY", suffix=" px",
                          step=1),
              prop("Scale", NumberSlider("", 100, min=1, max=100, suffix=" %", step=1)),
              prop("Frame Rate", Dropdown(items=["24 fps", "25 fps", "30 fps", "60 fps"])),
              icon="OUTPUT"),
        Panel("Output",
              prop("File Format", Dropdown(items=["PNG", "OpenEXR", "JPEG", "FFmpeg Video"])),
              prop("Color", EnumRow(items=["BW", "RGB", "RGBA"], value="RGBA"), False),
              prop("Color Depth", EnumRow(items=["8", "16"], value="16"), False)),
    )


def build(size=(500, 640)) -> App:
    status = Label("Rendering frame 42 / 120  ·  Time 00:01:37", align="left", icon="INFO")
    progress = ProgressBar(0.62, "Path Tracing Sample 2540/4096")

    def say(text):
        return lambda: setattr(status, "text", text)

    menubar = MenuBar(
        ("File", [MenuItem("New", say("New"), icon="FILE", shortcut="Ctrl N"),
                  MenuItem("Open...", say("Open"), icon="FILE_FOLDER", shortcut="Ctrl O"),
                  MenuItem("Open Recent", submenu=[MenuItem("robot_arm.blend"),
                                                   MenuItem("forest.blend")],
                           shortcut="Shift Ctrl O"),
                  "-",
                  MenuItem("Save", say("Saved"), icon="FILE", shortcut="Ctrl S"),
                  MenuItem("Save As...", shortcut="Shift Ctrl S"),
                  "-",
                  MenuItem("Import", submenu=[MenuItem("glTF 2.0 (.glb/.gltf)"),
                                              MenuItem("FBX (.fbx)"),
                                              MenuItem("Wavefront (.obj)")], icon="IMPORT"),
                  MenuItem("Export", submenu=[MenuItem("glTF 2.0 (.glb/.gltf)"),
                                              MenuItem("Universal Scene Description"),
                                              MenuItem("Stanford PLY (.ply)")],
                           icon="EXPORT"),
                  MenuItem("Export Image", icon="IMAGE_DATA"),
                  "-",
                  MenuItem("Quit", shortcut="Ctrl Q")]),
        ("Edit", [MenuItem("Undo", icon="LOOP_BACK", shortcut="Ctrl Z"),
                  MenuItem("Redo", icon="LOOP_FORWARDS", shortcut="Shift Ctrl Z"),
                  "-", MenuItem("Preferences...", icon="PREFERENCES")]),
        ("Render", [MenuItem("Render Image", say("Render"), icon="RENDER_STILL",
                             shortcut="F12"),
                    MenuItem("Render Animation", icon="RENDER_ANIMATION",
                             shortcut="Ctrl F12"),
                    "-", MenuItem("View Render", shortcut="F11")]),
        ("Window", [MenuItem("New Window"), MenuItem("Toggle Fullscreen")]),
        ("Help", [MenuItem("Manual", icon="HELP")]),
        right=[IconButton("PLAY", description="Play animation"),
               IconButton("SEARCH", description="Search (F3)")],
    )
    def stub(title, *rows):
        return page(Panel(title, *rows))

    pages = SideTabs([
        ("TOOL_SETTINGS", "Tool", stub("Options", Checkbox("Auto Merge", False))),
        ("SCENE", "Render", render_page(status)),
        ("OUTPUT", "Output", output_page()),
        ("RENDERLAYERS", "View Layer", stub("View Layer", Checkbox("Use for Rendering", True))),
        ("SCENE_DATA", "Scene", stub("Scene", prop("Camera", Dropdown(items=["Camera"])))),
        ("WORLD", "World", stub("Surface", prop("Color", ColorSwatch(value=(0.05, 0.05, 0.05, 1))))),
        ("OBJECT_DATA", "Object", stub("Transform", VectorField("Location", (0.0, 0.0, 1.2)))),
        ("MODIFIER", "Modifiers", stub("Modifiers", Button("Add Modifier", icon="ADD"))),
        ("PARTICLES", "Particles", stub("Particles", Checkbox("Enabled"))),
        ("PHYSICS", "Physics", stub("Physics", Checkbox("Enabled"))),
        ("MATERIAL", "Material", stub("Surface", prop("Base Color", ColorSwatch(
            value=(0.8, 0.3, 0.2, 1))))),
    ], current=1)
    app = App("Render Properties", size, pages,
              Column(progress, status, padding=8, spacing=4),
              header=menubar, padding=0, spacing=0)
    app.menubar, app.status, app.progress = menubar, status, progress
    app.denoise, app.background = status.denoise, status.background

    def pie():
        PieMenu.show(app.ctx, [
            MenuItem("Object Mode", say("Object Mode"), icon="OBJECT_DATAMODE"),
            MenuItem("Edit Mode", say("Edit Mode"), icon="EDITMODE_HLT"),
            MenuItem("Sculpt Mode", say("Sculpt Mode"), icon="SCULPTMODE_HLT"),
            MenuItem("Vertex Paint", say("Vertex Paint"), icon="VPAINT_HLT"),
            MenuItem("Weight Paint", say("Weight Paint"), icon="WPAINT_HLT"),
            MenuItem("Texture Paint", say("Texture Paint"), icon="TPAINT_HLT"),
            MenuItem("Pose Mode", say("Pose Mode"), icon="POSE_HLT"),
            MenuItem("Draw Mode", say("Draw Mode"), icon="GREASEPENCIL"),
        ], "Mode")

    app.pie = pie
    app.bind("q", pie)
    return app


if __name__ == "__main__":
    build().run()
