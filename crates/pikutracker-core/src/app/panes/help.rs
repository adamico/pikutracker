use super::{HEADING, VALUE, draw_title, midi_dialog};
use crate::app::layout::Area;
use crate::app::{Action, Direction, KeyContext, change_speed, key_hints};
use crate::keymap::Keymap;
use crate::screen::grid::Grid;
use std::borrow::Cow;

const TITLE: &str = "HELP";
const FIRST_LINE_ROW: usize = 1;
const NAME_WIDTH: usize = 14;
const MIN_KEYS_WIDTH: usize = 8;
const COLUMN_GAP: usize = 1;
const KEY_GAP: &str = "/";
const ARROWS: &str = "ARROWS";
const ALL_DIGITS: &str = "0-F";
const HEX_DIGITS: u8 = 16;
const KEYS_PER_OCTAVE: usize = 12;
const DIRECTIONS: [Direction; 4] = [
    Direction::Up,
    Direction::Down,
    Direction::Left,
    Direction::Right,
];

const NUDGE: Entry = Entry::group(
    "NUDGE",
    Keys::Of(&[
        Action::Nudge {
            up: true,
            coarse: false,
        },
        Action::Nudge {
            up: false,
            coarse: false,
        },
        Action::Nudge {
            up: true,
            coarse: true,
        },
        Action::Nudge {
            up: false,
            coarse: true,
        },
    ]),
);

const MOVE_ROWS: Entry = Entry::group(
    "MOVE ROWS",
    Keys::Of(&[
        Action::MoveRows { up: true },
        Action::MoveRows { up: false },
    ]),
);

const SELECT: Entry = Entry::group(
    "SELECT",
    Keys::Arrows(&[Action::StartSelection], Action::Select),
);

const PHRASE_CONTEXTS: &[KeyContext] = &[KeyContext::PhrasePitch, KeyContext::PhraseValue];

const PHRASE_KEYS: &[Entry] = &[
    Entry::group("MOVE", Keys::Arrows(&[], Action::Move)),
    Entry::group("JUMP", Keys::Arrows(&[], Action::Jump)),
    Entry::named(Keys::Notes),
    Entry::named(Keys::Digits),
    Entry::group("OCTAVE", Keys::Of(&[Action::OctaveDown, Action::OctaveUp])),
    NUDGE,
    Entry::named(Keys::Of(&[Action::Clear])),
    Entry::named(Keys::Of(&[Action::InsertRow])),
    Entry::named(Keys::Of(&[Action::DeleteRow])),
    SELECT,
    Entry::group("SELECT JUMP", Keys::Arrows(&[], Action::SelectJump)),
    Entry::named(Keys::Of(&[Action::Copy])),
    Entry::named(Keys::Of(&[Action::Cut])),
    Entry::named(Keys::Of(&[Action::Paste])),
    MOVE_ROWS,
    Entry::named(Keys::Of(&[Action::Undo])),
    Entry::named(Keys::Of(&[Action::Redo])),
    Entry::named(Keys::Of(&[Action::ChangeSpeed])),
    Entry::named(Keys::Of(&[Action::ScaleSpeed])),
    Entry::named(Keys::Of(&[Action::PlayFocusedPane])),
    Entry::named(Keys::Of(&[Action::Confirm])),
    CANCEL,
    Entry::named(Keys::Of(&[Action::FocusNext])),
    Entry::named(Keys::Of(&[Action::FocusPrevious])),
];

const REFERENCE_CONTEXTS: &[KeyContext] = &[KeyContext::ReferenceValue];

const CHAIN_KEYS: &[Entry] = &[
    Entry::group("MOVE", Keys::Arrows(&[], Action::Move)),
    Entry::group("JUMP", Keys::Arrows(&[], Action::Jump)),
    Entry::named(Keys::Digits),
    NUDGE,
    Entry::named(Keys::Of(&[Action::Clear])),
    Entry::named(Keys::Of(&[Action::InsertRow])),
    Entry::named(Keys::Of(&[Action::DeleteRow])),
    SELECT,
    Entry::group("SELECT JUMP", Keys::Arrows(&[], Action::SelectJump)),
    Entry::named(Keys::Of(&[Action::Copy])),
    Entry::named(Keys::Of(&[Action::Cut])),
    Entry::named(Keys::Of(&[Action::Paste])),
    Entry::named(Keys::Of(&[Action::Clone])),
    Entry::named(Keys::Of(&[Action::NewUnused])),
    MOVE_ROWS,
    Entry::named(Keys::Of(&[Action::Undo])),
    Entry::named(Keys::Of(&[Action::Redo])),
    Entry::named(Keys::Of(&[Action::ChangeSpeed])),
    Entry::named(Keys::Of(&[Action::ScaleSpeed])),
    Entry::named(Keys::Of(&[Action::PlayFocusedPane])),
    CANCEL,
    Entry::named(Keys::Of(&[Action::FocusNext])),
    Entry::named(Keys::Of(&[Action::FocusPrevious])),
];

