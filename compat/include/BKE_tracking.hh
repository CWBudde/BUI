/* BUI seam: clip-tracking sample used by scope drawing only.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#pragma once

#include "IMB_imbuf_types.hh"
#include "DNA_tracking_types.h"

#ifdef __cplusplus
namespace blender {
ImBuf *BKE_tracking_sample_pattern(int frame_width,
                                   int frame_height,
                                   const ImBuf *search_ib,
                                   const struct MovieTrackingTrack *track,
                                   const struct MovieTrackingMarker *marker,
                                   bool from_anchor,
                                   bool use_mask,
                                   int num_samples_x,
                                   int num_samples_y,
                                   float pos[2]);
}  // namespace blender
#else
struct ImBuf;
ImBuf *BKE_tracking_sample_pattern(int frame_width,
                                   int frame_height,
                                   const struct ImBuf *search_ib,
                                   const struct MovieTrackingTrack *track,
                                   const struct MovieTrackingMarker *marker,
                                   bool from_anchor,
                                   bool use_mask,
                                   int num_samples_x,
                                   int num_samples_y,
                                   float pos[2]);
#endif
