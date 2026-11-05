//! Input the table understands, in framework-neutral form. Adapters
//! translate their events into these; the logic that reacts to them is in
//! [`crate::interact`].

/// Modifier keys held during an event.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Mods {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
}

/// A pointer button.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Button {
    Left,
    Right,
    Middle,
}

/// The keys a table reacts to.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Home,
    End,
    Escape,
    /// `a` (with control: select all)
    A,
    /// `c` (with control: copy)
    C,
    Other,
}

impl Key {
    /// A key from its name as the frameworks spell it: Blender-style
    /// (`Down`, `Prior`, `Next`) and gpui-style (`down`, `pageup`,
    /// `pagedown`), any case.
    #[must_use]
    pub fn parse(name: &str) -> Self {
        const NAMES: [(&str, Key); 17] = [
            ("up", Key::Up),
            ("arrowup", Key::Up),
            ("down", Key::Down),
            ("arrowdown", Key::Down),
            ("left", Key::Left),
            ("arrowleft", Key::Left),
            ("right", Key::Right),
            ("arrowright", Key::Right),
            ("pageup", Key::PageUp),
            ("prior", Key::PageUp),
            ("pagedown", Key::PageDown),
            ("next", Key::PageDown),
            ("home", Key::Home),
            ("end", Key::End),
            ("escape", Key::Escape),
            ("cancel", Key::Escape),
            ("esc", Key::Escape),
        ];
        if let Some(&(_, k)) = NAMES.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)) {
            return k;
        }
        if name.eq_ignore_ascii_case("a") {
            Self::A
        } else if name.eq_ignore_ascii_case("c") {
            Self::C
        } else {
            Self::Other
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_spellings_parse() {
        assert_eq!(Key::parse("Down"), Key::Down);
        assert_eq!(Key::parse("down"), Key::Down);
        assert_eq!(Key::parse("Next"), Key::PageDown);
        assert_eq!(Key::parse("pagedown"), Key::PageDown);
        assert_eq!(Key::parse("Prior"), Key::PageUp);
        assert_eq!(Key::parse("A"), Key::A);
        assert_eq!(Key::parse("c"), Key::C);
        assert_eq!(Key::parse("Cancel"), Key::Escape);
        assert_eq!(Key::parse("F13"), Key::Other);
        assert_eq!(Key::parse(""), Key::Other);
    }
}
