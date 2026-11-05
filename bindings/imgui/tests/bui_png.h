/* Minimal PNG writer for raster screenshots (0x00RRGGBB rows, as BUI_raster_pixels
 * returns them): 8-bit RGB, zlib "stored" blocks (no compression), no dependencies.
 * Used by examples/imgui_demo.cpp and the ImGui backend test.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */
#pragma once

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static uint32_t bui_png_crc(uint32_t crc, const unsigned char *p, size_t n)
{
  static uint32_t table[256];
  if (!table[1]) {
    for (uint32_t i = 0; i < 256; i++) {
      uint32_t c = i;
      for (int k = 0; k < 8; k++) {
        c = (c & 1) ? 0xEDB88320u ^ (c >> 1) : c >> 1;
      }
      table[i] = c;
    }
  }
  crc = ~crc;
  for (size_t i = 0; i < n; i++) {
    crc = table[(crc ^ p[i]) & 0xFF] ^ (crc >> 8);
  }
  return ~crc;
}

static void bui_png_u32(unsigned char *p, uint32_t v)
{
  p[0] = (unsigned char)(v >> 24);
  p[1] = (unsigned char)(v >> 16);
  p[2] = (unsigned char)(v >> 8);
  p[3] = (unsigned char)v;
}

static void bui_png_chunk(FILE *f, const char *type, const unsigned char *data, size_t n)
{
  unsigned char hdr[8];
  bui_png_u32(hdr, (uint32_t)n);
  memcpy(hdr + 4, type, 4);
  fwrite(hdr, 1, 8, f);
  if (n) {
    fwrite(data, 1, n, f);
  }
  uint32_t crc = bui_png_crc(0, hdr + 4, 4);
  crc = bui_png_crc(crc, data, n);
  unsigned char c[4];
  bui_png_u32(c, crc);
  fwrite(c, 1, 4, f);
}

/* Returns 1 on success. */
static int bui_png_write(const char *path, const uint32_t *px, int w, int h)
{
  FILE *f = fopen(path, "wb");
  if (!f) {
    return 0;
  }
  static const unsigned char sig[8] = {137, 80, 78, 71, 13, 10, 26, 10};
  fwrite(sig, 1, 8, f);
  unsigned char ihdr[13];
  bui_png_u32(ihdr, (uint32_t)w);
  bui_png_u32(ihdr + 4, (uint32_t)h);
  ihdr[8] = 8;  /* bit depth */
  ihdr[9] = 2;  /* RGB */
  ihdr[10] = ihdr[11] = ihdr[12] = 0;
  bui_png_chunk(f, "IHDR", ihdr, 13);
  /* raw scanlines: filter byte 0 + RGB */
  const size_t row = (size_t)w * 3 + 1, raw_n = row * (size_t)h;
  unsigned char *raw = (unsigned char *)malloc(raw_n);
  const size_t nblocks = (raw_n + 65534) / 65535;
  unsigned char *z = (unsigned char *)malloc(raw_n + nblocks * 5 + 6);
  if (!raw || !z) {
    free(raw);
    free(z);
    fclose(f);
    return 0;
  }
  for (int y = 0; y < h; y++) {
    unsigned char *r = raw + row * (size_t)y;
    r[0] = 0;
    for (int x = 0; x < w; x++) {
      const uint32_t c = px[(size_t)y * w + x];
      r[1 + x * 3] = (unsigned char)(c >> 16);
      r[2 + x * 3] = (unsigned char)(c >> 8);
      r[3 + x * 3] = (unsigned char)c;
    }
  }
  size_t zn = 0;
  z[zn++] = 0x78;
  z[zn++] = 0x01;
  uint32_t a = 1, b = 0; /* adler32 */
  for (size_t at = 0; at < raw_n;) {
    const size_t n = raw_n - at < 65535 ? raw_n - at : 65535;
    z[zn++] = at + n == raw_n ? 1 : 0;
    z[zn++] = (unsigned char)(n & 0xFF);
    z[zn++] = (unsigned char)(n >> 8);
    z[zn++] = (unsigned char)(~n & 0xFF);
    z[zn++] = (unsigned char)((~n >> 8) & 0xFF);
    memcpy(z + zn, raw + at, n);
    for (size_t i = 0; i < n; i++) {
      a = (a + raw[at + i]) % 65521;
      b = (b + a) % 65521;
    }
    zn += n;
    at += n;
  }
  bui_png_u32(z + zn, (b << 16) | a);
  zn += 4;
  bui_png_chunk(f, "IDAT", z, zn);
  bui_png_chunk(f, "IEND", NULL, 0);
  free(raw);
  free(z);
  return fclose(f) == 0;
}
