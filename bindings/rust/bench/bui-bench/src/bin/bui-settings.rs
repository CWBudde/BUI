//! The settings window (common/settings.rs) on blenderui. `BUI_GPU=1` for the
//! GPU renderer, `BUI_GPU=0` for the CPU one.
use blenderui::prelude::*;

include!("../../../common/settings.rs");

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds { origin: point(px(100.0), px(100.0)), size: size(px(360.0), px(260.0)) };
        cx.open_window(WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), ..Default::default() },
                       |_, cx| cx.new(|_| Settings { renders: 0 }))
            .unwrap();
    });
}
