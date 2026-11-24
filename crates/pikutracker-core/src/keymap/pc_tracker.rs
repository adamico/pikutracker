use super::Binding;
use super::key::Key;
use super::shared::{
    self, ALT, ALT_SHIFT, ARROWS, CTRL, CTRL_SHIFT, SHIFT, anywhere, common_presses, in_context,
    nudge,
};
use super::trigger::{Scope, Trigger};
use crate::app::{Action, KeyContext};

const LOWER_OCTAVE_KEYS: &str = "zsxdcvgbhnjm";
const UPPER_OCTAVE_KEYS: &str = "q2w3er5t6y7u";

pub(super) fn bindings() -> Vec<Binding> {
    let own = [
        (Key::Char('n'), CTRL_SHIFT, Action::NewUnused),
        (Key::Char('c'), CTRL, Action::Copy),
        (Key::Char('x'), CTRL, Action::Cut),
        (Key::Char('v'), CTRL, Action::Paste),
        (Key::Char('d'), CTRL, Action::Clone),
        (Key::Char('t'), CTRL, Action::ToggleCart),
        (Key::Char('l'), CTRL, Action::ToggleLog),
        (Key::Char('h'), CTRL, Action::ToggleHelp),
        (Key::Char('r'), CTRL, Action::ToggleLoopStart),
        (Key::Char('r'), CTRL_SHIFT, Action::ToggleLoopBack),
        (Key::Char('p'), CTRL, Action::ToggleStop),
        (Key::Char('t'), ALT, Action::ScaleSpeed),
        (Key::Up, CTRL, nudge(true, false)),
        (Key::Down, CTRL, nudge(false, false)),
        (Key::Up, CTRL_SHIFT, nudge(true, true)),
        (Key::Down, CTRL_SHIFT, nudge(false, true)),
    ];
    let arrows = ARROWS.into_iter().flat_map(|(key, direction)| {
        [
            (key, ALT, Action::Jump(direction)),
            (key, SHIFT, Action::Select(direction)),
            (key, ALT_SHIFT, Action::SelectJump(direction)),
        ]
    });
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
    let selecting = [(Key::Up, true), (Key::Down, false)].map(|(key, up)| Binding {
        scope: Scope::Selecting,
        trigger: Trigger::Press(key, ALT),
        action: Action::MoveRows { up },
    });
    anywhere(common_presses().into_iter().chain(own).chain(arrows))
        .chain(selecting)
        .chain(shared::piano(piano))
        .chain(shared::typed_characters())
        .chain(midi_import)
        .collect()
}
