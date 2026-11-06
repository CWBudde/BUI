/* BUI: jconfigint.h for the vendored libjpeg-turbo 3.1.2 decoder, portable
 * across GCC, Clang and MSVC (CMake generates one per compiler upstream). */
#include <stdint.h>
#define BUILD  "bui"
#define PACKAGE_NAME  "libjpeg-turbo"
#define VERSION  "3.1.2"
#if defined(_MSC_VER) && !defined(__clang__)
#  define HIDDEN
#  define INLINE  __forceinline
#  define THREAD_LOCAL  __declspec(thread)
#  define HAVE_INTRIN_H
#else
#  define HIDDEN  __attribute__((visibility("hidden")))
#  define INLINE  __inline__ __attribute__((always_inline))
#  define THREAD_LOCAL  __thread
#endif
#if SIZE_MAX == 0xffffffffffffffffu
#  define SIZEOF_SIZE_T  8
#else
#  define SIZEOF_SIZE_T  4
#endif
#if defined(__GNUC__) && !defined(_WIN32) && defined(__LP64__)
#  define HAVE_BUILTIN_CTZL
#endif
#if defined(_MSC_VER) && defined(HAVE_INTRIN_H)
#  if (SIZEOF_SIZE_T == 8)
#    define HAVE_BITSCANFORWARD64
#  elif (SIZEOF_SIZE_T == 4)
#    define HAVE_BITSCANFORWARD
#  endif
#endif
#if defined(__has_attribute)
#  if __has_attribute(fallthrough)
#    define FALLTHROUGH  __attribute__((fallthrough));
#  else
#    define FALLTHROUGH
#  endif
#else
#  define FALLTHROUGH
#endif
#ifndef BITS_IN_JSAMPLE
#  define BITS_IN_JSAMPLE  8
#endif
#undef C_ARITH_CODING_SUPPORTED
#undef D_ARITH_CODING_SUPPORTED
#undef WITH_SIMD
#if BITS_IN_JSAMPLE == 8
#  define D_ARITH_CODING_SUPPORTED 1
#endif
