//! Images through the kit's decoders: every format, `<img>` and canvas
//! files decoded off the UI thread (leaks: image_memory.rs).

use blenderui::prelude::*;
use blenderui::{sys, Image};

fn fixture(name: &str) -> String {
    format!("{}/../../../tests/images/{name}", env!("CARGO_MANIFEST_DIR")).replace('\\', "/")
}

#[test]
fn every_format_decodes() {
    for (name, w, h) in [("quad.png", 8, 6), ("quad.jpg", 8, 6), ("quad.webp", 8, 6), ("quad_lossy.webp", 8, 6),
                         ("quad.gif", 8, 6), ("quad.bmp", 8, 6), ("quad.tga", 8, 6), ("quad.qoi", 8, 6),
                         ("quad.hdr", 8, 6), ("quad.ppm", 8, 6), ("halves.svg", 16, 12), ("anim.gif", 8, 8),
                         ("anim.webp", 8, 8)] {
        let img = Image::open(&fixture(name)).unwrap_or_else(|| panic!("{name}"));
        assert_eq!((img.width, img.height), (w, h), "{name}");
        let bytes = std::fs::read(fixture(name)).unwrap();
        let small = Image::decode(&bytes, 4, 4).unwrap();
        assert!(small.width <= 4 && small.height <= 4, "{name} at size");
    }
    let png = Image::open(&fixture("quad.png")).unwrap();
    assert_eq!(&png.to_rgba()[..4], &[255, 0, 0, 255]);
    let copy = Image::from_rgba(png.width, png.height, &png.to_rgba()).unwrap();
    assert_eq!(copy.to_rgba(), png.to_rgba());
    assert!(Image::decode(b"not an image", 0, 0).is_none());
    assert!(Image::from_rgba(4, 4, &[0; 8]).is_none(), "short pixels rejected at the boundary");
}

struct Files;
impl Render for Files {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let png = fixture("quad.png");
        div()
            .child(img().id("img").src(png.clone()).w(px(16.0)).h(px(12.0)))
            .child(canvas(move |b, p| p.image_file(Bounds { origin: point(b.origin.x, b.origin.y), size: size(16.0, 12.0) }, &png))
                .id("cv")
                .w(px(16.0))
                .h(px(12.0)))
    }
}

#[test]
fn img_and_canvas_files_load_off_thread() {
    Application::headless().run(|cx| {
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds { origin: point(px(0.0), px(0.0)), size: size(px(64.0), px(64.0)) })),
            ..Default::default()
        };
        let win = cx.open_window(opts, |_, cx| cx.new(|_| Files)).unwrap();
        // draw waits for the decodes it started: the pixels are there
        let (pw, _, px) = cx.draw(win.id);
        let at = |x: usize, y: usize| px[y * pw + x] & 0xffffff;
        let b = cx.element_bounds(win.id, "img").unwrap();
        let (x0, y0) = (b.origin.x as usize, b.origin.y as usize);
        assert_eq!(at(x0 + 1, y0 + 1), 0xff0000, "<img> top left red");
        assert_eq!(at(x0 + 14, y0 + 10), 0xffffff, "<img> bottom right white");
        let c = cx.element_bounds(win.id, "cv").unwrap();
        assert_eq!(at(c.origin.x as usize + 1, c.origin.y as usize + 1), 0xff0000, "canvas image_file top left red");
        let mut st = sys::BKImageStats::default();
        // SAFETY: plain stats read.
        unsafe { sys::BK_image_stats(&raw mut st) };
        assert!(st.decoded >= 1 && st.pending == 0, "{st:?}");
        // a settled redraw decodes nothing new
        let decoded = st.decoded;
        cx.draw(win.id);
        unsafe { sys::BK_image_stats(&raw mut st) };
        assert_eq!(st.decoded, decoded);
    });
}

struct Anim;
impl Render for Anim {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let gif = fixture("anim.gif");
        div()
            .child(img().id("play").src(gif.clone()).w(px(16.0)).h(px(16.0)))
            .child(img().id("held").src(gif).attr("frame", "2").w(px(16.0)).h(px(16.0)))
    }
}

#[test]
fn img_animations_play_or_hold_a_frame() {
    Application::headless().run(|cx| {
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds { origin: point(px(0.0), px(0.0)), size: size(px(64.0), px(64.0)) })),
            ..Default::default()
        };
        let win = cx.open_window(opts, |_, cx| cx.new(|_| Anim)).unwrap();
        let at = |cx: &mut App, id: &str| {
            let b = cx.element_bounds(win.id, id).unwrap();
            let (pw, _, px) = cx.draw(win.id);
            px[(b.origin.y as usize + 4) * pw + b.origin.x as usize + 4] & 0xffffff
        };
        // the anim.gif frames are red, green, blue
        let blue = |c: u32| c & 0xff >= 0xf0 && (c >> 16) & 0xff < 0x10;
        let first = at(cx, "play");
        assert!(blue(at(cx, "held")), "frame=\"2\" holds the blue frame");
        let mut seen = vec![first];
        for _ in 0..6 {
            std::thread::sleep(std::time::Duration::from_millis(60));
            let c = at(cx, "play");
            if !seen.contains(&c) {
                seen.push(c);
            }
        }
        assert!(seen.len() >= 2, "it plays: {seen:x?}");
        assert!(blue(at(cx, "held")));
    });
}

#[test]
fn animation_frames_into_a_texture() {
    let bytes = std::fs::read(fixture("anim.webp")).unwrap();
    let mut anim = blenderui::Animation::new(&bytes, 4, 4).expect("an animation");
    assert_eq!((anim.width, anim.height, anim.frames), (8, 8, 3));
    let (first, delay) = anim.next_frame().unwrap();
    assert_eq!((first.width, first.height, delay.as_millis()), (4, 4, 100));
    let tex = first.texture();
    assert!(tex > 0);
    let (second, _) = anim.next_frame().unwrap();
    assert!(second.update_texture(tex));
    assert_ne!(first.to_rgba(), second.to_rgba());
    // SAFETY: frees the texture made above.
    unsafe { sys::BUI_texture_free(tex) };
    assert!(blenderui::Animation::new(b"GIF89a", 0, 0).is_none());
}
