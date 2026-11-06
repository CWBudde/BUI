"""tkinter images: Image, PhotoImage, BitmapImage.

Pixels are RGBA bytes handed to the kit (labels, buttons, canvas items).
Every format is decoded by the kit's image module (the same C decoders as
every BUI binding): PNG, GIF (frames by `format="gif -index n"`), PPM/PGM,
and beyond Tk's own: JPEG, WebP, BMP, TGA, QOI, Radiance HDR, SVG. PNG and
PPM/PGM are encoded here (stdlib zlib) for `write` and `data`.

SPDX-License-Identifier: GPL-2.0-or-later
"""

from __future__ import annotations

import base64
import itertools
import re
import struct
import weakref
import zlib

import buipy

from ._core import TclError, color

_image_ids = itertools.count(1)
_images: "weakref.WeakValueDictionary[str, Image]" = weakref.WeakValueDictionary()


def image_names():
    return tuple(_images)


def image_types():
    return ("photo", "bitmap")


class Image:
    """Base class for images (tkinter.Image)."""

    _TYPE = "image"

    def __init__(self, imgtype, name=None, cnf={}, master=None, **kw):  # noqa: B006
        self.name = name or "pyimage%d" % next(_image_ids)
        self._opts = {}
        self._w = self._h = 0
        self._px = bytearray()
        self._users: "weakref.WeakSet" = weakref.WeakSet()
        _images[self.name] = self
        kw = dict(cnf, **kw) if cnf else kw
        self.configure(**kw)

    def __str__(self):
        return self.name

    def __del__(self):
        pass

    def __setitem__(self, key, value):
        self.configure(**{key: value})

    def __getitem__(self, key):
        return self.cget(key)

    def configure(self, **kw):
        for k, v in kw.items():
            self._opts[k] = v
        self._configured(kw)

    config = configure

    def cget(self, key):
        return self._opts.get(key.lstrip("-"), "")

    def height(self):
        return self._h

    def width(self):
        return self._w

    def type(self):
        return self._TYPE

    def _configured(self, kw):
        pass

    # -- kit side -----------------------------------------------------------------
    def rgba(self) -> bytes:
        return bytes(self._px)

    def _attach(self, user):
        """`user._image_changed()` runs when the pixels change."""
        self._users.add(user)

    def _changed(self):
        for u in list(self._users):
            u._image_changed(self)


