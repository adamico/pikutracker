use super::Binding;
use super::key::{Key, Modifiers};
use super::trigger::{Scope, Trigger};
use crate::app::{Action, Direction, KeyContext};
use crate::project::TRACKS;

pub(super) const NONE: Modifiers = Modifiers::NONE;
pub(super) const CTRL: Modifiers = Modifiers::CTRL;
pub(super) const SHIFT: Modifiers = Modifiers::SHIFT;
pub(super) const CTRL_SHIFT: Modifiers = Modifiers::CTRL_SHIFT;
pub(super) const ALT: Modifiers = Modifiers::ALT;
pub(super) const ALT_SHIFT: Modifiers = Modifiers::ALT_SHIFT;

pub(super) const ARROWS: [(Key, Direction); 4] = [
    (Key::Up, Direction::Up),
    (Key::Down, Direction::Down),
    (Key::Left, Direction::Left),
    (Key::Right, Direction::Right),
];

const HEX_DIGITS: &str = "0123456789abcdef";

pub(super) const fn nudge(up: bool, coarse: bool) -> Action {
    Action::Nudge { up, coarse }
}

pub(super) fn anywhere(
    presses: impl IntoIterator<Item = (Key, Modifiers, Action)>,
) -> impl Iterator<Item = Binding> {
    presses.into_iter().map(|(key, modifiers, action)| Binding {
        scope: Scope::Anywhere,
        trigger: Trigger::Press(key, modifiers),
        action,
    })
}

pub(super) const fn in_context(context: KeyContext, key: Key, action: Action) -> Binding {
    Binding {
        scope: Scope::Context(context),
        trigger: Trigger::Press(key, NONE),
        action,
    }
}

pub(super) fn common_presses() -> Vec<(Key, Modifiers, Action)> {
    let mut presses = vec![
        (Key::Up, NONE, Action::Move(Direction::Up)),
        (Key::Down, NONE, Action::Move(Direction::Down)),
        (Key::Left, NONE, Action::Move(Direction::Left)),
        (Key::Right, NONE, Action::Move(Direction::Right)),
        (Key::Tab, NONE, Action::FocusNext),
        (Key::Tab, SHIFT, Action::FocusPrevious),
        (Key::Enter, NONE, Action::Confirm),
        (Key::Escape, NONE, Action::Cancel),
        (Key::Char('n'), CTRL, Action::NewProject),
        (Key::Char('o'), CTRL, Action::OpenProject),
        (Key::Char('s'), CTRL, Action::Save),
        (Key::Char('s'), CTRL_SHIFT, Action::SaveAs),
        (Key::Char('i'), CTRL, Action::Import),
        (Key::Char('e'), CTRL, Action::Export),
        (Key::Char('q'), CTRL, Action::Quit),
        (Key::Char('d'), CTRL_SHIFT, Action::DeepClone),
        (Key::Char('z'), CTRL, Action::Undo),
        (Key::Char('y'), CTRL, Action::Redo),
        (Key::Insert, NONE, Action::InsertRow),
        (Key::Delete, CTRL, Action::DeleteRow),
        (Key::Char('t'), CTRL_SHIFT, Action::ChangeSpeed),
        (Key::Char('p'), CTRL_SHIFT, Action::PurgeUnused),
        (Key::Space, NONE, Action::PlayFocusedPane),
        (Key::Space, SHIFT, Action::PlaySongFromCursor),
        (Key::F(9), NONE, Action::ClearMutesAndSolos),
        (Key::F(10), NONE, Action::ToggleFollow),
        (Key::Delete, NONE, Action::Clear),
        (Key::Char('['), NONE, Action::OctaveDown),
        (Key::Char(']'), NONE, Action::OctaveUp),
        (Key::Char('r'), SHIFT, Action::ResetMidiTable),
    ];
    for track_number in 1..=TRACKS as u8 {
        let key = Key::F(track_number);
        presses.push((key, NONE, Action::Mute { track_number }));
        presses.push((key, SHIFT, Action::Solo { track_number }));
    }
    presses
}

pub(super) fn piano(keys: impl Iterator<Item = char>) -> impl Iterator<Item = Binding> {
    (0..).zip(keys).map(|(semitone, ch)| {
        in_context(
            KeyContext::PhrasePitch,
            Key::Char(ch),
            Action::PianoKey(semitone),
        )
    })
}

pub(super) fn typed_characters() -> Vec<Binding> {
    let digits = [
        KeyContext::PhraseValue,
        KeyContext::ReferenceValue,
        KeyContext::SpeedEntry,
        KeyContext::ClaimText,
    ]
    .into_iter()
    .flat_map(|context| {
        (0..)
            .zip(HEX_DIGITS.chars())
            .map(move |(digit, ch)| in_context(context, Key::Char(ch), Action::HexDigit(digit)))
    });
    let others = [
        (KeyContext::SpeedEntry, Key::Backspace, Action::Backspace),
        (KeyContext::ClaimText, Key::Backspace, Action::Backspace),
        (KeyContext::ClaimText, Key::Char('-'), Action::TypeChar('-')),
        (KeyContext::ClaimText, Key::Char('='), Action::SuggestClaim),
        (
            KeyContext::SongPicker,
            Key::Char('p'),
            Action::PlayFocusedPane,
        ),
    ]
    .map(|(context, key, action)| in_context(context, key, action));
    digits.chain(others).collect()
}
