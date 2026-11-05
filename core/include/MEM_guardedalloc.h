/* Minimal stand-in for Blender's guarded memory allocator.
 * Same API surface, plain malloc/realloc backing. Leak debugging is out of
 * scope for the BUI core; swap in a real allocator behind this header later.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#ifndef __MEM_GUARDEDALLOC_H__
#define __MEM_GUARDEDALLOC_H__

#include <stddef.h>
#include <stdint.h> /* int64_t etc.: the C++ BLI headers use them unqualified */

/* Size-class helpers from upstream MEM_guardedalloc.hh (kept API-compatible). */
#define MEM_SIZE_OPTIMAL(size) ((size))
/* Upstream reserves bytes for per-block bookkeeping; this shim keeps none. */
#define MEM_SIZE_OVERHEAD ((size_t)0)

/* Alignment guaranteed by MEM_new_zeroed / malloc on all supported targets. */
#define MEM_MIN_CPP_ALIGNMENT ((size_t)16)

#ifdef __cplusplus
extern "C" {
#endif

/* Modern MEM API used by the C++ BLI headers (C linkage: guardedalloc.c is C,
 * and the C++ files in core/intern call it). */
void *MEM_new_uninitialized(size_t size, const char *name);
void *MEM_new_zeroed(size_t size, const char *name);
void *MEM_new_uninitialized_aligned(size_t size, size_t alignment, const char *name);
void MEM_delete_void(void *ptr);
void *MEM_dupalloc_void(const void *vmemh);

void *MEM_mallocN(size_t len, const char *str);
void *MEM_malloc_arrayN(size_t len, size_t size, const char *str);
void *MEM_mallocN_aligned(size_t len, size_t alignment, const char *str);
void *MEM_callocN(size_t len, const char *str);
void *MEM_calloc_arrayN(size_t len, size_t size, const char *str);
void *MEM_reallocN_id(void *vmemh, size_t len, const char *str);
void *MEM_recallocN_id(void *vmemh, size_t len, const char *str);
void *MEM_dupallocN(const void *vmemh);
void MEM_freeN(void *vmemh);
size_t MEM_allocN_len(const void *vmemh);

#ifdef __cplusplus
}
#endif

/* C++ helpers mirroring upstream MEM_guardedalloc.hh. */
#ifdef __cplusplus
#include <new>
#include <type_traits>

/* Upstream forms: allocate a single object. */
template<typename T> inline T *MEM_new(const char *name)
{
  void *mem = MEM_new_uninitialized(sizeof(T), name);
  return new (mem) T();
}
template<typename T> inline T *MEM_new_uninitialized(const char *name)
{
  return static_cast<T *>(MEM_new_uninitialized(sizeof(T), name));
}
template<typename T> inline T *MEM_new_zeroed(const char *name)
{
  void *mem = MEM_new_zeroed(sizeof(T), name);
  return new (mem) T();
}
/* Array forms. */
template<typename T> inline T *MEM_new_uninitialized(size_t count, const char *name)
{
  return static_cast<T *>(MEM_new_uninitialized(count * sizeof(T), name));
}
template<typename T> inline T *MEM_new_zeroed(size_t count, const char *name)
{
  return static_cast<T *>(MEM_new_zeroed(count * sizeof(T), name));
}
template<typename T> inline T *MEM_new_array_uninitialized(size_t count, const char *name)
{
  return static_cast<T *>(MEM_new_uninitialized(count * sizeof(T), name));
}

template<typename T> void MEM_delete(T *ptr)
{
  if (ptr) {
    if constexpr (!std::is_trivially_destructible_v<T>) {
      ptr->~T();
    }
    MEM_freeN(ptr);
  }
}

template<typename T> T *MEM_dupalloc(const T *vmemh)
{
  return static_cast<T *>(MEM_dupalloc_void(vmemh));
}

#define MEM_SAFE_DELETE(ptr) \
  do { \
    if (ptr) { \
      MEM_delete(ptr); \
      ptr = nullptr; \
    } \
  } while (0)
#endif

#endif /* __MEM_GUARDEDALLOC_H__ */