const SONG_KEYS: &[Entry] = &[
    Entry::group("MOVE", Keys::Arrows(&[], Action::Move)),
    Entry::group(
        "JUMP",
        Keys::Of(&[Action::Jump(Direction::Up), Action::Jump(Direction::Down)]),
    ),
    Entry::named(Keys::Digits),
    NUDGE,
    Entry::named(Keys::Of(&[Action::Clear])),
    Entry::named(Keys::Of(&[Action::InsertRow])),
    Entry::named(Keys::Of(&[Action::DeleteRow])),
    SELECT,
    Entry::group(
        "SELECT JUMP",
        Keys::Of(&[
            Action::SelectJump(Direction::Up),
            Action::SelectJump(Direction::Down),
        ]),
    ),
    Entry::named(Keys::Of(&[Action::Copy])),
    Entry::named(Keys::Of(&[Action::Cut])),
    Entry::named(Keys::Of(&[Action::Paste])),
    Entry::named(Keys::Of(&[Action::Clone])),
    Entry::named(Keys::Of(&[Action::DeepClone])),
    Entry::named(Keys::Of(&[Action::NewUnused])),
    MOVE_ROWS,
    Entry::named(Keys::Of(&[Action::ToggleLoopStart])),
    Entry::named(Keys::Of(&[Action::ToggleLoopBack])),
    Entry::named(Keys::Of(&[Action::ToggleStop])),
    Entry::named(Keys::Of(&[Action::Undo])),
    Entry::named(Keys::Of(&[Action::Redo])),
    Entry::named(Keys::Of(&[Action::ChangeSpeed])),
    Entry::named(Keys::Of(&[Action::ScaleSpeed])),
    Entry::named(Keys::Of(&[Action::PlayFocusedPane])),
    CANCEL,
    Entry::named(Keys::Of(&[Action::FocusNext])),
    Entry::named(Keys::Of(&[Action::FocusPrevious])),
];

const MOVE_UP_DOWN: Entry = Entry::group(
    "MOVE",
    Keys::Of(&[Action::Move(Direction::Up), Action::Move(Direction::Down)]),
);

const NAVIGATION_CONTEXTS: &[KeyContext] = &[KeyContext::Navigation];

const CLAIM_TEXT_CONTEXTS: &[KeyContext] = &[KeyContext::ClaimText];

const CART_VIEW_KEYS: &[Entry] = &[
    MOVE_UP_DOWN,
    Entry::named(Keys::Digits),
    Entry::group("DASH", Keys::Of(&[Action::TypeChar('-')])),
    Entry::named(Keys::Of(&[Action::Backspace])),
    Entry::named(Keys::Of(&[Action::SuggestClaim])),
    Entry::named(Keys::Of(&[Action::Confirm])),
    CANCEL,
    Entry::named(Keys::Of(&[Action::Undo])),
    Entry::named(Keys::Of(&[Action::Redo])),
    Entry::named(Keys::Of(&[Action::PlayFocusedPane])),
    Entry::named(Keys::Of(&[Action::FocusNext])),
    Entry::named(Keys::Of(&[Action::FocusPrevious])),
];

const LOG_KEYS: &[Entry] = &[
    MOVE_UP_DOWN,
    Entry::named(Keys::Of(&[Action::Confirm])),
    CANCEL,
    Entry::named(Keys::Of(&[Action::Undo])),
    Entry::named(Keys::Of(&[Action::Redo])),
    Entry::named(Keys::Of(&[Action::PlayFocusedPane])),
    Entry::named(Keys::Of(&[Action::FocusNext])),
    Entry::named(Keys::Of(&[Action::FocusPrevious])),
];

const CANCEL: Entry = Entry::named(Keys::Of(&midi_dialog::CANCEL));
const BACKSPACE: Entry = Entry::named(Keys::Of(&[Action::Backspace]));
const PICK_ENTRY: Entry = Entry::group(key_hints::PICK, Keys::Of(&key_hints::UP_DOWN));
const CHOOSE_ENTRY: Entry = Entry::group(key_hints::CHOOSE, Keys::Of(&key_hints::CONFIRM));

const OPEN_LIST_KEYS: &[Entry] = &[PICK_ENTRY, CHOOSE_ENTRY, CANCEL];

