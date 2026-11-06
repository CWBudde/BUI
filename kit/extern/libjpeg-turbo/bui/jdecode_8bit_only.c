/* BUI: the decoder is built for 8-bit samples only (what images on screen
 * use). libjpeg-turbo 3 compiles its sample-size-specific modules once per
 * precision (src/wrapper/<module>-12.c, -16.c); instead of those, 12- and
 * 16-bit JPEGs (rare: medical and lossless archives) fail with
 * JERR_BAD_PRECISION here, and so does color quantization (jquant1/2.c),
 * which BUI never asks for. This saves ~200 KB of code. */
#define JPEG_INTERNALS
#include "jinclude.h"
#include "jpeglib.h"
#include "jerror.h"

#define FAIL ERREXIT1(cinfo, JERR_BAD_PRECISION, cinfo->data_precision)
#define STUB(name) \
  GLOBAL(void) name(j_decompress_ptr cinfo) \
  { \
    FAIL; \
  }
#define STUB_BUF(name) \
  GLOBAL(void) name(j_decompress_ptr cinfo, boolean need_full_buffer) \
  { \
    (void)need_full_buffer; \
    FAIL; \
  }

STUB(jinit_1pass_quantizer)
STUB(jinit_2pass_quantizer)
STUB(j12init_1pass_quantizer)
STUB(j12init_2pass_quantizer)
STUB(j12init_color_deconverter)
STUB_BUF(j12init_d_coef_controller)
STUB_BUF(j12init_d_diff_controller)
STUB_BUF(j12init_d_main_controller)
STUB_BUF(j12init_d_post_controller)
STUB(j12init_inverse_dct)
STUB(j12init_lossless_decompressor)
STUB(j12init_merged_upsampler)
STUB(j12init_upsampler)
STUB(j16init_color_deconverter)
STUB_BUF(j16init_d_diff_controller)
STUB_BUF(j16init_d_main_controller)
STUB_BUF(j16init_d_post_controller)
STUB(j16init_lossless_decompressor)
STUB(j16init_upsampler)
