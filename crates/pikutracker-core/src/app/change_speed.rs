use super::messages::Report;
use super::selection::{CellPosition, CellRange};
use super::{Action, App, CHAIN_ENTRIES, Focus, NO_PHRASE_HERE};
use crate::project::{ChainId, PhraseId, SONG_ROWS, TRACKS};
use crate::wording::counted;

const SPEED_HEX_DIGITS: usize = 2;
const MIN_SPEED: u32 = 1;
const MAX_SPEED: u32 = 255;
const ARPEGGIO_SPEED: u8 = 8;
const BAD_SPEED: &str = "SPEED MUST BE 01-FF  ";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SpeedChange {
    Set,
    Scale,
}

impl SpeedChange {
    const fn verb(self) -> &'static str {
        match self {
            SpeedChange::Set => "SETS",
            SpeedChange::Scale => "SCALES",
        }
    }

    fn new_speed_and_exactness(self, speed: u8, new: u8, shown: u8) -> (u8, bool) {
        match self {
            SpeedChange::Set => (new, true),
            SpeedChange::Scale => scaled_speed_and_exactness(speed, new, shown),
        }
    }
}

#[derive(Debug)]
pub(super) struct SpeedEntry {
    change: SpeedChange,
    shown: PhraseId,
    shown_speed: u8,
    phrases: Vec<PhraseId>,
    used_elsewhere: usize,
    in_selection: bool,
    digits: Vec<u8>,
    refused: bool,
    text: String,
}

impl SpeedEntry {
    fn new(
        change: SpeedChange,
        shown: PhraseId,
        shown_speed: u8,
        (phrases, used_elsewhere): (Vec<PhraseId>, usize),
        in_selection: bool,
    ) -> Self {
        let mut entry = Self {
            change,
            shown,
            shown_speed,
            phrases,
            used_elsewhere,
            in_selection,
            digits: Vec::new(),
            refused: false,
            text: String::new(),
        };
        entry.write_question();
        entry
    }

    pub(super) fn text(&self) -> &str {
        &self.text
    }

    fn write_question(&mut self) {
        let typed: String = self
            .digits
            .iter()
            .map(|digit| format!("{digit:X}"))
            .collect();
        self.text = format!(
            "{}PHRASE {:02X} SPEED (NOW {:02X}): {typed:_<width$}  {} {}: {}  ENTER APPLY  ESC CANCEL",
            if self.refused { BAD_SPEED } else { "" },
            self.shown.number(),
            self.shown_speed,
            self.change.verb(),
            counted(self.phrases.len(), "PHRASE"),
            if self.in_selection {
                "SELECTION"
            } else {
                "WHOLE SONG"
            },
            width = SPEED_HEX_DIGITS,
        );
    }

    fn type_digit(&mut self, digit: u8) {
        if self.digits.len() < SPEED_HEX_DIGITS {
            self.digits.push(digit);
        }
        self.refused = false;
        self.write_question();
    }

    fn backspace(&mut self) {
        self.digits.pop();
        self.refused = false;
        self.write_question();
    }

    fn refuse(&mut self) {
        self.refused = true;
        self.write_question();
    }

    fn typed_speed(&self) -> Option<u8> {
        let speed = self
            .digits
            .iter()
            .fold(0, |speed, digit| speed << 4 | digit);
        (speed > 0).then_some(speed)
    }
}

fn scaled_speed_and_exactness(speed: u8, new: u8, old: u8) -> (u8, bool) {
    let (new, old) = (u32::from(new), u32::from(old));
    let product = u32::from(speed) * new;
    let rounded = (2 * product + old) / (2 * old);
    let kept = rounded.clamp(MIN_SPEED, MAX_SPEED);
    (kept as u8, product % old == 0 && kept == rounded)
}

const fn crossed_arpeggio_speed(before: u8, after: u8) -> bool {
    (before <= ARPEGGIO_SPEED) != (after <= ARPEGGIO_SPEED)
}

fn all_chains() -> impl Iterator<Item = ChainId> {
    (0..=u8::MAX).map(ChainId::new)
}

fn whole_song() -> CellRange {
    CellRange::between(
        CellPosition { column: 0, row: 0 },
        CellPosition {
            column: TRACKS - 1,
            row: SONG_ROWS - 1,
        },
    )
}

