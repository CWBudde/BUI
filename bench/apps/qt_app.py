"""Benchmark app, Qt syntax: the shared small UI. Prints "ready" once the
first frame is on screen, then idles until the harness stops it."""
from blenderui.qtsyntax import (QApplication, QCheckBox, QLabel, QLineEdit, QPushButton, QSlider, QTimer,
                                QVBoxLayout, QWidget, Qt)

app = QApplication([])
w = QWidget()
w.setWindowTitle("bench")
w.resize(360, 260)
lay = QVBoxLayout()
lay.addWidget(QLabel("Settings"))
lay.addWidget(QLineEdit("Cube"))
lay.addWidget(QCheckBox("Shadows"))
lay.addWidget(QSlider(Qt.Horizontal))
lay.addWidget(QPushButton("Apply"))
w.setLayout(lay)
w.show()


def poll():
    if w._window is not None and w._window.frames > 0:
        print("ready", flush=True)
    else:
        QTimer.singleShot(1, poll)


QTimer.singleShot(1, poll)
app.exec()