const FILE_NAME_CONTEXTS: &[KeyContext] = &[KeyContext::FileName];

const SAVE_LIST_KEYS: &[Entry] = &[PICK_ENTRY, BACKSPACE, CHOOSE_ENTRY, CANCEL];

const SPEED_ENTRY_CONTEXTS: &[KeyContext] = &[KeyContext::SpeedEntry];

const SPEED_ENTRY_KEYS: &[Entry] = &[
    Entry::named(Keys::Digits),
    BACKSPACE,
    Entry::group(change_speed::APPLY, Keys::Of(&key_hints::CONFIRM)),
    CANCEL,
];

const MIDI_IMPORT_CONTEXTS: &[KeyContext] = &[KeyContext::MidiImport];

const MIDI_NEXT: Entry = Entry::group(midi_dialog::NEXT, Keys::Of(&key_hints::CONFIRM));
const MIDI_BACK: Entry = Entry::group(midi_dialog::BACK, Keys::Of(&midi_dialog::CANCEL));
const MIDI_INSTRUMENT: Entry =
    Entry::group(midi_dialog::INSTRUMENT, Keys::Of(&midi_dialog::LEFT_RIGHT));

const MIDI_VOICES_KEYS: &[Entry] = &[
    PICK_ENTRY,
    Entry::named(Keys::Of(&[Action::ToggleVoice])),
    Entry::named(Keys::Of(&[Action::ToggleSource])),
    MIDI_NEXT,
    CANCEL,
];

const MIDI_STEPS_PER_BEAT_KEYS: &[Entry] = &[
    Entry::group(key_hints::PICK, Keys::Of(&midi_dialog::ARROWS)),
    MIDI_NEXT,
    MIDI_BACK,
];

const MIDI_PROGRAMS_KEYS: &[Entry] = &[
    PICK_ENTRY,
    MIDI_INSTRUMENT,
    Entry::named(Keys::Of(&[Action::SwitchMidiTable])),
    Entry::named(Keys::Of(&[Action::ResetMidiRow])),
    Entry::named(Keys::Of(&[Action::ResetMidiTable])),
    MIDI_NEXT,
    MIDI_BACK,
];

const MIDI_DRUM_KIT_KEYS: &[Entry] = &[
    PICK_ENTRY,
    MIDI_INSTRUMENT,
    Entry::group(midi_dialog::NOTE, Keys::Of(&midi_dialog::DRUM_NOTE)),
    Entry::named(Keys::Of(&[Action::CycleDrumEffect])),
    Entry::named(Keys::Of(&[Action::SwitchMidiTable])),
    Entry::named(Keys::Of(&[Action::ResetMidiRow])),
    Entry::named(Keys::Of(&[Action::ResetMidiTable])),
    MIDI_NEXT,
    MIDI_BACK,
];

