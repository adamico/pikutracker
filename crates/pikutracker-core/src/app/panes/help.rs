use super::{HEADING, VALUE, draw_title};
use crate::app::focus::Focus;
use crate::app::layout::Area;
use crate::app::{Action, Direction, KeyContext};
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
    Entry::named(Keys::Of(&[Action::Cancel])),
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
    Entry::named(Keys::Of(&[Action::Cancel])),
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
    Entry::named(Keys::Of(&[Action::Cancel])),
    Entry::named(Keys::Of(&[Action::FocusNext])),
    Entry::named(Keys::Of(&[Action::FocusPrevious])),
];

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
}

impl Lookup<'_> {
    fn keys(&self, action: Action) -> Vec<String> {
        let mut keys: Vec<String> = Vec::new();
        for &context in self.contexts {
            for selecting in [false, true] {
                for trigger in self.keymap.triggers_when(action, context, selecting) {
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

pub fn draw(grid: &mut Grid, area: Area, keymap: &Keymap, focus: Focus) {
    draw_title(grid, area.column, area.row, TITLE, false);
    let entries = entries_with_keys(keymap, focus);
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

fn entries_with_keys(keymap: &Keymap, focus: Focus) -> Vec<(Cow<'static, str>, Vec<String>)> {
    let (entries, contexts): (&[Entry], &'static [KeyContext]) = match focus {
        Focus::Phrase => (PHRASE_KEYS, PHRASE_CONTEXTS),
        Focus::Chain => (CHAIN_KEYS, REFERENCE_CONTEXTS),
        Focus::Song => (SONG_KEYS, REFERENCE_CONTEXTS),
        Focus::RightSlot => (&[], &[]),
    };
    let lookup = Lookup { keymap, contexts };
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
