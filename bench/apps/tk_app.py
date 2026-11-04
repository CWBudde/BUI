"""Benchmark app, tkinter syntax: the shared small UI. Prints "ready" once the
first frame is on screen, then idles until the harness stops it."""
import sys

import blenderui.tksyntax as tk
from blenderui.tksyntax import ttk

root = tk.Tk()
root.title("bench")
root.geometry("360x260")
ttk.Label(root, text="Settings").pack(anchor="w", padx=12, pady=(12, 4))
name = tk.StringVar(value="Cube")
ttk.Entry(root, textvariable=name).pack(fill="x", padx=12, pady=4)
ttk.Checkbutton(root, text="Shadows").pack(anchor="w", padx=12, pady=4)
ttk.Scale(root, from_=0, to=10).pack(fill="x", padx=12, pady=4)
ttk.Button(root, text="Apply").pack(anchor="w", padx=12, pady=4)


def poll():
    if root._window.frames > 0:
        print("ready", flush=True)
    else:
        root.after(1, poll)


root.after(1, poll)
root.mainloop()
sys.exit(0)
