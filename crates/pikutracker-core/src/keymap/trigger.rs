use super::key::{Key, Modifiers};
use crate::app::KeyContext;
use std::fmt;

const DOUBLE_TAP_PREFIX: &str = "Double-tap ";
const DOUBLE_TAP_NAME: &str = "DOUBLE-TAP ";
const MAX_FUNCTION_KEY: u8 = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Trigger {
    Press(Key, Modifiers),
    Chord { held: Key, key: Key },
    DoubleTap(Key),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scope {
    Anywhere,
    Selecting,
    Context(KeyContext),
}

impl Scope {
    pub(super) const fn width(self) -> u8 {
        match self {
            Scope::Context(_) => 0,
            Scope::Selecting => 1,
            Scope::Anywhere => 2,
        }
    }
}

impl Trigger {
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        if let Some(rest) = text
            .get(..DOUBLE_TAP_PREFIX.len())
            .filter(|prefix| prefix.eq_ignore_ascii_case(DOUBLE_TAP_PREFIX))
            .map(|_| &text[DOUBLE_TAP_PREFIX.len()..])
        {
            return key_named(rest).map(Trigger::DoubleTap);
        }
        let mut modifiers = Modifiers::NONE;
        let mut keys = Vec::new();
        for part in text.split('+') {
            match part.trim().to_ascii_lowercase().as_str() {
                "ctrl" => modifiers.ctrl = true,
                "shift" => modifiers.shift = true,
                "alt" => modifiers.alt = true,
                _ => keys.push(key_named(part)?),
            }
        }
        match keys[..] {
            [key] => Some(Trigger::Press(key, modifiers)),
            [held, key] if modifiers == Modifiers::NONE && held != key => {
                Some(Trigger::Chord { held, key })
            }
            _ => None,
        }
    }
}

fn key_named(name: &str) -> Option<Key> {
    let name = name.trim();
    let mut chars = name.chars();
    if let (Some(ch), None) = (chars.next(), chars.next()) {
        return Some(Key::Char(ch).with_letter_lowercased());
    }
    let lower = name.to_ascii_lowercase();
    Some(match lower.as_str() {
        "up" => Key::Up,
        "down" => Key::Down,
        "left" => Key::Left,
        "right" => Key::Right,
        "tab" => Key::Tab,
        "space" => Key::Space,
        "enter" => Key::Enter,
        "esc" | "escape" => Key::Escape,
        "backspace" => Key::Backspace,
        "ins" | "insert" => Key::Insert,
        "del" | "delete" => Key::Delete,
        _ => {
            let number: u8 = lower.strip_prefix('f')?.parse().ok()?;
            (1..=MAX_FUNCTION_KEY)
                .contains(&number)
                .then_some(Key::F(number))?
        }
    })
}

impl Trigger {
    pub(super) const fn rank(self) -> u8 {
        match self {
            Trigger::Press(_, Modifiers::NONE) => 0,
            Trigger::Press(..) => 1,
            Trigger::Chord { .. } => 2,
            Trigger::DoubleTap(_) => 3,
        }
    }
}

impl fmt::Display for Trigger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Trigger::Press(key, modifiers) => {
                let ctrl = if modifiers.ctrl { "^" } else { "" };
                let alt = if modifiers.alt { "ALT+" } else { "" };
                let shift = if modifiers.shift { "SHIFT+" } else { "" };
                write!(f, "{ctrl}{alt}{shift}{}", KeyName(key))
            }
            Trigger::Chord { held, key } => write!(f, "{}+{}", KeyName(held), KeyName(key)),
            Trigger::DoubleTap(key) => write!(f, "{DOUBLE_TAP_NAME}{}", KeyName(key)),
        }
    }
}

struct KeyName(Key);

impl fmt::Display for KeyName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Key::Up => f.write_str("UP"),
            Key::Down => f.write_str("DOWN"),
            Key::Left => f.write_str("LEFT"),
            Key::Right => f.write_str("RIGHT"),
            Key::Tab => f.write_str("TAB"),
            Key::Space => f.write_str("SPACE"),
            Key::Enter => f.write_str("ENTER"),
            Key::Escape => f.write_str("ESC"),
            Key::Backspace => f.write_str("BACKSPACE"),
            Key::Insert => f.write_str("INS"),
            Key::Delete => f.write_str("DEL"),
            Key::F(number) => write!(f, "F{number}"),
            Key::Char(ch) => write!(f, "{}", ch.to_ascii_uppercase()),
        }
    }
}
