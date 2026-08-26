/* Minimal stand-in for Blender's guarded memory allocator.
 * Same API surface, plain malloc/realloc backing. Leak debugging is out of
 * scope for the BUI core; swap in a real allocator behind this header later.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#ifndef __MEM_GUARDEDALLOC_H__
#define __MEM_GUARDEDALLOC_H__

#include <stddef.h>

/* Size-class helpers from upstream MEM_guardedalloc.hh (kept API-compatible). */
#define MEM_SIZE_OPTIMAL(size) ((size))
/* Upstream reserves bytes for per-block bookkeeping; this shim keeps none. */
#define MEM_SIZE_OVERHEAD ((size_t)0)
#ifdef __cplusplus
extern "C" {
#endif

void *MEM_mallocN(size_t len, const char *str) /* ATTR_MALLOC */ /* ATTR_WARN_UNUSED_RESULT */;
void *MEM_malloc_arrayN(size_t len, size_t size, const char *str) /* ATTR_WARN_UNUSED_RESULT */;
void *MEM_mallocN_aligned(size_t len, size_t alignment, const char *str)
    /* ATTR_MALLOC */ /* ATTR_WARN_UNUSED_RESULT */;
void *MEM_callocN(size_t len, const char *str) /* ATTR_WARN_UNUSED_RESULT */;
void *MEM_calloc_arrayN(size_t len, size_t size, const char *str) /* ATTR_WARN_UNUSED_RESULT */;
void *MEM_reallocN_id(void *vmemh, size_t len, const char *str) /* ATTR_WARN_UNUSED_RESULT */;
void *MEM_recallocN_id(void *vmemh, size_t len, const char *str) /* ATTR_WARN_UNUSED_RESULT */;
void *MEM_dupallocN(const void *vmemh) /* ATTR_WARN_UNUSED_RESULT */;
void MEM_freeN(void *vmemh);
size_t MEM_allocN_len(const void *vmemh) /* ATTR_WARN_UNUSED_RESULT */;

#ifdef __cplusplus
}
#endif

#endif /* __MEM_GUARDEDALLOC_H__ */

