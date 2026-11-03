//! CSS Color Level 4 / 5 on tokens: `rgb()` / `hsl()` (legacy and modern
//! syntax, `none`), `hwb()`, `lab()`, `lch()`, `oklab()`, `oklch()`,
//! `color()` in every predefined space, `color-mix()` (all interpolation
//! spaces and hue methods, premultiplied), relative colors
//! (`oklch(from var(--c) calc(l + 0.1) c h / 50%)`), `light-dark()`,
//! system colors, `calc()` in channels, and CSS gamut mapping (chroma
//! reduction in OKLCh) into sRGB.

use crate::color::Rgba;
use crate::css::{FuncTok, Tok};

type V3 = [f64; 3];

fn mul(m: &[[f64; 3]; 3], v: V3) -> V3 {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

const SRGB_TO_XYZ: [[f64; 3]; 3] = [
    [0.41239079926595934, 0.357584339383878, 0.1804807884018343],
    [0.21263900587151027, 0.715168678767756, 0.07219231536073371],
    [0.01933081871559182, 0.11919477979462598, 0.9505321522496607],
];
const XYZ_TO_SRGB: [[f64; 3]; 3] = [
    [3.2409699419045226, -1.537383177570094, -0.4986107602930034],
    [-0.9692436362808796, 1.8759675015077202, 0.04155505740717559],
    [0.05563007969699366, -0.20397695888897652, 1.0569715142428786],
];
const D65_TO_D50: [[f64; 3]; 3] = [
    [1.0479298208405488, 0.022946793341019088, -0.05019222954313557],
    [0.029627815688159344, 0.990434484573249, -0.01707382502938514],
    [-0.009243058152591178, 0.015055144896577895, 0.7518742899580008],
];
const D50_TO_D65: [[f64; 3]; 3] = [
    [0.9554734527042182, -0.023098536874261423, 0.0632593086610217],
    [-0.028369706963208136, 1.0099954580058226, 0.021041398966943008],
    [0.012314001688319899, -0.020507696433477912, 1.3303659366080753],
];
const XYZ_TO_LMS: [[f64; 3]; 3] = [
    [0.8190224379967030, 0.3619062600528904, -0.1288737815209879],
    [0.0329836539323885, 0.9292868615863434, 0.0361446663506424],
    [0.0481771893596242, 0.2642395317527308, 0.6335478284694309],
];
const LMS_TO_OKLAB: [[f64; 3]; 3] = [
    [0.2104542683093140, 0.7936177747023054, -0.0040720430116193],
    [1.9779985324311684, -2.4285922420485799, 0.4505937096174110],
    [0.0259040424655478, 0.7827717124575296, -0.8086757549230774],
];
const OKLAB_TO_LMS: [[f64; 3]; 3] = [
    [1.0, 0.3963377773761749, 0.2158037573099136],
    [1.0, -0.1055613458156586, -0.0638541728258133],
    [1.0, -0.0894841775298119, -1.2914855480194092],
];
const LMS_TO_XYZ: [[f64; 3]; 3] = [
    [1.2268798758459243, -0.5578149944602171, 0.2813910456659647],
    [-0.0405757452148008, 1.1122868032803170, -0.0717110580655164],
    [-0.0763729366746601, -0.4214933324022432, 1.5869240198367816],
];
const P3_TO_XYZ: [[f64; 3]; 3] = [
    [0.48657094864821626, 0.26566769316909294, 0.1982172852343625],
    [0.22897456406974884, 0.6917385218365062, 0.079286914093745],
    [0.0, 0.045113381858902575, 1.0439443689009757],
];
const A98_TO_XYZ: [[f64; 3]; 3] = [
    [0.5766690429101305, 0.1855582379065463, 0.1882286462349947],
    [0.29734497525053605, 0.6273635662554661, 0.07529145849399788],
    [0.02703136138641234, 0.07068885253582723, 0.9913375368376388],
];
const PROPHOTO_TO_XYZ50: [[f64; 3]; 3] = [
    [0.7977666449006423, 0.13518129740053308, 0.0313477341283922],
    [0.2880748288194013, 0.711835234241873, 0.00008993693872564],
    [0.0, 0.0, 0.8251046025104602],
];
const REC2020_TO_XYZ: [[f64; 3]; 3] = [
    [0.6369580483012914, 0.14461690358620832, 0.1688809751641721],
    [0.2627002120112671, 0.6779980715188708, 0.05930171646986196],
    [0.0, 0.028072693049087428, 1.060985057710791],
];
const D50_WHITE: V3 = [0.3457 / 0.3585, 1.0, (1.0 - 0.3457 - 0.3585) / 0.3585];

fn inv(m: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let [[a, b, c], [d, e, f], [g, h, i]] = *m;
    let det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
    let k = 1.0 / det;
    [
        [(e * i - f * h) * k, (c * h - b * i) * k, (b * f - c * e) * k],
        [(f * g - d * i) * k, (a * i - c * g) * k, (c * d - a * f) * k],
        [(d * h - e * g) * k, (b * g - a * h) * k, (a * e - b * d) * k],
    ]
}

fn srgb_to_lin(v: f64) -> f64 {
    let a = v.abs();
    let r = if a <= 0.04045 { a / 12.92 } else { ((a + 0.055) / 1.055).powf(2.4) };
    r.copysign(v)
}
fn lin_to_srgb(v: f64) -> f64 {
    let a = v.abs();
    let r = if a > 0.0031308 { 1.055 * a.powf(1.0 / 2.4) - 0.055 } else { 12.92 * a };
    r.copysign(v)
}

/// The color spaces CSS can name, for `color()` and `color-mix()`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Space {
    Srgb,
    SrgbLinear,
    DisplayP3,
    A98,
    ProPhoto,
    Rec2020,
    XyzD50,
    XyzD65,
    Lab,
    Lch,
    Oklab,
    Oklch,
    Hsl,
    Hwb,
}

