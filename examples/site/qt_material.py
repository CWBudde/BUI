"""Qt syntax: a material editor (QListWidget, QFormLayout, QGroupBox ...).

    python examples/site/qt_material.py

SPDX-License-Identifier: GPL-2.0-or-later
"""

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..",
                                "bindings", "python"))

from bui.qtsyntax import (QApplication, QCheckBox, QComboBox, QDoubleSpinBox,  # noqa: E402
                          QFormLayout, QGroupBox, QHBoxLayout, QLabel, QLineEdit,
                          QListWidget, QProgressBar, QPushButton, QSlider, QVBoxLayout,
                          QWidget)

MATERIALS = ["Brushed Steel", "Car Paint", "Frosted Glass", "Oak Veneer", "Rubber",
             "Ceramic Glaze", "Neon Tube"]


def slider_row(value, maximum=100):
    row = QHBoxLayout()
    s = QSlider()
    s.setRange(0, maximum)
    s.setValue(value)
    spin = QDoubleSpinBox()
    spin.setRange(0.0, 1.0)
    spin.setSingleStep(0.05)
    spin.setValue(value / maximum)
    s.valueChanged.connect(lambda v: spin.setValue(v / maximum))
    row.addWidget(s, 3)
    row.addWidget(spin, 1)
    return row


def build():
    window = QWidget()
    window.setWindowTitle("Material Editor")
    window.resize(560, 340)
    root = QHBoxLayout()

    left = QVBoxLayout()
    search = QLineEdit()
    search.setPlaceholderText("Filter materials")
    left.addWidget(search)
    mats = QListWidget()
    mats.addItems(MATERIALS)
    mats.setCurrentRow(1)
    left.addWidget(mats)
    add = QHBoxLayout()
    add.addWidget(QPushButton("New"))
    add.addWidget(QPushButton("Duplicate"))
    left.addLayout(add)
    root.addLayout(left, 2)

    right = QVBoxLayout()
    name = QLineEdit("Car Paint")
    right.addWidget(name)
    surface = QGroupBox("Surface")
    form = QFormLayout()
    shader = QComboBox()
    shader.addItems(["Principled BSDF", "Diffuse BSDF", "Glossy BSDF", "Emission"])
    form.addRow("Shader", shader)
    form.addRow("Metallic", slider_row(85))
    form.addRow("Roughness", slider_row(22))
    form.addRow("Coat", slider_row(100))
    ior = QDoubleSpinBox()
    ior.setRange(1.0, 3.0)
    ior.setSingleStep(0.01)
    ior.setValue(1.5)
    form.addRow("IOR", ior)
    blend = QComboBox()
    blend.addItems(["Opaque", "Alpha Clip", "Alpha Blend"])
    form.addRow("Blend Mode", blend)
    surface.setLayout(form)
    right.addWidget(surface)

    flags = QHBoxLayout()
    cull = QCheckBox("Backface Culling")
    cull.setChecked(True)
    flags.addWidget(cull)
    flags.addWidget(QCheckBox("Show Backface"))
    right.addLayout(flags)

    bar = QProgressBar()
    bar.setRange(0, 100)
    bar.setValue(74)
    bar.setFormat("Compiling shaders %p%")
    right.addWidget(bar)
    buttons = QHBoxLayout()
    buttons.addWidget(QLabel("Used by 12 objects"))
    buttons.addWidget(QPushButton("Revert"))
    buttons.addWidget(QPushButton("Apply"))
    right.addLayout(buttons)
    root.addLayout(right, 3)
    window.setLayout(root)
    return window


def main() -> int:
    app = QApplication(sys.argv)
    w = build()
    w.show()
    return app.exec()


if __name__ == "__main__":
    sys.exit(main())
