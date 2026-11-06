/* Images: Blender's imbuf reduced to what a UI draws, shared by every
 * binding through BUI_kit.h.
 *
 *  imbuf.c       BKImBuf (ImBuf): one allocation per image, premultiplied
 *                0xAARRGGBB pixels (the raster's and the GPU atlas' layout),
 *                box-filter downscale (IMB_scale), EXIF orientation, fit.
 *  filetype.c    the format table (IMB_FILE_TYPES) and the load path
 *                (IMB_load_image_from_memory): probe, decode at the size
 *                drawn, scale, crop.
 *  format_*.c    one decoder per format (Wuffs, libjpeg-turbo, libwebp,
 *                Radiance HDR, PNM, nanosvg).
 *  cache.c       decoded images by (source, size, fit), within a memory
 *                budget (Blender's moviecache / TextCache generations).
 *  loader.c      the worker pool: requests by priority, cancellation,
 *                results handed to the UI thread.
 *  draw.c        drawing images on nodes and canvas items.
 *
 * Memory: every block comes from MEM_* (MEM_get_memory_in_use counts it).
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */
#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "BUI_kit.h"

#ifndef BK_MIN /* as kit_intern.h */
#  define BK_MIN(a, b) ((a) < (b) ? (a) : (b))
#  define BK_MAX(a, b) ((a) > (b) ? (a) : (b))
#  define BK_CLAMP(v, a, b) ((v) < (a) ? (a) : ((v) > (b) ? (b) : (v)))
#endif

/* Largest side and total pixels a decode may produce (validated at the
 * format boundary, before any allocation). */
#define BK_IMB_MAX_SIDE 16384
#define BK_IMB_MAX_PIXELS ((size_t)1 << 28) /* frames * w * h: 1 GiB of pixels */

struct BKImBuf {
  int x, y;       /* size in pixels (ImBuf::x, ::y) */
  int frames;     /* >= 1 */
  int ftype;      /* BK_IMAGE_* it was decoded from */
  int src_x, src_y; /* intrinsic size of the source (after EXIF orientation) */
  float crop[4];    /* the part to draw (x, y, w, h in these pixels): the visible
                     * region of a fitted decode, sub-pixel exact; else all */
  int loops;      /* animation repeat count, 0 = forever */
  uint64_t key;   /* unique per decoded image (never reused): GPU atlas key */
  size_t size;    /* bytes of this allocation (the cache budget) */
  int *delays;    /* `frames` frame durations in ms (NULL for one frame) */
  uint32_t *rect; /* frames * x * y premultiplied 0xAARRGGBB, top-down rows */
};

/* -- imbuf.c ------------------------------------------------------------------ */

/* One block: header, delays, pixels (uninitialized). NULL when the size is
 * out of range or allocation fails. */
BKImBuf *imb_alloc(int x, int y, int frames);
static inline uint32_t *imb_frame(const BKImBuf *ib, int frame)
{
  return ib->rect + (size_t)frame * ib->x * ib->y;
}
/* Box-filter (area average) resample of every frame to w x h, on
 * premultiplied pixels: the source is freed. Downscales only. */
BKImBuf *imb_scale(BKImBuf *ib, int w, int h);
/* The x, y, w, h part of every frame (freed source). */
BKImBuf *imb_crop(BKImBuf *ib, int x, int y, int w, int h);
/* EXIF orientation 1..8 applied to every frame (freed source). */
BKImBuf *imb_orient(BKImBuf *ib, int orientation);
/* A fresh key: the pixels changed in place (GPU atlases upload again). */
void imb_rekey(BKImBuf *ib);
/* Straight RGBA (8 bits, any stride) <-> premultiplied 0xAARRGGBB rows. */
void imb_premultiply_rgba(uint32_t *dst, const uint8_t *rgba, int n);
void imb_unpremultiply(uint8_t *rgba, const uint32_t *src, int n);

/* Where an image of intrinsic size iw x ih shows in a bw x bh box (CSS
 * object-fit / object-position): `dst` is the visible rect inside the box
 * (box-relative), `src` the part of the image it shows (intrinsic pixels).
 * A box of 0 x 0 is the intrinsic size. */
