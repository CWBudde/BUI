/* PNM: PGM and PPM, ASCII (P2, P3) and binary (P5, P6), 8 or 16 bits: the
 * formats Tk's photo image reads, so tkinter programs keep working. Every
 * read is bounds-checked against the buffer.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <string.h>

#include "image.h"

bool imb_is_a_pnm(const uint8_t *mem, size_t size)
{
  return size >= 3 && mem[0] == 'P' && mem[1] >= '2' && mem[1] <= '6' && mem[1] != '4' &&
         (mem[2] == ' ' || mem[2] == '\n' || mem[2] == '\r' || mem[2] == '\t');
}

static bool is_space(uint8_t c)
{
  return c == ' ' || c == '\n' || c == '\r' || c == '\t' || c == '\v' || c == '\f';
}

/* The next decimal number at *p (skipping white space and # comments). */
static bool number(const uint8_t *mem, size_t size, size_t *p, unsigned *r_value)
{
  while (*p < size && (is_space(mem[*p]) || mem[*p] == '#')) {
    if (mem[*p] == '#') {
      while (*p < size && mem[*p] != '\n') {
        (*p)++;
      }
    }
    else {
      (*p)++;
    }
  }
  unsigned v = 0;
  int digits = 0;
  while (*p < size && mem[*p] >= '0' && mem[*p] <= '9' && digits < 9) {
    v = v * 10 + (unsigned)(mem[*p] - '0');
    (*p)++;
    digits++;
  }
  *r_value = v;
  return digits > 0;
}

/* Header: kind ('2'..'6'), size, maxval; *r_data is where samples start. */
static bool header(const uint8_t *mem, size_t size, int *r_kind, unsigned *r_w, unsigned *r_h,
                   unsigned *r_max, size_t *r_data)
{
  size_t p = 2;
  if (!imb_is_a_pnm(mem, size) || !number(mem, size, &p, r_w) || !number(mem, size, &p, r_h) ||
      !number(mem, size, &p, r_max) || *r_w == 0 || *r_h == 0 || *r_max == 0 || *r_max > 65535 ||
      *r_w > BK_IMB_MAX_SIDE || *r_h > BK_IMB_MAX_SIDE || p >= size)
  {
    return false;
  }
  *r_kind = mem[1];
  *r_data = p + 1; /* one white-space character ends the header */
  return true;
}

bool imb_header_pnm(const uint8_t *mem, size_t size, BKImageInfo *r_info)
{
  int kind;
  unsigned w, h, maxval;
  size_t data;
  if (!header(mem, size, &kind, &w, &h, &maxval, &data)) {
    return false;
  }
  r_info->width = (int)w;
  r_info->height = (int)h;
  r_info->frames = 1;
  return true;
}

BKImBuf *imb_load_pnm(const uint8_t *mem, size_t size, int want_w, int want_h)
{
  (void)want_w;
  (void)want_h;
  int kind;
  unsigned w, h, maxval;
  size_t p;
  if (!header(mem, size, &kind, &w, &h, &maxval, &p)) {
    return NULL;
  }
  const bool color = kind == '3' || kind == '6', ascii = kind == '2' || kind == '3';
  const int channels = color ? 3 : 1, bytes = maxval > 255 ? 2 : 1;
  const size_t n = (size_t)w * h * channels;
  if (!ascii && size - p < n * bytes) {
    return NULL; /* truncated */
  }
  BKImBuf *ib = imb_alloc((int)w, (int)h, 1);
  if (!ib) {
    return NULL;
  }
  unsigned s[3] = {0, 0, 0};
  for (size_t i = 0; i < (size_t)w * h; i++) {
    for (int c = 0; c < channels; c++) {
      if (ascii) {
        if (!number(mem, size, &p, &s[c])) {
          s[c] = 0; /* truncated: black */
        }
      }
      else {
        s[c] = bytes == 2 ? (unsigned)(mem[p] << 8 | mem[p + 1]) : mem[p];
        p += bytes;
      }
      s[c] = (BK_MIN(s[c], maxval) * 255 + maxval / 2) / maxval;
    }
    if (!color) {
      s[1] = s[2] = s[0];
    }
    ib->rect[i] = 0xFF000000u | s[0] << 16 | s[1] << 8 | s[2];
  }
  return ib;
}
