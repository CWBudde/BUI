"""tkinter syntax on BUI: canvas frame cost, rendered headless.

    py bench/tk_canvas_bench.py [items] [all|one]

Each frame moves every item (`all`) or one item (`one`) with Canvas.move,
then renders (damage only). Prints ms per frame: Python (the tkinter facade)
and render (the C kit).
"""

import sys
import time

import bui  # noqa: F401  (locates buipy)
import buipy
from bui.tksyntax import Canvas, Tk

N = int(sys.argv[1]) if len(sys.argv) > 1 else 2000
MODE = sys.argv[2] if len(sys.argv) > 2 else "all"
W, H = 1000, 800

root = Tk()
cv = Canvas(root, width=W, height=H, bg="#303030", highlightthickness=0)
cv.pack()
items = []
cols = 50
for k in range(N):
    x, y = (k % cols) * 20 + 2, (k // cols) * 20 % (H - 20) + 2
    kind = k % 6
    if kind == 0:
        i = cv.create_rectangle(x, y, x + 16, y + 16, fill="#4772b3", outline="#e6e6e6")
    elif kind == 1:
        i = cv.create_oval(x, y, x + 16, y + 16, fill="#e6a23c")
    elif kind == 2:
        i = cv.create_line(x, y, x + 16, y + 16, fill="#78aef0", width=2)
    elif kind == 3:
        i = cv.create_polygon(x, y + 16, x + 8, y, x + 16, y + 16, fill="#6aa84f", outline="#ffffff")
    elif kind == 4:
        i = cv.create_arc(x, y, x + 16, y + 16, start=0, extent=270, fill="#c27ba0")
    else:
        i = cv.create_text(x + 8, y + 8, text=str(k % 100), fill="#ffffff")
    items.append(i)

ctx = root._ctx
r = buipy.Raster(W, H)
ctx.render(W, H, r, True)
frames, t_py, t_render = 120, 0.0, 0.0
for f in range(frames):
    d = 1 if f % 2 == 0 else -1
    t0 = time.perf_counter()
    if MODE == "all":
        for i in items:
            cv.move(i, d, 0)
    else:
        cv.move(items[f % N], d, 0)
    t1 = time.perf_counter()
    ctx.render(W, H, r, False)
    t2 = time.perf_counter()
    t_py += t1 - t0
    t_render += t2 - t1
print(f"items={N} mode={MODE}: python {t_py / frames * 1000:.2f} ms/frame, "
      f"render {t_render / frames * 1000:.2f} ms/frame")
