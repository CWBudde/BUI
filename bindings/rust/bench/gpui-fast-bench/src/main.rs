//! The same scene source as gpui-bench, built against gpui-fast.
const APP_NAME: &str = "gpui-fast";
fn new_app() -> gpui::Application {
    gpui_platform::application()
}
include!("../../gpui-bench/src/scene.rs");