impl Space {
    pub fn named(s: &str) -> Option<Space> {
        Some(match s {
            "srgb" => Space::Srgb,
            "srgb-linear" => Space::SrgbLinear,
            "display-p3" | "display-p3-linear" => Space::DisplayP3,
            "a98-rgb" => Space::A98,
            "prophoto-rgb" => Space::ProPhoto,
            "rec2020" => Space::Rec2020,
            "xyz-d50" => Space::XyzD50,
            "xyz" | "xyz-d65" => Space::XyzD65,
            "lab" => Space::Lab,
            "lch" => Space::Lch,
            "oklab" => Space::Oklab,
            "oklch" => Space::Oklch,
            "hsl" => Space::Hsl,
            "hwb" => Space::Hwb,
            _ => return None,
        })
    }

    fn hue_index(self) -> Option<usize> {
        match self {
            Space::Lch | Space::Oklch => Some(2),
            Space::Hsl | Space::Hwb => Some(0),
            _ => None,
        }
    }

    /// Coordinates in this space -> CIE XYZ (D65).
    pub fn to_xyz(self, c: V3) -> V3 {
        match self {
            Space::Srgb => mul(&SRGB_TO_XYZ, c.map(srgb_to_lin)),
            Space::SrgbLinear => mul(&SRGB_TO_XYZ, c),
            Space::DisplayP3 => mul(&P3_TO_XYZ, c.map(srgb_to_lin)),
            Space::A98 => mul(&A98_TO_XYZ, c.map(|v| v.abs().powf(563.0 / 256.0).copysign(v))),
            Space::ProPhoto => {
                let lin = c.map(|v| {
                    let a = v.abs();
                    (if a <= 16.0 / 512.0 { a / 16.0 } else { a.powf(1.8) }).copysign(v)
                });
                mul(&D50_TO_D65, mul(&PROPHOTO_TO_XYZ50, lin))
            }
            Space::Rec2020 => {
                const A: f64 = 1.09929682680944;
                const B: f64 = 0.018053968510807;
                let lin = c.map(|v| {
                    let a = v.abs();
                    (if a < B * 4.5 { a / 4.5 } else { ((a + A - 1.0) / A).powf(1.0 / 0.45) }).copysign(v)
                });
                mul(&REC2020_TO_XYZ, lin)
            }
            Space::XyzD65 => c,
            Space::XyzD50 => mul(&D50_TO_D65, c),
            Space::Lab => mul(&D50_TO_D65, lab_to_xyz50(c)),
            Space::Lch => Space::Lab.to_xyz(lch_to_lab(c)),
            Space::Oklab => oklab_to_xyz(c),
            Space::Oklch => oklab_to_xyz(lch_to_lab(c)),
            Space::Hsl => Space::Srgb.to_xyz(hsl_to_srgb(c)),
            Space::Hwb => Space::Srgb.to_xyz(hwb_to_srgb(c)),
        }
    }

