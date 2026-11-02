"""Extract public API surfaces from the reference checkouts.

- tkinter: AST-parse Lib/tkinter/__init__.py (+ ttk.py) for class methods.
- Qt: regex the public method declarations out of the widget headers.

Output: reference/API_NOTES.md — the target surface for the syntax layers.
"""
import ast
import io
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REF = ROOT / "reference"
OUT = []

# ---------------------------------------------------------------- tkinter
def tk_classes(path):
    tree = ast.parse(path.read_text(encoding="utf-8"))
    for node in tree.body:
        if isinstance(node, ast.ClassDef):
            methods = []
            for item in node.body:
                if isinstance(item, (ast.FunctionDef, ast.AsyncFunctionDef)):
                    if not item.name.startswith("_"):
                        methods.append(item.name)
            yield node.name, methods


TK_FILES = {
    "tkinter/__init__.py": ["Misc", "Widget", "Tk", "Pack", "Place", "Grid",
                            "Frame", "Label", "Button", "Checkbutton",
                            "Radiobutton", "Entry", "Listbox", "Canvas",
                            "Scale", "Scrollbar", "Text", "Toplevel"],
    "tkinter/ttk.py": ["Combobox", "Progressbar", "Scale", "Spinbox",
                       "Notebook", "Separator", "Sizegrip", "Treeview"],
}

OUT.append("## tkinter (from CPython %s)\n" % "reference/cpython")
tk_total = set()
for rel, wanted in TK_FILES.items():
    p = REF / "cpython" / "Lib" / rel
    if not p.exists():
        continue
    for cls, methods in tk_classes(p):
        if cls in wanted:
            OUT.append("### tkinter.%s (%d public methods)\n%s\n" %
                       (cls, len(methods), ", ".join(sorted(methods))))
            tk_total.update(methods)
OUT.append("**tkinter surface: %d unique public methods across mirrored classes**\n" % len(tk_total))

# -------------------------------------------------------------------- Qt
QT_HEADERS = {
    "QApplication": "src/gui/kernel/qapplication.h",
    "QWidget": "src/widgets/kernel/qwidget.h",
    "QFrame": "src/widgets/widgets/qframe.h",
    "QLabel": "src/widgets/widgets/qlabel.h",
    "QPushButton": "src/widgets/widgets/qpushbutton.h",
    "QCheckBox": "src/widgets/widgets/qcheckbox.h",
    "QRadioButton": "src/widgets/widgets/qradiobutton.h",
    "QLineEdit": "src/widgets/widgets/qlineedit.h",
    "QComboBox": "src/widgets/widgets/qcombobox.h",
    "QSlider": "src/widgets/widgets/qslider.h",
    "QSpinBox": "src/widgets/widgets/qspinbox.h",
    "QProgressBar": "src/widgets/widgets/qprogressbar.h",
    "QGroupBox": "src/widgets/widgets/qgroupbox.h",
}

OUT.append("## Qt widgets (from qtbase headers)\n")
qt_total = set()
for cls, rel in QT_HEADERS.items():
    p = REF / "qtbase" / rel
    if not p.exists():
        OUT.append("### %s — header missing (%s)\n" % (cls, rel))
        continue
    text = p.read_text(encoding="utf-8", errors="ignore")
    names = set()
    # public methods: "type name(" inside the header, filtered to identifiers
    for m in re.finditer(r"^\s*(?:virtual\s+|static\s+)?[A-Za-z_][\w:<>,\s*&]*\s+\*?([a-zA-Z_]\w*)\s*\(",
                         text, re.M):
        name = m.group(1)
        if name in ("if", "for", "while", "switch", "return", "sizeof", "Q_DECL_DEPRECATED"):
            continue
        names.add(name)
    # signals/slots emit via macros; keep the list as-is
    OUT.append("### %s (%d candidate public methods)\n%s\n" %
               (cls, len(names), ", ".join(sorted(names))))
    qt_total.update(names)
OUT.append("**Qt surface: %d unique candidate public methods across mirrored classes**\n" % len(qt_total))

out_path = REF / "API_NOTES.md"
io.open(out_path, "w", encoding="utf-8", newline="\n").write("\n".join(OUT))
print("wrote", out_path, "— tkinter:", len(tk_total), "methods; qt:", len(qt_total), "methods")
