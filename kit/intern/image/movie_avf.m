/* Video on macOS: AVFoundation's AVAssetReader (the system demuxers and
 * VideoToolbox decoders: H.264, HEVC, ProRes, VP9/AV1 where the OS has
 * them), NV12 out of the decoder at the size drawn (kCVPixelBufferWidthKey
 * / HeightKey on the track output: VideoToolbox scales).
 *
 * UNTESTED: written against the documented API without a Mac to run it on;
 * it compiles only on Apple platforms (BUI_VIDEO_AVF) and the frameworks
 * are weak-linked, so a failure here can't affect other platforms or
 * start-up. Manual retain/release, as the Cocoa window host (-fno-objc-arc).
 *
 * AVAssetReader can't seek: a seek (or a new output size) starts a new
 * reader at that time.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#ifdef BUI_VIDEO_AVF

#  import <AVFoundation/AVFoundation.h>
#  import <CoreMedia/CoreMedia.h>
#  import <CoreVideo/CoreVideo.h>

#  include <string.h>

#  include "MEM_guardedalloc.h"

#  include "image.h"

typedef struct AvfMovie {
  AVURLAsset *asset;          /* retained */
  AVAssetTrack *track;        /* retained */
  AVAssetReader *reader;      /* retained; NULL until the first frame */
  AVAssetReaderTrackOutput *out;
  int w, h;                   /* output frame (NV12) */
  double start;               /* where the reader starts (seconds) */
  double frame_dur;
} AvfMovie;

static void reader_drop(AvfMovie *m)
{
  if (m->reader) {
    [m->reader cancelReading];
    [m->reader release];
    m->reader = nil;
  }
  m->out = nil; /* owned by the reader */
}

static void avf_close(void *state)
{
  AvfMovie *m = (AvfMovie *)state;
  @autoreleasepool {
    reader_drop(m);
    [m->track release];
    [m->asset release];
  }
  MEM_freeN(m);
}

/* A reader from m->start, decoding NV12 at m->w x m->h. */
static bool reader_start(AvfMovie *m)
{
  reader_drop(m);
  NSError *err = nil;
  AVAssetReader *r = [[AVAssetReader alloc] initWithAsset:m->asset error:&err];
  if (!r) {
    return false;
  }
  NSDictionary *settings = @{
    (id)kCVPixelBufferPixelFormatTypeKey : @(kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange),
    (id)kCVPixelBufferWidthKey : @(m->w),
    (id)kCVPixelBufferHeightKey : @(m->h),
  };
  AVAssetReaderTrackOutput *out = [AVAssetReaderTrackOutput
      assetReaderTrackOutputWithTrack:m->track
                       outputSettings:settings];
  out.alwaysCopiesSampleData = NO;
  if (![r canAddOutput:out]) {
    [r release];
    return false;
  }
  [r addOutput:out];
  r.timeRange = CMTimeRangeMake(CMTimeMakeWithSeconds(m->start, 90000), kCMTimePositiveInfinity);
  if (![r startReading]) {
    [r release];
    return false;
  }
  m->reader = r;
  m->out = out;
  return true;
}

static void *avf_open(const char *path, const uint8_t *mem, size_t size, BKImAnimInfo *r_info)
{
  (void)mem;
  (void)size;
  if (!path) {
    return NULL;
  }
  AvfMovie *m = (AvfMovie *)MEM_callocN(sizeof(AvfMovie), "AvfMovie");
  @autoreleasepool {
    NSURL *url = [NSURL fileURLWithPath:[NSString stringWithUTF8String:path]];
    m->asset = url ? [[AVURLAsset alloc] initWithURL:url options:nil] : nil;
#  pragma clang diagnostic push
#  pragma clang diagnostic ignored "-Wdeprecated-declarations" /* the synchronous API */
    NSArray<AVAssetTrack *> *tracks = [m->asset tracksWithMediaType:AVMediaTypeVideo];
    m->track = tracks.count ? [tracks[0] retain] : nil;
    const CGSize natural = m->track ? m->track.naturalSize : CGSizeZero;
    const double duration = m->asset ? CMTimeGetSeconds(m->asset.duration) : 0.0;
    const float fps = m->track ? m->track.nominalFrameRate : 0.0f;
#  pragma clang diagnostic pop
    if (!m->track || natural.width < 2 || natural.height < 2) {
      avf_close(m);
      return NULL;
    }
    m->frame_dur = fps > 0 ? 1.0 / fps : 1.0 / 30.0;
    r_info->src_w = (int)natural.width;
    r_info->src_h = (int)natural.height;
    r_info->duration = duration > 0.0 ? duration : 0.0;
    r_info->loops = 1;
    r_info->format = r_info->src_h >= 720 ? BUI_PIXMAP_NV12_BT709 : BUI_PIXMAP_NV12_BT601;
  }
  return m;
}