    /// CIE XYZ (D65) -> coordinates in this space.
    pub fn from_xyz(self, x: V3) -> V3 {
        match self {
            Space::Srgb => mul(&XYZ_TO_SRGB, x).map(lin_to_srgb),
            Space::SrgbLinear => mul(&XYZ_TO_SRGB, x),
            Space::DisplayP3 => mul(&inv(&P3_TO_XYZ), x).map(lin_to_srgb),
            Space::A98 => mul(&inv(&A98_TO_XYZ), x).map(|v| v.abs().powf(256.0 / 563.0).copysign(v)),
            Space::ProPhoto => mul(&inv(&PROPHOTO_TO_XYZ50), mul(&D65_TO_D50, x)).map(|v| {
                let a = v.abs();
                (if a >= 1.0 / 512.0 { a.powf(1.0 / 1.8) } else { 16.0 * a }).copysign(v)
            }),
            Space::Rec2020 => {
                const A: f64 = 1.09929682680944;
                const B: f64 = 0.018053968510807;
                mul(&inv(&REC2020_TO_XYZ), x).map(|v| {
                    let a = v.abs();
                    (if a > B { A * a.powf(0.45) - (A - 1.0) } else { 4.5 * a }).copysign(v)
                })
            }
            Space::XyzD65 => x,
            Space::XyzD50 => mul(&D65_TO_D50, x),
            Space::Lab => xyz50_to_lab(mul(&D65_TO_D50, x)),
            Space::Lch => lab_to_lch(Space::Lab.from_xyz(x)),
            Space::Oklab => xyz_to_oklab(x),
            Space::Oklch => lab_to_lch(xyz_to_oklab(x)),
            Space::Hsl => srgb_to_hsl(Space::Srgb.from_xyz(x)),
            Space::Hwb => srgb_to_hwb(Space::Srgb.from_xyz(x)),
        }
    }
}

fn lab_to_xyz50(lab: V3) -> V3 {
    const K: f64 = 24389.0 / 27.0;
    const E: f64 = 216.0 / 24389.0;
    let f1 = (lab[0] + 16.0) / 116.0;
    let f0 = lab[1] / 500.0 + f1;
    let f2 = f1 - lab[2] / 200.0;
    let x = if f0.powi(3) > E { f0.powi(3) } else { (116.0 * f0 - 16.0) / K };
    let y = if lab[0] > K * E { ((lab[0] + 16.0) / 116.0).powi(3) } else { lab[0] / K };
    let z = if f2.powi(3) > E { f2.powi(3) } else { (116.0 * f2 - 16.0) / K };
    [x * D50_WHITE[0], y * D50_WHITE[1], z * D50_WHITE[2]]
}

