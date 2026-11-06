/* Playing an animation held by a cache entry: the frame clock.
 *
 * The entry shows one frame; this keeps the decoder and the next frame,
 * decoded ahead on the workers (one step request at a time), and says when
 * the next frame is due (each frame's own delay, GIF and WebP play counts).
 * It only moves on when the frame on screen was drawn: a window that isn't
 * rendering (hidden, minimized) costs one decode, then nothing, and an
 * entry nobody shows is not stepped at all (cache.c).
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include "MEM_guardedalloc.h"

#include "image.h"

struct BKImPlay {
  BKImAnim *anim; /* NULL while a step request has it */
  BKImBuf *next;  /* the frame after the one shown (NULL while decoding) */
  int next_delay;
  bool next_wrapped; /* `next` starts another play */
  int delay;         /* how long the shown frame stays (ms) */
  double due;        /* when it is over (bk_time seconds; 0: not timed yet) */
  int plays, loops;  /* plays finished; loops 0 = forever */
  bool drawn;        /* the shown frame reached the screen */
  bool finished;     /* the last play's last frame is showing */
};

BKImPlay *imb_play_new(BKImAnim *anim, int delay)
{
  BKImPlay *p = (BKImPlay *)MEM_callocN(sizeof(BKImPlay), "BKImPlay");
  p->anim = anim;
  p->delay = delay;
  p->loops = imb_anim_loops(anim);
  return p;
}

void imb_play_free(BKImPlay *p)
{
  if (p) {
    imb_anim_free(p->anim);
    BK_imbuf_free(p->next);
    MEM_freeN(p);
  }
}

size_t imb_play_bytes(const BKImPlay *p)
{
  return sizeof(*p) + (p->anim ? imb_anim_bytes(p->anim) : 0) + (p->next ? p->next->size : 0);
}

BKImAnim *imb_play_take_step(BKImPlay *p)
{
  if (!p->anim || p->next || p->finished) {
    return NULL;
  }
  BKImAnim *a = p->anim;
  p->anim = NULL;
  return a;
}

void imb_play_step_done(BKImPlay *p, BKImAnim *anim, BKImBuf *next, int delay, bool wrapped)
{
  p->anim = anim;
  BK_imbuf_free(p->next);
  p->next = next;
  p->next_delay = delay;
  p->next_wrapped = wrapped;
  if (!next) {
    p->finished = true; /* a decoding error: stay on this frame */
  }
}

void imb_play_step_cancelled(BKImPlay *p, BKImAnim *anim)
{
  p->anim = anim; /* stepped again when it is wanted */
}

void imb_play_drawn(BKImPlay *p, double now)
{
  p->drawn = true;
  if (p->due == 0.0) {
    p->due = now + p->delay / 1000.0; /* the clock starts when it first shows */
  }
}

void imb_play_rewind_clock(BKImPlay *p)
{
  p->due = 0.0; /* resumed: the frame gets its whole delay again */
}

double imb_play_due(const BKImPlay *p)
{
  return p->drawn && !p->finished && p->next ? p->due : 0.0;
}

BKImBuf *imb_play_advance(BKImPlay *p, double now)
{
  if (imb_play_due(p) == 0.0 || now < p->due) {
    return NULL;
  }
  if (p->next_wrapped && p->loops > 0 && p->plays + 1 >= p->loops) {
    p->finished = true; /* the last frame of the last play stays */
    BK_imbuf_free(p->next);
    p->next = NULL;
    return NULL;
  }
  BKImBuf *frame = p->next;
  p->next = NULL;
  p->plays += p->next_wrapped;
  p->delay = p->next_delay;
  /* keep the beat, unless far behind (paused, slow decodes): restart it */
  p->due = now - p->due > 0.25 ? now + p->delay / 1000.0 : p->due + p->delay / 1000.0;
  p->drawn = false;
  return frame;
}
