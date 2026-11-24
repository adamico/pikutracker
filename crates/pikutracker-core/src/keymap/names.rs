use crate::app::{Action, Direction};
use crate::project::TRACKS;

const UNIT_ACTIONS: [Action; 46] = [
    Action::FocusNext,
    Action::FocusPrevious,
    Action::NewProject,
    Action::OpenProject,
    Action::Save,
    Action::SaveAs,
    Action::Import,
    Action::Export,
    Action::Quit,
    Action::NewUnused,
    Action::StartSelection,
    Action::Copy,
    Action::Cut,
    Action::Paste,
    Action::Clone,
    Action::DeepClone,
    Action::Undo,
    Action::Redo,
    Action::InsertRow,
    Action::DeleteRow,
    Action::ChangeSpeed,
    Action::ScaleSpeed,
    Action::PurgeUnused,
    Action::ToggleCart,
    Action::SuggestClaim,
    Action::ToggleLog,
    Action::ToggleHelp,
    Action::PlayFocusedPane,
    Action::PlaySongFromCursor,
    Action::ClearMutesAndSolos,
    Action::ToggleFollow,
    Action::OctaveUp,
    Action::OctaveDown,
    Action::Clear,
    Action::ToggleLoopStart,
    Action::ToggleLoopBack,
    Action::ToggleStop,
    Action::Confirm,
    Action::Cancel,
    Action::Backspace,
    Action::ToggleVoice,
    Action::ToggleSource,
    Action::SwitchMidiTable,
    Action::CycleDrumEffect,
    Action::ResetMidiRow,
    Action::ResetMidiTable,
];

const DIRECTIONS: [(Direction, &str); 4] = [
    (Direction::Up, "Up"),
    (Direction::Down, "Down"),
    (Direction::Left, "Left"),
    (Direction::Right, "Right"),
];

pub fn named_actions() -> Vec<(String, Action)> {
    let units = UNIT_ACTIONS
        .into_iter()
        .map(|action| (format!("{action:?}"), action));
    let moves = DIRECTIONS.into_iter().flat_map(|(direction, name)| {
        [
            (format!("Move{name}"), Action::Move(direction)),
            (format!("Jump{name}"), Action::Jump(direction)),
            (format!("Select{name}"), Action::Select(direction)),
            (format!("SelectJump{name}"), Action::SelectJump(direction)),
        ]
    });
    let tracks = (1..=TRACKS as u8).flat_map(|track_number| {
        [
            (format!("Mute{track_number}"), Action::Mute { track_number }),
            (format!("Solo{track_number}"), Action::Solo { track_number }),
        ]
    });
    let nudges = [(true, "Up"), (false, "Down")]
        .into_iter()
        .flat_map(|(up, name)| {
            [
                (format!("Nudge{name}"), Action::Nudge { up, coarse: false }),
                (
                    format!("Nudge{name}Coarse"),
                    Action::Nudge { up, coarse: true },
                ),
                (format!("MidiDrumNote{name}"), Action::MidiDrumNote { up }),
                (format!("MoveRows{name}"), Action::MoveRows { up }),
            ]
        });
    units.chain(moves).chain(tracks).chain(nudges).collect()
}

pub(super) fn action_named(name: &str) -> Option<Action> {
    named_actions()
        .into_iter()
        .find_map(|(named, action)| (named == name).then_some(action))
}
