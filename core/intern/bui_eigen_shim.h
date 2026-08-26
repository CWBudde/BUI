/* Drop-in replacement for the single Eigen call the extracted core used
 * (#EIG_invert_m4_m4): Gauss-Jordan elimination with partial pivoting.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#ifndef __BUI_EIGEN_SHIM_H__
#define __BUI_EIGEN_SHIM_H__

#ifdef __cplusplus
extern "C" {
#endif

bool EIG_invert_m4_m4(float inverse[4][4], const float matrix[4][4]);

#ifdef __cplusplus
}
#endif

#endif /* __BUI_EIGEN_SHIM_H__ */
