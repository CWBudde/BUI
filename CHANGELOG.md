# Changelog

All notable changes to **blenderui** (BUI). The format follows
[Keep a Changelog](https://keepachangelog.com); versions follow
[Semantic Versioning](https://semver.org).

## [Unreleased]

## [0.1.0] - first release

### Added
- **C kit** (`kit/`): Blender's UI as a C runtime. It has the node tree,
  Tk's packer and gridder in C, input and interaction (hover, drag grabs,
  continuous cursor, menus, tooltips), widget drawing ported from Blender's
  `interface_widgets.cc`, and damage-driven raster rendering. Also: undo and
  redo, animation, the text area, canvas, scroll views, scrollbars and
  multi-select.
- **Native window host** for Windows (Win32): DPI-aware, vsync'd
  animation, clipboard, cursors, window-manager features. Other platforms use
  a stdlib-Tk-hosted window.
- **`bui.tksyntax`**: the complete tkinter API (7846/7846 public names of
  the tkinter and ttk classes), plus `ttk`, `messagebox`, `filedialog`,
  `simpledialog`, `colorchooser`, `font` and `scrolledtext`. Run existing apps
  unchanged with `python -m bui.tksyntax app.py`, `blenderui-tk app.py` or
  `install()`.
- **`bui.qtsyntax`**: a Qt-style API subset (widgets, layouts, signals,
  `QPainter`, menus, timers).
- **`bui.components`**: 45 Blender components (layouts, values, views,
  menus, pie menus, the color picker, editors, templates).
- Packaging as `blenderui` (scikit-build-core), with `import blenderui` as
  an alias of `bui`.
