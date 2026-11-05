"""Lossless PNG writing / re-encoding with the standard library only.

    python tools/pngopt.py image.png [...]      # recompress in place

write_png(path, width, height, rgb_bytes) writes 8-bit RGB; recompress(path)
decodes any 8-bit RGB/RGBA PNG, drops an all-opaque alpha channel, picks the
best of several row-filter strategies and deflates at zlib level 9.

SPDX-License-Identifier: GPL-2.0-or-later
"""
from __future__ import annotations

import struct
import sys
import zlib


def _chunk(kind: bytes, body: bytes) -> bytes:
    return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body))


def _paeth_row(row: bytes, prev: bytes, bpp: int) -> bytes:
    out = bytearray(len(row))
    for i in range(len(row)):
        a = row[i - bpp] if i >= bpp else 0
        b = prev[i]
        c = prev[i - bpp] if i >= bpp else 0
        p = a + b - c
        pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
        pr = a if pa <= pb and pa <= pc else (b if pb <= pc else c)
        out[i] = (row[i] - pr) & 255
    return bytes(out)


def _filter_rows(pix: bytes, w: int, h: int, bpp: int):
    """Each row in filters 0 (None), 1 (Sub), 2 (Up), 4 (Paeth)."""
    stride = w * bpp
    prev = bytes(stride)
    rows = []
    for y in range(h):
        row = pix[y * stride:(y + 1) * stride]
        sub = bytes((row[i] - (row[i - bpp] if i >= bpp else 0)) & 255 for i in range(stride))
        up = bytes((row[i] - prev[i]) & 255 for i in range(stride))
        if row == prev:
            paeth = up  # identical rows: Up is all zeros, Paeth too
        else:
            paeth = _paeth_row(row, prev, bpp)
        rows.append({0: row, 1: sub, 2: up, 4: paeth})
        prev = row
    return rows


def _encode(pix: bytes, w: int, h: int, bpp: int) -> bytes:
    rows = _filter_rows(pix, w, h, bpp)

    def cost(r: bytes) -> int:  # the usual minimum-sum-of-absolute-differences heuristic
        return sum(v if v < 128 else 256 - v for v in r)

    candidates = []
    for f in (0, 1, 2, 4):
        candidates.append(b"".join(bytes([f]) + r[f] for r in rows))
    adaptive = bytearray()
    for r in rows:
        f = min((0, 1, 2, 4), key=lambda k: cost(r[k]))
        adaptive += bytes([f]) + r[f]
    candidates.append(bytes(adaptive))
    best = min((zlib.compress(c, 9) for c in candidates), key=len)
    return best


def encode_png(w: int, h: int, pix: bytes, channels: int = 3) -> bytes:
    ctype = {3: 2, 4: 6}[channels]
    ihdr = struct.pack(">IIBBBBB", w, h, 8, ctype, 0, 0, 0)
    return (b"\x89PNG\r\n\x1a\n" + _chunk(b"IHDR", ihdr) +
            _chunk(b"IDAT", _encode(pix, w, h, channels)) + _chunk(b"IEND", b""))


def write_png(path: str, w: int, h: int, rgb: bytes) -> int:
    data = encode_png(w, h, rgb, 3)
    with open(path, "wb") as fh:
        fh.write(data)
    return len(data)


def read_png(path: str):
    """(width, height, channels, pixel bytes) of an 8-bit RGB / RGBA PNG."""
    data = open(path, "rb").read()
    assert data[:8] == b"\x89PNG\r\n\x1a\n", path
    i, idat = 8, b""
    while i < len(data):
        n, kind = struct.unpack(">I4s", data[i:i + 8])
        body = data[i + 8:i + 8 + n]
        if kind == b"IHDR":
            w, h, depth, ctype, _, _, interlace = struct.unpack(">IIBBBBB", body)
            assert depth == 8 and interlace == 0 and ctype in (2, 6), (path, depth, ctype)
            bpp = 3 if ctype == 2 else 4
        elif kind == b"IDAT":
            idat += body
        i += 12 + n
    raw = zlib.decompress(idat)
    stride = w * bpp
    out = bytearray()
    prev = bytearray(stride)
    p = 0
    for _ in range(h):
        f = raw[p]
        line = bytearray(raw[p + 1:p + 1 + stride])
        p += 1 + stride
        if f == 1:
            for x in range(bpp, stride):
                line[x] = (line[x] + line[x - bpp]) & 255
        elif f == 2:
            for x in range(stride):
                line[x] = (line[x] + prev[x]) & 255
        elif f == 3:
            for x in range(stride):
                a = line[x - bpp] if x >= bpp else 0
                line[x] = (line[x] + (a + prev[x]) // 2) & 255
        elif f == 4:
            for x in range(stride):
                a = line[x - bpp] if x >= bpp else 0
                b = prev[x]
                c = prev[x - bpp] if x >= bpp else 0
                pp = a + b - c
                pa, pb, pc = abs(pp - a), abs(pp - b), abs(pp - c)
                pr = a if pa <= pb and pa <= pc else (b if pb <= pc else c)
                line[x] = (line[x] + pr) & 255
        out += line
        prev = line
    return w, h, bpp, bytes(out)


def recompress(src: str, dst: str = None) -> int:
    w, h, ch, pix = read_png(src)
    if ch == 4 and all(a == 255 for a in pix[3::4]):
        rgb = bytearray(w * h * 3)
        rgb[0::3], rgb[1::3], rgb[2::3] = pix[0::4], pix[1::4], pix[2::4]
        pix, ch = bytes(rgb), 3
    data = encode_png(w, h, pix, ch)
    old = open(src, "rb").read()
    if dst is None and len(data) >= len(old):
        return len(old)
    with open(dst or src, "wb") as fh:
        fh.write(data)
    return len(data)


if __name__ == "__main__":
    for p in sys.argv[1:]:
        before = len(open(p, "rb").read())
        print("%s: %d -> %d bytes" % (p, before, recompress(p)))
