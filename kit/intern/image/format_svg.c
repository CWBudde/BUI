/* SVG through the nanosvg the kit already carries for Blender's icons
 * (kit/extern/nanosvg, Blender's copy): parsed, then rasterized at the size
 * it is drawn (vector: no downscale needed). BUI_IMAGE_SVG (CMake option,
 * ON) puts it in the format table; the icons use imb_svg_rasterize always.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <math.h>
#include <stdio.h>
#include <string.h>

#include "MEM_guardedalloc.h"

#include "image.h"

#include "nanosvg.h"
#include "nanosvgrast.h"

bool imb_is_a_svg(const uint8_t *mem, size_t size)
{
  /* XML (optionally with a BOM) whose first kilobyte opens an <svg> element */
  size_t p = size >= 3 && mem[0] == 0xEF && mem[1] == 0xBB && mem[2] == 0xBF ? 3 : 0;
  while (p < size && (mem[p] == ' ' || mem[p] == '\t' || mem[p] == '\r' || mem[p] == '\n')) {
    p++;
  }
  if (p >= size || mem[p] != '<') {
    return false;
  }
  const size_t n = BK_MIN(size, (size_t)1024);
  for (size_t i = p; i + 4 <= n; i++) {
    if (memcmp(mem + i, "<svg", 4) == 0) {
      return true;
    }
  }
  return false;
}

/* nanosvg parses in place: a NUL-terminated copy. */
static NSVGimage *parse(const uint8_t *mem, size_t size)
{
  char *text = (char *)MEM_mallocN(size + 1, "svg text");
  if (!text) {
    return NULL;
  }
  memcpy(text, mem, size);
  text[size] = 0;
  NSVGimage *image = nsvgParse(text, "px", 96.0f);
  MEM_freeN(text);
  if (image && (image->width <= 0.0f || image->height <= 0.0f)) {
    nsvgDelete(image);
    return NULL;
  }
  return image;
}

bool imb_header_svg(const uint8_t *mem, size_t size, BKImageInfo *r_info)
{
  NSVGimage *image = parse(mem, size);
  if (!image) {
    return false;
  }
  r_info->width = (int)ceilf(image->width);
  r_info->height = (int)ceilf(image->height);
  r_info->frames = 1;
  nsvgDelete(image);
  return true;
}

static BKImBuf *rasterize(NSVGimage *image, int want_w, int want_h)
{
  float scale = 1.0f;
  if (want_w > 0 && want_h > 0) {
    scale = fminf((float)want_w / image->width, (float)want_h / image->height);
  }
  const int w = BK_MAX(1, (int)ceilf(image->width * scale));
  const int h = BK_MAX(1, (int)ceilf(image->height * scale));
  scale = (float)w / image->width;
  BKImBuf *ib = imb_alloc(w, h, 1);
  NSVGrasterizer *rast = ib ? nsvgCreateRasterizer() : NULL;
  if (!rast) {
    BK_imbuf_free(ib);
    return NULL;
  }
  /* straight RGBA into the buffer, then premultiplied BGRA in place */
  nsvgRasterize(rast, image, 0.0f, 0.0f, scale, (unsigned char *)ib->rect, w, h, w * 4);
  nsvgDeleteRasterizer(rast);
  imb_premultiply_rgba(ib->rect, (const uint8_t *)ib->rect, w * h);
  return ib;
}

BKImBuf *imb_load_svg(const uint8_t *mem, size_t size, int want_w, int want_h)
{
  NSVGimage *image = parse(mem, size);
  BKImBuf *ib = image ? rasterize(image, want_w, want_h) : NULL;
  if (image) {
    nsvgDelete(image);
  }
  return ib;
}

BKImBuf *imb_svg_rasterize(const char *svg, int want_w, int want_h)
{
  NSVGimage *image = parse((const uint8_t *)svg, strlen(svg));
  BKImBuf *ib = image ? rasterize(image, want_w, want_h) : NULL;
  if (image) {
    nsvgDelete(image);
  }
  return ib;
}
