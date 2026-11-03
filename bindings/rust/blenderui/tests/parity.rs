//! Coverage of HTML elements and CSS properties (the numbers in the README).
//! `cargo test --test parity -- --nocapture` prints the report.

use blenderui::html::{tag_kind, TagKind, HTML_ELEMENT_COUNT, TAGS};
use blenderui::style::{implemented_properties, is_implemented, is_supported, ACCEPTED};

/// The CSS 2.1 property index minus aural (speech) and paged-media properties:
/// everything CSS 2.1 defines for screens.
const CSS21_VISUAL: &[&str] = &[
    "background-attachment", "background-color", "background-image", "background-position",
    "background-repeat", "background", "border-collapse", "border-color", "border-spacing",
    "border-style", "border-top", "border-right", "border-bottom", "border-left",
    "border-top-color", "border-right-color", "border-bottom-color", "border-left-color",
    "border-top-style", "border-right-style", "border-bottom-style", "border-left-style",
    "border-top-width", "border-right-width", "border-bottom-width", "border-left-width",
    "border-width", "border", "bottom", "caption-side", "clear", "clip", "color", "content",
    "counter-increment", "counter-reset", "cursor", "direction", "display", "empty-cells", "float",
    "font-family", "font-size", "font-style", "font-variant", "font-weight", "font", "height", "left",
    "letter-spacing", "line-height", "list-style-image", "list-style-position", "list-style-type",
    "list-style", "margin-right", "margin-left", "margin-top", "margin-bottom", "margin",
    "max-height", "max-width", "min-height", "min-width", "outline-color", "outline-style",
    "outline-width", "outline", "overflow", "padding-top", "padding-right", "padding-bottom",
    "padding-left", "padding", "position", "quotes", "right", "table-layout", "text-align",
    "text-decoration", "text-indent", "text-transform", "top", "unicode-bidi", "vertical-align",
    "visibility", "white-space", "width", "word-spacing", "z-index",
];

/// CSS Flexible Box Layout Module Level 1.
const FLEXBOX: &[&str] = &[
    "flex", "flex-basis", "flex-direction", "flex-flow", "flex-grow", "flex-shrink", "flex-wrap",
    "order", "align-content", "align-items", "align-self", "justify-content",
];

/// CSS Grid Layout Module Level 2 plus Box Alignment's gaps and place-* shorthands.
const GRID: &[&str] = &[
    "grid", "grid-area", "grid-auto-columns", "grid-auto-flow", "grid-auto-rows", "grid-column",
    "grid-column-end", "grid-column-start", "grid-row", "grid-row-end", "grid-row-start",
    "grid-template", "grid-template-areas", "grid-template-columns", "grid-template-rows", "gap",
    "row-gap", "column-gap", "justify-items", "justify-self", "place-items", "place-content",
    "place-self",
];

/// (implemented, supported): supported adds properties parsed without effect.
fn coverage(name: &str, list: &[&str]) -> (usize, usize) {
    let no_effect: Vec<&&str> = list.iter().filter(|p| !is_implemented(p)).collect();
    let unknown: Vec<&&str> = list.iter().filter(|p| !is_supported(p)).collect();
    let have = list.len() - no_effect.len();
    println!("{name}: {have}/{} rendered, {}/{} parsed; no effect: {no_effect:?}; unknown: {unknown:?}",
             list.len(), list.len() - unknown.len(), list.len());
    (have, list.len() - unknown.len())
}

#[test]
fn report() {
    let html = TAGS.iter().filter(|(t, _)| !matches!(*t, "svg" | "math")).count() + 1;
    assert_eq!(html, HTML_ELEMENT_COUNT);
    let widgets = TAGS.iter().filter(|(_, k)| *k == TagKind::Widget).count();
    println!("HTML elements: {html}/{HTML_ELEMENT_COUNT} (+ svg, math roots; {widgets} as Blender widgets)");
    assert_eq!(tag_kind("canvas"), TagKind::Replaced);
    println!("CSS properties: {} rendered, {} more parsed without effect",
             implemented_properties().count(), ACCEPTED.len());
    let (css21, css21_parsed) = coverage("CSS 2.1 (screen)", CSS21_VISUAL);
    let (flex, _) = coverage("Flexbox", FLEXBOX);
    let (grid, grid_parsed) = coverage("Grid", GRID);
    assert_eq!(flex, FLEXBOX.len());
    assert_eq!(css21_parsed, CSS21_VISUAL.len());
    assert_eq!(grid_parsed, GRID.len());
    assert!(css21 * 100 / CSS21_VISUAL.len() >= 80, "CSS 2.1 coverage fell");
    assert!(grid + 1 >= GRID.len(), "grid coverage fell");
}

#[test]
fn ua_stylesheet_parse_cost() {
    let t = std::time::Instant::now();
    let sheet = blenderui::css::Stylesheet::parse(blenderui::html::UA_CSS);
    println!("UA stylesheet: {} rules parsed in {:.3} ms", sheet.rules.len(), t.elapsed().as_secs_f64() * 1e3);
}
