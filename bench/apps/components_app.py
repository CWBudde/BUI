"""Benchmark app, Blender components: the shared small UI. Prints "ready" once
the first frame is on screen, then idles until the harness stops it."""
from blenderui import host
from blenderui.components import App, Button, Checkbox, Label, NumberSlider, Panel, TextField

app = App("bench", (360, 260),
          Panel("Settings",
                TextField("Name", "Cube"),
                Checkbox("Shadows", True),
                NumberSlider("Strength", 1.0, min=0, max=10)),
          Button("Apply"))
win = app.open()


def poll():
    if win.frames > 0:
        print("ready", flush=True)
    else:
        win.after(1, poll)


win.after(1, poll)
host.run()
