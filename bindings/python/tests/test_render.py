"""Native raster pipeline + host behavior regressions.

Covers the Raster renderer (AA primitives, blending, damage tracking, text),
redraw coalescing in the window host, input semantics (hover vs. drag,
pointer grab), popup z-order, Toplevel windows, timers.

SPDX-License-Identifier: GPL-2.0-or-later
"""

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))

import bui  # noqa: E402,F401  (locates buipy)
import buipy  # noqa: E402
from bui import host  # noqa: E402

BG = (0.11, 0.11, 0.13, 1.0)



def pos(w):
    """Window coordinates of a widget (Qt widgets are kit nodes; tk widgets wrap one)."""
    return w.absolutePos() if hasattr(w, "absolutePos") else tuple(w._node.rect[:2])

def frame(r, w, h, draw):
    buipy.set_backend(r)
    try:
        buipy.begin_frame(w, h)
        buipy.draw_rect(0, 0, w, h, BG)
        draw()
        buipy.end_frame()
    finally:
        buipy.set_backend(None)


def test_raster_primitives():
    r = buipy.Raster(200, 120)
    frame(r, 200, 120, lambda: (
        buipy.draw_rect(10, 10, 50, 20, (1.0, 0.0, 0.0, 1.0)),
        buipy.draw_rect(100, 10, 50, 20, (1.0, 1.0, 1.0, 0.5)),
        buipy.draw_roundrect(10, 50, 80, 40, 12, (0.0, 0.0, 1.0, 1.0)),
        buipy.draw_ellipse(150, 80, 30, 15, (0.0, 1.0, 0.0, 1.0)),
        buipy.draw_polygon([(100, 100), (130, 100), (115, 118)], (1.0, 1.0, 0.0, 1.0)),
    ))
    assert r.size == (200, 120)
    assert r.pixel(30, 20) == (255, 0, 0), r.pixel(30, 20)            # solid fill
    assert r.pixel(9, 20) == (28, 28, 33), r.pixel(9, 20)             # crisp edge
    half = r.pixel(120, 20)                                           # 50% white over bg
    assert all(abs(a - b) <= 2 for a, b in zip(half, (142, 142, 144))), half
    assert r.pixel(11, 51) == (28, 28, 33)                            # outside the arc
    corner = r.pixel(13, 53)                                          # AA on the arc
    assert corner != (0, 0, 255) and corner != (28, 28, 33), corner
    assert r.pixel(50, 70) == (0, 0, 255)                             # round-rect interior
    assert r.pixel(170, 80)[1] > 200 and r.pixel(150, 64) == (28, 28, 33)  # ellipse, not circle
    assert r.pixel(115, 104) == (255, 255, 0)                         # polygon filled


def test_damage_tracking():
    r = buipy.Raster(256, 128)
    draw = lambda c: (lambda: buipy.draw_rect(70, 40, 20, 10, c))  # noqa: E731
    frame(r, 256, 128, draw((1.0, 0.0, 0.0, 1.0)))
    assert r.damage() == [(0, 0, 256, 128)]          # first frame: everything
    frame(r, 256, 128, draw((1.0, 0.0, 0.0, 1.0)))
    assert r.damage() == []                          # identical frame: nothing
    frame(r, 256, 128, draw((0.0, 1.0, 0.0, 1.0)))
    dmg = r.damage()
    assert len(dmg) == 1 and dmg[0][0] <= 70 and dmg[0][2] <= 128, dmg  # just that tile


def test_damage_reaches_right_edge():
    # Width not a multiple of the 64px tile: a change touching the right edge
    # must still be reported (it used to be dropped -> stale window).
    r = buipy.Raster(400, 64)
    frame(r, 400, 64, lambda: buipy.draw_rect(300, 10, 100, 20, (1.0, 0.0, 0.0, 1.0)))
    frame(r, 400, 64, lambda: buipy.draw_rect(300, 10, 100, 20, (0.0, 1.0, 0.0, 1.0)))
    dmg = r.damage()
    assert dmg and max(x + w for x, y, w, h in dmg) == 400, dmg