typedef struct BKImageFit {
  int fit;          /* BK_FIT_* */
  int box_w, box_h; /* 0 x 0: intrinsic */
  float pos_x, pos_y; /* object-position, 0..1 (0.5 = centered) */
  int frame;        /* animations: 0 plays (all frames); n > 0 shows frame n - 1 */
} BKImageFit;
void bk_image_fit(const BKImageFit *f, int iw, int ih, float dst[4], float src[4]);
/* Contain within max_w x max_h (0: that side free; both 0: intrinsic). */
BKImageFit imb_fit_within(int max_w, int max_h);

/* How a decode becomes what is drawn: the whole image at `fw` x `fh` (shown
 * orientation, never above intrinsic), then the visible part `src`. */
typedef struct BKImPlan {
  int iw, ih, orient;
  int fw, fh;
  float src[4];
} BKImPlan;
/* False when nothing of the image shows. */
bool imb_plan(const BKImageFit *t, int iw, int ih, int orient, BKImPlan *r_plan);
/* Scale (to fw x fh), orient and crop a decode (freed) to the plan. */
BKImBuf *imb_finish(BKImBuf *ib, const BKImPlan *p);

/* -- filetype.c ----------------------------------------------------------------- */

typedef struct BKImFileType {
  int ftype;        /* BK_IMAGE_* */
  const char *name; /* "png" */
  bool (*is_a)(const uint8_t *mem, size_t size);
  /* Size and frame count from the header; no pixels. */
  bool (*load_header)(const uint8_t *mem, size_t size, BKImageInfo *r_info);
  /* Decode at least `want_w` x `want_h` (a hint: 0 = full size; formats that
   * decode smaller for free, JPEG DCT scaling and SVG, use it). The result
   * is in the file's orientation; src_x/src_y are set by the caller. */
  BKImBuf *(*load)(const uint8_t *mem, size_t size, int want_w, int want_h);
} BKImFileType;

extern const BKImFileType BK_IMB_FILE_TYPES[];
extern const int BK_IMB_FILE_TYPES_NUM;

const BKImFileType *imb_file_type(const uint8_t *mem, size_t size);
/* The whole decode path for one target: probe, load, scale, crop, orient. */
BKImBuf *imb_load(const uint8_t *mem, size_t size, const BKImageFit *target);
/* Reads a whole file (MEM_mallocN'd), or its first `max` bytes (0: all). */
uint8_t *imb_read_file(const char *path, size_t max, size_t *r_size);

/* -- anim.c: animations, one frame at a time ---------------------------------------- */

/* What a format provides for animations: frames composited (disposal,
 * blending) into a full-size premultiplied canvas copy. */
typedef struct BKImAnimCodec {
  void *(*open)(const uint8_t *mem, size_t size, int *r_w, int *r_h, int *r_loops);
  /* 1 with the next frame in `dst` (w * h), 0 past the last, -1 on errors */
  int (*next)(void *state, uint32_t *dst, int *r_delay_ms);
  void (*rewind)(void *state);
  size_t (*bytes)(const void *state);
  void (*close)(void *state);
} BKImAnimCodec;
extern const BKImAnimCodec imb_gif_codec, imb_webp_codec;
/* The codec for an animation (GIF always, animated WebP). */
const BKImAnimCodec *imb_anim_codec(const uint8_t *mem, size_t size);

typedef struct BKImAnim BKImAnim;
/* Takes `file` (MEM-allocated; freed with the animation, or now on failure). */
BKImAnim *imb_anim_open(uint8_t *file, size_t size, const BKImageFit *target);
void imb_anim_free(BKImAnim *a);
int imb_anim_loops(const BKImAnim *a);
int imb_anim_frames(const BKImAnim *a); /* from the header (at least what was decoded) */
size_t imb_anim_bytes(const BKImAnim *a); /* its decoder state and file */
/* The next frame at the target size, wrapping (`r_wrapped`) after the last. */
BKImBuf *imb_anim_next(BKImAnim *a, int *r_delay_ms, bool *r_wrapped);
/* Frame `frame` alone, or every frame (synchronous API; at most 128 MiB). */
BKImBuf *imb_anim_frame(const uint8_t *mem, size_t size, const BKImageFit *target, int frame);
BKImBuf *imb_anim_all(const uint8_t *mem, size_t size, const BKImageFit *target);

/* -- format_*.c: the decoders ------------------------------------------------------ */

