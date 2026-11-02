"""TkSurface — hosts one BUI widget tree in a real tk window.

This is the fast path behind ``bui.tksyntax`` and the tk fallback of
``bui.qtsyntax``. It renders the way Blender does, adapted to a CPU host:

* **One native frame.** The whole tree is rasterized in C into a single
  frame buffer (``buipy.Raster``: anti-aliased SDF boxes/circles/lines and a
  cached native text engine) — no per-primitive Tk canvas items.
* **One present.** Only the tiles that changed since the last frame are
  sent to the screen: a direct GDI blit to the window on Windows, otherwise
  a dirty-rect ``PhotoImage.put`` of binary PPM.
* **Coalesced redraws.** Input, timers and ``widget.update()`` only mark the
  surface dirty; one render runs when Tk goes idle, however many events
  arrived. Hover with no handler costs nothing.

SPDX-License-Identifier: GPL-2.0-or-later
"""

from __future__ import annotations

import sys
from bui import _realtk

tk = _realtk.module()
tkfont = _realtk.module("tkinter.font")

import buipy


_BG = "#1d1d22"
_BUTTON1_MASK = 0x0100

# Native present + native text exist together (Win32 GDI) — see GPU_raster.h.
NATIVE_PRESENT = sys.platform == "win32" and buipy.has_native_text()


def _hex(rgba) -> str:
    r, g, b = (max(0, min(255, int(c * 255 + 0.5))) for c in rgba[:3])
    return "#%02x%02x%02x" % (r, g, b)


class _TkTextOverlay:
    """Host-drawn text for platforms without a native text engine.

    Canvas text items are pooled and only reconfigured when a run changes,
    so a steady frame costs no Tk calls.
    """

    def __init__(self, canvas: tk.Canvas):
        self._c = canvas
        self._items: list = []  # [(item_id, spec)]
        self._fonts: dict = {}

    def _font(self, family, size, flags):
        key = (family, int(round(size)), flags)
        f = self._fonts.get(key)
        if f is None:
            f = tkfont.Font(root=self._c, family=family or "TkDefaultFont",
                            size=-max(1, key[1]),
                            weight="bold" if flags & 1 else "normal",
                            slant="italic" if flags & 2 else "roman",
                            underline=bool(flags & 4))
            self._fonts[key] = (f, f.metrics("descent"))
            f = self._fonts[key]
        return f

    def update(self, runs) -> None:
        c = self._c
        for i, run in enumerate(runs):
            x, y, text, size, rgba, family, flags = run
            font, descent = self._font(family, size, flags)
            if i < len(self._items):
                item, old = self._items[i]
                if old == run:
                    continue
                c.coords(item, x, y + descent)
                c.itemconfigure(item, text=text, fill=_hex(rgba), font=font,
                                state="normal")
                self._items[i] = (item, run)
            else:
                item = c.create_text(x, y + descent, text=text, fill=_hex(rgba),
                                     font=font, anchor="sw")
                self._items.append((item, run))
        for i in range(len(runs), len(self._items)):
            item, old = self._items[i]
            if old is not None:
                c.itemconfigure(item, state="hidden")
                self._items[i] = (item, None)


def parse_geometry(spec):
    """'420x480+10+10' -> (420, 480); None when no size is given."""
    try:
        size = str(spec).split("+")[0].split("-")[0]
        gw, gh = (int(v) for v in size.split("x"))
        return gw, gh
    except ValueError:
        return None


class Modifiers:
    """Keyboard modifier + button state decoded from a Tk event."""

    __slots__ = ("shift", "ctrl", "alt", "button1")

    _ALT = 0x20000 if sys.platform == "win32" else 0x0008

    def __init__(self, state=0):
        state = state if isinstance(state, int) else 0
        self.shift = bool(state & 0x0001)
        self.ctrl = bool(state & 0x0004)
        self.alt = bool(state & self._ALT)
        self.button1 = bool(state & _BUTTON1_MASK)


