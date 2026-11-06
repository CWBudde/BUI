/* BUI: libjpeg's system-dependent memory manager (replaces jmemnobs.c, which
 * it otherwise matches: no backing store) on Blender's guarded allocator, so
 * decoder memory is counted by MEM_get_memory_in_use() like the rest of the
 * image module. */
#define JPEG_INTERNALS
#include "jinclude.h"
#include "jpeglib.h"
#include "jmemsys.h"

#include "MEM_guardedalloc.h"

GLOBAL(void *)
jpeg_get_small(j_common_ptr cinfo, size_t sizeofobject)
{
  (void)cinfo;
  return MEM_mallocN(sizeofobject, "libjpeg");
}

GLOBAL(void)
jpeg_free_small(j_common_ptr cinfo, void *object, size_t sizeofobject)
{
  (void)cinfo;
  (void)sizeofobject;
  MEM_freeN(object);
}

GLOBAL(void *)
jpeg_get_large(j_common_ptr cinfo, size_t sizeofobject)
{
  (void)cinfo;
  return MEM_mallocN(sizeofobject, "libjpeg");
}

GLOBAL(void)
jpeg_free_large(j_common_ptr cinfo, void *object, size_t sizeofobject)
{
  (void)cinfo;
  (void)sizeofobject;
  MEM_freeN(object);
}

GLOBAL(size_t)
jpeg_mem_available(j_common_ptr cinfo, size_t min_bytes_needed, size_t max_bytes_needed,
                   size_t already_allocated)
{
  (void)min_bytes_needed;
  if (cinfo->mem->max_memory_to_use) {
    return (size_t)cinfo->mem->max_memory_to_use > already_allocated ?
               cinfo->mem->max_memory_to_use - already_allocated :
               0;
  }
  return max_bytes_needed;
}

GLOBAL(void)
jpeg_open_backing_store(j_common_ptr cinfo, backing_store_ptr info, long total_bytes_needed)
{
  (void)info;
  (void)total_bytes_needed;
  ERREXIT(cinfo, JERR_NO_BACKING_STORE);
}

GLOBAL(long)
jpeg_mem_init(j_common_ptr cinfo)
{
  (void)cinfo;
  return 0;
}

GLOBAL(void)
jpeg_mem_term(j_common_ptr cinfo)
{
  (void)cinfo;
}
