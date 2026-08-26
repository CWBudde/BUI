/* Minimal thread-primitive implementation backing BLI_threads.h.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#include <stdlib.h>

#include "BLI_threads.h"

#ifdef _WIN32
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#endif

void BLI_mutex_init(ThreadMutex *mutex)
{
#ifdef _WIN32
  CRITICAL_SECTION *cs = malloc(sizeof(CRITICAL_SECTION));
  InitializeCriticalSection(cs);
  mutex->impl = cs;
#else
  pthread_mutex_init(&mutex->mutex, NULL);
#endif
}

void BLI_mutex_lock(ThreadMutex *mutex)
{
#ifdef _WIN32
  EnterCriticalSection((CRITICAL_SECTION *)mutex->impl);
#else
  pthread_mutex_lock(&mutex->mutex);
#endif
}

void BLI_mutex_unlock(ThreadMutex *mutex)
{
#ifdef _WIN32
  LeaveCriticalSection((CRITICAL_SECTION *)mutex->impl);
#else
  pthread_mutex_unlock(&mutex->mutex);
#endif
}

void BLI_mutex_destroy(ThreadMutex *mutex)
{
#ifdef _WIN32
  CRITICAL_SECTION *cs = (CRITICAL_SECTION *)mutex->impl;
  if (cs) {
    DeleteCriticalSection(cs);
    free(cs);
    mutex->impl = NULL;
  }
#else
  pthread_mutex_destroy(&mutex->mutex);
#endif
}

/* BLI_task.h stub backend: parallel mempool iteration is not scheduled. */
#include "BLI_mempool.h"
#include "BLI_task.h"

void BLI_task_parallel_mempool(struct BLI_mempool *UNUSED(pool),
                               void *UNUSED(userdata),
                               TaskParallelMempoolFunc UNUSED(func),
                               const bool UNUSED(use_threading))
{
  /* no-op */
}
