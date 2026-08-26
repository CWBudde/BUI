/* Declarations extracted from the C++-only upstream headers.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#ifndef __BLI_HASH_MM2_H__
#define __BLI_HASH_MM2_H__

#include "BLI_sys_types.h"

#ifdef __cplusplus
extern "C" {
#endif

uint32_t BLI_hash_mm2(const uchar *data, size_t len, uint32_t seed);

#ifdef __cplusplus
}
#endif

#endif /* __BLI_HASH_MM2_H__ */
