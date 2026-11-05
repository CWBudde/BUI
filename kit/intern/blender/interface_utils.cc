/* SPDX-FileCopyrightText: 2009 Blender Authors
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

/** \file
 * \ingroup edinterface
 */

/* BUI: an excerpt of editors/interface/interface_utils.cc — only
 * calc_float_precision (UI_calc_float_precision), which number fields use for
 * display so 0.00001 is not drawn as "0.000" (kit_widgets.c format_number).
 * BUI edits, marked "BUI:": UI_PRECISION_FLOAT_MAX from UI_interface_c.hh is
 * defined here, and a C entry point is added at the end. */

#include <algorithm>
#include <cmath>

#include "BLI_assert.h"
#include "BLI_utildefines.h"

#include "bk_blender.h"

/* BUI: from UI_interface_c.hh. */
#define UI_PRECISION_FLOAT_MAX 6

namespace blender::ui {

/********************************** Misc **************************************/

int calc_float_precision(int prec, double value)
{
  static const double pow10_neg[UI_PRECISION_FLOAT_MAX + 1] = {
      1e0, 1e-1, 1e-2, 1e-3, 1e-4, 1e-5, 1e-6};
  static const double max_pow = 10000000.0; /* pow(10, UI_PRECISION_FLOAT_MAX) */

  /* BUI: clamp first (Blender asserts; kit nodes take any "precision"). */
  CLAMP(prec, 0, UI_PRECISION_FLOAT_MAX);
  BLI_assert(prec <= UI_PRECISION_FLOAT_MAX);
  BLI_assert(fabs(pow10_neg[prec] - pow(10, -prec)) < 1e-16);

  /* Check on the number of decimal places need to display the number,
   * this is so 0.00001 is not displayed as 0.00,
   * _but_, this is only for small values as 10.0001 will not get the same treatment.
   */
  value = fabs(value);
  if ((value < pow10_neg[prec]) && (value > (1.0 / max_pow))) {
    int value_i = int(lround(value * max_pow));
    if (value_i != 0) {
      const int prec_span = 3; /* show: 0.01001, 5 would allow 0.0100001 for eg. */
      int test_prec;
      int prec_min = -1;
      int dec_flag = 0;
      int i = UI_PRECISION_FLOAT_MAX;
      while (i && value_i) {
        if (value_i % 10) {
          dec_flag |= 1 << i;
          prec_min = i;
        }
        value_i /= 10;
        i--;
      }

      /* even though its a small value, if the second last digit is not 0, use it */
      test_prec = prec_min;

      dec_flag = (dec_flag >> (prec_min + 1)) & ((1 << prec_span) - 1);

      while (dec_flag) {
        test_prec++;
        dec_flag = dec_flag >> 1;
      }

      prec = std::max(test_prec, prec);
    }
  }

  CLAMP(prec, 0, UI_PRECISION_FLOAT_MAX);

  return prec;
}

}  // namespace blender::ui

/* BUI: C entry point. */
int bk_calc_float_precision(int prec, double value)
{
  return blender::ui::calc_float_precision(prec, value);
}
