//! Themes: Blender's Dark (default) and Light presets, for Blender widgets and
//! the `styled` HTML sheet. One test (the theme is process-global).

use blenderui::prelude::*;

struct Page;

impl Render for Page {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        body()
            .m_0()
            .child(div().id("themed").styled().h(px(40.0)).child(pre().id("pre").child("x")))
            .child(div().id("plain").h(px(20.0)))
    }
}

fn draw(cx: &mut App) -> (usize, usize, Vec<u32>) {
    let opts = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.0), px(0.0)),
            size: size(px(200.0), px(80.0)),
        })),
        ..Default::default()
    };
    let win = cx.open_window(opts, |_, cx| cx.new(|_| Page)).unwrap();
    let img = cx.draw(win.id);
    cx.close_window(win.id);
    img
}

#[test]
fn dark_and_light_themes() {
    assert_eq!(blenderui::theme_name(), "dark");
    assert_eq!(blenderui::theme_color("region_bg"), Some([48.0 / 255.0, 48.0 / 255.0, 48.0 / 255.0, 1.0]));
    assert!(blenderui::theme_color("regular.inner").is_some());
    assert!(blenderui::theme_color("no.such").is_none());
    assert!(!blenderui::set_theme("solarized"));

    // the light sheet substitutes every dark colour it lists
    let light = blenderui::html::ua_styled_css("light");
    assert!(light.contains("background-color: #b3b3b3; color: #1a1a1a;"));
    assert!(!light.contains("#303030"));
    assert_eq!(blenderui::html::ua_styled_css("dark"), blenderui::html::UA_STYLED_CSS);

    let at = |img: &(usize, usize, Vec<u32>), x: f32, y: f32| {
        let s = img.0 as f32 / 200.0;
        img.2[(y * s) as usize * img.0 + (x * s) as usize]
    };
    Application::headless().run(|cx| {
        let dark = draw(cx);
        assert_eq!(at(&dark, 190.0, 38.0), 0x303030, "styled: Blender's dark background");
        assert_eq!(at(&dark, 190.0, 70.0), 0xffffff, "plain HTML stays white");
    });
    Application::headless().theme("light").run(|cx| {
        assert_eq!(blenderui::theme_name(), "light");
        let light = draw(cx);
        assert_eq!(at(&light, 190.0, 38.0), 0xb3b3b3, "styled: Blender_Light.xml's background");
        assert_eq!(at(&light, 190.0, 70.0), 0xffffff, "plain HTML stays white");
    });
    assert!(blenderui::set_theme("dark"));
    Application::headless().run(|cx| {
        let again = draw(cx);
        assert_eq!(at(&again, 190.0, 38.0), 0x303030);
    });
}
