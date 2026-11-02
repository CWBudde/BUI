//! The benchmark scene on gpui 0.2.2.
const APP_NAME: &str = "gpui";
fn new_app() -> gpui::Application {
    gpui::Application::new()
}
include!("scene.rs");