class TkSurface:
    """A tk widget that displays (and feeds input to) a BUI scene.

    ``scene`` is any object implementing the scene protocol:

        render(w, h) -> int                draw one frame; 0 = nothing changed
        press / release / motion / context_menu(x, y, mods)
        key(char, keysym, keycode, mods)
        wheel(x, y, delta, mods)           delta: +1 up / -1 down
    """

    def __init__(self, master, scene, width=None, height=None):
        self.master = master
        self.scene = scene
        self.raster = buipy.Raster(1, 1)
        self._pending = None
        self._rendered = False
        self._size = (0, 0)
        self.frames = 0  # rendered frame count (diagnostics/tests)
        # Requested window size (from geometry()) used until the window is
        # mapped — Tk reports 1x1 before that.
        self.size_hint = None

        opts = {"highlightthickness": 0, "borderwidth": 0}
        if width:
            opts["width"] = int(width)
        if height:
            opts["height"] = int(height)
        if NATIVE_PRESENT:
            # bg="" — Tk never paints this window; every pixel is ours.
            self.widget = tk.Frame(master, bg="", takefocus=1, **opts)
            self._photo = None
            self._overlay = None
        else:
            self.widget = tk.Canvas(master, bg=_BG, takefocus=1, **opts)
            self._photo = tk.PhotoImage(master=self.widget, width=1, height=1)
            self.widget.create_image(0, 0, image=self._photo, anchor="nw")
            self._overlay = None if buipy.has_native_text() else _TkTextOverlay(self.widget)
        self.widget.pack(fill="both", expand=True)

        w = self.widget
        w.bind("<ButtonPress-1>", self.on_press)
        w.bind("<ButtonRelease-1>", self.on_release)
        if hasattr(scene, "context_menu"):     # right click (Button-2 on macOS)
            w.bind("<ButtonPress-2>" if sys.platform == "darwin" else "<ButtonPress-3>",
                   self.on_context)
        if hasattr(scene, "button"):           # raw <Button-2>, <ButtonRelease-2/3>
            for num in (2, 3):
                w.bind("<ButtonRelease-%d>" % num,
                       lambda e, n=num: self._raw_button(e, n, True))
            w.bind("<ButtonPress-2>", lambda e: self._raw_button(e, 2, False))
        if hasattr(scene, "key_release"):
            w.bind("<KeyRelease>", self._on_key_release)
        w.bind("<Motion>", self.on_motion)
        w.bind("<Key>", self.on_key)
        w.bind("<MouseWheel>", self.on_wheel)                 # Windows / macOS
        w.bind("<Button-4>", lambda e: self._wheel(e, +1))    # X11
        w.bind("<Button-5>", lambda e: self._wheel(e, -1))
        w.bind("<Configure>", self._on_configure)
        w.bind("<Expose>", self._on_expose)
        w.focus_set()

    # -- size -------------------------------------------------------------------
    def size(self):
        w = self.widget.winfo_width()
        h = self.widget.winfo_height()
        if w <= 1 or h <= 1:
            # Not mapped yet (headless/tests): use requested/geometry size.
            if self.size_hint is not None:
                return self.size_hint
            w = max(w, self.widget.winfo_reqwidth())
            h = max(h, self.widget.winfo_reqheight())
            w, h = max(w, 400), max(h, 300)
        return w, h

    # -- redraw scheduling ------------------------------------------------------
    def request(self) -> None:
        """Mark dirty; one render runs when Tk is idle (coalesces bursts)."""
        if self._pending is None:
            try:
                self._pending = self.widget.after_idle(self._flush)
            except tk.TclError:  # window destroyed
                self._pending = None

    def _flush(self) -> None:
        self._pending = None
        self.render()

    def render(self) -> None:
        """Render now and present the changed regions."""
        try:
            self._render()
        except tk.TclError:
            pass  # window destroyed while a redraw was queued

    def _render(self) -> None:
        if self._pending is not None:
            try:
                self.widget.after_cancel(self._pending)
            except tk.TclError:
                pass
            self._pending = None
        w, h = self.size()
        resized = (w, h) != self._size
        self._size = (w, h)
        buipy.set_backend(self.raster)
        try:
            regions = self.scene.render(w, h)
        finally:
            buipy.set_backend(None)
        if regions == 0 and not resized and self._rendered:
            return  # damage-driven scene: nothing changed, nothing to blit
        self.frames += 1
        self._rendered = True
        self._present(full=resized)

    def _present(self, full=False) -> None:
        r = self.raster
        try:
            if NATIVE_PRESENT:
                if not self.widget.winfo_ismapped():
                    return  # Expose will present once it is on screen
                r.present_native(self.widget.winfo_id(), full)
                return
            w, h = r.size
            if full or self._photo.width() != w or self._photo.height() != h:
                self._photo.configure(width=w, height=h)
                rects = [(0, 0, w, h)]
            else:
                rects = r.damage()
            for x, y, rw, rh in rects:
                self._photo.put(r.ppm(x, y, rw, rh), to=(x, y))
            if self._overlay is not None:
                self._overlay.update(r.text_runs())
        except tk.TclError:
            pass  # window destroyed mid-frame

    # -- tk events --------------------------------------------------------------
    def _on_configure(self, event) -> None:
        self.request()

    def _on_expose(self, event) -> None:
        if not self._rendered:
            self.request()
        elif NATIVE_PRESENT:
            # The OS lost our pixels (uncovered/restored): re-blit, no render.
            self.raster.present_native(self.widget.winfo_id(), True)

    @staticmethod
    def _mods(event):
        state = getattr(event, "state", None)
        return Modifiers(state) if isinstance(state, int) else None

    def on_press(self, event) -> None:
        try:
            self.widget.focus_set()
        except tk.TclError:
            pass
        self.scene.press(event.x, event.y, self._mods(event))
        self.request()

    def on_context(self, event) -> None:
        self.scene.context_menu(event.x, event.y, self._mods(event))
        self.request()

    def on_release(self, event) -> None:
        self.scene.release(event.x, event.y, self._mods(event))
        self.request()

    def _raw_button(self, e, num, release):
        if num == 2 and sys.platform == "darwin":
            num = 3  # macOS reports the right button as 2
        self.scene.button(e.x, e.y, num, release, Modifiers(e.state))
        self.request()

    def _on_key_release(self, e):
        self.scene.key_release(e.keysym, Modifiers(e.state))
        self.request()

    def on_motion(self, event) -> None:
        if self.scene.motion(event.x, event.y, self._mods(event)):
            self.request()

    def on_key(self, event) -> None:
        self.scene.key(event.char, event.keysym, getattr(event, "keycode", 0),
                       self._mods(event))
        self.request()

    def on_wheel(self, event) -> None:
        self._wheel(event, 1 if getattr(event, "delta", 0) > 0 else -1)

    def _wheel(self, event, delta) -> None:
        if self.scene.wheel(event.x, event.y, delta, self._mods(event)):
            self.request()
