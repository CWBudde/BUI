//! `BUI_PROFILE=1`: per-phase frame timings, averaged and printed to stderr
//! every 60 frames. Costs one thread-local bool check when off.

use std::cell::Cell;
use std::time::Instant;

/// The timed phases of a frame.
#[derive(Clone, Copy)]
pub(crate) enum Phase {
    FrameInterval,
    Render,
    Cascade,
    Build,
    Boxes,
    Taffy,
    Place,
    Paint,
    Flush,
    Widgets,
}

const NAMES: [&str; 10] = [
    "frame interval", "render", "  cascade", "build", "boxes", "taffy", "place", "paint", "flush",
    "widgets+hover",
];

thread_local! {
    static ON: bool = std::env::var_os("BUI_PROFILE").is_some();
    static ACC: [Cell<f64>; 10] = const { [const { Cell::new(0.0) }; 10] };
    static FRAMES: Cell<u32> = const { Cell::new(0) };
}

pub(crate) fn on() -> bool {
    ON.with(|o| *o)
}

/// Times its scope into a phase.
pub(crate) struct Span(Option<(Phase, Instant)>);

pub(crate) fn span(phase: Phase) -> Span {
    Span(on().then(|| (phase, Instant::now())))
}

impl Drop for Span {
    fn drop(&mut self) {
        if let Some((phase, t)) = self.0 {
            add(phase, t.elapsed().as_secs_f64() * 1000.0);
        }
    }
}

/// Add `ms` to a phase.
pub(crate) fn add(phase: Phase, ms: f64) {
    ACC.with(|a| {
        let c = &a[phase as usize];
        c.set(c.get() + ms);
    });
}

/// End of a frame: print the averages every 60 frames.
pub(crate) fn frame_done() {
    if !on() {
        return;
    }
    let n = FRAMES.with(|f| {
        f.set(f.get() + 1);
        f.get()
    });
    if n < 60 {
        return;
    }
    FRAMES.with(|f| f.set(0));
    let line: Vec<String> = ACC.with(|a| {
        NAMES.iter().zip(a.iter())
            .filter(|(_, c)| c.get() > 0.0)
            .map(|(name, c)| format!("{name} {:.2}", c.replace(0.0) / 60.0))
            .collect()
    });
    eprintln!("frame ms: {}", line.join("  "));
}
