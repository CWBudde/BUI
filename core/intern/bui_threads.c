/* Minimal thread-primitive implementation backing BLI_threads.h.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#include <stdlib.h>

#include "BLI_threads.h"
#include "BLI_listbase.h"
#include "DNA_listBase.h"
#include "MEM_guardedalloc.h"

#ifdef _WIN32
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#else
#include <unistd.h>
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

void BLI_condition_init(ThreadCondition *cond)
{
#ifdef _WIN32
  InitializeConditionVariable((CONDITION_VARIABLE *)&cond->impl);
#else
  pthread_cond_init(cond, NULL);
#endif
}

void BLI_condition_wait(ThreadCondition *cond, ThreadMutex *mutex)
{
#ifdef _WIN32
  SleepConditionVariableCS(
      (CONDITION_VARIABLE *)&cond->impl, (CRITICAL_SECTION *)mutex->impl, INFINITE);
#else
  pthread_cond_wait(cond, &mutex->mutex);
#endif
}

void BLI_condition_notify_one(ThreadCondition *cond)
{
#ifdef _WIN32
  WakeConditionVariable((CONDITION_VARIABLE *)&cond->impl);
#else
  pthread_cond_signal(cond);
#endif
}

void BLI_condition_notify_all(ThreadCondition *cond)
{
#ifdef _WIN32
  WakeAllConditionVariable((CONDITION_VARIABLE *)&cond->impl);
#else
  pthread_cond_broadcast(cond);
#endif
}

void BLI_condition_end(ThreadCondition *cond)
{
#ifdef _WIN32
  (void)cond; /* Win32 condition variables need no cleanup */
#else
  pthread_cond_destroy(cond);
#endif
}

/* -- thread pool (Blender's BLI_threadpool_*, BLI_threads.cc) -------------------- */

typedef struct ThreadSlot {
  struct ThreadSlot *next, *prev;
  void *(*do_thread)(void *);
  void *callerdata;
#ifdef _WIN32
  HANDLE handle;
#else
  pthread_t pthread;
#endif
  int avail;
} ThreadSlot;

#ifdef _WIN32
static DWORD WINAPI thread_main(LPVOID arg)
{
  ThreadSlot *slot = (ThreadSlot *)arg;
  slot->do_thread(slot->callerdata);
  return 0;
}
#endif

void BLI_threadpool_init(ListBase *threadbase, void *(*do_thread)(void *), int tot)
{
  BLI_listbase_clear(threadbase);
  for (int a = 0; a < tot; a++) {
    ThreadSlot *slot = (ThreadSlot *)MEM_callocN(sizeof(ThreadSlot), "threadslot");
    slot->do_thread = do_thread;
    slot->avail = 1;
    BLI_addtail(threadbase, slot);
  }
}

void BLI_threadpool_insert(ListBase *threadbase, void *callerdata)
{
  for (ThreadSlot *slot = (ThreadSlot *)threadbase->first; slot; slot = slot->next) {
    if (slot->avail) {
      slot->avail = 0;
      slot->callerdata = callerdata;
#ifdef _WIN32
      slot->handle = CreateThread(NULL, 0, thread_main, slot, 0, NULL);
      slot->avail = slot->handle == NULL;
#else
      slot->avail = pthread_create(&slot->pthread, NULL, slot->do_thread, callerdata) != 0;
#endif
      return;
    }
  }
}

void BLI_threadpool_end(ListBase *threadbase)
{
  for (ThreadSlot *slot = (ThreadSlot *)threadbase->first; slot; slot = slot->next) {
    if (!slot->avail) {
#ifdef _WIN32
      WaitForSingleObject(slot->handle, INFINITE);
      CloseHandle(slot->handle);
#else
      pthread_join(slot->pthread, NULL);
#endif
    }
  }
  BLI_freelistN(threadbase);
}

int BLI_system_thread_count(void)
{
#ifdef _WIN32
  SYSTEM_INFO info;
  GetSystemInfo(&info);
  return (int)info.dwNumberOfProcessors;
#else
  const long n = sysconf(_SC_NPROCESSORS_ONLN);
  return n > 0 ? (int)n : 1;
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