const MIDI_REVIEW_KEYS: &[Entry] = &[
    Entry::group(midi_dialog::SCROLL, Keys::Of(&key_hints::UP_DOWN)),
    Entry::group(midi_dialog::IMPORT, Keys::Of(&key_hints::CONFIRM)),
    MIDI_BACK,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HelpList {
    Phrase,
    Chain,
    Song,
    CartView,
    Log,
    OpenList,
    SaveList,
    SpeedEntry,
    MidiVoices,
    MidiStepsPerBeat,
    MidiPrograms,
    MidiDrumKit,
    MidiReview,
}

enum Keys {
    Of(&'static [Action]),
    Arrows(&'static [Action], fn(Direction) -> Action),
    Notes,
    Digits,
}

struct Entry {
    group: Option<&'static str>,
    keys: Keys,
}

impl Entry {
    const fn group(name: &'static str, keys: Keys) -> Self {
        Self {
            group: Some(name),
            keys,
        }
    }

    const fn named(keys: Keys) -> Self {
        Self { group: None, keys }
    }

    fn name(&self) -> Cow<'static, str> {
        if let Some(group) = self.group {
            return group.into();
        }
        match self.keys {
            Keys::Of(actions) | Keys::Arrows(actions, _) => {
                actions.first().map_or("".into(), |action| action.name())
            }
            Keys::Notes => Action::PianoKey(0).name(),
            Keys::Digits => Action::HexDigit(0).name(),
        }
    }
}

struct Lookup<'a> {
    keymap: &'a Keymap,
    contexts: &'static [KeyContext],
    selecting: &'static [bool],
}

impl Lookup<'_> {
    fn keys(&self, action: Action) -> Vec<String> {
        let mut keys: Vec<String> = Vec::new();
        for &context in self.contexts {
            for &selecting in self.selecting {
                let typed = self.keymap.typed_trigger(action, context);
                let bound = self.keymap.triggers_when(action, context, selecting);
                for trigger in typed.into_iter().chain(bound) {
                    let key = trigger.to_string();
                    if !keys.contains(&key) {
                        keys.push(key);
                    }
                }
            }
        }
        keys
    }

    fn first_key(&self, action: Action) -> Option<String> {
        self.keys(action).into_iter().next()
    }

    fn entry_keys(&self, keys: &Keys) -> Vec<String> {
        match *keys {
            Keys::Of(actions) => actions
                .iter()
                .flat_map(|&action| self.keys(action))
                .collect(),
            Keys::Arrows(first, each) => first
                .iter()
                .flat_map(|&action| self.keys(action))
                .chain(self.arrow_keys(each))
                .collect(),
            Keys::Notes => self.note_keys(),
            Keys::Digits => self.digit_keys(),
        }
    }

    fn arrow_keys(&self, each: fn(Direction) -> Action) -> Vec<String> {
        let by_direction: Vec<Vec<String>> = DIRECTIONS
            .iter()
            .map(|&direction| self.keys(each(direction)))
            .collect();
        let count = by_direction[0].len();
        let shared: Option<Vec<String>> = by_direction
            .iter()
            .all(|keys| keys.len() == count)
            .then(|| {
                (0..count)
                    .map(|index| shared_prefix(&by_direction, index))
                    .collect()
            })
            .flatten();
        shared.unwrap_or_else(|| by_direction.into_iter().flatten().collect())
    }

    fn note_keys(&self) -> Vec<String> {
        let keys: Vec<String> = (0..=u8::MAX)
            .map_while(|semitone| self.first_key(Action::PianoKey(semitone)))
            .collect();
        keys.chunks(KEYS_PER_OCTAVE)
            .map(|octave| octave.concat())
            .collect()
    }

    fn digit_keys(&self) -> Vec<String> {
        let keys: Vec<String> = (0..HEX_DIGITS)
            .filter_map(|digit| self.first_key(Action::HexDigit(digit)))
            .collect();
        let in_order = (0..HEX_DIGITS)
            .map(|digit| format!("{digit:X}"))
            .eq(keys.iter().cloned());
        if in_order {
            vec![ALL_DIGITS.to_owned()]
        } else if keys.is_empty() {
            Vec::new()
        } else {
            vec![keys.concat()]
        }
    }
}