bool imb_is_a_png(const uint8_t *mem, size_t size);
bool imb_is_a_gif(const uint8_t *mem, size_t size);
bool imb_is_a_bmp(const uint8_t *mem, size_t size);
bool imb_is_a_tga(const uint8_t *mem, size_t size);
bool imb_is_a_qoi(const uint8_t *mem, size_t size);
bool imb_header_wuffs(const uint8_t *mem, size_t size, BKImageInfo *r_info);
BKImBuf *imb_load_wuffs(const uint8_t *mem, size_t size, int want_w, int want_h);

bool imb_is_a_pnm(const uint8_t *mem, size_t size);
bool imb_header_pnm(const uint8_t *mem, size_t size, BKImageInfo *r_info);
BKImBuf *imb_load_pnm(const uint8_t *mem, size_t size, int want_w, int want_h);

bool imb_is_a_jpeg(const uint8_t *mem, size_t size);
bool imb_header_jpeg(const uint8_t *mem, size_t size, BKImageInfo *r_info);
BKImBuf *imb_load_jpeg(const uint8_t *mem, size_t size, int want_w, int want_h);
/* EXIF orientation (1..8, 1 when absent) of a JPEG. */
int imb_jpeg_orientation(const uint8_t *mem, size_t size);

bool imb_is_a_webp(const uint8_t *mem, size_t size);
bool imb_webp_animated(const uint8_t *mem, size_t size); /* VP8X animation flag */
bool imb_header_webp(const uint8_t *mem, size_t size, BKImageInfo *r_info);
BKImBuf *imb_load_webp(const uint8_t *mem, size_t size, int want_w, int want_h);

bool imb_is_a_hdr(const uint8_t *mem, size_t size);
bool imb_header_hdr(const uint8_t *mem, size_t size, BKImageInfo *r_info);
BKImBuf *imb_load_hdr(const uint8_t *mem, size_t size, int want_w, int want_h);

bool imb_is_a_svg(const uint8_t *mem, size_t size);
/* Blender's icons (kit_draw.c) and SVG files: rasterized within want_w x want_h. */
BKImBuf *imb_svg_rasterize(const char *svg, int want_w, int want_h);
bool imb_header_svg(const uint8_t *mem, size_t size, BKImageInfo *r_info);
BKImBuf *imb_load_svg(const uint8_t *mem, size_t size, int want_w, int want_h);

/* -- cache.c / loader.c / draw.c: the kit side (UI thread) --------------------------- */

/* A source string interned once (hash + copy); nodes and items keep these. */
typedef struct BKImageSource BKImageSource;
BKImageSource *bk_image_source_get(const char *src); /* adds a reference */
void bk_image_source_release(BKImageSource *s);
const char *bk_image_source_path(const BKImageSource *s);
/* Intrinsic size from the file header (read once, then cached). */
bool bk_image_source_info(BKImageSource *s, BKImageInfo *r_info);

/* Decode priorities: what is drawn first, then prefetches. */
enum { BK_IMAGE_PRIORITY_PREFETCH = 0, BK_IMAGE_PRIORITY_VISIBLE = 1 };

/* Drawing passes (kit_core.c BK_render): images requested inside a pass
 * belong to `ctx`; a pending image that a pass over its whole rectangle
 * didn't ask for again is no longer shown there, and a request nobody shows
 * is cancelled. */
void bk_image_frame_begin(void); /* a drawn frame: next generation, evict, cancel */
void bk_image_pass_begin(BKContext *ctx, int x0, int y0, int x1, int y1);
void bk_image_pass_end(void);
void bk_image_forget_context(BKContext *ctx);
/* Animations: frames whose time came are swapped in (their rectangles
 * redraw), the next ones decoded ahead; only for what is on screen. */
void bk_image_tick(void);
/* When `ctx` next needs bk_image_tick (bk_time seconds; 0: never). */
double bk_image_next_due(BKContext *ctx);
/* The decoded image for (source, target) if cached (used this generation);
 * otherwise NULL, with a decode queued (visible priority) whose arrival
 * redraws `rect` (window pixels, x0 y0 x1 y1) of the pass' context. */
const BKImBuf *bk_image_cache_get(BKImageSource *s, const BKImageFit *target, const int rect[4]);
void bk_image_cache_prefetch(BKImageSource *s, const BKImageFit *target);

