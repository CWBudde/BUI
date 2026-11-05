"""tkinter compatibility: the same programs run on the stdlib tkinter and on
BUI (``bui.tksyntax``), and must observe the same results.

Each program is ordinary tkinter code written against ``tk`` / ``ttk``
modules handed in; it returns a dict of observations. The stdlib run is the
reference (skipped when Tk isn't installed).

SPDX-License-Identifier: GPL-2.0-or-later
"""

import os
import subprocess
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))

import bui  # noqa: E402,F401  (locates buipy)


# ==========================================================================
# Programs (plain tkinter)


def settle(root, widget):
    """Run the event loop until `widget` is mapped (a window manager maps
    windows asynchronously)."""
    for _ in range(200):
        root.update()
        if widget.winfo_viewable():
            break
        root.after(5)
    root.update()


def prog_calculator(tk, ttk):
    root = tk.Tk()
    root.title("Calc")
    expr = tk.StringVar()
    entry = tk.Entry(root, textvariable=expr, width=16, justify="right")
    entry.grid(row=0, column=0, columnspan=4, sticky="we", padx=4, pady=4)
    out = []

    def press(key):
        if key == "=":
            try:
                expr.set(str(eval(expr.get() or "0")))
            except Exception:
                expr.set("error")
        elif key == "C":
            expr.set("")
        else:
            expr.set(expr.get() + key)
    buttons = {}
    for r, row in enumerate(("789/", "456*", "123-", "0C=+")):
        for c, key in enumerate(row):
            b = tk.Button(root, text=key, width=4, command=lambda k=key: press(k))
            b.grid(row=r + 1, column=c, sticky="nsew")
            buttons[key] = b
    for c in range(4):
        root.columnconfigure(c, weight=1)
    for key in "12+30=":
        buttons[key].invoke()
    out.append(expr.get())
    buttons["C"].invoke()
    entry.insert(0, "6*7")
    buttons["="].invoke()
    out.append(entry.get())
    info = buttons["5"].grid_info()
    result = {"results": out, "grid": (info["row"], info["column"]), "size": root.grid_size(),
              "slaves": len(root.grid_slaves(row=1)), "class": buttons["1"].winfo_class(),
              "name": str(buttons["1"]).startswith(".!button")}
    root.destroy()
    return result


def prog_todo(tk, ttk):
    root = tk.Tk()
    frame = tk.Frame(root)
    frame.pack(fill="both", expand=True)
    sb = tk.Scrollbar(frame)
    sb.pack(side="right", fill="y")
    lb = tk.Listbox(frame, yscrollcommand=sb.set, selectmode="extended", height=5)
    lb.pack(side="left", fill="both", expand=True)
    sb.config(command=lb.yview)
    entry = tk.Entry(root)
    entry.pack(fill="x")
    events = []
    lb.bind("<<ListboxSelect>>", lambda e: events.append("select"))

    def add():
        if entry.get():
            lb.insert("end", entry.get())
            entry.delete(0, "end")
    tk.Button(root, text="Add", command=add).pack()
    for item in ("milk", "eggs", "bread", "tea", "jam", "rice", "beans", "soap"):
        entry.insert(0, item)
        add()
    lb.selection_set(1, 3)
    sel = lb.curselection()
    lb.selection_clear(2)
    sel2 = lb.curselection()
    lb.delete(0)
    after_delete = lb.curselection()
    lb.insert(0, "first")
    root.update()
    first, last = lb.yview()
    lb.yview_moveto(1.0)
    root.update()
    moved = lb.yview()[0] > 0
    lb.see(0)
    root.update()
    result = {"size": lb.size(), "sel": sel, "sel2": sel2, "after_delete": after_delete,
              "get": lb.get(0, 2), "end": lb.get("end"), "includes": lb.selection_includes(3),
              "view_start": first == 0.0 and last < 1.0, "moved": moved,
              "back": lb.yview()[0] == 0.0, "events": events, "entry": entry.get(),
              "sb": tuple(round(v, 1) for v in sb.get())[:1]}
    root.destroy()
    return result


