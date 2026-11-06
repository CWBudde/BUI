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

/* Condition variables (Blender's BLI_condition_*), waiting on a ThreadMutex. */
#ifdef _WIN32
typedef struct BUI_ThreadConditionImpl {
  void *impl; /* CONDITION_VARIABLE (pointer-sized) */
} ThreadCondition;
#else
typedef pthread_cond_t ThreadCondition;
#endif
void BLI_condition_init(ThreadCondition *cond);
void BLI_condition_wait(ThreadCondition *cond, ThreadMutex *mutex);
void BLI_condition_notify_one(ThreadCondition *cond);
void BLI_condition_notify_all(ThreadCondition *cond);
void BLI_condition_end(ThreadCondition *cond);

/* Blender's thread pool (BLI_threads.cc): `tot` slots in `threadbase`, each
 * running `do_thread(callerdata)` once inserted; end joins them all and
 * frees the slots. */
struct ListBase;
void BLI_threadpool_init(struct ListBase *threadbase, void *(*do_thread)(void *), int tot);
void BLI_threadpool_insert(struct ListBase *threadbase, void *callerdata);
void BLI_threadpool_end(struct ListBase *threadbase);
int BLI_system_thread_count(void);

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
