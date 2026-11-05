//! Blender components from Rust: camera properties built from `NumberField`,
//! `NumberSlider`, `Dropdown`, `Checkbox`, `ColorSwatch`, `TextField`,
//! `Button` and `ProgressBar` (always Blender's widgets), laid out with CSS.
//!
//! ```text
//! cargo run --release --example site_components
//! ```

#[path = "site/common.rs"]
mod common;

use blenderui::prelude::*;

const CSS: &str = r#"
body { margin: 0; padding: 8px; }
.panel { border-radius: 4px; background: rgba(128, 128, 128, 0.14); margin-bottom: 4px; padding-bottom: 6px; }
.panel h3 { margin: 0; padding: 4px 10px; font-size: 13px; font-weight: normal; }
.row { display: flex; align-items: center; height: 22px; padding: 0 10px; gap: 6px; }
.row .k { flex: none; width: 38%; text-align: right; padding-right: 6px; font-size: 12px; }
.row .v { flex: 1; display: flex; gap: 2px; }
.row .v > * { flex: 1; min-width: 0; width: 0; }
.check { padding-left: 38%; }
.foot { display: flex; gap: 6px; padding: 4px 10px 0 10px; }
.foot > * { flex: 1; }
"#;

struct Camera;

fn row(label: &'static str, value: Div) -> Div {
    div().class("row").child(span().class("k").child(label)).child(div().class("v").child(value))
}

impl Render for Camera {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        rsx! {
            <html styled>
                <head><style>{CSS}</style></head>
                <body>
                    <div class="panel">
                        <h3>"Lens"</h3>
                        {row("Type", Dropdown().child(option().child("Perspective")).child(option().child("Orthographic")))}
                        {row("Focal Length", NumberField().value("50").min(1.0).max(5000.0).attr("suffix", " mm"))}
                        {row("Lens Unit", Dropdown().child(option().child("Millimeters")).child(option().child("Field of View")))}
                        <div class="row"><span class="k">"Shift"</span><div class="v">
                            <NumberField value="0.000" step="0.01" />
                            <NumberField value="0.125" step="0.01" />
                        </div></div>
                        {row("Clip Start", NumberField().value("0.1").step(0.01))}
                        {row("End", NumberField().value("1000").step(1.0))}
                    </div>
                    <div class="panel">
                        <h3>"Depth of Field"</h3>
                        <div class="row check"><Checkbox checked={true} />"Enabled"</div>
                        {row("Focus Object", TextField().value("Robot Arm"))}
                        {row("F-Stop", NumberSlider().min(0.1).max(32.0).value("2.8").step(0.1))}
                        {row("Blades", NumberField().value("6").min(0.0).max(16.0))}
                        {row("Rotation", NumberSlider().min(-180.0).max(180.0).value("15"))}
                    </div>
                    <div class="panel">
                        <h3>"Viewport Display"</h3>
                        {row("Size", NumberSlider().min(0.0).max(10.0).value("1.0").step(0.1))}
                        {row("Passepartout", NumberSlider().min(0.0).max(1.0).value("0.85").step(0.05))}
                        {row("Guide Color", ColorSwatch().value("#e6a23c"))}
                        <div class="row check"><Checkbox checked={true} />"Show Limits"</div>
                        <div class="row check"><Checkbox />"Show Mist"</div>
                    </div>
                    <div class="foot">
                        <ProgressBar max="100" value="40" />
                    </div>
                    <div class="foot">
                        <Button>"Set Active Camera"</Button>
                        <Button>"Frame Selected"</Button>
                    </div>
                </body>
            </html>
        }
    }
}

fn main() {
    common::run(400.0, 520.0, "Camera", || Camera);
}
