/* The image worker pool: requests queued by priority (visible, then
 * prefetch; newest first within each, so the latest scroll position wins),
 * decoded on a few threads (Blender's BLI_threadpool), handed back to the
 * UI thread as a list it takes at once. Queued requests can be cancelled;
 * a running decode finishes (its result is kept by the cache).
 *
 * Ownership: a request belongs to the loader from bk_loader_push until
 * bk_loader_take_done returns it; the UI thread frees it.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <string.h>

#include "BLI_threads.h"
#include "DNA_listBase.h"
#include "MEM_guardedalloc.h"

#include "image.h"
#include "kit_intern.h" /* bk_time */

static struct {
  bool started, quit;
  ThreadMutex lock;
  ThreadCondition cond;
  ListBase threads;
  int nthreads;    /* running workers */
  int want;        /* BK_image_set_threads (0 = automatic) */
  BKImRequest *queue[2]; /* by priority, doubly linked, newest at the head */
  BKImRequest *done;     /* singly linked */
  int pending;           /* queued + running */
  double decode_s;       /* time the workers spent decoding */
  void (*notify)(void *data);
  void *notify_data;
} L;

static void unlink_queued(BKImRequest *req)
{
  if (req->prev) {
    req->prev->next = req->next;
  }
  else {
    L.queue[req->priority] = req->next;
  }
  if (req->next) {
    req->next->prev = req->prev;
  }
  req->prev = req->next = NULL;
}

static void push_queued(BKImRequest *req)
{
  req->prev = NULL;
  req->next = L.queue[req->priority];
  if (req->next) {
    req->next->prev = req;
  }
  L.queue[req->priority] = req;
}

/* Under the lock; returns whether the done list was empty (notify then). */
static bool push_done(BKImRequest *req)
{
  const bool first = L.done == NULL;
  req->prev = NULL;
  req->next = L.done;
  L.done = req;
  L.pending--;
  return first;
}

void bk_loader_notify(void)
{
  if (L.notify) {
    L.notify(L.notify_data);
  }
  else {
    BK_window_wake(); /* the window loop polls (kit_window_*.c) */
  }
}

/* What a request decodes (on a worker). */
static BKImBuf *decode(BKImRequest *req)
{
  if (req->anim) { /* the next frame of a playing animation */
    return imb_anim_next(req->anim, &req->delay, &req->wrapped);
  }
  size_t size;
  uint8_t *mem = imb_read_file(bk_image_source_path(req->source), 0, &size);
  if (!mem || req->target.frame != 0 || !imb_anim_codec(mem, size)) {
    BKImBuf *ib = mem ? imb_load(mem, size, &req->target) : NULL;
    MEM_freeN(mem);
    return ib;
  }
  /* an animation that plays: its first frame now, the decoder kept (it owns
   * the file) for the next ones; one frame is just a still */
  BKImAnim *a = imb_anim_open(mem, size, &req->target);
  BKImBuf *ib = a ? imb_anim_next(a, &req->delay, &req->wrapped) : NULL;
  if (ib && imb_anim_frames(a) > 1) {
    req->anim = a;
  }
  else {
    imb_anim_free(a);
  }
  return ib;
}

static void *worker(void *arg)
{
  (void)arg;
  BLI_mutex_lock(&L.lock);
  for (;;) {
    while (!L.quit && !L.queue[BK_IMAGE_PRIORITY_VISIBLE] && !L.queue[BK_IMAGE_PRIORITY_PREFETCH]) {
      BLI_condition_wait(&L.cond, &L.lock);
    }
    if (L.quit) {
      break;
    }
    BKImRequest *req = L.queue[BK_IMAGE_PRIORITY_VISIBLE] ? L.queue[BK_IMAGE_PRIORITY_VISIBLE] :
                                                            L.queue[BK_IMAGE_PRIORITY_PREFETCH];
    unlink_queued(req);
    req->state = BK_IMREQ_RUNNING;
    BLI_mutex_unlock(&L.lock);

    const double t0 = bk_time();
    BKImBuf *ib = decode(req);
    const double spent = bk_time() - t0;

    BLI_mutex_lock(&L.lock);
    req->result = ib;
    req->state = BK_IMREQ_DONE;
    L.decode_s += spent;
    if (push_done(req)) {
      BLI_mutex_unlock(&L.lock);
      bk_loader_notify();
      BLI_mutex_lock(&L.lock);
    }
  }
  BLI_mutex_unlock(&L.lock);
  return NULL;
}

