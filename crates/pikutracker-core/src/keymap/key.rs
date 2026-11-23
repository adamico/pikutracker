#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Tab,
    Space,
    Enter,
    Escape,
    Backspace,
    Insert,
    Delete,
    F(u8),
    Char(char),
}

impl Key {
    pub(super) fn with_letter_lowercased(self) -> Self {
        match self {
            Key::Char(ch) => Key::Char(ch.to_ascii_lowercase()),
            key => key,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Modifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl Modifiers {
    pub const NONE: Self = Self {
        ctrl: false,
        shift: false,
        alt: false,
    };
    pub const CTRL: Self = Self {
        ctrl: true,
        ..Self::NONE
    };
    pub const SHIFT: Self = Self {
        shift: true,
        ..Self::NONE
    };
    pub const ALT: Self = Self {
        alt: true,
        ..Self::NONE
    };
    pub const CTRL_SHIFT: Self = Self {
        ctrl: true,
        shift: true,
        alt: false,
    };
    pub const ALT_SHIFT: Self = Self {
        ctrl: false,
        shift: true,
        alt: true,
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyPress {
    pub key: Key,
    pub modifiers: Modifiers,
    pub repeat: bool,
}

impl KeyPress {
    pub const fn new(key: Key, modifiers: Modifiers) -> Self {
        Self {
            key,
            modifiers,
            repeat: false,
        }
    }

    pub const fn repeated(key: Key, modifiers: Modifiers) -> Self {
        Self {
            key,
            modifiers,
            repeat: true,
        }
    }
}
