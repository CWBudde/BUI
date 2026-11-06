/* Animations (GIF, animated WebP) decoded one frame at a time, at the size
 * they are drawn: the cache keeps the frame on screen and the next one,
 * not the whole animation. Each format provides a BKImAnimCodec that
 * composites its frames (GIF disposal and blending, WebP's canvas) into a
 * full-size premultiplied frame; this file fits frames to the target (the
 * same plan as stills, imb_plan / imb_finish) and loops.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <string.h>

#include "MEM_guardedalloc.h"

#include "image.h"

struct BKImAnim {
  const BKImAnimCodec *codec;
  void *state;
  uint8_t *file; /* owned copy of the encoded file (the codecs read from it) */
  size_t size;
  int w, h;      /* canvas (intrinsic) size */
  int frames, loops;
  int ftype;     /* BK_IMAGE_* */
  int index;     /* the frame next() returns next */
  BKImPlan plan; /* how frames are fitted */
};

const BKImAnimCodec *imb_anim_codec(const uint8_t *mem, size_t size)
{
  if (imb_is_a_gif(mem, size)) {
    return &imb_gif_codec;
  }
  if (imb_webp_animated(mem, size)) {
    return &imb_webp_codec;
  }
  return NULL;
}

BKImAnim *imb_anim_open(uint8_t *file, size_t size, const BKImageFit *target)
{
  BKImageInfo info;
  const BKImFileType *t = imb_file_type(file, size);
  const BKImAnimCodec *codec = NULL;
  BKImAnim *a = NULL;
  if (t && t->load_header(file, size, &info) && info.width > 0 && info.height > 0) {
    codec = imb_anim_codec(file, size);
  }
  if (codec) {
    a = (BKImAnim *)MEM_callocN(sizeof(BKImAnim), "BKImAnim");
    a->codec = codec;
    a->ftype = t->ftype;
    a->file = file;
    a->size = size;
    a->state = codec->open(file, size, &a->w, &a->h, &a->loops);
    a->frames = BK_MAX(1, info.frames);
    if (!a->state || !imb_plan(target, a->w, a->h, 1, &a->plan)) {
      imb_anim_free(a); /* frees `file` too */
      return NULL;
    }
    return a;
  }
  MEM_freeN(file);
  return NULL;
}

void imb_anim_free(BKImAnim *a)
{
  if (a) {
    if (a->state) {
      a->codec->close(a->state);
    }
    MEM_freeN(a->file);
    MEM_freeN(a);
  }
}

int imb_anim_loops(const BKImAnim *a)
{
  return a->loops;
}

int imb_anim_frames(const BKImAnim *a)
{
  return a->frames;
}

size_t imb_anim_bytes(const BKImAnim *a)
{
  return sizeof(*a) + a->size + a->codec->bytes(a->state);
}

/* The next composited frame at full size (a fresh buffer), wrapping to the
 * first after the last. NULL on a decoding error. */
static BKImBuf *next_full(BKImAnim *a, int *r_delay, bool *r_wrapped)
{
  BKImBuf *ib = imb_alloc(a->w, a->h, 1);
  if (!ib) {
    return NULL;
  }
  ib->ftype = a->ftype;
  *r_wrapped = false;
  int got = a->codec->next(a->state, ib->rect, r_delay);
  if (got == 0 && a->index > 0) { /* past the last frame: from the top */
    a->codec->rewind(a->state);
    a->index = 0;
    *r_wrapped = true;
    got = a->codec->next(a->state, ib->rect, r_delay);
  }
  if (got <= 0) {
    MEM_freeN(ib);
    return NULL;
  }
  a->index++;
  a->frames = BK_MAX(a->frames, a->index); /* headers can undercount */
  if (*r_delay <= 10) {
    *r_delay = 100; /* 0 and 10 ms mean "as fast as possible": browsers show 100 ms */
  }
  return ib;
}

