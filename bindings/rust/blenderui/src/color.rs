//! Colors: gpui-style constructors (`rgb`, `rgba`, `hsla`) and CSS color parsing.

/// A color with straight (non-premultiplied) alpha, channels in 0..=1.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

/// gpui's color type name; here the same as [`Rgba`].
pub type Hsla = Rgba;

impl Rgba {
    pub const TRANSPARENT: Rgba = Rgba { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };

    /// The same color with its alpha multiplied by `factor`.
    pub fn opacity(self, factor: f32) -> Rgba {
        Rgba { a: self.a * factor, ..self }
    }

    /// The same color with alpha `a`.
    pub fn alpha(self, a: f32) -> Rgba {
        Rgba { a, ..self }
    }

    pub fn is_transparent(&self) -> bool {
        self.a <= 0.0
    }

    pub(crate) fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

/// `rgb(0xRRGGBB)`.
pub fn rgb(hex: u32) -> Rgba {
    rgba((hex << 8) | 0xff)
}

/// `rgba(0xRRGGBBAA)`.
pub fn rgba(hex: u32) -> Rgba {
    let c = |shift: u32| ((hex >> shift) & 0xff) as f32 / 255.0;
    Rgba { r: c(24), g: c(16), b: c(8), a: c(0) }
}

/// `hsla(h, s, l, a)` with every component in 0..=1 (as in gpui).
pub fn hsla(h: f32, s: f32, l: f32, a: f32) -> Rgba {
    let h = h.rem_euclid(1.0) * 6.0;
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    Rgba { r: r + m, g: g + m, b: b + m, a }
}

pub fn black() -> Rgba {
    rgb(0x000000)
}
pub fn white() -> Rgba {
    rgb(0xffffff)
}
pub fn red() -> Rgba {
    rgb(0xff0000)
}
pub fn green() -> Rgba {
    rgb(0x008000)
}
pub fn blue() -> Rgba {
    rgb(0x0000ff)
}
pub fn yellow() -> Rgba {
    rgb(0xffff00)
}
pub fn transparent_black() -> Rgba {
    Rgba::TRANSPARENT
}
pub fn transparent_white() -> Rgba {
    Rgba { r: 1.0, g: 1.0, b: 1.0, a: 0.0 }
}

/// The 148 CSS named colors (CSS Color Module Level 4).
pub(crate) const NAMED: &[(&str, u32)] = &[
    ("aliceblue", 0xf0f8ff), ("antiquewhite", 0xfaebd7), ("aqua", 0x00ffff),
    ("aquamarine", 0x7fffd4), ("azure", 0xf0ffff), ("beige", 0xf5f5dc), ("bisque", 0xffe4c4),
    ("black", 0x000000), ("blanchedalmond", 0xffebcd), ("blue", 0x0000ff),
    ("blueviolet", 0x8a2be2), ("brown", 0xa52a2a), ("burlywood", 0xdeb887),
    ("cadetblue", 0x5f9ea0), ("chartreuse", 0x7fff00), ("chocolate", 0xd2691e),
    ("coral", 0xff7f50), ("cornflowerblue", 0x6495ed), ("cornsilk", 0xfff8dc),
    ("crimson", 0xdc143c), ("cyan", 0x00ffff), ("darkblue", 0x00008b), ("darkcyan", 0x008b8b),
    ("darkgoldenrod", 0xb8860b), ("darkgray", 0xa9a9a9), ("darkgreen", 0x006400),
    ("darkgrey", 0xa9a9a9), ("darkkhaki", 0xbdb76b), ("darkmagenta", 0x8b008b),
    ("darkolivegreen", 0x556b2f), ("darkorange", 0xff8c00), ("darkorchid", 0x9932cc),
    ("darkred", 0x8b0000), ("darksalmon", 0xe9967a), ("darkseagreen", 0x8fbc8f),
    ("darkslateblue", 0x483d8b), ("darkslategray", 0x2f4f4f), ("darkslategrey", 0x2f4f4f),
    ("darkturquoise", 0x00ced1), ("darkviolet", 0x9400d3), ("deeppink", 0xff1493),
    ("deepskyblue", 0x00bfff), ("dimgray", 0x696969), ("dimgrey", 0x696969),
    ("dodgerblue", 0x1e90ff), ("firebrick", 0xb22222), ("floralwhite", 0xfffaf0),
    ("forestgreen", 0x228b22), ("fuchsia", 0xff00ff), ("gainsboro", 0xdcdcdc),
    ("ghostwhite", 0xf8f8ff), ("gold", 0xffd700), ("goldenrod", 0xdaa520), ("gray", 0x808080),
    ("green", 0x008000), ("greenyellow", 0xadff2f), ("grey", 0x808080), ("honeydew", 0xf0fff0),
    ("hotpink", 0xff69b4), ("indianred", 0xcd5c5c), ("indigo", 0x4b0082), ("ivory", 0xfffff0),
    ("khaki", 0xf0e68c), ("lavender", 0xe6e6fa), ("lavenderblush", 0xfff0f5),
    ("lawngreen", 0x7cfc00), ("lemonchiffon", 0xfffacd), ("lightblue", 0xadd8e6),
    ("lightcoral", 0xf08080), ("lightcyan", 0xe0ffff), ("lightgoldenrodyellow", 0xfafad2),
    ("lightgray", 0xd3d3d3), ("lightgreen", 0x90ee90), ("lightgrey", 0xd3d3d3),
    ("lightpink", 0xffb6c1), ("lightsalmon", 0xffa07a), ("lightseagreen", 0x20b2aa),
    ("lightskyblue", 0x87cefa), ("lightslategray", 0x778899), ("lightslategrey", 0x778899),
    ("lightsteelblue", 0xb0c4de), ("lightyellow", 0xffffe0), ("lime", 0x00ff00),
    ("limegreen", 0x32cd32), ("linen", 0xfaf0e6), ("magenta", 0xff00ff), ("maroon", 0x800000),
    ("mediumaquamarine", 0x66cdaa), ("mediumblue", 0x0000cd), ("mediumorchid", 0xba55d3),
    ("mediumpurple", 0x9370db), ("mediumseagreen", 0x3cb371), ("mediumslateblue", 0x7b68ee),
    ("mediumspringgreen", 0x00fa9a), ("mediumturquoise", 0x48d1cc),
    ("mediumvioletred", 0xc71585), ("midnightblue", 0x191970), ("mintcream", 0xf5fffa),
    ("mistyrose", 0xffe4e1), ("moccasin", 0xffe4b5), ("navajowhite", 0xffdead),
    ("navy", 0x000080), ("oldlace", 0xfdf5e6), ("olive", 0x808000), ("olivedrab", 0x6b8e23),
    ("orange", 0xffa500), ("orangered", 0xff4500), ("orchid", 0xda70d6),
    ("palegoldenrod", 0xeee8aa), ("palegreen", 0x98fb98), ("paleturquoise", 0xafeeee),
    ("palevioletred", 0xdb7093), ("papayawhip", 0xffefd5), ("peachpuff", 0xffdab9),
    ("peru", 0xcd853f), ("pink", 0xffc0cb), ("plum", 0xdda0dd), ("powderblue", 0xb0e0e6),
    ("purple", 0x800080), ("rebeccapurple", 0x663399), ("red", 0xff0000),
    ("rosybrown", 0xbc8f8f), ("royalblue", 0x4169e1), ("saddlebrown", 0x8b4513),
    ("salmon", 0xfa8072), ("sandybrown", 0xf4a460), ("seagreen", 0x2e8b57),
    ("seashell", 0xfff5ee), ("sienna", 0xa0522d), ("silver", 0xc0c0c0), ("skyblue", 0x87ceeb),
    ("slateblue", 0x6a5acd), ("slategray", 0x708090), ("slategrey", 0x708090),
    ("snow", 0xfffafa), ("springgreen", 0x00ff7f), ("steelblue", 0x4682b4), ("tan", 0xd2b48c),
    ("teal", 0x008080), ("thistle", 0xd8bfd8), ("tomato", 0xff6347), ("turquoise", 0x40e0d0),
    ("violet", 0xee82ee), ("wheat", 0xf5deb3), ("white", 0xffffff), ("whitesmoke", 0xf5f5f5),
    ("yellow", 0xffff00), ("yellowgreen", 0x9acd32),
];

/// Parse a CSS color: hex, named and system colors, `transparent`, and every
/// CSS Color 4/5 function (`rgb`, `hsl`, `hwb`, `lab`, `lch`, `oklab`,
/// `oklch`, `color()`, `color-mix()`, relative colors, `light-dark()`).
/// `currentcolor` is resolved by the cascade.
pub fn parse_color(s: &str) -> Option<Rgba> {
    let s = s.trim().to_ascii_lowercase();
    if s == "transparent" {
        return Some(Rgba::TRANSPARENT);
    }
    if let Some(hex) = s.strip_prefix('#') {
        let v = u32::from_str_radix(hex, 16).ok()?;
        let nib = |i: usize| (((v >> (4 * i)) & 0xf)) * 0x11;
        return Some(match hex.len() {
            3 => rgb((nib(2) << 16) | (nib(1) << 8) | nib(0)),
            4 => rgba((nib(3) << 24) | (nib(2) << 16) | (nib(1) << 8) | nib(0)),
            6 => rgb(v),
            8 => rgba(v),
            _ => return None,
        });
    }
    if s.contains('(') {
        return match crate::css::tokenize(&s).as_slice() {
            [crate::css::Tok::Func(fx)] => crate::color4::color_fn(fx, Rgba::TRANSPARENT),
            _ => None,
        };
    }
    NAMED.iter().find(|(n, _)| *n == s).map(|(_, v)| rgb(*v))
}

impl From<u32> for Rgba {
    fn from(hex: u32) -> Self {
        rgb(hex)
    }
}

impl From<&str> for Rgba {
    /// CSS color text; invalid colors become transparent.
    fn from(s: &str) -> Self {
        parse_color(s).unwrap_or(Rgba::TRANSPARENT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_css_colors() {
        assert_eq!(parse_color("#f00"), Some(rgb(0xff0000)));
        assert_eq!(parse_color("#00ff0080").unwrap().a, 128.0 / 255.0);
        assert_eq!(parse_color("rgb(0 0 255)"), Some(rgb(0x0000ff)));
        assert_eq!(parse_color("rgba(255, 0, 0, 0.5)").unwrap().a, 0.5);
        assert_eq!(parse_color("rebeccapurple"), Some(rgb(0x663399)));
        let h = parse_color("hsl(120 100% 50%)").unwrap();
        assert!((h.g - 1.0).abs() < 1e-5 && h.r.abs() < 1e-5);
        assert_eq!(NAMED.len(), 148);
    }
}