fn shared_prefix(by_direction: &[Vec<String>], index: usize) -> Option<String> {
    let mut prefixes = DIRECTIONS
        .iter()
        .zip(by_direction)
        .map(|(direction, keys)| keys[index].strip_suffix(direction.name()));
    let first = prefixes.next()??;
    prefixes
        .all(|prefix| prefix == Some(first))
        .then(|| format!("{first}{ARROWS}"))
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Line {
    name: Option<String>,
    keys: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Placed {
    column: usize,
    row: usize,
    line: Line,
}

pub fn draw(grid: &mut Grid, area: Area, keymap: &Keymap, list: HelpList) {
    let blank = " ".repeat(area.width);
    for row in area.row..area.row + area.rows {
        grid.put(area.column, row, &blank, VALUE);
    }
    draw_title(grid, area.column, area.row, TITLE, false);
    let entries = entries_with_keys(keymap, list);
    let body = Area {
        column: area.column,
        row: area.row + FIRST_LINE_ROW,
        width: area.width,
        rows: area.rows - FIRST_LINE_ROW,
    };
    for placed in laid_out(&entries, body) {
        if let Some(name) = &placed.line.name {
            grid.put(placed.column, placed.row, name, HEADING);
        }
        grid.put(
            placed.column + NAME_WIDTH,
            placed.row,
            &placed.line.keys,
            VALUE,
        );
    }
}

fn entries_with_keys(keymap: &Keymap, list: HelpList) -> Vec<(Cow<'static, str>, Vec<String>)> {
    let (entries, contexts, selecting): (&[Entry], &'static [KeyContext], &'static [bool]) =
        match list {
            HelpList::Phrase => (PHRASE_KEYS, PHRASE_CONTEXTS, &[false, true]),
            HelpList::Chain => (CHAIN_KEYS, REFERENCE_CONTEXTS, &[false, true]),
            HelpList::Song => (SONG_KEYS, REFERENCE_CONTEXTS, &[false, true]),
            HelpList::CartView => (CART_VIEW_KEYS, CLAIM_TEXT_CONTEXTS, &[false]),
            HelpList::Log => (LOG_KEYS, NAVIGATION_CONTEXTS, &[false]),
            HelpList::OpenList => (OPEN_LIST_KEYS, NAVIGATION_CONTEXTS, &[false]),
            HelpList::SaveList => (SAVE_LIST_KEYS, FILE_NAME_CONTEXTS, &[false]),
            HelpList::SpeedEntry => (SPEED_ENTRY_KEYS, SPEED_ENTRY_CONTEXTS, &[false]),
            HelpList::MidiVoices => (MIDI_VOICES_KEYS, MIDI_IMPORT_CONTEXTS, &[false]),
            HelpList::MidiStepsPerBeat => {
                (MIDI_STEPS_PER_BEAT_KEYS, MIDI_IMPORT_CONTEXTS, &[false])
            }
            HelpList::MidiPrograms => (MIDI_PROGRAMS_KEYS, MIDI_IMPORT_CONTEXTS, &[false]),
            HelpList::MidiDrumKit => (MIDI_DRUM_KIT_KEYS, MIDI_IMPORT_CONTEXTS, &[false]),
            HelpList::MidiReview => (MIDI_REVIEW_KEYS, MIDI_IMPORT_CONTEXTS, &[false]),
        };
    let lookup = Lookup {
        keymap,
        contexts,
        selecting,
    };
    entries
        .iter()
        .map(|entry| (entry.name(), lookup.entry_keys(&entry.keys)))
        .filter(|(_, keys)| !keys.is_empty())
        .collect()
}

fn laid_out(entries: &[(Cow<'static, str>, Vec<String>)], area: Area) -> Vec<Placed> {
    let mut columns = 1;
    loop {
        let width = (area.width + COLUMN_GAP) / columns - COLUMN_GAP;
        let keys_width = width.saturating_sub(NAME_WIDTH);
        let lines: Vec<Line> = entries
            .iter()
            .flat_map(|(name, keys)| wrapped(name, keys, keys_width))
            .collect();
        let narrower_fits =
            (area.width + COLUMN_GAP) / (columns + 1) - COLUMN_GAP >= NAME_WIDTH + MIN_KEYS_WIDTH;
        if lines.len() <= area.rows * columns || !narrower_fits {
            return lines
                .into_iter()
                .enumerate()
                .map(|(index, line)| Placed {
                    column: area.column + index / area.rows * (width + COLUMN_GAP),
                    row: area.row + index % area.rows,
                    line,
                })
                .collect();
        }
        columns += 1;
    }
}

fn wrapped(name: &str, keys: &[String], width: usize) -> Vec<Line> {
    let width = width.max(1);
    let mut rows: Vec<String> = Vec::new();
    let mut row = String::new();
    for key in keys {
        let mut key: Vec<char> = key.chars().collect();
        while key.len() > width {
            if !row.is_empty() {
                rows.push(std::mem::take(&mut row));
            }
            rows.push(key.drain(..width).collect());
        }
        let needed = row.chars().count() + KEY_GAP.len() + key.len();
        if !row.is_empty() && needed > width {
            rows.push(std::mem::take(&mut row));
        }
        if !row.is_empty() {
            row.push_str(KEY_GAP);
        }
        row.extend(key);
    }
    if !row.is_empty() {
        rows.push(row);
    }
    rows.into_iter()
        .enumerate()
        .map(|(index, keys)| Line {
            name: (index == 0).then(|| name.to_owned()),
            keys,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENTRIES: usize = 60;
    const ROWS: usize = 40;
    const AREA: Area = Area {
        column: 51,
        row: 1,
        width: 55,
        rows: ROWS,
    };

    fn many_entries() -> Vec<(Cow<'static, str>, Vec<String>)> {
        (0..ENTRIES)
            .map(|index| (format!("ACTION {index}").into(), vec![format!("F{index}")]))
            .collect()
    }

    #[test]
    fn a_list_taller_than_the_pane_rows_continues_in_a_second_column() {
        let placed = laid_out(&many_entries(), AREA);

        assert_eq!(placed.len(), ENTRIES);
        let columns: Vec<usize> = placed.iter().map(|placed| placed.column).collect();
        assert_eq!(columns[..ROWS], [AREA.column; ROWS]);
        assert!(columns[ROWS..].iter().all(|&column| column == 79));
        assert_eq!(placed[ROWS].row, AREA.row);
        assert_eq!(
            placed[ENTRIES - 1].line,
            Line {
                name: Some("ACTION 59".to_owned()),
                keys: "F59".to_owned()
            }
        );
    }

    #[test]
    fn a_list_that_fits_stays_in_one_column() {
        let placed = laid_out(&many_entries()[..ROWS], AREA);

        assert!(placed.iter().all(|placed| placed.column == AREA.column));
    }
}
