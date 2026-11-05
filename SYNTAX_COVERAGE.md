# Syntax layer coverage

`bui.qtsyntax` and `bui.tksyntax` are thin Python facades over the C kit
(`kit/`, Python: `buipy.KitNode` / `buipy.KitContext`). They own no layout,
drawing or input code:

| Concern | Where it runs |
|---|---|
| Widgets | one kit node each (`label`, `button`, `number`, `slider`, `text`, `textarea`, `list`, `tree`, `tabs`, `canvas`, …) |
| `pack` / `grid` / `place`, Qt box / grid / form layouts | kit C geometry managers (`kit/intern/kit_frame.c`, a port of Tk's packer and gridder) |
| `Canvas` items, `QPainter` | kit canvas display list (rect, round rect, oval, line, polygon, arc, text, image) |
| Input, hover, focus, popups, menus, undo | `BKContext` in C |
| Windows, timers | `bui.host` → native window (`kit/intern/kit_window_win32.c` on Windows, `kit_window_x11.c` on Linux, `kit_window_cocoa.m` on macOS, experimental) or Tk fallback (`BUI_HOST=tk`) |
| Signals, variables, traces, item/tag models | Python (small) |

Anything not listed raises `AttributeError` — nothing fails silently.

## Qt syntax (`bui.qtsyntax`)

| Class | Members |
|---|---|
| `QApplication` | `exec/exec_`, `quit/exit`, `processEvents`, `instance` |
| `QWidget` (+ `QMainWindow`, `QDialog`, `QFrame`) | `show/hide/close/setVisible/isVisible`, `resize/move/setGeometry/setFixedSize/Width/Height/setMinimumWidth/Height`, `width/height/x/y/pos/size/geometry/rect/sizeHint`, `setLayout/layout/setParent/parentWidget/children/window`, `setEnabled/isEnabled`, `setToolTip`, `setStyleSheet`¹, `setFont`, `setSizePolicy`, `setWindowTitle`, `setMenuBar`, `update/repaint`; overridable `paintEvent`, `mousePress/Release/Move/DoubleClickEvent`, `keyPressEvent`, `wheelEvent`, `enter/leaveEvent`, `closeEvent`; `QMainWindow.setCentralWidget/menuBar/statusBar`; `QDialog.exec/accept/reject` |
| Layouts | `QVBoxLayout/QHBoxLayout`: `addWidget(stretch)/addLayout/insertWidget/addStretch/addSpacing/removeWidget/setStretch/setSpacing/setContentsMargins/count/itemAt`; `QGridLayout`: `addWidget/addLayout(r, c, rowspan, colspan, alignment)`, `setRow/ColumnStretch`, `rowCount/columnCount`; `QFormLayout.addRow`; `QStackedLayout`. Qt size policies: line edits, sliders, combos, progress bars expand horizontally; text edits, lists, trees, tabs, custom-painted widgets vertically |
| `QLabel` | `setText/text`, `setAlignment`, `setPixmap` |
| `QPushButton` / `QToolButton` | `clicked/pressed/released`, `click`, `setText/text`, `setIcon` |
| `QCheckBox` | `isChecked/setChecked/checkState/setCheckState/toggle`, `toggled/stateChanged/clicked` |
| `QRadioButton`, `QButtonGroup` | auto-exclusive among siblings or within a group; `toggled/clicked`, `buttonClicked/idClicked/checkedButton` |
| `QLineEdit` | `text/setText/clear/setPlaceholderText/setReadOnly`, `textChanged/textEdited/returnPressed/editingFinished` |
| `QTextEdit` / `QPlainTextEdit` | `toPlainText/setPlainText/setText/append/insertPlainText/clear/setReadOnly`, `textChanged` |
| `QSlider` / `QDial` / `QSpinBox` / `QDoubleSpinBox` | `setRange/setMinimum/setMaximum/setValue/value/setSingleStep`, `setSuffix/setPrefix/setDecimals/stepUp/stepDown`, `valueChanged/sliderMoved/rangeChanged` |
| `QProgressBar` | `setRange/setValue/value/setFormat/setTextVisible/reset`, `valueChanged` |
| `QComboBox` | `addItem(s)/insertItem/removeItem/clear/count/itemText/itemData/currentIndex/setCurrentIndex/currentText/setCurrentText/currentData/findText`, `currentIndexChanged/currentTextChanged/activated` |
| `QListWidget` / `QListWidgetItem` | `addItem(s)/insertItem/takeItem/clear/count/item/currentRow/setCurrentRow/currentItem/row/selectedItems`, `currentRowChanged/itemClicked/itemDoubleClicked/itemSelectionChanged` |
| `QTreeWidget` / `QTreeWidgetItem` | `setHeaderLabels` (column headers), `addTopLevelItem(s)/topLevelItem/topLevelItemCount/clear/currentItem/setCurrentItem/expandAll/collapseAll`; items: `text/setText(col)/addChild/child/childCount/parent/setExpanded/isExpanded` |
| `QTableWidget` / `QTableWidgetItem` | `setRowCount/setColumnCount/setHorizontalHeaderLabels/setItem/item/currentRow/setCurrentCell/clear`, `cellClicked/cellDoubleClicked/currentCellChanged` |
| `QTabWidget`, `QGroupBox`, `QScrollArea`, `QStatusBar` | `addTab/currentIndex/setCurrentIndex/widget/tabText/setTabText`; `setTitle/title`; `setWidget`; `showMessage/clearMessage` |
| `QMenuBar` / `QMenu` / `QAction` | `addMenu/addAction(text, slot, shortcut)/addSeparator/popup/exec`; actions: `triggered/toggled`, checkable, shortcuts (shown in Blender menus) |
| `QPainter` | `setPen/setBrush/setFont/setOpacity/save/restore/translate`, `fillRect/drawRect/drawRoundedRect/drawEllipse/drawLine/drawPolyline/drawPolygon/drawPoint/drawArc/drawPie/drawText/drawPixmap` |
| `Signal` (`pyqtSignal`, `Signal_`) | class-level signals; `connect/disconnect/emit`; slots taking fewer arguments are called with fewer |
| `QTimer`, `QPropertyAnimation`, `QEasingCurve` | `start/stop/singleShot/setInterval/setSingleShot`, `timeout`; animation at 120 Hz with easing, `finished/valueChanged` |
| Value types | `QColor`, `QPen`, `QBrush`, `QFont`, `QPoint`, `QSize`, `QRect`, `QPixmap`/`QImage`, `QIcon`, `QMargins`, `QSizePolicy`, `Qt` constants, events (`QMouseEvent`, `QKeyEvent`, `QWheelEvent`, `QPaintEvent`, `QCloseEvent`) |
| `QMessageBox` / `QFileDialog` | non-blocking stubs (not drawn yet) / Blender's File Browser (`BUI_FILEDIALOG=native` for the OS dialogs) |

¹ `setStyleSheet`: `color`, `background(-color)` on containers, `font-weight: bold`.

## tkinter syntax (`bui.tksyntax`)

`bui.tksyntax` is a package laid out like `tkinter` itself: the package
(`Tk`, widgets, variables, `mainloop` ...) plus `ttk`, `messagebox`,
`filedialog`, `simpledialog`, `colorchooser`, `commondialog`, `dialog`,
`font`, `scrolledtext` and `constants`. Existing programs run unchanged:

```
python -m bui.tksyntax my_tk_app.py      # or call bui.tksyntax.install() first
```

**Coverage:** every public member of every tkinter / ttk widget class,
`Tk`, `Toplevel`, the variables, `Event`, `PhotoImage` and `BitmapImage`
exists (7846 of 7846 names, measured against the stdlib), and
`bindings/python/tests/test_tk_compat.py` runs nine real tkinter programs (calculator,
to-do list with scrollbar, validated form, text editor with menus, sortable
table, canvas, scrollable frame, notebook/paned window/option menu, events and
variables) on both the stdlib and BUI and requires identical results.

**Look:** always Blender's theme. Options that only change Tk's look
(`relief`, `borderwidth`, `highlightthickness`, `activebackground`, `ttk.Style`
settings ...) are accepted and returned by `cget`; `bg` colours frames and
canvases, `fg` colours text, `font` sets size and bold, `cursor` sets the
pointer.

**Behaviour follows Tk** (checked against the stdlib): `textvariable`
overrides `text` (also in `cget`), `Scale` commands and
`<<TreeviewSelect>>` run when idle, `validatecommand` sees programmatic
`insert`/`delete`, generated key events go to the focus widget, bindings run
per bindtag (widget, class, toplevel, `all`) and stop on `"break"`, a
`Notebook` requests its largest page, menus keep Tk's tearoff index 0 (off by
default on macOS, like Aqua), `Text` keeps the trailing newline (`"end-1c"`), `edit_undo` is a no-op while
`undo` is off, ttk `item()` converts integers.

