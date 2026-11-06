"""Images through the kit's decoders: every format from Python, tkinter
PhotoImage beyond Tk's own formats, and image files on labels and canvas
items decoded off the UI thread.

SPDX-License-Identifier: GPL-2.0-or-later
"""

import os
import sys
import time

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))

import bui  # noqa: E402,F401  (locates buipy)
import buipy  # noqa: E402

IMAGES = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "..", "tests", "images")


def path(name):
    return os.path.join(IMAGES, name)


def read(name):
    with open(path(name), "rb") as fh:
        return fh.read()


def test_decode_every_format():
    for name, fmt, size in [("quad.png", "png", (8, 6)), ("quad.jpg", "jpeg", (8, 6)),
                            ("quad.webp", "webp", (8, 6)), ("quad.gif", "gif", (8, 6)),
                            ("quad.bmp", "bmp", (8, 6)), ("quad.tga", "tga", (8, 6)),
                            ("quad.qoi", "qoi", (8, 6)), ("quad.hdr", "hdr", (8, 6)),
                            ("quad.ppm", "pnm", (8, 6)), ("halves.svg", "svg", (16, 12))]:
        data = read(name)
        assert buipy.image_probe(data) == (fmt, *size, 1), (name, buipy.image_probe(data))
        got, w, h, frames = buipy.image_decode(data)
        assert (got, w, h, len(frames)) == (fmt, *size, 1), name
        assert len(frames[0][0]) == w * h * 4
    _, w, h, frames = buipy.image_decode(read("quad.png"))
    assert frames[0][0][:4] == bytes((255, 0, 0, 255))
    _, w, h, _ = buipy.image_decode(read("quad.jpg"), 4, 4)  # DCT-scaled
    assert (w, h) == (4, 3)
    _, _, _, frames = buipy.image_decode(read("anim.webp"))
    assert len(frames) == 3 and frames[0][1] == 100
    tex, w, h = buipy.image_texture(read("quad.png"))  # a Dear ImGui ImTextureID
    assert tex > 0 and (w, h) == (8, 6)
    buipy.texture_free(tex)
    assert buipy.image_probe(b"nope") is None
    try:
        buipy.image_decode(b"nope")
    except ValueError:
        pass
    else:
        raise AssertionError("bad data must raise")


def test_photoimage_new_formats():
    from bui.tksyntax import PhotoImage, Tk
    root = Tk()
    for name in ("quad.jpg", "quad.webp", "quad.bmp", "quad.tga", "quad.qoi", "quad.hdr"):
        img = PhotoImage(file=path(name))
        assert (img.width(), img.height()) == (8, 6), name
        r, g, b = img.get(1, 1)
        assert r > 200 and g < 60 and b < 60, (name, (r, g, b))
    svg = PhotoImage(file=path("halves.svg"))
    assert (svg.width(), svg.height()) == (16, 12)
    frame2 = PhotoImage(file=path("anim.gif"), format="gif -index 2")
    assert frame2.width() == 8
    root.destroy()


def test_image_component():
    import bui
    from bui.components import Image
    img = Image(path("quad.png"), fit="cover")
    assert img.natural() == (8, 6), img.natural()  # from the header, before any pixels
    assert img.src.endswith("quad.png")
    assert bui.image_probe(read("quad.jpg"))[0] == "jpeg"
    try:
        Image(path("quad.png"), fit="stretch")
    except ValueError:
        pass
    else:
        raise AssertionError("bad fit must raise")


def test_animations():
    import time

    import bui
    data = read("anim.gif")
    fmt, w, h, px = bui.image_decode_frame(data, 2)  # one frame alone
    assert (fmt, w, h) == ("gif", 8, 8) and px[2] > 200 and px[0] < 30  # blue
    tex = bui.AnimatedTexture(read("anim.webp"), 4, 4)  # for Dear ImGui
    assert tex.texture > 0 and tex.size == (4, 4) and tex.frames == 3
    changed = 0
    for _ in range(30):
        time.sleep(0.01)
        changed += tex.update()
    assert 2 <= changed <= 4, changed  # 100 ms frames over ~0.3 s
    tex.close()
    from bui.components import Image
    img = Image(path("anim.gif"), frame=1)
    assert img.frame == 1
    img.frame = None  # plays
    try:
        bui.image_decode_frame(data, 3)
    except ValueError:
        pass
    else:
        raise AssertionError("frame 3 of 3 must raise")


def wait_decodes(ctx, r, w, h):
    for _ in range(2000):
        if buipy.image_stats()["pending"] == 0:
            break
        time.sleep(0.001)
    buipy.image_poll()
    ctx.render(w, h, r, False)


def test_files_on_labels_and_canvas():
    from bui.tksyntax import Canvas, Label, Tk
    root = Tk()
    lbl = Label(root)
    lbl.pack()
    lbl._node.set_image_source(path("quad.png"))  # header read now: the label has its size
    canvas = Canvas(root, width=64, height=40)
    canvas.pack()
    canvas._node.canvas_set(9001, 6, [0, 0, 32, 24], src=path("quad.jpg"), fit=0)  # BK_ITEM_IMAGE, fill
    ctx = root._ctx
    r = buipy.Raster(200, 200)
    ctx.render(200, 200, r, True)
    wait_decodes(ctx, r, 200, 200)
    lx, ly = lbl._node.rect[:2]
    lw, lh = lbl._node.rect[2:4]
    assert (lw, lh) >= (8, 6)
    cx_, cy_ = lx + (lw - 8) // 2, ly + (lh - 6) // 2
    assert r.pixel(cx_ + 1, cy_ + 1) == (255, 0, 0), r.pixel(cx_ + 1, cy_ + 1)
    kx, ky = canvas._node.rect[:2]
    red = r.pixel(kx + 2, ky + 2)
    assert red[0] > 200 and red[1] < 60, red
    st = buipy.image_stats()
    assert st["decoded"] >= 2 and st["pending"] == 0, st
    root.destroy()


def main():
    test_decode_every_format()
    test_photoimage_new_formats()
    test_image_component()
    test_animations()
    test_files_on_labels_and_canvas()
    print("image tests passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
