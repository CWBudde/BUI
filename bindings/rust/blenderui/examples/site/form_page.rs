//! One HTML page for site_plain_html (a browser's look) and site_styled
//! (the same markup inside `styled`: Blender's theme and widgets).

#![allow(dead_code)]

use blenderui::prelude::*;

const CSS: &str = r#"
body { margin: 0; padding: 16px 20px; }
h2 { margin: 0 0 2px 0; }
p.lead { margin: 0 0 12px 0; opacity: 0.7; }
form { display: grid; grid-template-columns: 120px 1fr; gap: 8px 12px; align-items: center; }
label.k { text-align: right; }
.inline { display: flex; gap: 14px; align-items: center; }
.actions { grid-column: 2; display: flex; gap: 8px; margin-top: 6px; }
fieldset { margin-top: 14px; }
table { width: 100%; border-collapse: collapse; }
th, td { text-align: left; padding: 3px 6px; }
"#;

pub fn page(styled: bool) -> Div {
    let root = html().child(head().child(style().child(CSS)));
    let root = if styled { root.styled() } else { root };
    root.child(body()
        .child(h2().child("Export Animation"))
        .child(p().class("lead").child("Encode the rendered frames of ").child(code().child("robot_arm.blend")))
        .child(form()
            .child(label().class("k").child("File name"))
            .child(input().value("robot_arm_final"))
            .child(label().class("k").child("Container"))
            .child(select().child(option().child("MPEG-4 (.mp4)")).child(option().child("Matroska (.mkv)"))
                .child(option().child("QuickTime (.mov)")))
            .child(label().class("k").child("Frame rate"))
            .child(input().input_type("number").value("24").min(1.0).max(120.0))
            .child(label().class("k").child("Quality"))
            .child(input().input_type("range").min(0.0).max(100.0).value("72"))
            .child(label().class("k").child("Audio"))
            .child(div().class("inline")
                .child(label().child(input().input_type("radio").name("audio").checked(true)).child(" AAC"))
                .child(label().child(input().input_type("radio").name("audio")).child(" FLAC"))
                .child(label().child(input().input_type("radio").name("audio")).child(" None")))
            .child(label().class("k").child("Tint"))
            .child(div().class("inline")
                .child(input().input_type("color").value("#e6a23c"))
                .child(label().child(input().input_type("checkbox").checked(true)).child(" Burn in frame numbers")))
            .child(label().class("k").child("Progress"))
            .child(progress().value("0.64"))
            .child(div().class("actions")
                .child(button().child("Export"))
                .child(button().child("Cancel"))))
        .child(fieldset()
            .child(legend().child("Recent exports"))
            .child(table()
                .child(thead().child(tr().child(th().child("File")).child(th().child("Size")).child(th().child("Date"))))
                .child(tbody().children([("robot_arm_v12.mp4", "184 MB", "Today 10:42"),
                                         ("forest_flyby.mkv", "1.2 GB", "Yesterday")]
                    .map(|(a, b, c)| tr().child(td().child(a)).child(td().child(b)).child(td().child(c))))))))
}