class PhotoImage(Image):
    """Widget which can display images in PGM, PPM, GIF and PNG format
    (and, through the kit's decoders, JPEG, WebP, BMP, TGA, QOI, HDR, SVG)."""

    _TYPE = "photo"

    def __init__(self, name=None, cnf={}, master=None, **kw):  # noqa: B006
        super().__init__("photo", name, cnf, master, **kw)

    def _configured(self, kw):
        if "width" in kw or "height" in kw:
            w = int(kw.get("width", self._w) or 0)
            h = int(kw.get("height", self._h) or 0)
            self._resize(w, h)
        if kw.get("file") is not None:
            with open(kw["file"], "rb") as fh:
                self._load(fh.read(), kw.get("format"))
        elif kw.get("data") is not None:
            data = kw["data"]
            raw = data.encode("latin-1") if isinstance(data, str) else bytes(data)
            if buipy.image_probe(raw) is None:  # Tk takes base64 too
                try:
                    raw = base64.b64decode(raw, validate=False)
                except ValueError:
                    pass
            self._load(raw, kw.get("format"))
        if kw:
            self._changed()

    def _resize(self, w, h):
        old, ow, oh = self._px, self._w, self._h
        self._w, self._h = max(0, w), max(0, h)
        self._px = bytearray(self._w * self._h * 4)
        for y in range(min(oh, self._h)):
            n = min(ow, self._w) * 4
            self._px[y * self._w * 4:y * self._w * 4 + n] = old[y * ow * 4:y * ow * 4 + n]

    # -- decoding ------------------------------------------------------------------
    def _load(self, raw: bytes, fmt=None):
        w, h, px = _decode(raw, fmt)
        if not self._opts.get("width"):
            self._w = w
        if not self._opts.get("height"):
            self._h = h
        if (self._w, self._h) == (w, h):
            self._px = bytearray(px)
        else:  # fixed-size image: copy what fits
            self._px = bytearray(self._w * self._h * 4)
            self._paste(px, w, h, 0, 0)

    def _paste(self, px, w, h, x0, y0):
        for y in range(h):
            ty = y0 + y
            if 0 <= ty < self._h:
                xa, xb = max(0, x0), min(self._w, x0 + w)
                if xb > xa:
                    sa = (y * w + (xa - x0)) * 4
                    self._px[(ty * self._w + xa) * 4:(ty * self._w + xb) * 4] = \
                        px[sa:sa + (xb - xa) * 4]

    # -- pixels ---------------------------------------------------------------------
    def blank(self):
        """Display a transparent image."""
        self._px = bytearray(self._w * self._h * 4)
        self._changed()

    def get(self, x, y):
        """Return the color (red, green, blue) of the pixel at X,Y."""
        x, y = int(x), int(y)
        if not (0 <= x < self._w and 0 <= y < self._h):
            raise TclError('photo get: coordinates out of range')
        i = 4 * (y * self._w + x)
        return tuple(self._px[i:i + 3])

    def put(self, data, to=None):
        """Put row formatted colors to image starting from position TO,
        e.g. image.put("{red green} {blue yellow}", to=(4,6))."""
        x0, y0 = (to or (0, 0))[:2]
        if isinstance(data, str):
            rows = re.findall(r"\{([^}]*)\}", data) or [data]
            rows = [r.split() for r in rows]
        else:
            rows = [r.split() if isinstance(r, str) else list(r) for r in data]
            if rows and not isinstance(rows[0], list):
                rows = [rows]
        if to is not None and len(to) == 4:  # fill a rectangle with the pattern
            x1, y1 = to[2], to[3]
            for y in range(y0, y1):
                for x in range(x0, x1):
                    row = rows[(y - y0) % len(rows)]
                    self._put_pixel(x, y, row[(x - x0) % len(row)])
        else:
            for dy, row in enumerate(rows):
                for dx, c in enumerate(row):
                    self._put_pixel(x0 + dx, y0 + dy, c)
        self._changed()

    def _put_pixel(self, x, y, c):
        if x >= self._w or y >= self._h:  # Tk grows unsized images
            if not self._opts.get("width") and not self._opts.get("height"):
                self._resize(max(self._w, x + 1), max(self._h, y + 1))
            else:
                return
        if x < 0 or y < 0:
            return
        rgba = color(c)
        i = 4 * (y * self._w + x)
        self._px[i:i + 4] = bytes(int(round(v * 255)) for v in rgba)

    def transparency_get(self, x, y):
        """Return True if the pixel at x,y is transparent."""
        return self._px[4 * (int(y) * self._w + int(x)) + 3] == 0

    def transparency_set(self, x, y, boolean):
        """Set the transparency of the pixel at x,y."""
        self._px[4 * (int(y) * self._w + int(x)) + 3] = 0 if boolean else 255
        self._changed()

    # -- copies ------------------------------------------------------------------------
    def _region(self, from_coords):
        if not from_coords:
            return 0, 0, self._w, self._h
        c = list(from_coords)
        x0, y0 = int(c[0]), int(c[1])
        x1, y1 = (int(c[2]), int(c[3])) if len(c) >= 4 else (self._w, self._h)
        return x0, y0, x1, y1

    def _sample(self, sx, sy, zx=1, zy=1, from_coords=None):
        x0, y0, x1, y1 = self._region(from_coords)
        w = max(1, ((x1 - x0) * zx + sx - 1) // sx)
        h = max(1, ((y1 - y0) * zy + sy - 1) // sy)
        out = PhotoImage(width=w, height=h)
        for y in range(h):
            syy = y0 + (y * sy) // zy
            for x in range(w):
                sxx = x0 + (x * sx) // zx
                si, di = 4 * (syy * self._w + sxx), 4 * (y * w + x)
                out._px[di:di + 4] = self._px[si:si + 4]
        out._opts.pop("width", None)
        out._opts.pop("height", None)
        return out

    def copy(self, *, from_coords=None, zoom=None, subsample=None):
        """Return a new PhotoImage with the same image as this widget."""
        z = _pair(zoom) if zoom else (1, 1)
        s = _pair(subsample) if subsample else (1, 1)
        return self._sample(s[0], s[1], z[0], z[1], from_coords)

    def zoom(self, x, y="", *, from_coords=None):
        """Return a new PhotoImage with the same image as this widget but
        zoom it with a factor of X in the X direction and Y in the Y direction."""
        return self._sample(1, 1, int(x), int(y or x), from_coords)

    def subsample(self, x, y="", *, from_coords=None):
        """Return a new PhotoImage based on the same image as this widget
        but use only every Xth or Yth pixel."""
        return self._sample(int(x), int(y or x), 1, 1, from_coords)

    def copy_replace(self, sourceImage, *, from_coords=None, to=None, shrink=False, zoom=None,
                     subsample=None, compositingrule=None):
        """Copy a region from the source image into this image."""
        src = sourceImage if isinstance(sourceImage, PhotoImage) else _images[str(sourceImage)]
        part = src.copy(from_coords=from_coords, zoom=zoom, subsample=subsample)
        tx, ty = (to or (0, 0))[:2]
        if shrink:
            self._resize(tx + part._w, ty + part._h)
        elif tx + part._w > self._w or ty + part._h > self._h:
            if not self._opts.get("width") and not self._opts.get("height"):
                self._resize(max(self._w, tx + part._w), max(self._h, ty + part._h))
        self._paste(part._px, part._w, part._h, tx, ty)
        self._changed()

    # -- files -------------------------------------------------------------------------------
    def read(self, filename, format=None, *, from_coords=None, to=None, shrink=False):
        """Reads image data from the file named FILENAME into the image."""
        src = PhotoImage(file=filename, format=format)
        self.copy_replace(src, from_coords=from_coords, to=to, shrink=shrink)

    def data(self, format=None, *, from_coords=None, background=None, grayscale=False):
        """Return image data: rows of colors, or (format='png'/'ppm') encoded."""
        img = self.copy(from_coords=from_coords) if from_coords else self
        if format is None:
            return tuple(" ".join("#%02x%02x%02x" % tuple(img._px[4 * (y * img._w + x):
                                                                   4 * (y * img._w + x) + 3])
                                  for x in range(img._w)) for y in range(img._h))
        return base64.b64encode(img._encode(str(format).split()[0].lower(), background,
                                            grayscale)).decode("ascii")

    def write(self, filename, format=None, from_coords=None, *, background=None,
              grayscale=False):
        """Write image to file FILENAME in FORMAT (png, ppm, pgm)."""
        img = self.copy(from_coords=from_coords) if from_coords else self
        fmt = (format or str(filename).rsplit(".", 1)[-1]).split()[0].lower()
        with open(filename, "wb") as fh:
            fh.write(img._encode(fmt, background, grayscale))

    def _encode(self, fmt, background=None, grayscale=False):
        px = self._px
        if background is not None or grayscale or fmt in ("ppm", "pgm", "pnm"):
            bg = color(background) if background else (0, 0, 0, 1)
            flat = bytearray()
            for i in range(0, len(px), 4):
                a = px[i + 3] / 255.0
                rgb = [px[i + k] * a + bg[k] * 255 * (1 - a) for k in range(3)]
                if grayscale:
                    rgb = [0.299 * rgb[0] + 0.587 * rgb[1] + 0.114 * rgb[2]] * 3
                flat.extend(int(round(v)) for v in rgb)
                flat.append(255)
            px = flat
        if fmt in ("ppm", "pnm"):
            rgb = bytes(b for i, b in enumerate(px) if i % 4 != 3)
            return b"P6\n%d %d\n255\n" % (self._w, self._h) + rgb
        if fmt == "pgm":
            return b"P5\n%d %d\n255\n" % (self._w, self._h) + bytes(px[0::4])
        if fmt == "png":
            return _encode_png(self._w, self._h, bytes(px))
        raise TclError('image format "%s" is not supported' % fmt)


class BitmapImage(Image):
    """Widget which can display images in XBM format."""

    _TYPE = "bitmap"

    def __init__(self, name=None, cnf={}, master=None, **kw):  # noqa: B006
        super().__init__("bitmap", name, cnf, master, **kw)

    def _configured(self, kw):
        data = self._opts.get("data")
        if self._opts.get("file"):
            with open(self._opts["file"], encoding="latin-1") as fh:
                data = fh.read()
        if not data:
            return
        w, h, bits = _decode_xbm(data)
        mask = None
        maskdata = self._opts.get("maskdata")
        if self._opts.get("maskfile"):
            with open(self._opts["maskfile"], encoding="latin-1") as fh:
                maskdata = fh.read()
        if maskdata:
            mask = _decode_xbm(maskdata)[2]
        fg = color(self._opts.get("foreground", "black"))
        bg = color(self._opts.get("background")) if self._opts.get("background") else None
        self._w, self._h = w, h
        px = bytearray(w * h * 4)
        for i, on in enumerate(bits):
            c = fg if on else bg
            if mask is not None and not mask[i]:
                c = None
            if c is not None:
                px[4 * i:4 * i + 4] = bytes(int(round(v * 255)) for v in c)
        self._px = px
        self._changed()


# ==========================================================================
# Codecs


def _pair(v):
    if isinstance(v, (tuple, list)):
        return int(v[0]), int(v[1] if len(v) > 1 else v[0])
    return int(v), int(v)


def _decode(raw: bytes, fmt=None):
    """(w, h, RGBA bytes) through the kit's decoders; `fmt` may pick a frame
    of an animation the Tk way: "gif -index 2"."""
    m = re.search(r"-index\s+(\d+)", str(fmt or ""))
    if buipy.image_probe(raw) is None:
        raise TclError("couldn't recognize image data")
    try:  # one frame, decoded alone (an animation is not decoded whole)
        _, w, h, px = buipy.image_decode_frame(raw, int(m.group(1)) if m else 0)
    except ValueError:
        raise TclError('no image data for this index') from None
    return w, h, px


def _encode_png(w, h, rgba: bytes) -> bytes:
    def chunk(kind, body):
        return struct.pack(">I", len(body)) + kind + body + \
            struct.pack(">I", zlib.crc32(kind + body) & 0xFFFFFFFF)
    raw = b"".join(b"\x00" + rgba[y * w * 4:(y + 1) * w * 4] for y in range(h))
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 6)) + chunk(b"IEND", b""))


def _decode_xbm(text: str):
    w = int(re.search(r"_width\s+(\d+)", text).group(1))
    h = int(re.search(r"_height\s+(\d+)", text).group(1))
    body = text[text.index("{") + 1:text.rindex("}")]
    data = [int(v, 16) for v in re.findall(r"0x[0-9a-fA-F]+", body)]
    per_row = (w + 7) // 8
    bits = []
    for y in range(h):
        for x in range(w):
            byte = data[y * per_row + x // 8] if y * per_row + x // 8 < len(data) else 0
            bits.append((byte >> (x % 8)) & 1)
    return w, h, bits
