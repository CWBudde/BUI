/* JPEG through libjpeg-turbo, as Blender's imbuf (format_jpeg.cc): DCT
 * scaling decodes thumbnails at a fraction of the cost (Blender's
 * imb_thumbnail_jpeg), CMYK/YCCK (Adobe) is converted like Blender's
 * ycck/cmyk path, and the EXIF orientation is read for imb_load to apply.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <setjmp.h>
#include <stdio.h>
#include <string.h>

#include "MEM_guardedalloc.h"

#include "image.h"

#include "jpeglib.h"

bool imb_is_a_jpeg(const uint8_t *mem, size_t size)
{
  return size >= 3 && mem[0] == 0xFF && mem[1] == 0xD8 && mem[2] == 0xFF;
}

/* -------------------------------------------------------------------- */
/** \name EXIF orientation
 * \{ */

static unsigned rd16(const uint8_t *p, bool le)
{
  return le ? (unsigned)(p[0] | p[1] << 8) : (unsigned)(p[0] << 8 | p[1]);
}

static uint32_t rd32(const uint8_t *p, bool le)
{
  return le ? (uint32_t)p[0] | (uint32_t)p[1] << 8 | (uint32_t)p[2] << 16 | (uint32_t)p[3] << 24 :
              (uint32_t)p[0] << 24 | (uint32_t)p[1] << 16 | (uint32_t)p[2] << 8 | (uint32_t)p[3];
}

/* Tag 0x0112 of IFD0 in an APP1 "Exif" block (TIFF layout). */
static int exif_orientation(const uint8_t *tiff, size_t n)
{
  if (n < 8 || !((tiff[0] == 'I' && tiff[1] == 'I') || (tiff[0] == 'M' && tiff[1] == 'M'))) {
    return 1;
  }
  const bool le = tiff[0] == 'I';
  const uint32_t ifd = rd32(tiff + 4, le);
  if (ifd > n - 2) {
    return 1;
  }
  const unsigned count = rd16(tiff + ifd, le);
  for (unsigned i = 0; i < count; i++) {
    const size_t e = ifd + 2 + (size_t)i * 12;
    if (e + 12 > n) {
      break;
    }
    if (rd16(tiff + e, le) == 0x0112) {
      const unsigned v = rd16(tiff + e + 8, le);
      return v >= 1 && v <= 8 ? (int)v : 1;
    }
  }
  return 1;
}

