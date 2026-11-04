//! The settings window (common/settings.rs) on gpui-fast.
use gpui::{
    App, AppContext, Bounds, Context, IntoElement, ParentElement, Render, Styled, Window, WindowBounds,
    WindowOptions, div, px, rgb, size,
};

include!("../../../common/settings.rs");

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(360.0), px(260.0)), cx);
        cx.open_window(WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), ..Default::default() },
                       |_, cx| cx.new(|_| Settings { renders: 0 }))
            .unwrap();
        cx.activate(true);
    });
}
