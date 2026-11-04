//! Blender's widgets as components: capitalized, always drawn and handled
//! like Blender, whether or not they sit inside a `styled` subtree. Their
//! lowercase HTML counterparts (`button()`, `input()` ...) look like a
//! browser's form controls instead (and like Blender inside `styled`).
//!
//! ```ignore
//! rsx! {
//!     <NumberSlider min={0.0} max={10.0} value="2.5" onChange={|e: &ChangeEvent| println!("{:?}", e.number)} />
//!     <Button onClick={|| println!("apply")}>"Apply"</Button>
//!     <Dropdown><option>"Cube"</option><option>"Sphere"</option></Dropdown>
//! }
//! // gpui syntax: the same components are functions
//! div().child(Checkbox().checked(true)).child(Button().child("Apply"))
//! ```
//!
//! Each component is the element it stands for, so every element method is
//! a prop (`min`, `max`, `step`, `value`, `checked`, `placeholder`,
//! `disabled`, `on_change`, `on_click`, styling ...).

use crate::element::Div;
use crate::html;

/// Marks an element as a Blender widget (see the module docs).
pub const BLENDER_ATTR: &str = "bui-blender";

macro_rules! blender_component {
    ($($(#[$m:meta])* $name:ident => $make:expr;)*) => {$(
        $(#[$m])*
        pub struct $name {
            _private: (),
        }

        impl $name {
            #[doc(hidden)]
            pub fn __builder() -> Div {
                $make.attr(BLENDER_ATTR, "")
            }
        }

        $(#[$m])*
        #[allow(non_snake_case)]
        pub fn $name() -> Div {
            $name::__builder()
        }
    )*};
}

blender_component! {
    /// Blender's push button (`<Button>"Apply"</Button>`).
    Button => html::button();
    /// Blender's checkbox.
    Checkbox => html::input().input_type("checkbox");
    /// Blender's radio button (grouped by `name`).
    Radio => html::input().input_type("radio");
    /// Blender's number field: drag to scrub, click to step, double-click to type.
    NumberField => html::input().input_type("number");
    /// Blender's number slider: the field fills with the value.
    NumberSlider => html::input().input_type("range");
    /// Blender's single-line text field.
    TextField => html::input().input_type("text");
    /// Blender's multi-line text editor.
    TextArea => html::textarea();
    /// Blender's color swatch (opens Blender's color picker).
    ColorSwatch => html::input().input_type("color");
    /// Blender's dropdown menu; options are `option()` children.
    Dropdown => html::select();
    /// Blender's progress bar (`value`, `max`).
    ProgressBar => html::progress();
}

impl crate::hooks::Complete for Div {
    type Props = Div;
    fn finish(self) -> Div {
        self
    }
}
