use super::{
    DIMMED, EMPTY, HEADING, LOOP_MARK, SELECTED, VALUE, draw_out_of_range_flag, draw_row_label,
    draw_styled_value, draw_title, id_text, stepped,
};
use crate::app::layout::Area;
use crate::app::selection::{CellPosition, CellRange};
use crate::app::{Direction, KeyContext};
use crate::project::{
    FIRST_CUSTOM_INSTRUMENT, LAST_CUSTOM_INSTRUMENT, MAX_STEPS, NOTE_NAMES, Phrase, PhraseId, Step,
};
use crate::screen::grid::Grid;
use crate::settings::{DEFAULT_OCTAVE, MAX_OCTAVE};
use std::ops::RangeInclusive;

const HEADER_ROWS: usize = 5;
const FIRST_HEADER_ROW: usize = 1;

const SEMITONES_PER_OCTAVE: i32 = 12;
const COARSE_SPEED: i32 = 16;
const COARSE_LOOP: i32 = 4;
const LOOP_MARK_COLUMN: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum StepCell {
    Pitch,
    Instrument,
    Volume,
    Effect,
}

pub(super) const STEP_CELLS: [(StepCell, usize, &str); 4] = [
    (StepCell::Pitch, 3, "..."),
    (StepCell::Instrument, 7, "."),
    (StepCell::Volume, 9, "."),
    (StepCell::Effect, 11, "."),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HeaderCell {
    Speed,
    LoopStart,
    LoopEnd,
    Noiz,
    Buzz,
    Detune,
    Reverb,
    Dampen,
}

const HEADER_CELLS: [&[(HeaderCell, usize)]; 3] = [
    &[
        (HeaderCell::Speed, 4),
        (HeaderCell::LoopStart, 12),
        (HeaderCell::LoopEnd, 15),
    ],
    &[(HeaderCell::Noiz, 12), (HeaderCell::Buzz, 17)],
    &[
        (HeaderCell::Detune, 3),
        (HeaderCell::Reverb, 8),
        (HeaderCell::Dampen, 13),
    ],
];

#[derive(Clone, Copy, Debug)]
pub struct PhraseView<'a> {
    pub id: Option<PhraseId>,
    pub phrase: &'a Phrase,
    pub transpose: i8,
    pub playing_step: Option<usize>,
}

#[derive(Debug)]
pub struct PhrasePane {
    step: usize,
    column: usize,
    header_row: Option<usize>,
    octave: u8,
    instrument: u8,
    volume: u8,
    first_digit: Option<u8>,
}

