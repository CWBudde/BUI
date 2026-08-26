/* Forced-include compat shim: lets upstream blenlib sources compile as C11
 * with zero per-file edits. Only GNU-compatible compilers (GCC/Clang).
 * SPDX-License-Identifier: GPL-2.0-or-later */
#ifndef __BUI_C_COMPAT_H__
#define __BUI_C_COMPAT_H__

#include "BLI_utildefines.h"
#include "MEM_guardedalloc.h"

#ifndef BUI_SWAP
/* Generic swap for fixed-size lvalues (replaces std::swap in converted C). */
#  define BUI_SWAP(a, b) \
    do { \
      unsigned char _bui_sw[sizeof(a)]; \
      memcpy(_bui_sw, &(a), sizeof(a)); \
      memcpy(&(a), &(b), sizeof(a)); \
      memcpy(&(b), _bui_sw, sizeof(a)); \
    } while (0)
#endif

#ifndef MEM_SAFE_DELETE
#define MEM_SAFE_DELETE(p) \
  do { \
    MEM_freeN(p); \
    (p) = NULL; \
  } while (0)
#endif

#endif /* __BUI_C_COMPAT_H__ */

