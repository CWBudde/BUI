/* BUI: jconfig.h for the vendored libjpeg-turbo 3.1.2 decoder (8-bit samples,
 * no SIMD: the x86 SIMD needs NASM, which BUI's builds don't require).
 * Same content as the one CMake generates from src/jconfig.h.in. */
#define JPEG_LIB_VERSION  62
#define LIBJPEG_TURBO_VERSION  3.1.2
#define LIBJPEG_TURBO_VERSION_NUMBER  3001002
#define D_ARITH_CODING_SUPPORTED 1
#define MEM_SRCDST_SUPPORTED  1
#ifndef BITS_IN_JSAMPLE
#define BITS_IN_JSAMPLE  8
#endif
#ifdef _WIN32
#undef RIGHT_SHIFT_IS_UNSIGNED
#ifndef __RPCNDR_H__
typedef unsigned char boolean;
#endif
#define HAVE_BOOLEAN
#if !(defined(_BASETSD_H_) || defined(_BASETSD_H))
typedef short INT16;
typedef signed int INT32;
#endif
#define XMD_H
#endif