impl App {
    pub(super) fn ask_for_speed(&mut self, change: SpeedChange) {
        let Some(shown) = self.shown_phrase_id() else {
            self.remark = Some(NO_PHRASE_HERE);
            return;
        };
        let shown_speed = self.project.phrase(shown).speed();
        let (scope, in_selection) = self.speed_scope();
        self.speed_entry = Some(SpeedEntry::new(
            change,
            shown,
            shown_speed,
            scope,
            in_selection,
        ));
    }

    fn speed_scope(&self) -> ((Vec<PhraseId>, usize), bool) {
        match (self.focus, self.selected_cells()) {
            (Focus::Song, Some(cells)) => (self.song_scope(cells), true),
            (Focus::Chain, Some(cells)) => (self.chain_scope(cells), true),
            _ => (self.song_scope(whole_song()), false),
        }
    }

    fn song_scope(&self, cells: CellRange) -> (Vec<PhraseId>, usize) {
        let chains = self.chains_in(cells);
        let phrases = self.phrases_of(&chains);
        let outside: Vec<ChainId> = all_chains()
            .filter(|chain| !chains.contains(chain) || self.placed_outside(*chain, cells))
            .collect();
        let played_outside = self.phrases_of(&outside);
        let used_elsewhere = phrases
            .iter()
            .filter(|phrase| played_outside.contains(phrase))
            .count();
        (phrases, used_elsewhere)
    }

    fn placed_outside(&self, chain: ChainId, cells: CellRange) -> bool {
        (0..SONG_ROWS).any(|row| {
            (0..TRACKS).any(|track| {
                let inside = cells.rows().contains(&row) && cells.columns().contains(&track);
                !inside && self.project.song_cell(row, track) == Some(chain)
            })
        })
    }

    fn chain_scope(&self, cells: CellRange) -> (Vec<PhraseId>, usize) {
        let Some(id) = self.shown_chain_id() else {
            return (Vec::new(), 0);
        };
        let chain = self.project.chain(id);
        let phrases_at = |rows: &mut dyn Iterator<Item = usize>| -> Vec<PhraseId> {
            rows.filter_map(|row| chain.entry(row).map(|entry| entry.phrase()))
                .collect()
        };
        let mut phrases = phrases_at(&mut cells.rows());
        let mut seen = Vec::new();
        phrases.retain(|phrase| {
            let first = !seen.contains(phrase);
            seen.push(*phrase);
            first
        });
        let others: Vec<ChainId> = all_chains().filter(|other| *other != id).collect();
        let mut played_outside = self.phrases_of(&others);
        played_outside.extend(phrases_at(
            &mut (0..CHAIN_ENTRIES).filter(|row| !cells.rows().contains(row)),
        ));
        let used_elsewhere = phrases
            .iter()
            .filter(|phrase| played_outside.contains(phrase))
            .count();
        (phrases, used_elsewhere)
    }

    pub(super) fn use_speed_entry(&mut self, action: Action) {
        let Some(entry) = self.speed_entry.as_mut() else {
            return;
        };
        match action {
            Action::HexDigit(digit) => entry.type_digit(digit),
            Action::Backspace => entry.backspace(),
            Action::Confirm => match entry.typed_speed() {
                Some(speed) => self.apply_speed(speed),
                None => entry.refuse(),
            },
            Action::Cancel => {
                self.speed_entry = None;
                self.selection_start = None;
            }
            _ => {}
        }
    }

    fn apply_speed(&mut self, new: u8) {
        let Some(entry) = self.speed_entry.take() else {
            return;
        };
        let (mut changed, mut inexact, mut crossed) = (0, 0, 0);
        for id in &entry.phrases {
            let before = self.project.phrase(*id).speed();
            let (after, exact) =
                entry
                    .change
                    .new_speed_and_exactness(before, new, entry.shown_speed);
            self.project.phrase_mut(*id).set_speed(i32::from(after));
            changed += usize::from(after != before);
            inexact += usize::from(!exact);
            crossed += usize::from(crossed_arpeggio_speed(before, after));
        }
        self.selection_start = None;
        let inexact_count = match entry.change {
            SpeedChange::Set => String::new(),
            SpeedChange::Scale => format!("  {inexact} INEXACT"),
        };
        let summary = format!(
            "{} CHANGED{inexact_count}  {} USED ELSEWHERE  {crossed} CROSSED SPEED 8",
            counted(changed, "PHRASE"),
            entry.used_elsewhere,
        );
        self.report(if inexact + entry.used_elsewhere + crossed > 0 {
            Report::warning(summary)
        } else {
            Report::info(summary)
        });
    }
}