def prog_form(tk, ttk):
    root = tk.Tk()
    vcmd = (root.register(lambda p: p.isdigit() or p == ""), "%P")
    age = ttk.Entry(root, validate="key", validatecommand=vcmd)
    age.grid(row=0, column=1)
    age.insert(0, "42")
    age.insert("end", "x")  # rejected
    color = tk.StringVar(value="red")
    combo = ttk.Combobox(root, textvariable=color, values=("red", "green", "blue"),
                         state="readonly")
    combo.grid(row=1, column=1)
    picked = []
    combo.bind("<<ComboboxSelected>>", lambda e: picked.append(combo.get()))
    combo.current(2)
    agree = tk.BooleanVar()
    chk = ttk.Checkbutton(root, text="Agree", variable=agree)
    chk.grid(row=2, column=1)
    chk.invoke()
    level = tk.IntVar(value=1)
    radios = [ttk.Radiobutton(root, text=t, variable=level, value=v)
              for t, v in (("low", 1), ("mid", 2), ("high", 3))]
    radios[2].invoke()
    spin = tk.Spinbox(root, from_=0, to=10, increment=2)
    spin.grid(row=4, column=1)
    spin.invoke("buttonup")
    spin.invoke("buttonup")
    words = tk.Spinbox(root, values=("a", "b", "c"))
    words.invoke("buttonup")
    scale_log = []
    scale = tk.Scale(root, from_=0, to=100, orient="horizontal",
                     command=lambda v: scale_log.append(v))
    scale.set(30)
    tscale = ttk.Scale(root, from_=0, to=1)
    tscale.set(0.25)
    result = {"age": age.get(), "color": color.get(), "combo_current": combo.current(),
              "agree": agree.get(), "level": level.get(), "spin": spin.get(),
              "words": words.get(), "scale": scale.get(), "scale_log": scale_log[-1:],
              "tscale": round(tscale.get(), 2), "chk_state": "selected" in chk.state(),
              "combo_class": combo.winfo_class(), "radio_val": radios[0].cget("value")}
    chk.state(["disabled"])
    result["disabled"] = chk.instate(["disabled"])
    root.destroy()
    return result


def prog_editor(tk, ttk):
    root = tk.Tk()
    text = tk.Text(root, width=40, height=8, undo=True)
    sb = ttk.Scrollbar(root, orient="vertical", command=text.yview)
    text.configure(yscrollcommand=sb.set)
    text.pack(side="left", fill="both", expand=True)
    sb.pack(side="right", fill="y")
    menubar = tk.Menu(root)
    filemenu = tk.Menu(menubar, tearoff=0)
    log = []
    filemenu.add_command(label="New", command=lambda: log.append("new"), accelerator="Ctrl+N")
    filemenu.add_separator()
    filemenu.add_command(label="Quit", command=lambda: log.append("quit"))
    menubar.add_cascade(label="File", menu=filemenu)
    wrap = tk.BooleanVar(value=False)
    view = tk.Menu(menubar)  # with the default tearoff entry
    view.add_checkbutton(label="Wrap", variable=wrap)
    menubar.add_cascade(label="View", menu=view)
    root.config(menu=menubar)
    filemenu.invoke(0)
    filemenu.invoke("Quit")
    view.invoke(1)
    text.insert("1.0", "hello world\nsecond line\n")
    text.insert("end", "third")
    text.tag_configure("hl", foreground="red", background="yellow")
    text.tag_add("hl", "1.0", "1.5")
    text.mark_set("m", "2.3")
    text.insert("1.0", ">> ")
    pos = text.search("second", "1.0")
    count = tk.IntVar()
    text.search(r"l+", "1.0", regexp=True, count=count)
    text.delete("1.0", "1.3")
    text.edit_separator()
    text.insert("end", "!")
    text.edit_undo()
    mods = []
    text.bind("<<Modified>>", lambda e: mods.append(1))
    text.edit_modified(False)
    for _ in range(25):
        text.insert("end", "\nline")
    root.update()
    text.see("end")
    root.update()
    result = {"menu_log": log, "wrap": wrap.get(), "view_types": (view.type(0), view.type(1)),
              "end_index": filemenu.index("end"), "accel": filemenu.entrycget(0, "accelerator"),
              "content": text.get("1.0", "4.0"), "hl": text.tag_ranges("hl"),
              "mark": text.index("m"), "search": pos, "count": count.get(),
              "line3": text.get("3.0", "3.end"), "lineend": text.index("1.2 lineend"),
              "wordstart": text.index("2.3 wordstart"), "compare": text.compare("1.0", "<", "2.0"),
              "end": text.index("end"), "names": "hl" in text.tag_names("1.1"),
              "scrolled": text.yview()[1] == 1.0 and text.yview()[0] > 0,
              "sb_follows": sb.get()[1] == 1.0,
              "count_lines": text.count("1.0", "3.0", "lines")}
    root.destroy()
    return result


