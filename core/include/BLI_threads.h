/* BUI-owned minimal threading surface for the core library.
 * Replaces Blender's full task-scheduler header; only the primitives the
 * extracted core actually uses are provided.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#ifndef __BUI_THREADS_H__
#define __BUI_THREADS_H__

#include "BLI_sys_types.h"

#ifdef __cplusplus
extern "C" {
#endif

#ifdef _WIN32
typedef struct BUI_ThreadMutexImpl {
  void *impl;
} ThreadMutex;
#else
#include <pthread.h>
typedef struct BUI_ThreadMutexImpl {
  pthread_mutex_t mutex;
} ThreadMutex;
#endif

void BLI_mutex_init(ThreadMutex *mutex);
void BLI_mutex_lock(ThreadMutex *mutex);
void BLI_mutex_unlock(ThreadMutex *mutex);
void BLI_mutex_destroy(ThreadMutex *mutex);

/* Placeholder spin-lock alias kept for source compatibility. */
typedef ThreadMutex SpinLock;

#define BLI_spin_init(mutex) BLI_mutex_init(mutex)
#define BLI_spin_lock(mutex) BLI_mutex_lock(mutex)
#define BLI_spin_unlock(mutex) BLI_mutex_unlock(mutex)
#define BLI_spin_destroy(mutex) BLI_mutex_destroy(mutex)

#ifdef __cplusplus
}
#endif

#endif /* __BUI_THREADS_H__ */