int imb_jpeg_orientation(const uint8_t *mem, size_t size)
{
  size_t p = 2;
  while (p + 4 <= size && mem[p] == 0xFF) {
    const uint8_t marker = mem[p + 1];
    if (marker == 0xD8 || (marker >= 0xD0 && marker <= 0xD7) || marker == 0x01) {
      p += 2; /* markers without a length */
      continue;
    }
    if (marker == 0xDA || marker == 0xD9) {
      break; /* image data: EXIF comes before it */
    }
    const size_t len = (size_t)(mem[p + 2] << 8 | mem[p + 3]);
    if (len < 2 || p + 2 + len > size) {
      break;
    }
    if (marker == 0xE1 && len >= 8 && memcmp(mem + p + 4, "Exif\0\0", 6) == 0) {
      return exif_orientation(mem + p + 10, len - 8);
    }
    p += 2 + len;
  }
  return 1;
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Decoding
 * \{ */

typedef struct JpegError {
  struct jpeg_error_mgr pub;
  jmp_buf jump;
} JpegError;

static void jpeg_error_exit(j_common_ptr cinfo)
{
  longjmp(((JpegError *)cinfo->err)->jump, 1);
}

static void jpeg_output_message(j_common_ptr cinfo)
{
  (void)cinfo; /* corrupt-data warnings: not for the console */
}

/* A decompressor on `mem` whose errors longjmp to err->jump (set by the caller). */
static void jpeg_open(struct jpeg_decompress_struct *cinfo, JpegError *err, const uint8_t *mem,
                      size_t size)
{
  cinfo->err = jpeg_std_error(&err->pub);
  err->pub.error_exit = jpeg_error_exit;
  err->pub.output_message = jpeg_output_message;
  jpeg_create_decompress(cinfo);
  jpeg_mem_src(cinfo, mem, (unsigned long)size);
}

bool imb_header_jpeg(const uint8_t *mem, size_t size, BKImageInfo *r_info)
{
  struct jpeg_decompress_struct cinfo;
  JpegError err;
  jpeg_open(&cinfo, &err, mem, size);
  if (setjmp(err.jump)) {
    jpeg_destroy_decompress(&cinfo);
    return false;
  }
  jpeg_read_header(&cinfo, TRUE);
  const bool swap = imb_jpeg_orientation(mem, size) >= 5;
  r_info->width = (int)(swap ? cinfo.image_height : cinfo.image_width);
  r_info->height = (int)(swap ? cinfo.image_width : cinfo.image_height);
  r_info->frames = 1;
  jpeg_destroy_decompress(&cinfo);
  return true;
}

BKImBuf *imb_load_jpeg(const uint8_t *mem, size_t size, int want_w, int want_h)
{
  struct jpeg_decompress_struct cinfo;
  JpegError err;
  BKImBuf *volatile ib = NULL;
  uint8_t *volatile row = NULL;
  jpeg_open(&cinfo, &err, mem, size);
  if (setjmp(err.jump)) {
    /* corrupt data: keep the rows that decoded (the rest transparent), like
     * browsers; nothing when it failed before the first row */
    const JDIMENSION done = ib ? cinfo.output_scanline : 0;
    jpeg_destroy_decompress(&cinfo);
    MEM_freeN(row);
    if (done == 0) {
      BK_imbuf_free(ib);
      return NULL;
    }
    memset(ib->rect + (size_t)done * ib->x, 0, sizeof(uint32_t) * (size_t)ib->x * (ib->y - done));
    return ib;
  }
  jpeg_read_header(&cinfo, TRUE);
  const int w = (int)cinfo.image_width, h = (int)cinfo.image_height;
  if (w <= 0 || h <= 0 || w > BK_IMB_MAX_SIDE || h > BK_IMB_MAX_SIDE) {
    jpeg_destroy_decompress(&cinfo);
    return NULL;
  }
  /* DCT scaling: the smallest N/8 that still covers the wanted size */
  if (want_w > 0 && want_h > 0 && (want_w < w || want_h < h)) {
    const double need = BK_MAX((double)want_w / w, (double)want_h / h);
    unsigned num = 1;
    while (num < 8 && num < need * 8.0) {
      num++;
    }
    cinfo.scale_num = num;
    cinfo.scale_denom = 8;
  }
  const bool cmyk = cinfo.jpeg_color_space == JCS_CMYK || cinfo.jpeg_color_space == JCS_YCCK;
  cinfo.out_color_space = cmyk ? JCS_CMYK : JCS_EXT_BGRA; /* 0xAARRGGBB on little-endian */
  cinfo.dct_method = JDCT_ISLOW;
  cinfo.do_fancy_upsampling = TRUE;
  jpeg_start_decompress(&cinfo);
  ib = imb_alloc((int)cinfo.output_width, (int)cinfo.output_height, 1);
  if (!ib) {
    jpeg_destroy_decompress(&cinfo);
    return NULL;
  }
  if (cmyk) {
    row = (uint8_t *)MEM_mallocN((size_t)ib->x * 4, "jpeg cmyk row");
  }
  /* Adobe writes inverted CMYK (Blender: cinfo->saw_Adobe_marker) */
  const bool inverted = cinfo.saw_Adobe_marker;
  while (cinfo.output_scanline < cinfo.output_height) {
    uint32_t *dst = ib->rect + (size_t)cinfo.output_scanline * ib->x;
    JSAMPROW line = cmyk ? row : (JSAMPROW)dst;
    if (cmyk && !row) {
      break;
    }
    jpeg_read_scanlines(&cinfo, &line, 1);
    if (cmyk) {
      for (int x = 0; x < ib->x; x++) {
        const uint8_t *p = row + 4 * x;
        unsigned c = p[0], m = p[1], y = p[2], k = p[3];
        if (!inverted) {
          c = 255 - c;
          m = 255 - m;
          y = 255 - y;
          k = 255 - k;
        }
        dst[x] = 0xFF000000u | ((c * k / 255) << 16) | ((m * k / 255) << 8) | (y * k / 255);
      }
    }
  }
  jpeg_finish_decompress(&cinfo);
  jpeg_destroy_decompress(&cinfo);
  MEM_freeN(row);
  return ib;
}

/** \} */
