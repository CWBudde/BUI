/* SVD of a 3x3 matrix via one-sided Jacobi rotations.
 * C implementation replacing the Eigen-backed upstream #BLI_svd_m3.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#include <math.h>
#include <string.h>

#include "BLI_math_solvers.h"

#define BUI_SVD_MAX_SWEEPS 30
#define BUI_SVD_EPS 1e-12f

static void svd_swap_cols_f3(float m[3][3], int p, int q)
{
  for (int i = 0; i < 3; i++) {
    float tmp = m[i][p];
    m[i][p] = m[i][q];
    m[i][q] = tmp;
  }
}

void BLI_svd_m3(const float m3[3][3], float r_U[3][3], float r_S[3], float r_V[3][3])
{
  /* Working copy whose columns are orthogonalized. */
  float w[3][3];
  memcpy(w, m3, sizeof(w));
  memset(r_V, 0, sizeof(float[3][3]));
  r_V[0][0] = r_V[1][1] = r_V[2][2] = 1.0f;

  for (int sweep = 0; sweep < BUI_SVD_MAX_SWEEPS; sweep++) {
    float off = 0.0f;
    for (int p = 0; p < 3; p++) {
      for (int q = p + 1; q < 3; q++) {
        float alpha = 0.0f, beta = 0.0f, gamma = 0.0f;
        for (int i = 0; i < 3; i++) {
          alpha += w[i][p] * w[i][p];
          beta += w[i][q] * w[i][q];
          gamma += w[i][p] * w[i][q];
        }
        off += fabsf(gamma);
        if (fabsf(gamma) > BUI_SVD_EPS * sqrtf(alpha * beta)) {
          float zeta = (beta - alpha) / (2.0f * gamma);
          float t = ((zeta >= 0.0f) ? 1.0f : -1.0f) /
                    (fabsf(zeta) + sqrtf(1.0f + zeta * zeta));
          float c = 1.0f / sqrtf(1.0f + t * t);
          float s = c * t;
          for (int i = 0; i < 3; i++) {
            float wp = w[i][p], wq = w[i][q];
            w[i][p] = c * wp - s * wq;
            w[i][q] = s * wp + c * wq;
            float vp = r_V[i][p], vq = r_V[i][q];
            r_V[i][p] = c * vp - s * vq;
            r_V[i][q] = s * vp + c * vq;
          }
        }
      }
    }
    if (off < BUI_SVD_EPS) {
      break;
    }
  }

  /* Singular values are column norms of W. */
  for (int j = 0; j < 3; j++) {
    float n = 0.0f;
    for (int i = 0; i < 3; i++) {
      n += w[i][j] * w[i][j];
    }
    r_S[j] = sqrtf(n);
  }

  /* Sort descending by singular value, permuting columns of W and V alike. */
  for (int p = 0; p < 2; p++) {
    for (int q = p + 1; q < 3; q++) {
      if (r_S[q] > r_S[p]) {
        float ts = r_S[p];
        r_S[p] = r_S[q];
        r_S[q] = ts;
        svd_swap_cols_f3(w, p, q);
        svd_swap_cols_f3(r_V, p, q);
      }
    }
  }

  /* Normalize U columns. */
  for (int j = 0; j < 3; j++) {
    if (r_S[j] > BUI_SVD_EPS) {
      for (int i = 0; i < 3; i++) {
        r_U[i][j] = w[i][j] / r_S[j];
      }
    }
    else {
      for (int i = 0; i < 3; i++) {
        r_U[i][j] = 0.0f;
      }
    }
  }

#undef BUI_SVD_MAX_SWEEPS
}
