/* BUI-owned minimal task-parallelism surface.
 * The extracted core only references the parallel-mempool iterator plumbing;
 * real scheduling is out of scope (single-threaded stubs for now).
 * SPDX-License-Identifier: GPL-2.0-or-later */
#ifndef __BUI_TASK_H__
#define __BUI_TASK_H__

#include "BLI_sys_types.h"
#include "BLI_threads.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct TaskParallelTLS {
  void *userdata_chunk;
} TaskParallelTLS;

typedef void (*TaskParallelMempoolFunc)(void *tls, void *elem);

/* Stub: runs nothing. Provide a scheduler-backed implementation later. */
struct BLI_mempool;
void BLI_task_parallel_mempool(struct BLI_mempool *pool,
                              void *userdata,
                              TaskParallelMempoolFunc func,
                              const bool use_threading);

#ifdef __cplusplus
}
#endif

#endif /* __BUI_TASK_H__ */
