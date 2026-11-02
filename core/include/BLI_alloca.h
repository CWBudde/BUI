/* SPDX-FileCopyrightText: 2023 Blender Authors
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#pragma once

/** \file
 * \ingroup bli
 *
 * Defines alloca and utility macro BLI_array_alloca
 */

/* BLI_array_alloca / alloca */

#include <stdlib.h> /* IWYU pragma: keep */

#ifdef __cplusplus
#define BLI_array_alloca(arr, realsize) \
  (std::remove_reference_t<decltype(arr)>)alloca(sizeof(*arr) * (realsize))
#elif defined(__GNUC__)
#define BLI_array_alloca(arr, realsize) \
  ((typeof(arr))alloca(sizeof(*arr) * (realsize)))
#else /* C: void * converts to the target pointer type (MSVC has no typeof) */
#define BLI_array_alloca(arr, realsize) alloca(sizeof(*arr) * (realsize))
#endif