/* play.c: the frame clock of a playing animation (owned by its cache entry). */
typedef struct BKImPlay BKImPlay;
BKImPlay *imb_play_new(BKImAnim *anim, int delay); /* takes `anim`; shows its first frame */
void imb_play_free(BKImPlay *p);
size_t imb_play_bytes(const BKImPlay *p);
/* The decoder for the next step request (NULL: nothing to decode now). */
BKImAnim *imb_play_take_step(BKImPlay *p);
/* The step came back: the decoder, and the next frame (NULL: an error). */
void imb_play_step_done(BKImPlay *p, BKImAnim *anim, BKImBuf *next, int delay, bool wrapped);
void imb_play_step_cancelled(BKImPlay *p, BKImAnim *anim); /* the decoder back, no frame */
void imb_play_drawn(BKImPlay *p, double now); /* the shown frame reached the screen */
double imb_play_due(const BKImPlay *p);       /* when to advance (0: not waiting on time) */
/* The frame to show from `now` (the caller frees the old one), or NULL. */
BKImBuf *imb_play_advance(BKImPlay *p, double now);

/* draw.c: what a node or canvas item shows. One of an owned copy (set from
 * RGBA), a borrowed BKImBuf (the host keeps it alive) or a source decoded
 * through the cache. */
typedef enum BKImageRefKind {
  BK_IMREF_NONE = 0,
  BK_IMREF_OWNED,
  BK_IMREF_BORROWED,
  BK_IMREF_SOURCE,
} BKImageRefKind;
typedef struct BKImageRef {
  BKImageRefKind kind;
  union {
    BKImBuf *owned;
    const BKImBuf *borrowed;
    BKImageSource *source;
  } u;
} BKImageRef;
void bk_image_ref_clear(BKImageRef *ref);
/* Straight RGBA, copied (premultiplied); same-size updates reuse the buffer. */
void bk_image_ref_set_rgba(BKImageRef *ref, const unsigned char *rgba, int w, int h);
void bk_image_ref_set_imbuf(BKImageRef *ref, const BKImBuf *ib);
void bk_image_ref_set_source(BKImageRef *ref, const char *src);
/* Intrinsic size (a source reads its header once). */
bool bk_image_ref_size(const BKImageRef *ref, int *r_w, int *r_h);

/* An image placed for drawing: `dst` in window pixels, `src` in its pixels. */
typedef struct BKImageDraw {
  const BKImBuf *ib;
  float dst[4], src[4];
} BKImageDraw;
/* Fits `ref` into `box` (x, y, w, h window pixels) with `fit` (BK_FIT_*);
 * false when there is nothing to draw (yet: a source still decoding). */
bool bk_image_ref_resolve(const BKImageRef *ref, int fit, int frame, const float box[4],
                          BKImageDraw *r);
void bk_image_draw_resolved(BuiRaster *r, const BKImageDraw *d, float alpha);
/* resolve + draw into the frame being rendered */
void bk_image_ref_draw(const BKImageRef *ref, int fit, int frame, const float box[4], float alpha);
void bk_imbuf_draw(const BKImBuf *ib, float x, float y, float w, float h, float alpha);

/* loader.c: a finished request travels back to the cache through these. */
typedef struct BKImRequest BKImRequest;
struct BKImRequest {
  BKImRequest *prev, *next; /* loader queue links */
  BKImageSource *source; /* referenced while queued */
  BKImageFit target;
  int priority;
  unsigned wanted;  /* the cache generation that last asked */
  int state;        /* BK_IMREQ_* */
  BKImBuf *result;  /* NULL when decoding failed */
  void *entry;      /* the cache entry waiting for it (cache.c) */
  BKImAnim *anim;   /* a step: the animation to decode the next frame of; a
                     * first load: set when the file plays (frames > 1) */
  int delay;        /* the result's delay (ms) and whether it starts a new play */
  bool wrapped;
};
enum { BK_IMREQ_QUEUED, BK_IMREQ_RUNNING, BK_IMREQ_DONE, BK_IMREQ_CANCELLED };
/* Queue `req` (ownership moves to the loader until it comes back). */
void bk_loader_push(BKImRequest *req);
/* Requests that finished or were cancelled; ownership returns to the caller. */
BKImRequest *bk_loader_take_done(void);
/* Remove a queued request (it comes back through take_done as cancelled);
 * one already running finishes. */
void bk_loader_cancel(BKImRequest *req);
/* Re-sort a queued request after its priority or generation changed. */
void bk_loader_touch(BKImRequest *req, int priority, unsigned wanted);
/* Requests queued or running, and the time workers spent decoding (seconds). */
int bk_loader_pending(double *r_decode_s);
void bk_loader_exit(void);
/* Called (on a worker thread) when results are ready. */
void bk_loader_notify(void);