| Area | What runs where |
|---|---|
| Geometry | `pack` / `grid` / `place` (all options, `in_`, `before`/`after`, propagation, `grid_remove`, `*_info`, `*_slaves`, `grid_bbox/location/size`): Tk's packer and gridder ported to C |
| Scrolling | `xview`/`yview`/`*_moveto`/`*_scroll` and `x/yscrollcommand` for Listbox, Text, Treeview, Canvas (`scrollregion`, `confine`, increments, `scan_*`): the kit's view protocol reports view changes after input and layout; `Scrollbar` is a Blender scroller following Tk's scrollbar protocol |
| Selection | `Listbox` `selectmode` browse / single / multiple / extended, `Treeview` extended / browse / none (Ctrl/Shift clicks in C) |
| Events | `bind` / `bind_all` / `bind_class` / `bindtags` / `event_add` / `event_generate` with modifiers, `Double-`, `<Button-1..3>`, `<B1-Motion>`, `<KeyPress>` / `<KeyRelease>`, `<MouseWheel>` (Button-4/5 on X11), `<Enter>` / `<Leave>`, `<Configure>`, `<Map>`, `<Destroy>`, `<FocusIn/Out>`, virtual events; `Event` has Tk's fields |
| Canvas | all item types (incl. `window` for scrollable frames, smoothed lines, arrows), tags and tag expressions, `find_*` / `addtag_*`, `tag_bind` with `current` and `<Enter>`/`<Leave>` per item, shape-accurate hit tests, `move` / `moveto` / `scale` / `coords`, text-item editing |
| Text | full index grammar (`line.char`, `end`, marks, `tag.first`, `@x,y`, `+N chars/lines`, `linestart`, `wordend` ...), tags with foreground/background and priorities, `tag_bind`, marks with gravity, `search` (regexp, backwards, count), `dump`, `count`, undo stack, `<<Modified>>`, peers |
| Windows | native window: `geometry` (size and position), `attributes` (`-topmost`, `-alpha`, `-fullscreen`, `-disabled`), `resizable`, `minsize` / `maxsize`, `overrideredirect`, `state` / `iconify` / `withdraw`, `transient`, `protocol`, `grab_set` (disables other windows), clipboard |
| Dialogs | `messagebox` and `simpledialog` (`Dialog`, `SimpleDialog`, `ask*`) as Blender-styled modal windows, `colorchooser` with Blender's color picker, `filedialog` with Blender's File Browser (`BUI_FILEDIALOG=native` for the OS dialogs) |
| Images | `PhotoImage` (PNG and PPM/PGM read and written in Python, GIF through Tk; `put`/`get`/`copy`/`zoom`/`subsample`/`copy_replace`/`data`/`write`), `BitmapImage` (XBM) |

Not supported, because nothing in the kit corresponds: `send` (raises like
Windows Tk), embedded windows and images inside `Text` (accepted and listed,
but not drawn), Tcl itself (`root.tk.call` handles a few common commands such
as `tk scaling` and `wm`).

## Not implemented

- **Qt:** QSS selectors/pseudo-states, `QGraphicsScene`, model/view
  (`QAbstractItemModel`), rich text, `QThread`, OpenGL widgets.

## Extending

Add the member to the facade class in `bindings/python/bui/qtsyntax.py` or
`bindings/python/bui/tksyntax/`, mapping it onto kit node properties
(`set/get`), geometry (`geom`), or the canvas/text-area calls. New drawing or
interaction belongs in the C kit (`kit/intern/`). Add a test in
`bindings/python/tests/` (for tkinter, a program in `test_tk_compat.py` that
must behave like the stdlib).
