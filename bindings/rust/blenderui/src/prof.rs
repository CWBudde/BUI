//! `BUI_PROFILE=1`: per-phase frame timings, averaged and printed to stderr
//! every 60 frames. Costs one thread-local bool check when off.

use std::cell::RefCell;
use std::time::Instant;

thread_local! {
    static ON: bool = std::env::var_os("BUI_PROFILE").is_some();
    static ACC: RefCell<(Vec<(&'static str, f64)>, u32)> = const { RefCell::new((Vec::new(), 0)) };
}

pub(crate) fn on() -> bool {
    ON.with(|o| *o)
}

/// Times its scope into the phase `name`.
pub(crate) struct Span(Option<(&'static str, Instant)>);

pub(crate) fn span(name: &'static str) -> Span {
    Span(on().then(|| (name, Instant::now())))
}

impl Drop for Span {
    fn drop(&mut self) {
        if let Some((name, t)) = self.0 {
            add(name, t.elapsed().as_secs_f64() * 1000.0);
        }
    }
}

/// Add `ms` to phase `name`.
pub(crate) fn add(name: &'static str, ms: f64) {
    ACC.with(|a| {
        let mut a = a.borrow_mut();
        match a.0.iter_mut().find(|(n, _)| *n == name) {
            Some(e) => e.1 += ms,
            None => a.0.push((name, ms)),
        }
    });
}

/// End of a frame: print the averages every 60 frames.
pub(crate) fn frame_done() {
    if !on() {
        return;
    }
    ACC.with(|a| {
        let mut a = a.borrow_mut();
        a.1 += 1;
        if a.1 == 60 {
            let line: Vec<String> = a.0.iter().map(|(n, ms)| format!("{n} {:.2}", ms / 60.0)).collect();
            eprintln!("frame ms: {}", line.join("  "));
            a.0.clear();
            a.1 = 0;
        }
    });
}