def prog_table(tk, ttk):
    root = tk.Tk()
    tree = ttk.Treeview(root, columns=("size", "kind"), show="headings", height=4)
    order = []

    def sort_by(col):
        rows = [(tree.set(i, col), i) for i in tree.get_children("")]
        rows.sort(reverse=True)
        for k, (_, i) in enumerate(rows):
            tree.move(i, "", k)
        order.append(col)
    for col in ("size", "kind"):
        tree.heading(col, text=col.title(), command=lambda c=col: sort_by(c))
        tree.column(col, width=80, anchor="e")
    ids = [tree.insert("", "end", values=(n, k)) for n, k in
           (("10", "a"), ("30", "b"), ("20", "c"))]
    folder = tree.insert("", "end", iid="dir", values=("5", "d"), open=False)
    child = tree.insert("dir", "end", values=("1", "e"))
    sort_by("size")
    sel = []
    tree.bind("<<TreeviewSelect>>", lambda e: sel.append(tree.selection()))
    tree.selection_set(ids[0], ids[2])
    tree.selection_add("dir")
    tree.selection_remove(ids[0])
    tree.item(ids[1], values=("31", "b2"))
    tree.set(ids[2], "kind", "z")
    tree.detach(ids[1])
    detached = tree.get_children("")
    tree.reattach(ids[1], "", 0)
    tree.see(child)
    result = {"order": order, "children": len(tree.get_children("")),
              "first": tree.set(tree.get_children("")[0], "size"),
              "selection": tree.selection(), "sel_events": len(sel) > 0,
              "item": tree.item(ids[1])["values"], "set": tree.set(ids[2], "kind"),
              "detached": len(detached), "parent": tree.parent(child),
              "next": tree.next(ids[1]) != "", "exists": tree.exists("dir"),
              "open": tree.item("dir", "open") in (True, 1),
              "heading": tree.heading("size")["text"], "index": tree.index(ids[1]),
              "focus_empty": tree.focus() in ("", ids[0], ids[2], "dir"),
              "tag_has": tree.tag_has("x")}
    tree.delete("dir")
    result["after_delete"] = len(tree.get_children(""))
    result["exists_child"] = tree.exists(child)
    root.destroy()
    return result


def prog_canvas(tk, ttk):
    root = tk.Tk()
    canvas = tk.Canvas(root, width=200, height=150, scrollregion=(0, 0, 400, 300))
    hbar = tk.Scrollbar(root, orient="horizontal", command=canvas.xview)
    canvas.configure(xscrollcommand=hbar.set)
    canvas.pack()
    hbar.pack(fill="x")
    r = canvas.create_rectangle(10, 10, 60, 40, fill="blue", tags=("box", "shape"))
    o = canvas.create_oval(100, 100, 140, 140, outline="red", tags="shape")
    ln = canvas.create_line(0, 0, 50, 50, 90, 10, arrow="last", smooth=True)
    t = canvas.create_text(30, 80, text="hi", anchor="nw")
    clicks = []
    canvas.tag_bind("box", "<Button-1>", lambda e: clicks.append("box"))
    canvas.move("shape", 5, 5)
    canvas.itemconfigure(r, fill="green")
    canvas.addtag_withtag("all_shapes", "shape")
    canvas.dtag(o, "shape")
    settle(root, canvas)
    canvas.xview_moveto(0.5)
    root.update()
    x_after = canvas.canvasx(0)
    canvas.xview_scroll(-1, "pages")
    result = {"coords": canvas.coords(r), "fill": canvas.itemcget(r, "fill"),
              "shape": canvas.find_withtag("shape"), "all_shapes": canvas.find_withtag(
                  "all_shapes"), "type": canvas.type(t), "gettags": canvas.gettags(o),
              "overlap": canvas.find_overlapping(0, 0, 70, 50)[:1],
              "enclosed": canvas.find_enclosed(0, 0, 200, 200)[:2], "above": canvas.find_above(r),
              "xview": canvas.xview()[0] < 0.5, "canvasx": x_after,
              "hbar": round(hbar.get()[0], 2), "bbox_box": canvas.bbox("box")[:2],
              "line_type": canvas.type(ln)}
    canvas.delete("shape")
    result["after_delete"] = canvas.find_all()
    root.destroy()
    return result


