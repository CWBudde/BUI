#!/bin/sh
# Test clips for bui_video_bench (H.264 High, yuv420p, 30 fps, 12 s, a keyframe
# a second): 1080p and 4K. Generated where you run the benchmark, never
# committed. Usage: make_clips.sh <out dir>   (needs ffmpeg with libx264)
set -e
out=${1:-.}
mkdir -p "$out"
for size in 1920x1080 3840x2160; do
  ffmpeg -v error -y -f lavfi -i "testsrc2=size=$size:rate=30:duration=12" \
    -c:v libx264 -preset veryfast -crf 23 -pix_fmt yuv420p -g 30 -movflags +faststart \
    "$out/clip_$size.mp4"
done
ls -l "$out"
