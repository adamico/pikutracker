use super::Binding;
use super::key::Key;
use super::shared::{
    self, ALT, CTRL, CTRL_SHIFT, SHIFT, anywhere, common_presses, in_context, nudge,
};
use crate::app::{Action, Direction, KeyContext};

const LOWER_OCTAVE_KEYS: &str = "zsxdcvgbhnjm";
const UPPER_OCTAVE_KEYS: &str = "q2w3er5t6y7u";

pub(super) fn bindings() -> Vec<Binding> {
    let own = [
        (Key::Char('n'), CTRL_SHIFT, Action::NewUnused),
        (Key::Char('b'), CTRL, Action::StartSelection),
        (Key::Char('c'), CTRL, Action::Copy),
        (Key::Char('x'), CTRL, Action::Cut),
        (Key::Char('v'), CTRL, Action::Paste),
        (Key::Char('d'), CTRL, Action::Clone),
        (Key::Char('t'), CTRL, Action::ToggleCart),
        (Key::Char('l'), CTRL, Action::ToggleLog),
        (Key::Char('f'), CTRL, Action::ToggleFill),
        (Key::Char('r'), CTRL, Action::ToggleSongLoopRow),
        (Key::Char('t'), ALT, Action::ScaleSpeed),
        (Key::Up, SHIFT, nudge(true, false)),
        (Key::Down, SHIFT, nudge(false, false)),
        (Key::Up, CTRL, nudge(true, true)),
        (Key::Down, CTRL, nudge(false, true)),
        (Key::Up, ALT, Action::Jump(Direction::Up)),
        (Key::Down, ALT, Action::Jump(Direction::Down)),
        (Key::Left, ALT, Action::Jump(Direction::Left)),
        (Key::Right, ALT, Action::Jump(Direction::Right)),
    ];
    let piano = LOWER_OCTAVE_KEYS.chars().chain(UPPER_OCTAVE_KEYS.chars());
    let midi_import = [
        (Key::Space, Action::ToggleVoice),
        (Key::Char('s'), Action::ToggleSource),
        (Key::Tab, Action::SwitchMidiTable),
        (Key::Char('['), Action::MidiDrumNote { up: false }),
        (Key::Char(']'), Action::MidiDrumNote { up: true }),
        (Key::Char('f'), Action::CycleDrumEffect),
        (Key::Char('r'), Action::ResetMidiRow),
    ]
    .map(|(key, action)| in_context(KeyContext::MidiImport, key, action));
    anywhere(common_presses().into_iter().chain(own))
        .chain(shared::piano(piano))
        .chain(shared::typed_characters())
        .chain(midi_import)
        .collect()
}