fn xyz50_to_lab(x: V3) -> V3 {
    const K: f64 = 24389.0 / 27.0;
    const E: f64 = 216.0 / 24389.0;
    let f = |v: f64| if v > E { v.cbrt() } else { (K * v + 16.0) / 116.0 };
    let [fx, fy, fz] = [f(x[0] / D50_WHITE[0]), f(x[1] / D50_WHITE[1]), f(x[2] / D50_WHITE[2])];
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

fn oklab_to_xyz(lab: V3) -> V3 {
    mul(&LMS_TO_XYZ, mul(&OKLAB_TO_LMS, lab).map(|v| v * v * v))
}

fn xyz_to_oklab(x: V3) -> V3 {
    mul(&LMS_TO_OKLAB, mul(&XYZ_TO_LMS, x).map(f64::cbrt))
}

fn lch_to_lab(c: V3) -> V3 {
    let h = c[2].to_radians();
    [c[0], c[1] * h.cos(), c[1] * h.sin()]
}

fn lab_to_lch(c: V3) -> V3 {
    let ch = (c[1] * c[1] + c[2] * c[2]).sqrt();
    let h = if ch < 1e-6 { 0.0 } else { c[2].atan2(c[1]).to_degrees().rem_euclid(360.0) };
    [c[0], ch, h]
}

/// hsl(h deg, s 0..1, l 0..1) -> sRGB 0..1
fn hsl_to_srgb(c: V3) -> V3 {
    let (h, s, l) = (c[0].rem_euclid(360.0), c[1], c[2]);
    let f = |n: f64| {
        let k = (n + h / 30.0) % 12.0;
        let a = s * l.min(1.0 - l);
        l - a * (k - 3.0).min(9.0 - k).clamp(-1.0, 1.0)
    };
    [f(0.0), f(8.0), f(4.0)]
}

fn srgb_to_hsl(c: V3) -> V3 {
    let (r, g, b) = (c[0], c[1], c[2]);
    let (mx, mn) = (r.max(g).max(b), r.min(g).min(b));
    let l = (mx + mn) / 2.0;
    let d = mx - mn;
    if d.abs() < 1e-9 {
        return [0.0, 0.0, l];
    }
    let s = if l == 0.0 || l == 1.0 { 0.0 } else { (mx - l) / l.min(1.0 - l) };
    let h = if mx == r {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if mx == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    [h * 60.0, s, l]
}

fn hwb_to_srgb(c: V3) -> V3 {
    let (w, b) = (c[1], c[2]);
    if w + b >= 1.0 {
        let g = w / (w + b);
        return [g, g, g];
    }
    hsl_to_srgb([c[0], 1.0, 0.5]).map(|v| v * (1.0 - w - b) + w)
}

fn srgb_to_hwb(c: V3) -> V3 {
    let h = srgb_to_hsl(c)[0];
    let w = c[0].min(c[1]).min(c[2]);
    let b = 1.0 - c[0].max(c[1]).max(c[2]);
    [h, w, b]
}

/// CSS Color 4 gamut mapping: reduce OKLCh chroma until the color is in
/// sRGB (within a just-noticeable difference), instead of plain clipping.
fn gamut_map_srgb(xyz: V3) -> V3 {
    let rgb = Space::Srgb.from_xyz(xyz);
    let inside = |c: &V3| c.iter().all(|v| (-1e-6..=1.0 + 1e-6).contains(v));
    if inside(&rgb) {
        return rgb.map(|v| v.clamp(0.0, 1.0));
    }
    let lch = Space::Oklch.from_xyz(xyz);
    if lch[0] >= 1.0 {
        return [1.0; 3];
    }
    if lch[0] <= 0.0 {
        return [0.0; 3];
    }
    let clip = |c: V3| c.map(|v| v.clamp(0.0, 1.0));
    let delta = |a: V3, b: V3| {
        let (x, y) = (xyz_to_oklab(Space::Srgb.to_xyz(a)), xyz_to_oklab(Space::Srgb.to_xyz(b)));
        ((x[0] - y[0]).powi(2) + (x[1] - y[1]).powi(2) + (x[2] - y[2]).powi(2)).sqrt()
    };
    let (mut lo, mut hi) = (0.0, lch[1]);
    let mut best = clip(rgb);
    for _ in 0..24 {
        let mid = (lo + hi) / 2.0;
        let cand = Space::Srgb.from_xyz(Space::Oklch.to_xyz([lch[0], mid, lch[2]]));
        if inside(&cand) {
            lo = mid;
            best = clip(cand);
        } else {
            let clipped = clip(cand);
            if delta(clipped, cand) < 0.02 {
                return clipped;
            }
            hi = mid;
        }
    }
    best
}

fn to_rgba(space: Space, c: V3, alpha: f64) -> Rgba {
    let rgb = if space == Space::Srgb && c.iter().all(|v| (0.0..=1.0).contains(v)) {
        c
    } else {
        gamut_map_srgb(space.to_xyz(c))
    };
    Rgba { r: rgb[0] as f32, g: rgb[1] as f32, b: rgb[2] as f32, a: alpha.clamp(0.0, 1.0) as f32 }
}

fn coords(space: Space, c: Rgba) -> V3 {
    let rgb = [c.r as f64, c.g as f64, c.b as f64];
    if space == Space::Srgb { rgb } else { space.from_xyz(Space::Srgb.to_xyz(rgb)) }
}

// -- parsing ---------------------------------------------------------------------------

/// What `light-dark()` picks: the UI is dark.
pub const DARK: bool = true;

/// A system color (`Canvas`, `LinkText` ...), in the kit's dark theme.
pub fn system(name: &str) -> Option<Rgba> {
    use crate::color::rgb;
    Some(match name {
        "canvas" | "field" | "window" | "background" | "appworkspace" => rgb(0x282828),
        "canvastext" | "fieldtext" | "windowtext" | "captiontext" | "infotext" | "menutext"
        | "buttontext" => rgb(0xe6e6e6),
        "linktext" => rgb(0x5c9ce6),
        "visitedtext" => rgb(0xb07ce6),
        "activetext" => rgb(0xe65c5c),
        "buttonface" | "buttonhighlight" | "buttonshadow" | "threedface" | "threedhighlight"
        | "threedshadow" | "threeddarkshadow" | "threedlightshadow" | "menu" | "scrollbar" => rgb(0x545454),
        "buttonborder" | "activeborder" | "inactiveborder" | "windowframe" => rgb(0x3d3d3d),
        "highlight" | "selecteditem" | "accentcolor" | "activecaption" => rgb(0x4772b3),
        "highlighttext" | "selecteditemtext" | "accentcolortext" | "inactivecaptiontext" => rgb(0xffffff),
        "mark" => rgb(0xe6c84a),
        "marktext" => rgb(0x000000),
        "graytext" | "inactivecaption" | "infobackground" | "inactiveborder2" => rgb(0x8c8c8c),
        _ => return None,
    })
}

/// Channel keywords of a relative color (`from <color>`), in the target space.
type Channels = [(&'static str, f64); 4];

/// One component: number, percentage (of `pct_ref`), angle (hue), `none`,
/// `calc()`, or a channel keyword of a relative color.
fn component(toks: &[Tok], pct_ref: f64, hue: bool, ch: Option<&Channels>) -> Option<f64> {
    let t = match toks {
        [t] => t,
        _ => return None,
    };
    let chan = |name: &str| ch.and_then(|c| c.iter().find(|(n, _)| *n == name).map(|(_, v)| *v));
    Some(match t {
        Tok::Num(v) => *v as f64,
        Tok::Pct(p) => *p as f64 / 100.0 * pct_ref,
        Tok::Dim(v, u) if hue => match u.as_ref() {
            "deg" => *v as f64,
            "rad" => (*v as f64).to_degrees(),
            "grad" => *v as f64 * 0.9,
            "turn" => *v as f64 * 360.0,
            _ => return None,
        },
        Tok::Ident(s) if s == "none" => 0.0,
        Tok::Ident(s) => chan(s)?,
        Tok::Func(fx) if crate::calc::is_math_fn(&fx.name) => {
            // channel keywords inside calc() become their numbers
            let args: Vec<Vec<Tok>> = fx.args.iter().map(|a| a.iter().map(|t| match t {
                Tok::Ident(s) => chan(s).map(|v| Tok::Num(v as f32)).unwrap_or_else(|| t.clone()),
                other => other.clone(),
            }).collect()).collect();
            let c = crate::calc::Calc::parse_fn(&fx.name, &args)?;
            let u = crate::geometry::Units { font_size: 16.0, viewport: (0.0, 0.0) };
            let v = c.eval_units(&u, 100.0) as f64;
            // a percentage inside calc resolves against 100: rescale it
            if c.has_pct() { v / 100.0 * pct_ref } else { v }
        }
        _ => return None,
    })
}

/// Components of a color function: (c1, c2, c3, alpha tokens), modern
/// (`a b c / d`) or legacy (`a, b, c, d`) syntax.
fn split_components(args: &[Vec<Tok>]) -> Option<(Vec<Vec<Tok>>, Option<Vec<Tok>>)> {
    if args.len() >= 3 {
        let parts: Vec<Vec<Tok>> = args.iter().take(3).cloned().collect();
        return Some((parts, args.get(3).cloned()));
    }
    let toks = args.first()?;
    let (main, alpha) = match toks.iter().position(|t| matches!(t, Tok::Slash)) {
        Some(i) => (&toks[..i], Some(toks[i + 1..].to_vec())),
        None => (&toks[..], None),
    };
    Some((main.iter().map(|t| vec![t.clone()]).collect(), alpha))
}

/// A color function token (`rgb(...)`, `color-mix(...)` ...).
pub fn color_fn(fx: &FuncTok, current: Rgba) -> Option<Rgba> {
    let name = fx.name.as_ref();
    match name {
        "light-dark" => {
            let pick = if DARK { fx.args.get(1)? } else { fx.args.first()? };
            return color_toks(pick, current);
        }
        "color-mix" => return color_mix(fx, current),
        _ => {}
    }
    // relative color: `from <color>` first
    let (base, args): (Option<Rgba>, Vec<Vec<Tok>>) = match fx.args.first().and_then(|a| a.first()) {
        Some(Tok::Ident(s)) if s == "from" => {
            let a0 = fx.args.first()?;
            let base = color_toks(&a0[1..2], current)?;
            let mut rest = vec![a0[2..].to_vec()];
            rest.extend(fx.args[1..].iter().cloned());
            (Some(base), rest)
        }
        _ => (None, fx.args.clone()),
    };
    let (space, args) = if name == "color" {
        let first = args.first()?;
        let sp = Space::named(first.first()?.ident()?)?;
        let mut rest = vec![first[1..].to_vec()];
        rest.extend(args[1..].iter().cloned());
        (sp, rest)
    } else {
        (match name {
            "rgb" | "rgba" => Space::Srgb,
            "hsl" | "hsla" => Space::Hsl,
            "hwb" => Space::Hwb,
            "lab" => Space::Lab,
            "lch" => Space::Lch,
            "oklab" => Space::Oklab,
            "oklch" => Space::Oklch,
            _ => return None,
        }, args)
    };
    let (parts, alpha) = split_components(&args)?;
    if parts.len() != 3 {
        return None;
    }
    // channel keywords for relative colors, in the units the syntax uses
    let channels: Option<Channels> = base.map(|b| {
        let c = coords(space, b);
        let a = b.a as f64;
        match name {
            "rgb" | "rgba" => [("r", c[0] * 255.0), ("g", c[1] * 255.0), ("b", c[2] * 255.0), ("alpha", a)],
            "hsl" | "hsla" => [("h", c[0]), ("s", c[1] * 100.0), ("l", c[2] * 100.0), ("alpha", a)],
            "hwb" => [("h", c[0]), ("w", c[1] * 100.0), ("b", c[2] * 100.0), ("alpha", a)],
            "lab" | "oklab" => [("l", c[0]), ("a", c[1]), ("b", c[2]), ("alpha", a)],
            "lch" | "oklch" => [("l", c[0]), ("c", c[1]), ("h", c[2]), ("alpha", a)],
            _ => match space {
                Space::XyzD50 | Space::XyzD65 => [("x", c[0]), ("y", c[1]), ("z", c[2]), ("alpha", a)],
                _ => [("r", c[0]), ("g", c[1]), ("b", c[2]), ("alpha", a)],
            },
        }
    });
    let ch = channels.as_ref();
    let comp = |i: usize, pct: f64, hue: bool| component(&parts[i], pct, hue, ch);
    let c = match name {
        "rgb" | "rgba" => [comp(0, 255.0, false)? / 255.0, comp(1, 255.0, false)? / 255.0,
                           comp(2, 255.0, false)? / 255.0],
        // s / l (w / b) as percentages; bare numbers in modern syntax mean percent too
        "hsl" | "hsla" | "hwb" => [comp(0, 0.0, true)?, comp(1, 100.0, false)? / 100.0,
                                   comp(2, 100.0, false)? / 100.0],
        "lab" => [comp(0, 100.0, false)?, comp(1, 125.0, false)?, comp(2, 125.0, false)?],
        "lch" => [comp(0, 100.0, false)?, comp(1, 150.0, false)?, comp(2, 0.0, true)?],
        "oklab" => [comp(0, 1.0, false)?, comp(1, 0.4, false)?, comp(2, 0.4, false)?],
        "oklch" => [comp(0, 1.0, false)?, comp(1, 0.4, false)?, comp(2, 0.0, true)?],
        _ => [comp(0, 1.0, false)?, comp(1, 1.0, false)?, comp(2, 1.0, false)?],
    };
    let a = match &alpha {
        Some(t) => component(t, 1.0, false, ch)?,
        None => base.map(|b| b.a as f64).unwrap_or(1.0),
    };
    let c = match space {
        Space::Lch | Space::Oklch => [c[0], c[1].max(0.0), c[2]],
        Space::Lab | Space::Oklab => c,
        _ => c,
    };
    Some(to_rgba(space, c, a.clamp(0.0, 1.0)))
}

/// A color from a token list (one color value).
pub fn color_toks(toks: &[Tok], current: Rgba) -> Option<Rgba> {
    match toks {
        [t] => crate::style::color(t, current),
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum HueMethod {
    Shorter,
    Longer,
    Increasing,
    Decreasing,
}

fn color_mix(fx: &FuncTok, current: Rgba) -> Option<Rgba> {
    // in <space> [<method> hue], <color> [<pct>], <color> [<pct>]
    let head = fx.args.first()?;
    if head.first()?.ident()? != "in" {
        return None;
    }
    let space = Space::named(head.get(1)?.ident()?)?;
    let method = match head.get(2).and_then(Tok::ident) {
        Some("longer") => HueMethod::Longer,
        Some("increasing") => HueMethod::Increasing,
        Some("decreasing") => HueMethod::Decreasing,
        _ => HueMethod::Shorter,
    };
    let item = |toks: &Vec<Tok>| -> Option<(Rgba, Option<f64>)> {
        let pct = toks.iter().find_map(|t| match t {
            Tok::Pct(p) => Some(*p as f64 / 100.0),
            _ => None,
        });
        let col: Vec<Tok> = toks.iter().filter(|t| !matches!(t, Tok::Pct(_))).cloned().collect();
        Some((color_toks(&col, current)?, pct))
    };
    let (c1, p1) = item(fx.args.get(1)?)?;
    let (c2, p2) = item(fx.args.get(2)?)?;
    let (mut p1, mut p2) = match (p1, p2) {
        (None, None) => (0.5, 0.5),
        (Some(a), None) => (a, 1.0 - a),
        (None, Some(b)) => (1.0 - b, b),
        (Some(a), Some(b)) => (a, b),
    };
    let sum = p1 + p2;
    if sum <= 0.0 {
        return None;
    }
    let alpha_mult = sum.min(1.0);
    p1 /= sum;
    p2 /= sum;
    let (mut a, mut b) = (coords(space, c1), coords(space, c2));
    if let Some(h) = space.hue_index() {
        let (h1, h2) = (a[h].rem_euclid(360.0), b[h].rem_euclid(360.0));
        let d = h2 - h1;
        let h2 = match method {
            HueMethod::Shorter if d > 180.0 => h2 - 360.0,
            HueMethod::Shorter if d < -180.0 => h2 + 360.0,
            HueMethod::Longer if (0.0..=180.0).contains(&d) && d != 0.0 => h2 - 360.0,
            HueMethod::Longer if (-180.0..0.0).contains(&d) => h2 + 360.0,
            HueMethod::Increasing if d < 0.0 => h2 + 360.0,
            HueMethod::Decreasing if d > 0.0 => h2 - 360.0,
            _ => h2,
        };
        a[h] = h1;
        b[h] = h2;
    }
    // premultiplied interpolation (hue is not premultiplied)
    let (a1, a2) = (c1.a as f64, c2.a as f64);
    let alpha = a1 * p1 + a2 * p2;
    let mut out = [0.0; 3];
    for k in 0..3 {
        out[k] = if Some(k) == space.hue_index() {
            a[k] * p1 + b[k] * p2
        } else if alpha > 0.0 {
            (a[k] * a1 * p1 + b[k] * a2 * p2) / alpha
        } else {
            a[k] * p1 + b[k] * p2
        };
    }
    Some(to_rgba(space, out, alpha * alpha_mult))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::tokenize;

    fn col(s: &str) -> Rgba {
        let t = tokenize(s);
        crate::style::color(&t[0], Rgba::TRANSPARENT).unwrap_or_else(|| panic!("{s}"))
    }

    fn hex(c: Rgba) -> u32 {
        let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
        (q(c.r) << 16) | (q(c.g) << 8) | q(c.b)
    }

    #[test]
    fn modern_functions() {
        assert_eq!(hex(col("rgb(255 0 0 / 50%)")), 0xff0000);
        assert_eq!(col("rgb(255 0 0 / 50%)").a, 0.5);
        assert_eq!(hex(col("rgb(100% 50% 0%)")), 0xff8000);
        assert_eq!(hex(col("hsl(120deg 100% 50%)")), 0x00ff00);
        assert_eq!(hex(col("hsl(0.5turn 100 50)")), 0x00ffff);
        assert_eq!(hex(col("hwb(0 0% 0%)")), 0xff0000);
        assert_eq!(hex(col("lab(100 0 0)")), 0xffffff);
        assert_eq!(hex(col("lch(0% 0 0)")), 0x000000);
        assert_eq!(hex(col("oklab(1 0 0)")), 0xffffff);
        assert_eq!(hex(col("oklch(62.8% 0.2577 29.23)")), 0xff0000);
        assert_eq!(hex(col("color(srgb 1 0.5 0)")), 0xff8000);
        assert_eq!(hex(col("color(srgb-linear 1 1 1)")), 0xffffff);
        assert_eq!(hex(col("color(xyz-d65 0.9505 1 1.089)")), 0xffffff);
        assert_eq!(hex(col("rgb(none 0 0)")), 0x000000);
    }

    #[test]
    fn out_of_gamut_is_mapped() {
        // display-p3 pure red is outside sRGB: chroma-reduced, still clearly red
        let c = col("color(display-p3 1 0 0)");
        assert!(c.r > 0.95 && c.g < 0.25 && c.b < 0.25, "{c:?}");
    }

    #[test]
    fn mixing_and_relative_colors() {
        assert_eq!(hex(col("color-mix(in srgb, red, blue)")), 0x800080);
        assert_eq!(hex(col("color-mix(in srgb, red 25%, blue)")), 0x4000bf);
        let m = col("color-mix(in srgb, red 30%, blue 30%)");
        assert!((m.a - 0.6).abs() < 1e-3);
        let ok = col("color-mix(in oklch, red, blue)");
        assert!(ok.r > 0.5 && ok.b > 0.5, "{ok:?}");
        assert_eq!(hex(col("rgb(from red r g 255)")), 0xff00ff);
        assert_eq!(hex(col("rgb(from #808080 calc(r * 2) g b)")), 0xff8080);
        assert_eq!(col("hsl(from red h s l / 0.25)").a, 0.25);
        assert_eq!(hex(col("light-dark(white, black)")), 0x000000);
    }
}