def test_window_matches_buffer_after_real_events():
    """Drive the native window with real Win32 messages and compare what the
    window shows with the frame buffer."""
    if not host.native_available():
        return
    import ctypes
    import time
    from bui.tksyntax import Entry, Scale, Tk
    root = Tk()
    root.geometry("400x200")
    e = Entry(root)
    e.pack(fill="x", padx=10, pady=10)
    s = Scale(root, from_=0, to=100, orient="horizontal")
    s.pack(fill="x", padx=10)
    for _ in range(10):
        root.update()
        time.sleep(0.01)
    win = root._window
    hwnd = win.handle
    u, g = ctypes.windll.user32, ctypes.windll.gdi32
    u.GetDC.restype = ctypes.c_void_p
    u.GetDC.argtypes = [ctypes.c_void_p]
    u.ReleaseDC.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
    g.GetPixel.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_int]
    u.PostMessageW.argtypes = [ctypes.c_void_p, ctypes.c_uint, ctypes.c_size_t, ctypes.c_ssize_t]

    def stale():
        hdc = u.GetDC(hwnd)
        bad = 0
        w, h = win.size
        for y in range(0, h, 4):
            for x in range(0, w, 4):
                c = g.GetPixel(hdc, x, y)
                if (c & 255, (c >> 8) & 255, (c >> 16) & 255) != win.pixel(x, y):
                    bad += 1
        u.ReleaseDC(hwnd, hdc)
        return bad

    def post(msg, x, y, wp=0):
        # PostMessage (not SendMessage): the window procedure runs Python
        # callbacks and must be entered from the pump, holding the GIL.
        u.PostMessageW(hwnd, msg, wp, (y << 16) | (x & 0xFFFF))
        root.update()

    sx, sy = pos(s)
    post(0x200, sx + 10, sy + 5)
    post(0x201, sx + 10, sy + 5, 1)
    post(0x200, sx + 3000, sy + 5, 1)
    post(0x202, sx + 3000, sy + 5)
    assert s.get() > 50, s.get()
    assert stale() == 0, "slider drag not visible in the window"
    ex, ey = pos(e)
    post(0x201, ex + 10, ey + 5, 1)
    post(0x202, ex + 10, ey + 5)
    for ch in "hi":
        u.PostMessageW(hwnd, 0x102, ord(ch), 0)  # WM_CHAR
        root.update()
    assert e.get() == "hi", e.get()
    assert stale() == 0, "typed text not visible in the window"
    root.destroy()


def test_text():
    w = buipy.measure_text("Hello", 13)
    if not buipy.has_native_text():
        assert w is None
        return
    assert 20 < w < 60, w
    assert buipy.measure_text("Hello Hello", 13) > w
    r = buipy.Raster(120, 40)
    frame(r, 120, 40, lambda: buipy.draw_text(4, 26, 16, "Wg", (1.0, 1.0, 1.0, 1.0)))
    lit = sum(1 for x in range(120) for y in range(40) if r.pixel(x, y)[0] > 128)
    assert lit > 30, lit


def test_coalesced_redraws():
    import time
    from bui.tksyntax import Button, Tk
    root = Tk()
    btns = [Button(root, text="b%d" % i) for i in range(5)]
    for b in btns:
        b.pack()
    for _ in range(5):                   # settle mapping/first frames
        root.update()
        time.sleep(0.01)
    before = root._window.frames
    for i in range(50):                  # 50 changes ...
        btns[i % 5].configure(text="x%d" % i)
    root.update()                        # ... one frame
    assert root._window.frames == before + 1, root._window.frames - before
    root.destroy()


def test_hover_does_not_drag_and_grab():
    from bui.tksyntax import Scale, Tk
    root = Tk()
    sc = Scale(root, from_=0, to=100, orient="horizontal")
    sc.pack(fill="x")
    ctx = root._ctx
    r = buipy.Raster(420, 200)
    ctx.render(420, 200, r, True)
    sx, sy = pos(sc)
    ctx.motion(sx + 5, sy + 5, 0)        # enter: hover highlight
    assert ctx.hover is sc._node
    ctx.render(420, 200, r, False)
    for dx in (10, 60, 120):             # plain hover across the slider
        ctx.motion(sx + dx, sy + 5, 0)
    assert sc.get() == 0, "hover changed the slider value"
    assert not ctx.needs_render(), "hover within one widget should not redraw"
    ctx.press(sx + 10, sy + 5, 0)
    ctx.motion(sx + 5000, sy + 300, 8)   # drag far outside
    assert sc.get() == 100, sc.get()     # implicit grab keeps the drag
    ctx.release(sx + 5000, sy + 300, 0)
    root.destroy()


