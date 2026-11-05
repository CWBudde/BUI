/* Minimal guarded-allocator implementation (plain malloc backing).
 * SPDX-License-Identifier: GPL-2.0-or-later */
#include <stdlib.h>
#include <string.h>
#ifdef __GLIBC__
#  include <malloc.h> /* malloc_usable_size */
#elif defined(__APPLE__)
#  include <malloc/malloc.h> /* malloc_size */
#endif

#include "BLI_compiler_attrs.h"
#include "BLI_utildefines.h"
#include "MEM_guardedalloc.h"

static size_t bui_malloc_usable_size(void *ptr)
{
#ifdef _WIN32
  return ptr ? _msize(ptr) : 0;
#elif defined(__GLIBC__)
  return ptr ? malloc_usable_size(ptr) : 0;
#elif defined(__APPLE__)
  /* macOS: without it MEM_dupallocN returned NULL (BLI_duplicatelist) and
   * MEM_recallocN left the new tail uninitialized */
  return ptr ? malloc_size(ptr) : 0;
#else
  (void)ptr;
  return 0;
#endif
}

void *MEM_mallocN(size_t len, const char *UNUSED(str))
{
  return len ? malloc(len) : NULL;
}

void *MEM_malloc_arrayN(size_t len, size_t size, const char *str)
{
  return MEM_mallocN(len * size, str);
}

void *MEM_callocN(size_t len, const char *UNUSED(str))
{
  return len ? calloc(1, len) : NULL;
}

void *MEM_calloc_arrayN(size_t len, size_t size, const char *str)
{
  return MEM_callocN(len * size, str);
}

void *MEM_mallocN_aligned(size_t len, size_t alignment, const char *str)
{
#ifdef _WIN32
  return len ? _aligned_malloc(len, alignment) : NULL;
#else
  void *ptr = NULL;
  if (len && posix_memalign(&ptr, alignment, len) != 0) {
    ptr = NULL;
  }
  return ptr;
#endif
  (void)str;
}

void MEM_freeN(void *vmemh)
{
#ifdef _WIN32
  /* Cannot distinguish normal vs aligned here; BUI core does not use
   * aligned allocations, so plain free is correct for everything we emit. */
  if (vmemh && ((uintptr_t)vmemh & 0xF) == 0) {
    /* fall through to free(); _aligned_free would break non-aligned blocks */
  }
#endif
  free(vmemh);
}

void *MEM_reallocN_id(void *vmemh, size_t len, const char *UNUSED(str))
{
  if (!len) {
    free(vmemh);
    return NULL;
  }
  void *newp = realloc(vmemh, len);
  return newp;
}

void *MEM_recallocN_id(void *vmemh, size_t len, const char *str)
{
  size_t old_len = vmemh ? bui_malloc_usable_size(vmemh) : 0;
  void *newp = MEM_reallocN_id(vmemh, len, str);
  if (newp && len > old_len) {
    memset(((char *)newp) + old_len, 0, len - old_len);
  }
  return newp;
}

void *MEM_dupallocN(const void *vmemh)
{
  if (!vmemh) {
    return NULL;
  }
  size_t len = bui_malloc_usable_size((void *)vmemh);
  if (!len) { /* Unknown usable size: caller must have used public API. */
    return NULL;
  }
  void *newp = MEM_mallocN(len, "MEM_dupallocN");
  if (newp) {
    memcpy(newp, vmemh, len);
  }
  return newp;
}

size_t MEM_allocN_len(const void *vmemh)
{
  return bui_malloc_usable_size((void *)vmemh);
}

void *MEM_new_uninitialized(size_t size, const char *str)
{
  return MEM_mallocN(size, str);
}

void *MEM_new_zeroed(size_t size, const char *str)
{
  return MEM_callocN(size, str);
}

void *MEM_new_uninitialized_aligned(size_t size, size_t alignment, const char *str)
{
  /* malloc already aligns to 16 bytes; blocks from here are released with
   * MEM_delete_void -> free(), which must not get a Windows _aligned_malloc block. */
  if (alignment <= MEM_MIN_CPP_ALIGNMENT) {
    return MEM_mallocN(size ? size : 1, str);
  }
  return MEM_mallocN_aligned(size, alignment, str);
}

void MEM_delete_void(void *ptr)
{
  MEM_freeN(ptr);
}


void *MEM_dupalloc_void(const void *vmemh)
{
  return MEM_dupallocN(vmemh);
}
