use std::borrow::Cow;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub const fn name(self) -> &'static str {
        match self {
            Direction::Up => "UP",
            Direction::Down => "DOWN",
            Direction::Left => "LEFT",
            Direction::Right => "RIGHT",
        }
    }

    pub(super) const fn column_and_row_delta(self) -> (isize, isize) {
        match self {
            Direction::Up => (0, -1),
            Direction::Down => (0, 1),
            Direction::Left => (-1, 0),
            Direction::Right => (1, 0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    Move(Direction),
    Jump(Direction),
    Select(Direction),
    SelectJump(Direction),
    FocusNext,
    FocusPrevious,
    NewProject,
    OpenProject,
    Save,
    SaveAs,
    Import,
    Export,
    Quit,
    NewUnused,
    StartSelection,
    Copy,
    Cut,
    Paste,
    Clone,
    DeepClone,
    Undo,
    Redo,
    InsertRow,
    DeleteRow,
    ChangeSpeed,
    ScaleSpeed,
    PurgeUnused,
    ToggleCart,
    SuggestClaim,
    ToggleLog,
    PlayFocusedPane,
    PlaySongFromCursor,
    Mute { track_number: u8 },
    Solo { track_number: u8 },
    ClearMutesAndSolos,
    ToggleFollow,
    PianoKey(u8),
    HexDigit(u8),
    OctaveUp,
    OctaveDown,
    Clear,
    ToggleLoopStart,
    ToggleLoopBack,
    ToggleStop,
    Nudge { up: bool, coarse: bool },
    MoveRows { up: bool },
    Confirm,
    Cancel,
    TypeChar(char),
    Backspace,
    ToggleVoice,
    ToggleSource,
    SwitchMidiTable,
    MidiDrumNote { up: bool },
    CycleDrumEffect,
    ResetMidiRow,
    ResetMidiTable,
}

impl Action {
    pub fn name(self) -> Cow<'static, str> {
        let up_or_down = |up: bool| {
            if up {
                Direction::Up.name()
            } else {
                Direction::Down.name()
            }
        };
        match self {
            Action::Move(direction) => direction.name().into(),
            Action::Jump(direction) => format!("JUMP {}", direction.name()).into(),
            Action::Select(direction) => format!("SELECT {}", direction.name()).into(),
            Action::SelectJump(direction) => format!("SELECT JUMP {}", direction.name()).into(),
            Action::Mute { track_number } => format!("MUTE TRACK {track_number}").into(),
            Action::Solo { track_number } => format!("SOLO TRACK {track_number}").into(),
            Action::Nudge { up, coarse } => {
                let coarse = if coarse { " COARSE" } else { "" };
                format!("NUDGE {}{coarse}", up_or_down(up)).into()
            }
            Action::MoveRows { up } => format!("MOVE ROWS {}", up_or_down(up)).into(),
            Action::MidiDrumNote { up } => format!("DRUM NOTE {}", up_or_down(up)).into(),
            Action::PianoKey(_) => "NOTE".into(),
            Action::HexDigit(_) => "DIGIT".into(),
            Action::TypeChar(_) => "TYPE".into(),
            Action::FocusNext => "NEXT PANE".into(),
            Action::FocusPrevious => "PREVIOUS PANE".into(),
            Action::NewProject => "NEW".into(),
            Action::OpenProject => "OPEN".into(),
            Action::Save => "SAVE".into(),
            Action::SaveAs => "SAVE AS".into(),
            Action::Import => "IMPORT".into(),
            Action::Export => "EXPORT".into(),
            Action::Quit => "QUIT".into(),
            Action::NewUnused => "NEW UNUSED".into(),
            Action::StartSelection => "SELECT".into(),
            Action::Copy => "COPY".into(),
            Action::Cut => "CUT".into(),
            Action::Paste => "PASTE".into(),
            Action::Clone => "CLONE".into(),
            Action::DeepClone => "DEEP CLONE".into(),
            Action::Undo => "UNDO".into(),
            Action::Redo => "REDO".into(),
            Action::InsertRow => "INSERT ROW".into(),
            Action::DeleteRow => "DELETE ROW".into(),
            Action::ChangeSpeed => "CHANGE SPEED".into(),
            Action::ScaleSpeed => "SCALE SPEED".into(),
            Action::PurgeUnused => "PURGE UNUSED".into(),
            Action::ToggleCart => "CART".into(),
            Action::SuggestClaim => "SUGGEST CLAIM".into(),
            Action::ToggleLog => "LOG".into(),
            Action::PlayFocusedPane => "PLAY".into(),
            Action::PlaySongFromCursor => "PLAY SONG".into(),
            Action::ClearMutesAndSolos => "CLEAR MUTES AND SOLOS".into(),
            Action::ToggleFollow => "FOLLOW".into(),
            Action::OctaveUp => "OCTAVE UP".into(),
            Action::OctaveDown => "OCTAVE DOWN".into(),
            Action::Clear => "CLEAR".into(),
            Action::ToggleLoopStart => "LOOP START".into(),
            Action::ToggleLoopBack => "LOOP BACK".into(),
            Action::ToggleStop => "STOP".into(),
            Action::Confirm => "CONFIRM".into(),
            Action::Cancel => "CANCEL".into(),
            Action::Backspace => "BACKSPACE".into(),
            Action::ToggleVoice => "TICK".into(),
            Action::ToggleSource => "TICK SOURCE".into(),
            Action::SwitchMidiTable => "TABLE".into(),
            Action::CycleDrumEffect => "EFFECT".into(),
            Action::ResetMidiRow => "RESET".into(),
            Action::ResetMidiTable => "RESET ALL".into(),
        }
    }

    pub const fn acts_on_cells(self) -> bool {
        matches!(
            self,
            Action::Clear
                | Action::Copy
                | Action::Cut
                | Action::Paste
                | Action::Clone
                | Action::DeepClone
                | Action::NewUnused
                | Action::InsertRow
                | Action::DeleteRow
                | Action::MoveRows { .. }
                | Action::StartSelection
                | Action::Select(_)
                | Action::SelectJump(_)
                | Action::PianoKey(_)
                | Action::HexDigit(_)
                | Action::Nudge { .. }
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyContext {
    Navigation,
    PhrasePitch,
    PhraseValue,
    ReferenceValue,
    FileName,
    InstrumentName,
    MidiImport,
    SpeedEntry,
    ClaimText,
}