def test_popup_on_top():
    from bui.tksyntax import Button, Combobox, Tk
    root = Tk()
    cb = Combobox(root, values=["one", "two", "three"])
    cb.pack(fill="x")
    below = Button(root, text="below")
    below.pack(fill="x")
    ctx = root._ctx
    r = buipy.Raster(300, 200)
    ctx.render(300, 200, r, True)
    bx, by = pos(below)
    before = r.pixel(bx + 3, by + 3)
    cx, cy = pos(cb)
    ctx.press(cx + 5, cy + 5, 0)         # open the dropdown
    ctx.release(cx + 5, cy + 5, 0)
    assert ctx.popup_count() == 1
    ctx.render(300, 200, r, False)
    px, py, pw, ph = ctx.popup(0).rect
    assert py <= by + 3 < py + ph, "dropdown does not cover the button below"
    assert r.pixel(bx + 3, by + 3) != before, "dropdown painted under a later sibling"
    root.destroy()


def test_toplevel_has_its_own_content():
    from bui.tksyntax import Label, Tk, Toplevel
    root = Tk()
    top = Toplevel(root, title="second")
    top.geometry("200x100")
    lab = Label(top, text="in toplevel")
    lab.pack()
    root.update()
    assert lab.master is top, "child not parented to its Toplevel"
    assert lab._node in top._node.children and lab._node not in root._node.children
    assert top._ctx is not root._ctx and top._window.frames >= 1
    top.destroy()
    root.destroy()


def test_after_cancel():
    import time
    from bui.tksyntax import Tk
    root = Tk()
    root.update()
    fired = []
    tok = root.after(1, lambda: fired.append(1))
    root.after(1, lambda: fired.append(2))
    root.after_cancel(tok)
    deadline = time.time() + 1
    while not fired and time.time() < deadline:
        time.sleep(0.005)
        root.update()
    assert fired == [2], fired
    root.destroy()


def test_canvas_damage_matches_full_redraw():
    """Canvas edits redraw only the items' old and new boxes; the result must
    equal redrawing everything, for every item type and kind of edit."""
    import random
    from bui.tksyntax import Canvas, Tk

    W, H = 320, 220
    root = Tk()
    cv = Canvas(root, width=W, height=H, bg="#303030", highlightthickness=0)
    cv.pack()
    rnd = random.Random(7)

    def make(k):
        x, y = rnd.randrange(0, W - 40), rnd.randrange(0, H - 30)
        kind = k % 7
        if kind == 0:
            return cv.create_rectangle(x, y, x + 30, y + 20, fill="#4772b3", outline="#e6e6e6", width=2)
        if kind == 1:
            return cv.create_oval(x, y, x + 24, y + 24, fill="#e6a23c")
        if kind == 2:
            return cv.create_line(x, y, x + 30, y + 18, x + 10, y + 25, fill="#78aef0", width=3)
        if kind == 3:
            return cv.create_polygon(x, y + 20, x + 12, y, x + 24, y + 20, fill="#6aa84f", outline="#ffffff")
        if kind == 4:
            return cv.create_arc(x, y, x + 26, y + 26, start=30, extent=240, fill="#c27ba0", outline="#ffffff")
        if kind == 5:
            return cv.create_text(x + 10, y + 10, text="label %d" % k, fill="#ffffff", anchor="nw")
        return cv.create_text(x + 15, y + 10, text="ctr", fill="#ffff00")

    items = [make(k) for k in range(40)]
    ctx = root._ctx
    live = buipy.Raster(W, H)
    ctx.render(W, H, live, True)
    for step in range(80):
        i = rnd.choice(items)
        op = step % 6
        if op == 0:
            cv.move(i, rnd.randint(-15, 15), rnd.randint(-15, 15))
        elif op == 1:
            if cv.type(i) in ("text",):
                cv.itemconfigure(i, text="changed text %d" % step)
            else:
                cv.itemconfigure(i, fill=rnd.choice(["#ff0000", "#00ff00", "#2020ff"]))
        elif op == 2:
            cv.delete(i)
            items.remove(i)
            items.append(make(step))
        elif op == 3:
            cv.tag_raise(i)
        elif op == 4:
            cv.tag_lower(i)
        else:
            x, y = rnd.randrange(0, W - 40), rnd.randrange(0, H - 30)
            n = len(cv.coords(i))
            cv.coords(i, *([x, y, x + 20, y + 14, x + 5, y + 20][:n] if n <= 6 else cv.coords(i)))
        ctx.render(W, H, live, False)  # damage only
        full = buipy.Raster(W, H)
        ctx.render(W, H, full, True)  # everything
        assert live.ppm(0, 0, W, H) == full.ppm(0, 0, W, H), "step %d (op %d): damage redraw differs" % (step, op)
    root.destroy()


def main() -> int:
    tests = [v for k, v in sorted(globals().items()) if k.startswith("test_")]
    for t in tests:
        t()
        print("ok", t.__name__)
    print("all render tests passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
