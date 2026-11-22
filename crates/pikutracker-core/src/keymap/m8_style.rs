use super::Binding;
use super::key::Key;
use super::shared::{self, NONE, SHIFT, anywhere, common_presses, in_context, nudge};
use super::trigger::{Scope, Trigger};
use crate::app::{Action, Direction, KeyContext};

const EDIT: Key = Key::Char('z');
const OPTION: Key = Key::Char('x');

const PIANO_KEYS: &str = "q2w3er5t6y7ui9o0p";

pub(super) fn bindings() -> Vec<Binding> {
    let own = [
        (Key::Right, SHIFT, Action::FocusNext),
        (Key::Down, SHIFT, Action::FocusNext),
        (Key::Left, SHIFT, Action::FocusPrevious),
        (Key::Up, SHIFT, Action::FocusPrevious),
        (OPTION, SHIFT, Action::StartSelection),
        (EDIT, SHIFT, Action::Paste),
    ];
    let chords = [
        (EDIT, OPTION, Action::Cut),
        (OPTION, Key::Char('t'), Action::ToggleCart),
        (OPTION, Key::Char('l'), Action::ToggleLog),
        (OPTION, Key::Char('f'), Action::ToggleFill),
        (OPTION, Key::Char('r'), Action::ToggleSongLoopRow),
        (EDIT, Key::Char('t'), Action::ScaleSpeed),
        (EDIT, Key::Up, nudge(true, true)),
        (EDIT, Key::Down, nudge(false, true)),
        (EDIT, Key::Right, nudge(true, false)),
        (EDIT, Key::Left, nudge(false, false)),
        (OPTION, Key::Up, Action::Jump(Direction::Up)),
        (OPTION, Key::Down, Action::Jump(Direction::Down)),
        (OPTION, Key::Left, Action::Jump(Direction::Left)),
        (OPTION, Key::Right, Action::Jump(Direction::Right)),
    ]
    .map(|(held, key, action)| chord(Scope::Anywhere, held, key, action));
    let double_tap = Binding {
        scope: Scope::Anywhere,
        trigger: Trigger::DoubleTap(EDIT),
        action: Action::NewUnused,
    };
    let selecting = [(OPTION, Action::Copy), (EDIT, Action::Clone)].map(|(key, action)| Binding {
        scope: Scope::Selecting,
        trigger: Trigger::Press(key, NONE),
        action,
    });
    let in_midi_import = Scope::Context(KeyContext::MidiImport);
    let midi_import = [
        in_context(KeyContext::MidiImport, Key::Space, Action::ToggleVoice),
        in_context(KeyContext::MidiImport, Key::Char('s'), Action::ToggleSource),
        in_context(KeyContext::MidiImport, Key::Tab, Action::SwitchMidiTable),
        in_context(
            KeyContext::MidiImport,
            Key::Char('f'),
            Action::CycleDrumEffect,
        ),
        chord(
            in_midi_import,
            EDIT,
            Key::Up,
            Action::MidiDrumNote { up: true },
        ),
        chord(
            in_midi_import,
            EDIT,
            Key::Down,
            Action::MidiDrumNote { up: false },
        ),
        chord(in_midi_import, EDIT, OPTION, Action::ResetMidiRow),
    ];
    anywhere(common_presses().into_iter().chain(own))
        .chain(chords)
        .chain([double_tap])
        .chain(selecting)
        .chain(shared::piano(PIANO_KEYS.chars()))
        .chain(shared::typed_characters())
        .chain(midi_import)
        .collect()
}

const fn chord(scope: Scope, held: Key, key: Key, action: Action) -> Binding {
    Binding {
        scope,
        trigger: Trigger::Chord { held, key },
        action,
    }
}