static bool avf_output(void *state, int w, int h, BKImAnimInfo *r_info)
{
  AvfMovie *m = (AvfMovie *)state;
  m->w = BK_MAX(2, w & ~1);
  m->h = BK_MAX(2, h & ~1);
  bool ok;
  @autoreleasepool {
    ok = reader_start(m);
  }
  r_info->w = m->w;
  r_info->h = m->h;
  return ok;
}

static bool copy_frame(CVPixelBufferRef px, BKImBuf *dst)
{
  if (CVPixelBufferGetPlaneCount(px) != 2 || (int)CVPixelBufferGetWidth(px) != dst->x ||
      (int)CVPixelBufferGetHeight(px) != dst->y ||
      CVPixelBufferLockBaseAddress(px, kCVPixelBufferLock_ReadOnly) != kCVReturnSuccess)
  {
    return false;
  }
  uint8_t *y = (uint8_t *)dst->rect, *uv = y + (size_t)dst->x * dst->y;
  const uint8_t *sy = (const uint8_t *)CVPixelBufferGetBaseAddressOfPlane(px, 0);
  const uint8_t *suv = (const uint8_t *)CVPixelBufferGetBaseAddressOfPlane(px, 1);
  const size_t py = CVPixelBufferGetBytesPerRowOfPlane(px, 0);
  const size_t puv = CVPixelBufferGetBytesPerRowOfPlane(px, 1);
  for (int r = 0; r < dst->y; r++) {
    memcpy(y + (size_t)r * dst->x, sy + r * py, (size_t)dst->x);
  }
  for (int r = 0; r < dst->y / 2; r++) {
    memcpy(uv + (size_t)r * dst->x, suv + r * puv, (size_t)dst->x);
  }
  CVPixelBufferUnlockBaseAddress(px, kCVPixelBufferLock_ReadOnly);
  return true;
}

static int avf_next(void *state, BKImBuf *dst, double *r_pts, double *r_dur)
{
  AvfMovie *m = (AvfMovie *)state;
  int got = -1;
  @autoreleasepool {
    if (!m->reader && !reader_start(m)) {
      return -1;
    }
    CMSampleBufferRef sb = [m->out copyNextSampleBuffer];
    if (!sb) {
      got = m->reader.status == AVAssetReaderStatusCompleted ? 0 : -1;
    }
    else {
      CVPixelBufferRef px = CMSampleBufferGetImageBuffer(sb);
      if (px && copy_frame(px, dst)) {
        const CMTime pts = CMSampleBufferGetPresentationTimeStamp(sb);
        const CMTime dur = CMSampleBufferGetDuration(sb);
        *r_pts = CMTIME_IS_NUMERIC(pts) ? CMTimeGetSeconds(pts) : -1.0;
        *r_dur = CMTIME_IS_NUMERIC(dur) ? CMTimeGetSeconds(dur) : m->frame_dur;
        got = 1;
      }
      CFRelease(sb);
    }
  }
  return got;
}

static bool avf_seek(void *state, double t)
{
  AvfMovie *m = (AvfMovie *)state;
  m->start = t; /* readers start at a time (the keyframe before it: anim.c skips on) */
  bool ok;
  @autoreleasepool {
    ok = reader_start(m);
  }
  return ok;
}

static size_t avf_bytes(const void *state)
{
  const AvfMovie *m = (const AvfMovie *)state;
  return sizeof(*m) + (size_t)m->w * m->h * 3; /* decoded frames queued (an estimate) */
}

const BKImAnimCodec imb_movie_avf = {"avfoundation", true, imb_is_a_movie, avf_open, avf_output,
                                     avf_next, avf_seek, avf_bytes, avf_close};

#endif /* BUI_VIDEO_AVF */