impl Default for PhrasePane {
    fn default() -> Self {
        Self {
            step: 0,
            column: 0,
            header_row: None,
            octave: DEFAULT_OCTAVE,
            instrument: 2,
            volume: 5,
            first_digit: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhraseCursorPlace {
    step: usize,
    column: usize,
    header_row: Option<usize>,
}

impl PhrasePane {
    pub fn cursor_place(&self) -> PhraseCursorPlace {
        PhraseCursorPlace {
            step: self.step,
            column: self.column,
            header_row: self.header_row,
        }
    }

    pub fn return_to(&mut self, place: PhraseCursorPlace) {
        self.step = place.step;
        self.column = place.column;
        self.header_row = place.header_row;
    }

    pub fn step(&self) -> usize {
        self.step
    }

    pub fn note_step(&self) -> Option<usize> {
        matches!(
            self.step_cell(),
            Some(StepCell::Pitch | StepCell::Instrument)
        )
        .then_some(self.step)
    }

    pub fn jump_to(&mut self, step: usize) {
        self.step = step.min(MAX_STEPS - 1);
        self.header_row = None;
    }

    pub fn return_to_top(&mut self) {
        self.step = 0;
        self.column = 0;
        self.header_row = None;
    }

    pub fn key_context(&self) -> KeyContext {
        if self.step_cell() == Some(StepCell::Pitch) {
            KeyContext::PhrasePitch
        } else {
            KeyContext::PhraseValue
        }
    }

    pub fn move_cursor(&mut self, direction: Direction) {
        let (columns, rows) = direction.column_and_row_delta();
        match self.header_row {
            Some(row) => self.move_in_header(row, columns, rows),
            None if rows < 0 && self.step == 0 => self.enter_header(),
            None => {
                self.column = stepped(self.column, columns, STEP_CELLS.len());
                self.step = stepped(self.step, rows, MAX_STEPS);
            }
        }
    }

    fn move_in_header(&mut self, row: usize, columns: isize, rows: isize) {
        if rows > 0 && row + 1 == HEADER_CELLS.len() {
            self.header_row = None;
            return;
        }
        let row = stepped(row, rows, HEADER_CELLS.len());
        self.header_row = Some(row);
        self.column = stepped(self.column, columns, HEADER_CELLS[row].len());
    }

    fn enter_header(&mut self) {
        let row = HEADER_CELLS.len() - 1;
        self.header_row = Some(row);
        self.column = self.column.min(HEADER_CELLS[row].len() - 1);
    }

    fn step_cell(&self) -> Option<StepCell> {
        match self.header_row {
            Some(_) => None,
            None => Some(STEP_CELLS[self.column].0),
        }
    }

    fn header_cell(&self) -> Option<HeaderCell> {
        let row = self.header_row?;
        Some(HEADER_CELLS[row][self.column].0)
    }

    pub fn enter_note(&mut self, phrase: &mut Phrase, piano_key: u8) {
        if self.step_cell() != Some(StepCell::Pitch) {
            return;
        }
        let pitch = i32::from(self.octave) * SEMITONES_PER_OCTAVE + i32::from(piano_key);
        let step = phrase.step_mut(self.step);
        step.set_pitch(pitch);
        step.set_instrument(i32::from(self.instrument));
        step.set_volume(i32::from(self.volume));
        self.step = stepped(self.step, 1, MAX_STEPS);
    }

    pub fn instrument_under_cursor(&self, phrase: &Phrase) -> Option<u8> {
        (self.step_cell() == Some(StepCell::Instrument)).then(|| {
            let step = phrase.step(self.step);
            if step.is_silent() {
                self.instrument
            } else {
                step.instrument()
            }
        })
    }

    pub fn is_on_the_last_instrument(&self, phrase: &Phrase) -> bool {
        self.step_cell() == Some(StepCell::Instrument)
            && phrase.step(self.step).instrument() == LAST_CUSTOM_INSTRUMENT
    }

    pub fn octave(&self) -> u8 {
        self.octave
    }

    pub fn set_octave(&mut self, octave: u8) {
        self.octave = octave.min(MAX_OCTAVE);
    }

    pub fn change_octave(&mut self, delta: i8) {
        self.octave = self.octave.saturating_add_signed(delta).min(MAX_OCTAVE);
    }

    pub fn finish_typing(&mut self) {
        self.first_digit = None;
    }

    pub fn type_digit(&mut self, phrase: &mut Phrase, digit: u8) {
        let typed = self.first_digit.map_or(0, |high| high << 4) | digit;
        let two_digits = matches!(
            self.header_cell(),
            Some(HeaderCell::Speed | HeaderCell::LoopStart | HeaderCell::LoopEnd)
        );
        self.first_digit = two_digits.then_some(digit);
        let value = i32::from(if two_digits { typed } else { digit });
        self.set_cell(phrase, value);
    }

    pub fn nudge(&mut self, phrase: &mut Phrase, up: bool, coarse: bool) {
        let amount = if coarse { self.coarse_amount() } else { 1 };
        let delta = if up { amount } else { -amount };
        let value = self.cell_value(phrase) + delta;
        self.set_cell(phrase, value);
    }

    fn coarse_amount(&self) -> i32 {
        match (self.step_cell(), self.header_cell()) {
            (Some(StepCell::Pitch), _) => SEMITONES_PER_OCTAVE,
            (_, Some(HeaderCell::Speed)) => COARSE_SPEED,
            (_, Some(HeaderCell::LoopStart | HeaderCell::LoopEnd)) => COARSE_LOOP,
            _ => 1,
        }
    }

    fn cell_value(&self, phrase: &Phrase) -> i32 {
        let step = phrase.step(self.step);
        let filters = phrase.filters();
        match (self.step_cell(), self.header_cell()) {
            (Some(StepCell::Pitch), _) => i32::from(step.pitch()),
            (Some(StepCell::Instrument), _) => i32::from(step.instrument()),
            (Some(StepCell::Volume), _) => i32::from(step.volume()),
            (Some(StepCell::Effect), _) => i32::from(step.effect()),
            (_, Some(HeaderCell::Speed)) => i32::from(phrase.speed()),
            (_, Some(HeaderCell::LoopStart)) => i32::from(phrase.loop_start()),
            (_, Some(HeaderCell::LoopEnd)) => i32::from(phrase.loop_end()),
            (_, Some(HeaderCell::Noiz)) => i32::from(filters.noiz()),
            (_, Some(HeaderCell::Buzz)) => i32::from(filters.buzz()),
            (_, Some(HeaderCell::Detune)) => i32::from(filters.detune()),
            (_, Some(HeaderCell::Reverb)) => i32::from(filters.reverb()),
            (_, Some(HeaderCell::Dampen)) => i32::from(filters.dampen()),
            (None, None) => 0,
        }
    }

    fn set_cell(&mut self, phrase: &mut Phrase, value: i32) {
        if let Some(cell) = self.step_cell() {
            self.set_step_cell(phrase, cell, value);
        }
        match self.header_cell() {
            Some(HeaderCell::Speed) => phrase.set_speed(value),
            Some(HeaderCell::LoopStart) => phrase.set_loop_start(value),
            Some(HeaderCell::LoopEnd) => phrase.set_loop_end(value),
            Some(HeaderCell::Noiz) => phrase.filters_mut().set_noiz(value > 0),
            Some(HeaderCell::Buzz) => phrase.filters_mut().set_buzz(value > 0),
            Some(HeaderCell::Detune) => phrase.filters_mut().set_detune(value),
            Some(HeaderCell::Reverb) => phrase.filters_mut().set_reverb(value),
            Some(HeaderCell::Dampen) => phrase.filters_mut().set_dampen(value),
            None => {}
        }
    }

    fn set_step_cell(&mut self, phrase: &mut Phrase, cell: StepCell, value: i32) {
        let step = phrase.step_mut(self.step);
        match cell {
            StepCell::Pitch => step.set_pitch(value),
            StepCell::Instrument => {
                step.set_instrument(value);
                self.instrument = step.instrument();
            }
            StepCell::Volume => {
                step.set_volume(value);
                self.volume = step.volume();
            }
            StepCell::Effect => step.set_effect(value),
        }
    }

    pub fn cell(&self) -> Option<CellPosition> {
        self.header_row.is_none().then_some(CellPosition {
            column: self.column,
            row: self.step,
        })
    }

    pub fn nudge_cells(&self, phrase: &mut Phrase, cells: CellRange, up: bool, coarse: bool) {
        for index in cells.rows() {
            let step = phrase.step_mut(index);
            if !step.is_silent() {
                for column in cells.columns() {
                    nudge_step_cell(step, STEP_CELLS[column].0, up, coarse);
                }
            }
        }
    }

    pub fn clear_cells(&self, phrase: &mut Phrase, cells: CellRange) {
        for index in cells.rows() {
            let step = phrase.step_mut(index);
            for column in cells.columns() {
                match STEP_CELLS[column].0 {
                    StepCell::Effect => step.set_effect(0),
                    _ => step.set_volume(0),
                }
            }
        }
    }

    pub fn copy(&self, phrase: &Phrase, cells: CellRange) -> Vec<Step> {
        cells.rows().map(|index| phrase.step(index)).collect()
    }

    pub fn paste(
        &self,
        phrase: &mut Phrase,
        columns: &RangeInclusive<usize>,
        rows: &[Step],
    ) -> usize {
        let fitting = rows.len().min(MAX_STEPS.saturating_sub(self.step));
        for (index, source) in (self.step..).zip(&rows[..fitting]) {
            let step = phrase.step_mut(index);
            *step = pasted_step(*step, *source, columns);
        }
        rows.len() - fitting
    }

    pub fn draw_selection(&self, grid: &mut Grid, area: Area, cells: CellRange) {
        for index in cells.rows() {
            for column in cells.columns() {
                if (index, column) != (self.step, self.column) {
                    let (_, offset, empty) = STEP_CELLS[column];
                    let row = HEADER_ROWS + index;
                    grid.restyle(area.column + offset, row, empty.len(), SELECTED);
                }
            }
        }
    }

    pub fn follow(&mut self, step: usize) {
        if self.step != step {
            self.step = step;
            self.finish_typing();
        }
    }

    pub fn draw(&self, grid: &mut Grid, area: Area, focused: bool, view: PhraseView) {
        let PhraseView {
            id,
            phrase,
            transpose,
            playing_step,
        } = view;
        let number = id_text(id.map(PhraseId::number));
        let title = format!("PHRASE {number}   OCT {}", self.octave);
        draw_title(grid, area.column, 0, &title, focused);
        self.draw_header(grid, area, focused, phrase);
        grid.put(area.column, HEADER_ROWS - 1, "   NOT I V F", HEADING);
        let loop_start = phrase.phrase_loop().map(|steps| steps.start);

        for index in 0..MAX_STEPS {
            let row = HEADER_ROWS + index;
            let label = format!("{index:02X}");
            let plays = index < phrase.length();
            if plays {
                draw_row_label(grid, area.column, row, &label, playing_step == Some(index));
            } else {
                grid.put(area.column, row, &label, DIMMED);
            }
            if loop_start == Some(index) {
                grid.put(area.column + LOOP_MARK_COLUMN, row, "<", LOOP_MARK);
            }
            let on_this_step = focused && self.header_row.is_none() && index == self.step;
            let texts = step_texts(phrase.step(index));
            for (cell, ((_, offset, _), text)) in STEP_CELLS.iter().zip(&texts).enumerate() {
                let is_cursor = on_this_step && cell == self.column;
                let style = if !plays {
                    DIMMED
                } else if phrase.step(index).is_silent() {
                    EMPTY
                } else {
                    VALUE
                };
                draw_styled_value(grid, area.column + offset, row, text, is_cursor, style);
            }
            if phrase.step_is_out_of_range(index, transpose) {
                draw_out_of_range_flag(grid, area.column, row);
            }
        }
    }

    fn draw_header(&self, grid: &mut Grid, area: Area, focused: bool, phrase: &Phrase) {
        let filters = phrase.filters();
        let labels = [
            "SPD    LOOP".to_owned(),
            format!("BPM {:<5}NZ   BZ", phrase.bpm()),
            "DT   RV   DP".to_owned(),
        ];
        for (row, label) in labels.iter().enumerate() {
            grid.put(area.column, FIRST_HEADER_ROW + row, label, HEADING);
        }

        for (row, cells) in HEADER_CELLS.iter().enumerate() {
            for (column, (cell, offset)) in cells.iter().enumerate() {
                let text = match cell {
                    HeaderCell::Speed => format!("{:02X}", phrase.speed()),
                    HeaderCell::LoopStart => format!("{:02X}", phrase.loop_start()),
                    HeaderCell::LoopEnd => format!("{:02X}", phrase.loop_end()),
                    HeaderCell::Noiz => u8::from(filters.noiz()).to_string(),
                    HeaderCell::Buzz => u8::from(filters.buzz()).to_string(),
                    HeaderCell::Detune => filters.detune().to_string(),
                    HeaderCell::Reverb => filters.reverb().to_string(),
                    HeaderCell::Dampen => filters.dampen().to_string(),
                };
                let is_cursor = focused && self.header_row == Some(row) && self.column == column;
                let screen_row = FIRST_HEADER_ROW + row;
                draw_styled_value(
                    grid,
                    area.column + offset,
                    screen_row,
                    &text,
                    is_cursor,
                    VALUE,
                );
            }
        }
    }
}

fn nudge_step_cell(step: &mut Step, cell: StepCell, up: bool, coarse: bool) {
    let amount = if coarse && cell == StepCell::Pitch {
        SEMITONES_PER_OCTAVE
    } else {
        1
    };
    let delta = if up { amount } else { -amount };
    match cell {
        StepCell::Pitch => step.set_pitch(i32::from(step.pitch()) + delta),
        StepCell::Instrument if step.instrument() < FIRST_CUSTOM_INSTRUMENT => {
            let last_built_in = i32::from(FIRST_CUSTOM_INSTRUMENT) - 1;
            step.set_instrument((i32::from(step.instrument()) + delta).min(last_built_in));
        }
        StepCell::Instrument => {}
        StepCell::Volume => step.set_volume(i32::from(step.volume()) + delta),
        StepCell::Effect => step.set_effect(i32::from(step.effect()) + delta),
    }
}

fn pasted_step(target: Step, source: Step, columns: &RangeInclusive<usize>) -> Step {
    let copied = |cell| {
        STEP_CELLS
            .iter()
            .position(|(column, _, _)| *column == cell)
            .is_some_and(|column| columns.contains(&column))
    };
    let pick = |cell, from_source, from_target| {
        if copied(cell) {
            from_source
        } else {
            from_target
        }
    };
    let empties_note =
        source.is_silent() && (copied(StepCell::Pitch) || copied(StepCell::Instrument));
    let volume = if empties_note {
        0
    } else {
        pick(StepCell::Volume, source.volume(), target.volume())
    };
    Step::new(
        pick(StepCell::Pitch, source.pitch(), target.pitch()),
        pick(
            StepCell::Instrument,
            source.instrument(),
            target.instrument(),
        ),
        volume,
        pick(StepCell::Effect, source.effect(), target.effect()),
    )
    .unwrap_or(target)
}

pub(super) fn step_texts(step: Step) -> [String; 4] {
    if step.is_silent() {
        return STEP_CELLS.map(|(_, _, empty)| empty.to_owned());
    }
    let pitch = usize::from(step.pitch());
    [
        format!("{}{}", NOTE_NAMES[pitch % 12], pitch / 12),
        format!("{:X}", step.instrument()),
        step.volume().to_string(),
        step.effect().to_string(),
    ]
}
