/* A minimal PNG writer for the site examples' --screenshot (stored deflate
 * blocks, no zlib; tools/pngopt.py recompresses the files losslessly).
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */
#pragma once

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

static uint32_t site_crc_table[256];

static uint32_t site_crc(uint32_t crc, const unsigned char *p, size_t n)
{
  if (!site_crc_table[1]) {
    for (uint32_t i = 0; i < 256; i++) {
      uint32_t c = i;
      for (int k = 0; k < 8; k++) {
        c = (c & 1) ? 0xedb88320u ^ (c >> 1) : c >> 1;
      }
      site_crc_table[i] = c;
    }
  }
  crc = ~crc;
  for (size_t i = 0; i < n; i++) {
    crc = site_crc_table[(crc ^ p[i]) & 0xff] ^ (crc >> 8);
  }
  return ~crc;
}

static void site_be32(unsigned char *p, uint32_t v)
{
  p[0] = (unsigned char)(v >> 24), p[1] = (unsigned char)(v >> 16);
  p[2] = (unsigned char)(v >> 8), p[3] = (unsigned char)v;
}

static void site_chunk(FILE *f, const char *type, const unsigned char *body, uint32_t n)
{
  unsigned char hdr[8];
  site_be32(hdr, n);
  for (int i = 0; i < 4; i++) {
    hdr[4 + i] = (unsigned char)type[i];
  }
  fwrite(hdr, 1, 8, f);
  if (n) {
    fwrite(body, 1, n, f);
  }
  uint32_t crc = site_crc(0, hdr + 4, 4);
  crc = site_crc(crc, body, n);
  unsigned char c[4];
  site_be32(c, crc);
  fwrite(c, 1, 4, f);
}

/* `px`: 0x00RRGGBB (BUI_raster_pixels), `w` x `h`. Returns 0 on success. */
static int site_write_png(const char *path, const uint32_t *px, int w, int h)
{
  const size_t stride = (size_t)w * 3 + 1, raw_n = stride * (size_t)h;
  unsigned char *raw = (unsigned char *)malloc(raw_n);
  for (int y = 0; y < h; y++) {
    unsigned char *row = raw + y * stride;
    row[0] = 0;
    for (int x = 0; x < w; x++) {
      const uint32_t p = px[(size_t)y * w + x];
      row[1 + x * 3] = (unsigned char)(p >> 16);
      row[2 + x * 3] = (unsigned char)(p >> 8);
      row[3 + x * 3] = (unsigned char)p;
    }
  }
  /* zlib stream of stored blocks */
  const size_t nblocks = (raw_n + 65534) / 65535;
  const size_t z_n = 2 + raw_n + nblocks * 5 + 4;
  unsigned char *z = (unsigned char *)malloc(z_n);
  size_t o = 0;
  z[o++] = 0x78, z[o++] = 0x01;
  uint32_t a = 1, b = 0;
  for (size_t i = 0; i < raw_n; i++) {
    a = (a + raw[i]) % 65521u;
    b = (b + a) % 65521u;
  }
  for (size_t done = 0; done < raw_n;) {
    const size_t n = raw_n - done > 65535 ? 65535 : raw_n - done;
    z[o++] = done + n == raw_n ? 1 : 0;
    z[o++] = (unsigned char)n, z[o++] = (unsigned char)(n >> 8);
    z[o++] = (unsigned char)~n, z[o++] = (unsigned char)(~n >> 8);
    for (size_t i = 0; i < n; i++) {
      z[o++] = raw[done + i];
    }
    done += n;
  }
  site_be32(z + o, (b << 16) | a);
  o += 4;
  FILE *f = fopen(path, "wb");
  if (!f) {
    free(raw), free(z);
    return 1;
  }
  static const unsigned char sig[8] = {137, 80, 78, 71, 13, 10, 26, 10};
  fwrite(sig, 1, 8, f);
  unsigned char ihdr[13] = {0};
  site_be32(ihdr, (uint32_t)w);
  site_be32(ihdr + 4, (uint32_t)h);
  ihdr[8] = 8, ihdr[9] = 2;
  site_chunk(f, "IHDR", ihdr, 13);
  site_chunk(f, "IDAT", z, (uint32_t)o);
  site_chunk(f, "IEND", NULL, 0);
  fclose(f);
  free(raw), free(z);
  printf("wrote %s (%dx%d)\n", path, w, h);
  return 0;
}

/* BUI_UI_SCALE for off-screen renders (no monitor DPI); 1 without it. */
static float site_scale(void)
{
  const char *s = getenv("BUI_UI_SCALE");
  const float v = s ? (float)atof(s) : 1.0f;
  return v > 0.0f ? v : 1.0f;
}