def prog_scrollable_frame(tk, ttk):
    root = tk.Tk()
    root.geometry("300x200")
    container = ttk.Frame(root)
    canvas = tk.Canvas(container, height=150)
    scrollbar = ttk.Scrollbar(container, orient="vertical", command=canvas.yview)
    inner = ttk.Frame(canvas)
    inner.bind("<Configure>", lambda e: canvas.configure(scrollregion=canvas.bbox("all")))
    win = canvas.create_window((0, 0), window=inner, anchor="nw")
    canvas.configure(yscrollcommand=scrollbar.set)
    for i in range(30):
        ttk.Label(inner, text="Row %d" % i).pack(anchor="w")
    container.pack(fill="both", expand=True)
    canvas.pack(side="left", fill="both", expand=True)
    scrollbar.pack(side="right", fill="y")
    settle(root, inner)
    region = canvas.cget("scrollregion")
    canvas.yview_moveto(1.0)
    root.update()
    result = {"region_set": bool(region), "win_type": canvas.type(win),
              "scrolled": canvas.yview()[1] == 1.0, "sb": scrollbar.get()[1] == 1.0,
              "inner_children": len(inner.winfo_children()),
              "toplevel": inner.winfo_toplevel() is root}
    root.destroy()
    return result


def prog_widgets(tk, ttk):
    root = tk.Tk()
    nb = ttk.Notebook(root)
    tab1, tab2 = ttk.Frame(nb), ttk.Frame(nb)
    nb.add(tab1, text="One")
    nb.add(tab2, text="Two")
    nb.pack(fill="both", expand=True)
    changes = []
    nb.bind("<<NotebookTabChanged>>", lambda e: changes.append(nb.index("current")))
    nb.select(tab2)
    pw = tk.PanedWindow(root, orient="horizontal")
    left, right = tk.Label(pw, text="L"), tk.Label(pw, text="R")
    pw.add(left)
    pw.add(right)
    pw.pack(fill="both", expand=True)
    var = tk.StringVar(value="b")
    om = tk.OptionMenu(root, var, "a", "b", "c")
    om.pack()
    om["menu"].invoke(2)
    menu = om["menu"]
    menu.delete(0, "end")
    for v in ("x", "y"):
        menu.add_command(label=v, command=tk._setit(var, v))
    menu.invoke(1)
    mb = ttk.Menubutton(root, text="Menu")
    mbm = tk.Menu(mb, tearoff=0)
    mb["menu"] = mbm
    mbm.add_radiobutton(label="r1", variable=var, value="r1")
    mbm.invoke(0)
    pb = ttk.Progressbar(root, maximum=10, value=3)
    pb.step(2)
    sep = ttk.Separator(root, orient="horizontal")
    sep.pack(fill="x")
    lf = tk.LabelFrame(root, text="Group")
    lf.pack()
    msg = tk.Message(root, text="a long message " * 4, width=120)
    msg.pack()
    lbl = tk.Label(root, text="x", font=("Arial", 18, "bold"), fg="red", cursor="hand2")
    lbl.pack()
    img = tk.PhotoImage(width=4, height=3)
    img.put("#ff0000", to=(0, 0, 4, 3))
    img.put("#00ff00", (1, 1))
    lbl2 = tk.Label(root, image=img)
    lbl2.pack()
    style = ttk.Style()
    style.configure("My.TButton", padding=6)
    top = tk.Toplevel(root)
    top.title("second")
    top_lbl = tk.Label(top, text="top")
    top_lbl.pack()
    root.update()
    result = {"nb": (nb.index("current"), len(nb.tabs()), nb.tab(tab1, "text")),
              "nb_changes": changes[-1:], "panes": len(pw.panes()), "var": var.get(),
              "pb": float(pb["value"]), "lf": lf.cget("text"),
              "img": (img.width(), img.height(), img.get(1, 1), img.get(0, 0)),
              "style": style.configure("My.TButton")["padding"] if isinstance(
                  style.configure("My.TButton"), dict) else 6,
              "top_title": top.title(), "top_parent": top_lbl.winfo_toplevel() is top,
              "class_top": top.winfo_class(), "lbl_font": lbl.cget("font"),
              "keys": "text" in lbl.keys(), "cursor": lbl["cursor"]}
    top.destroy()
    result["top_gone"] = not top.winfo_exists()
    root.destroy()
    return result


