/* Radiance HDR (RGBE), after Blender's imbuf/intern/radiance_hdr.cc
 * (imb_loadhdr: the header lines, "-Y h +X w", flat and new-style RLE
 * scanlines). Linear radiance is shown the way Blender's "Standard" view
 * transform shows it: clipped to 0..1 and sRGB-encoded.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "MEM_guardedalloc.h"

#include "image.h"

#define MINELEN 8      /* scanline length limits for the RLE encoding */
#define MAXELEN 0x7fff

bool imb_is_a_hdr(const uint8_t *mem, size_t size)
{
  /* "#?RADIANCE" or "#?RGBE" (Blender's imb_is_a_hdr) */
  return size >= 11 && mem[0] == '#' && mem[1] == '?' &&
         (memcmp(mem + 2, "RADIANCE", 8) == 0 || memcmp(mem + 2, "RGBE", 4) == 0);
}

/* The header: returns the offset of the pixel data (0 = bad). */
static size_t parse_header(const uint8_t *mem, size_t size, int *r_w, int *r_h)
{
  size_t p = 0;
  /* header lines up to an empty line */
  for (;;) {
    const uint8_t *nl = (const uint8_t *)memchr(mem + p, '\n', size - p);
    if (!nl) {
      return 0;
    }
    const size_t len = (size_t)(nl - (mem + p));
    if (len >= 7 && memcmp(mem + p, "FORMAT=", 7) == 0 &&
        !(len >= 19 && memcmp(mem + p + 7, "32-bit_rle_rgbe", 15) == 0))
    {
      return 0; /* XYZE: not supported (Blender reads only RGBE too) */
    }
    p += len + 1;
    if (len == 0) {
      break;
    }
  }
  const uint8_t *nl = (const uint8_t *)memchr(mem + p, '\n', size - p);
  if (!nl) {
    return 0;
  }
  char line[64];
  const size_t len = BK_MIN((size_t)(nl - (mem + p)), sizeof(line) - 1);
  memcpy(line, mem + p, len);
  line[len] = 0;
  int w = 0, h = 0;
  if (sscanf(line, "-Y %d +X %d", &h, &w) != 2 || w <= 0 || h <= 0 || w > BK_IMB_MAX_SIDE ||
      h > BK_IMB_MAX_SIDE)
  {
    return 0;
  }
  *r_w = w;
  *r_h = h;
  return (size_t)(nl - mem) + 1;
}

bool imb_header_hdr(const uint8_t *mem, size_t size, BKImageInfo *r_info)
{
  r_info->frames = 1;
  return parse_header(mem, size, &r_info->width, &r_info->height) != 0;
}

/* One scanline of RGBE into `scan` (w * 4 bytes): returns bytes consumed, 0 on error. */
static size_t read_scanline(const uint8_t *mem, size_t size, uint8_t *scan, int w)
{
  if (w < MINELEN || w > MAXELEN || size < 4 || mem[0] != 2 || mem[1] != 2 || (mem[2] & 0x80)) {
    /* flat (uncompressed) scanline */
    const size_t n = (size_t)w * 4;
    if (size < n) {
      return 0;
    }
    memcpy(scan, mem, n);
    return n;
  }
  if (((int)mem[2] << 8 | mem[3]) != w) {
    return 0;
  }
  size_t p = 4;
  /* new RLE: the four components one after the other */
  for (int c = 0; c < 4; c++) {
    int x = 0;
    while (x < w) {
      if (p >= size) {
        return 0;
      }
      int code = mem[p++];
      if (code > 128) { /* run */
        code &= 127;
        if (p >= size || x + code > w) {
          return 0;
        }
        const uint8_t v = mem[p++];
        for (int i = 0; i < code; i++) {
          scan[4 * (x++) + c] = v;
        }
      }
      else { /* literal */
        if (code == 0 || x + code > w || p + (size_t)code > size) {
          return 0;
        }
        for (int i = 0; i < code; i++) {
          scan[4 * (x++) + c] = mem[p++];
        }
      }
    }
  }
  return p;
}

/* Linear 0..1 in 4096 steps -> sRGB 8-bit (the "Standard" view transform);
 * built per decode (~4k powf, nothing next to the pixels). */
static void srgb_lut(uint8_t lut[4097])
{
  for (int i = 0; i <= 4096; i++) {
    const float v = (float)i / 4096.0f;
    const float s = v <= 0.0031308f ? v * 12.92f : 1.055f * powf(v, 1.0f / 2.4f) - 0.055f;
    lut[i] = (uint8_t)(s * 255.0f + 0.5f);
  }
}

static inline uint32_t channel(const uint8_t *lut, unsigned m, float f)
{
  const float v = ((float)m + 0.5f) * f;
  return lut[v >= 1.0f ? 4096 : (int)(v * 4096.0f)];
}

BKImBuf *imb_load_hdr(const uint8_t *mem, size_t size, int want_w, int want_h)
{
  (void)want_w;
  (void)want_h;
  int w, h;
  size_t p = parse_header(mem, size, &w, &h);
  BKImBuf *ib = p ? imb_alloc(w, h, 1) : NULL;
  uint8_t *scan = ib ? (uint8_t *)MEM_mallocN((size_t)w * 4, "hdr scanline") : NULL;
  if (!scan) {
    BK_imbuf_free(ib);
    return NULL;
  }
  uint8_t lut[4097];
  srgb_lut(lut);
  for (int y = 0; y < h; y++) {
    const size_t n = read_scanline(mem + p, size - p, scan, w);
    if (n == 0) {
      /* truncated: the rest transparent */
      memset(ib->rect + (size_t)y * w, 0, sizeof(uint32_t) * (size_t)w * (h - y));
      break;
    }
    p += n;
    uint32_t *dst = ib->rect + (size_t)y * w;
    for (int x = 0; x < w; x++) {
      const uint8_t *c = scan + 4 * x;
      if (c[3] == 0) {
        dst[x] = 0xFF000000u;
        continue;
      }
      const float f = ldexpf(1.0f, (int)c[3] - (128 + 8)); /* RGBE2FLOAT */
      dst[x] = 0xFF000000u | channel(lut, c[0], f) << 16 | channel(lut, c[1], f) << 8 |
               channel(lut, c[2], f);
    }
  }
  MEM_freeN(scan);
  return ib;
}