static void start(void)
{
  if (L.started) {
    return;
  }
  BLI_mutex_init(&L.lock);
  BLI_condition_init(&L.cond);
  /* decoding is memory-bound past a few threads; leave one core to the UI */
  const int n = L.want > 0 ? L.want : BK_CLAMP(BLI_system_thread_count() - 1, 1, 4);
  BLI_threadpool_init(&L.threads, worker, n);
  for (int i = 0; i < n; i++) {
    BLI_threadpool_insert(&L.threads, NULL);
  }
  L.nthreads = n;
  L.started = true;
}

void bk_loader_push(BKImRequest *req)
{
  start();
  BLI_mutex_lock(&L.lock);
  req->state = BK_IMREQ_QUEUED;
  req->result = NULL;
  push_queued(req);
  L.pending++;
  BLI_mutex_unlock(&L.lock);
  BLI_condition_notify_one(&L.cond);
}

BKImRequest *bk_loader_take_done(void)
{
  if (!L.started) { /* after bk_loader_exit: what it handed back */
    BKImRequest *list = L.done;
    L.done = NULL;
    return list;
  }
  BLI_mutex_lock(&L.lock);
  BKImRequest *list = L.done;
  L.done = NULL;
  BLI_mutex_unlock(&L.lock);
  return list;
}

void bk_loader_cancel(BKImRequest *req)
{
  BLI_mutex_lock(&L.lock);
  if (req->state == BK_IMREQ_QUEUED) {
    unlink_queued(req);
    req->state = BK_IMREQ_CANCELLED;
    push_done(req);
  }
  BLI_mutex_unlock(&L.lock);
}

void bk_loader_touch(BKImRequest *req, int priority, unsigned wanted)
{
  BLI_mutex_lock(&L.lock);
  req->wanted = wanted;
  if (req->state == BK_IMREQ_QUEUED && (priority != req->priority || L.queue[priority] != req)) {
    unlink_queued(req);
    req->priority = priority;
    push_queued(req);
  }
  BLI_mutex_unlock(&L.lock);
}

int bk_loader_pending(double *r_decode_s)
{
  if (!L.started) {
    *r_decode_s = L.decode_s;
    return 0;
  }
  BLI_mutex_lock(&L.lock);
  const int n = L.pending;
  *r_decode_s = L.decode_s;
  BLI_mutex_unlock(&L.lock);
  return n;
}

void bk_loader_exit(void)
{
  if (!L.started) {
    return;
  }
  BLI_mutex_lock(&L.lock);
  L.quit = true;
  for (int p = 0; p < 2; p++) {
    while (L.queue[p]) {
      BKImRequest *req = L.queue[p];
      unlink_queued(req);
      req->state = BK_IMREQ_CANCELLED;
      push_done(req);
    }
  }
  BLI_mutex_unlock(&L.lock);
  BLI_condition_notify_all(&L.cond);
  BLI_threadpool_end(&L.threads); /* running decodes finish into the done list */
  BLI_condition_end(&L.cond);
  BLI_mutex_destroy(&L.lock);
  L.started = L.quit = false;
  L.nthreads = 0;
  L.pending = 0;
  /* L.done stays for the caller's last take (the cache frees them) */
}

/* -------------------------------------------------------------------- */
/** \name Settings (BUI_kit.h)
 * \{ */

void BK_image_set_threads(int n)
{
  L.want = BK_CLAMP(n, 0, 64);
}

void BK_image_set_notify(void (*notify)(void *data), void *data)
{
  L.notify = notify;
  L.notify_data = data;
}

/** \} */