def prog_events(tk, ttk):
    root = tk.Tk()
    log = []
    btn = tk.Button(root, text="b")
    btn.pack()
    settle(root, btn)  # Tk delivers events to mapped windows
    btn.bind("<<Custom>>", lambda e: log.append(("custom", e.widget is btn)))
    btn.bind("<Control-s>", lambda e: log.append("ctrl-s"))
    root.bind_all("<<Global>>", lambda e: log.append("global"))
    root.bind_class("Button", "<<Custom>>", lambda e: log.append("class"), add="+")
    btn.event_generate("<<Custom>>")
    btn.event_generate("<<Global>>")
    btn.bind("<<Stop>>", lambda e: (log.append("stop"), "break")[1])
    root.bind_class("Button", "<<Stop>>", lambda e: log.append("not reached"))
    btn.event_generate("<<Stop>>")
    root.event_add("<<Save>>", "<Control-q>")
    btn.bind("<<Save>>", lambda e: log.append("save"))
    btn.event_generate("<Control-q>")
    tags = btn.bindtags()
    fired = []
    v = tk.StringVar()
    cb = v.trace_add("write", lambda *a: fired.append(v.get()))
    v.set("one")
    v.trace_remove("write", cb)
    v.set("two")
    after_ids = []
    aid = root.after(10000, lambda: None)
    after_ids.append(aid in root.after_info() if hasattr(root, "after_info") else True)
    root.after_cancel(aid)
    ran = []
    root.after(1, lambda: ran.append(1))
    for _ in range(50):
        root.update()
        if ran:
            break
        root.after(5)
    result = {"log": log, "tags": (tags[1], tags[2], tags[3]), "traces": fired,
              "after_known": after_ids, "after_ran": ran, "getvar": root.getvar(str(v)),
              "bool": root.getboolean("yes"), "int": root.winfo_pixels("1i") > 50,
              "rgb": root.winfo_rgb("#ff0000")[0] > 60000}
    root.destroy()
    return result


PROGRAMS = [prog_calculator, prog_todo, prog_form, prog_editor, prog_table, prog_canvas,
            prog_scrollable_frame, prog_widgets, prog_events]


# ==========================================================================
# Harness


def _bui_modules():
    import bui.tksyntax as tk
    from bui.tksyntax import ttk
    return tk, ttk


def _real_modules():
    try:
        from bui import _realtk
        tk = _realtk.module("tkinter")
        import importlib
        ttk = importlib.import_module("tkinter.ttk")
        # Tk 9 on macOS: the next Tk() crashes if the only root was destroyed before
        # its idle events ran
        root = tk.Tk()
        root.update_idletasks()
        root.destroy()
        return tk, ttk
    except Exception:
        return None


def _normalize(v):
    """Values that legitimately differ in type between Tcl and Python."""
    if hasattr(v, "string"):  # Tcl_Obj
        return str(v)
    if isinstance(v, tuple):
        return tuple(_normalize(x) for x in v)
    if isinstance(v, list):
        return [_normalize(x) for x in v]
    if isinstance(v, dict):
        return {k: _normalize(x) for k, x in v.items()}
    if isinstance(v, (int, float)) and not isinstance(v, bool):
        return float(v)
    return v


# Observations that depend on fonts / pixel metrics: compared loosely.
_LOOSE = {"prog_canvas": {"canvasx", "hbar", "xview"}, "prog_widgets": {"lbl_font"},
          "prog_todo": {"sb"}, "prog_editor": {"scrolled", "sb_follows"},
          "prog_scrollable_frame": {"scrolled", "sb", "region_set"}}
