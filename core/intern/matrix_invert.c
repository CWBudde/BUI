/* 4x4 matrix inverse (Gauss-Jordan, partial pivoting).
 * Replaces intern/eigen dependency of upstream math_matrix.cc.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#include <math.h>
#include <string.h>

#include "BLI_sys_types.h"
#include "bui_eigen_shim.h"

bool EIG_invert_m4_m4(float inverse_out[4][4], const float matrix[4][4])
{
  double a[4][8];
  int i, j, col;

  for (i = 0; i < 4; i++) {
    for (j = 0; j < 4; j++) {
      a[i][j] = (double)matrix[i][j];
      a[i][4 + j] = (i == j) ? 1.0 : 0.0;
    }
  }

  for (col = 0; col < 4; col++) {
    /* Find pivot. */
    int pivot = col;
    for (i = col + 1; i < 4; i++) {
      if (fabs(a[i][col]) > fabs(a[pivot][col])) {
        pivot = i;
      }
    }
    if (fabs(a[pivot][col]) < 1e-12) {
      return false; /* Singular. */
    }
    if (pivot != col) {
      for (j = 0; j < 8; j++) {
        double tmp = a[col][j];
        a[col][j] = a[pivot][j];
        a[pivot][j] = tmp;
      }
    }
    /* Normalize pivot row. */
    double piv = a[col][col];
    for (j = 0; j < 8; j++) {
      a[col][j] /= piv;
    }
    /* Eliminate column. */
    for (i = 0; i < 4; i++) {
      if (i != col) {
        double factor = a[i][col];
        for (j = 0; j < 8; j++) {
          a[i][j] -= factor * a[col][j];
        }
      }
    }
  }

  for (i = 0; i < 4; i++) {
    for (j = 0; j < 4; j++) {
      inverse_out[i][j] = (float)a[i][4 + j];
    }
  }
  return true;
}
