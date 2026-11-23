#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyContext {
    Navigation,
    PhrasePitch,
    PhraseValue,
    ReferenceValue,
    FileName,
    InstrumentName,
    MidiImport,
    SongPicker,
    SpeedEntry,
    ClaimText,
}
