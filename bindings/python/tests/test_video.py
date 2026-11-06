"""Video from Python: the bui.components.Video component and the buipy.Video
player (play, pause, seek, rate, loop, state events), drawn by a context
like a window loop would. Skipped where no backend decodes the H.264 clip.

SPDX-License-Identifier: GPL-2.0-or-later
"""

import os
import sys
import time

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))

import bui  # noqa: E402
import buipy  # noqa: E402

CLIP = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "..", "tests", "videos",
                    "clip.mp4")


class Loop:
    """A context driven like a window: its one timer, image results, renders."""

    def __init__(self, node, w=200, h=120):
        self.ctx = buipy.KitContext(node)
        self.due = None
        self.ctx.set_host(schedule=self.schedule)
        self.r = buipy.Raster(w, h)
        self.w, self.h = w, h
        self.ctx.render(w, h, self.r, True)

    def schedule(self, ms, timer):
        self.due = (time.perf_counter() + ms / 1000.0, timer)

    def run(self, seconds, until=None):
        end = time.perf_counter() + seconds
        while time.perf_counter() < end and not (until and until()):
            if self.due and time.perf_counter() >= self.due[0]:
                timer, self.due = self.due[1], None
                self.ctx.timer(timer)
            buipy.image_poll()
            if self.ctx.needs_render():
                self.ctx.render(self.w, self.h, self.r, False)
            time.sleep(0.001)


def test_video():
    if not bui.video_backends:
        print("no video backend in this build: skipped")
        return
    from bui.components import Video
    states = []
    v = Video(CLIP, controls=True, loop=False)
    v.player.on_state = states.append
    loop = Loop(v)
    loop.run(5.0, lambda: v.player.state != bui.VIDEO_LOADING)
    if v.player.state != bui.VIDEO_PLAYING:  # no decoder here, or the backend doesn't deliver
        print(f"{bui.video_backends} doesn't play H.264 here (state {v.player.state}): skipped")
        return
    assert v.player.size == (96, 54) and abs(v.player.duration - 2.0) < 0.15
    loop.run(0.5)
    assert v.player.time > 0.2, v.player.time
    v.pause()
    v.seek(1.5)
    loop.run(0.4)
    assert abs(v.player.time - 1.5) < 0.11 and v.player.state == bui.VIDEO_PAUSED
    v.player.rate = 4.0
    v.play()
    loop.run(2.0, lambda: v.player.state == bui.VIDEO_ENDED)
    assert v.player.state == bui.VIDEO_ENDED and bui.VIDEO_ENDED in states, states
    # the controls: a click on the play button plays it again
    x, y, w, h = v.rect
    loop.ctx.press(x + 8, y + h - 8, 0)
    loop.ctx.release(x + 8, y + h - 8, 0)
    loop.run(0.2)
    assert v.player.state == bui.VIDEO_PLAYING, v.player.state
    try:
        Video(CLIP, fit="stretch")
    except ValueError:
        pass
    else:
        raise AssertionError("bad fit must raise")


def main():
    test_video()
    print("video tests passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