BKImBuf *imb_anim_next(BKImAnim *a, int *r_delay, bool *r_wrapped)
{
  BKImBuf *ib = next_full(a, r_delay, r_wrapped);
  return ib ? imb_finish(ib, &a->plan) : NULL;
}

BKImBuf *imb_anim_frame(const uint8_t *mem, size_t size, const BKImageFit *target, int frame)
{
  uint8_t *copy = (uint8_t *)MEM_mallocN(size, "anim file");
  if (!copy) {
    return NULL;
  }
  memcpy(copy, mem, size);
  BKImAnim *a = imb_anim_open(copy, size, target);
  BKImBuf *ib = NULL;
  for (int i = 0; a && i <= frame; i++) {
    int delay;
    bool wrapped;
    MEM_freeN(ib);
    ib = next_full(a, &delay, &wrapped);
    if (!ib || wrapped) { /* past the end: no such frame */
      MEM_freeN(ib);
      ib = NULL;
      break;
    }
  }
  if (ib) {
    ib = imb_finish(ib, &a->plan);
  }
  imb_anim_free(a);
  return ib;
}

BKImBuf *imb_anim_all(const uint8_t *mem, size_t size, const BKImageFit *target)
{
  uint8_t *copy = (uint8_t *)MEM_mallocN(size, "anim file");
  if (!copy) {
    return NULL;
  }
  memcpy(copy, mem, size);
  BKImAnim *a = imb_anim_open(copy, size, target);
  if (!a) {
    return NULL;
  }
  /* every frame in one buffer (the synchronous API), at most 128 MiB of them */
  const int frames = (int)BK_MIN((size_t)a->frames,
                                 BK_MAX((size_t)1, ((size_t)1 << 25) / ((size_t)a->w * a->h)));
  BKImBuf *all = imb_alloc(a->w, a->h, frames);
  int n = 0;
  while (all && n < frames) {
    int delay;
    bool wrapped;
    BKImBuf *ib = next_full(a, &delay, &wrapped);
    if (!ib || wrapped) {
      MEM_freeN(ib);
      break;
    }
    memcpy(imb_frame(all, n), ib->rect, sizeof(uint32_t) * (size_t)a->w * a->h);
    if (all->delays) {
      all->delays[n] = delay;
    }
    MEM_freeN(ib);
    n++;
  }
  if (all && n == 0) {
    MEM_freeN(all);
    all = NULL;
  }
  if (all) {
    all->frames = n;
    all->loops = a->loops;
    all->ftype = a->ftype;
    all = imb_finish(all, &a->plan);
  }
  imb_anim_free(a);
  return all;
}

/* -------------------------------------------------------------------- */
/** \name Public (BUI_kit.h): an animation stepped by the caller
 * \{ */

BKAnim *BK_anim_open(const void *data, size_t len, int max_w, int max_h)
{
  uint8_t *copy = data && len ? (uint8_t *)MEM_mallocN(len, "anim file") : NULL;
  if (!copy) {
    return NULL;
  }
  memcpy(copy, data, len);
  const BKImageFit f = imb_fit_within(max_w, max_h);
  return (BKAnim *)imb_anim_open(copy, len, &f);
}

BKImBuf *BK_anim_next(BKAnim *anim, int *r_delay_ms)
{
  int delay = 0;
  bool wrapped;
  BKImBuf *ib = anim ? imb_anim_next((BKImAnim *)anim, &delay, &wrapped) : NULL;
  if (r_delay_ms) {
    *r_delay_ms = delay;
  }
  return ib;
}

int BK_anim_info(const BKAnim *anim, int *r_w, int *r_h, int *r_loops)
{
  const BKImAnim *a = (const BKImAnim *)anim;
  if (r_w) {
    *r_w = a ? a->w : 0;
  }
  if (r_h) {
    *r_h = a ? a->h : 0;
  }
  if (r_loops) {
    *r_loops = a ? a->loops : 0;
  }
  return a ? a->frames : 0;
}

void BK_anim_free(BKAnim *anim)
{
  imb_anim_free((BKImAnim *)anim);
}

/** \} */
