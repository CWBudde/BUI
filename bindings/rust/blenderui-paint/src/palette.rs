//! Theme colors a view paints with, derived the way Blender derives its
//! editor colors (`TH_BACK` shaded for headers and lines).

use crate::geom::Rgba;

/// Theme colors a view uses.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Palette {
    pub back: Rgba,
    pub header: Rgba,
    pub line: Rgba,
    pub alt: Rgba,
    pub text: Rgba,
    pub dim: Rgba,
    pub sel: Rgba,
    pub hover: Rgba,
    pub accent: Rgba,
}

impl Palette {
    /// Blender's default dark theme (`region_bg`, `text`, `file_row_alternate`, `list_item`).
    #[must_use]
    pub fn blender_dark() -> Self {
        let c = |r: u8, g: u8, b: u8, a: f32| Rgba::new(f32::from(r) / 255.0, f32::from(g) / 255.0, f32::from(b) / 255.0, a);
        Self::new(c(29, 29, 29, 1.0), c(230, 230, 230, 1.0), c(255, 255, 255, 0.03), c(77, 112, 179, 1.0), c(77, 140, 217, 1.0))
    }

    /// Blender's default light theme.
    #[must_use]
    pub fn blender_light() -> Self {
        let c = |r: u8, g: u8, b: u8, a: f32| Rgba::new(f32::from(r) / 255.0, f32::from(g) / 255.0, f32::from(b) / 255.0, a);
        Self::new(c(153, 153, 153, 1.0), c(0, 0, 0, 1.0), c(0, 0, 0, 0.04), c(77, 112, 179, 1.0), c(77, 140, 217, 1.0))
    }

    /// A palette from the five colors a theme gives; the rest derives from
    /// them as Blender derives its spreadsheet colors (`TH_BACK` shaded by
    /// 11 for the header, -11 for lines).
    #[must_use]
    pub fn new(back: Rgba, text: Rgba, alt: Rgba, sel: Rgba, accent: Rgba) -> Self {
        Self {
            back,
            header: back.shade(11.0),
            line: back.shade(-11.0),
            alt,
            text,
            dim: text.opacity(0.6),
            sel: Rgba { a: sel.a.max(0.85), ..sel },
            hover: text.opacity(0.07),
            accent,
        }
    }
}
