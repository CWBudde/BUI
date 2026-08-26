/* MurmurHash2, C implementation replacing the C++-only upstream header API.
 * Algorithm by Austin Appleby (public domain).
 * SPDX-License-Identifier: GPL-2.0-or-later */
#include <string.h>

#include "BLI_sys_types.h"
#include "BLI_hash_mm2.h"

uint32_t BLI_hash_mm2(const uchar *data, size_t len, uint32_t seed)
{
  const uint32_t m = 0x5bd1e995u;
  const uint32_t r = 24;

  uint32_t h = seed ^ (uint32_t)len;

  while (len >= 4) {
    uint32_t k;
    memcpy(&k, data, sizeof(k));
    k *= m;
    k ^= k >> r;
    k *= m;

    h *= m;
    h ^= k;

    data += 4;
    len -= 4;
  }

  /* Handle tail bytes. */
  if (len >= 3) {
    h ^= (uint32_t)data[2] << 16;
  }
  if (len >= 2) {
    h ^= (uint32_t)data[1] << 8;
  }
  if (len >= 1) {
    h ^= (uint32_t)data[0];
    h *= m;
  }

  h ^= h >> 13;
  h *= m;
  h ^= h >> 15;

  return h;
}