# ... and what BUI must observe for them (stdlib Tk's depends on window mapping timing)
_BUI_EXPECT = {"prog_editor": {"scrolled": True, "sb_follows": True},
               "prog_canvas": {"xview": True, "canvasx": 200.0},
               "prog_scrollable_frame": {"scrolled": True, "sb": True, "region_set": True}}


def test_programs_match_stdlib():
    real = _real_modules()
    bui_tk, bui_ttk = _bui_modules()
    for prog in PROGRAMS:
        got = prog(bui_tk, bui_ttk)
        for key, value in _BUI_EXPECT.get(prog.__name__, {}).items():
            assert got[key] == value, "%s[%s]: bui %r" % (prog.__name__, key, got[key])
        if real is None:
            print("ran", prog.__name__, "(no stdlib Tk to compare)")
            continue
        want = prog(*real)
        loose = _LOOSE.get(prog.__name__, set())
        for key, ref in want.items():
            if key in loose:
                continue
            a, b = _normalize(got.get(key)), _normalize(ref)
            assert a == b, "%s[%s]: bui %r != tkinter %r" % (prog.__name__, key, a, b)
        print("ok", prog.__name__)


def test_install_runs_unchanged_scripts():
    """`import tkinter` after install() is BUI; dialogs modules resolve too."""
    import importlib
    import bui.tksyntax as tks
    saved = {k: v for k, v in sys.modules.items() if k == "tkinter" or k.startswith("tkinter.")}
    try:
        tks.install()
        tkinter = importlib.import_module("tkinter")
        from tkinter import messagebox, ttk  # noqa: F401
        from tkinter.scrolledtext import ScrolledText
        assert tkinter is tks and ttk is tks.ttk
        root = tkinter.Tk()
        st = ScrolledText(root, height=4)
        st.pack(fill="both", expand=True)
        st.insert("end", "x\n" * 20)
        root.update()
        assert st.vbar.get()[1] <= 1.0 and isinstance(st, tkinter.Text)
        root.destroy()
    finally:
        for k in [k for k in sys.modules if k == "tkinter" or k.startswith("tkinter.")]:
            del sys.modules[k]
        sys.modules.update(saved)


def test_hidden_root_is_not_default_root():
    """BUI's hidden Tk root (macOS, the Tk host) must not capture a real tkinter
    program's variables: they belong to the program's own Tk()."""
    real = _real_modules()
    if real is None:
        print("skipped hidden root test (no stdlib Tk)")
        return
    tk = real[0]
    from bui import host
    host._tk()
    root = tk.Tk()
    var = tk.StringVar()
    entry = tk.Entry(root, textvariable=var)
    var.set("linked")
    seen = entry.get()
    root.destroy()
    assert seen == "linked", "StringVar went to the hidden root: Entry shows %r" % seen


_FIT_AT_SCALE = """
import bui, buipy
from bui import host
from bui.tksyntax import Label, Listbox, Tk, Toplevel, simpledialog
if host.native_available():
    root = Tk()
    Listbox(root, height=5).pack()
    root.update()
    top = Toplevel(root)
    for _ in range(4):  # larger than the dialog's minimum size
        Label(top, text="wider than the minimum " * 5).pack()
    simpledialog._place_window(top)
    root.update()
    for win in (root, top):
        need, got = win._node.natural(), win._window.size
        assert 0 <= got[0] - need[0] <= 1 and 0 <= got[1] - need[1] <= 1, (need, got)
    assert buipy.kit_scale() == 2.0
    root.destroy()
"""


def test_window_fits_content_at_ui_scale():
    """HiDPI: a window fitting its content isn't scaled twice (natural() is in raster
    pixels). In a child process: the UI scale is read when the first window opens."""
    env = dict(os.environ, BUI_UI_SCALE="2", PYTHONPATH=os.pathsep.join(sys.path))
    subprocess.run([sys.executable, "-c", _FIT_AT_SCALE], env=env, check=True, timeout=60)


def main() -> int:
    test_hidden_root_is_not_default_root()
    test_window_fits_content_at_ui_scale()
    test_programs_match_stdlib()
    test_install_runs_unchanged_scripts()
    print("tkinter compatibility tests passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
