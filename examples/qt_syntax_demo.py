"""Qt-syntax demo: a real Qt-style application drawn entirely by BUI.

The window host is picked automatically: PySide6/PyQt6 if installed,
otherwise a stdlib tkinter window — same code, same BUI rendering.

    python examples/qt_syntax_demo.py

SPDX-License-Identifier: GPL-2.0-or-later
"""

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "bindings", "python"))

from bui.qtsyntax import (QApplication, QComboBox, QGridLayout, QHBoxLayout,
                          QLabel, QLineEdit, QProgressBar, QPushButton,
                          QSlider, QSpinBox, QVBoxLayout, QWidget)


def main() -> int:
    app = QApplication(sys.argv)
    window = QWidget()
    window.setWindowTitle("BUI — Qt syntax")
    window.resize(520, 420)

    root = QVBoxLayout()
    title = QLabel("Qt syntax, BUI renderer")
    title.setStyleSheet("font-size: 18px; color: #7aa2f7;")
    title.setAlignment("center")
    root.addWidget(title)

    grid = QGridLayout()
    grid.addWidget(QLabel("Name:"), 0, 0)
    name = QLineEdit()
    name.setPlaceholderText("type here…")
    grid.addWidget(name, 0, 1)

    grid.addWidget(QLabel("Choice:"), 1, 0)
    combo = QComboBox()
    combo.addItems(["widget", "engine", "renderer"])
    grid.addWidget(combo, 1, 1)

    grid.addWidget(QLabel("Level:"), 2, 0)
    slider = QSlider()
    slider.setRange(0, 100)
    slider.setValue(30)
    grid.addWidget(slider, 2, 1)

    grid.addWidget(QLabel("Count:"), 3, 0)
    spin = QSpinBox()
    spin.setRange(0, 20)
    grid.addWidget(spin, 3, 1)

    grid.addWidget(QLabel("Progress:"), 4, 0)
    bar = QProgressBar()
    bar.setRange(0, 100)
    bar.setValue(30)
    grid.addWidget(bar, 4, 1)
    slider.valueChanged.connect(bar.setValue)

    root.addLayout(grid)

    status = QLabel("Press the button.")
    status.setAlignment("center")
    root.addWidget(status)

    def on_click():
        count = getattr(on_click, "n", 0) + 1
        on_click.n = count
        status.setText("{} — clicked {} time(s)!".format(
            name.text() or "BUI", count))

    buttons = QHBoxLayout()
    go = QPushButton("Click me")
    go.clicked.connect(on_click)
    bye = QPushButton("Quit")
    bye.clicked.connect(app.quit)
    buttons.addWidget(go)
    buttons.addWidget(bye)
    root.addLayout(buttons)

    window.setLayout(root)
    window.show()
    return app.exec()


if __name__ == "__main__":
    sys.exit(main())
