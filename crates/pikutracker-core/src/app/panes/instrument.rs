use super::phrase::{STEP_CELLS, STEP_HEADING, StepCell, step_texts};
use super::{HEADING, VALUE, draw_row_label, draw_styled_value, draw_title, stepped, typed_value};
use crate::app::{Action, Direction, KeyContext};
use crate::project::{
    CustomInstrument, Filters, InstrumentSound, MAX_FILTER_LEVEL, MAX_INSTRUMENT_NAME, MAX_STEPS,
    Project, SfxInstrument, Step, WAVEFORM_SAMPLES, WaveformInstrument,
};
use crate::screen::grid::Grid;

const NAME_ROW: usize = 2;
const SETTINGS_ROW: usize = 4;
const FILTERS_ROW: usize = 5;
const STEP_HEADING_ROW: usize = 7;
const FIRST_STEP_ROW: usize = 8;
const FIRST_SAMPLE_ROW: usize = 7;
const SAMPLES_PER_ROW: usize = 8;
const SAMPLE_COLUMN_WIDTH: usize = 3;
const VALUE_COLUMN: usize = 5;
const EMPTY_NAME: char = '.';

const MAX_BUILT_IN_INSTRUMENT: i32 = 7;
const SEMITONES_PER_OCTAVE: i32 = 12;
const COARSE_SPEED: i32 = 16;
const COARSE_LENGTH: i32 = 4;
const COARSE_SAMPLE: i32 = 16;
const MIN_SPEED: i32 = 1;
const MAX_SPEED: i32 = 255;
const MAX_LOOP: i32 = MAX_STEPS as i32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Field {
    Name,
    Kind,
    Speed,
    Length,
    LoopStart,
    LoopEnd,
    Noiz,
    Buzz,
    Detune,
    Reverb,
    Dampen,
    Bass,
    Step(usize, StepCell),
    Sample(usize),
}

struct Row {
    screen_row: usize,
    fields: Vec<(Field, usize)>,
}

impl Row {
    fn new(screen_row: usize, fields: &[(Field, usize)]) -> Self {
        Self {
            screen_row,
            fields: fields.to_vec(),
        }
    }
}

fn rows(sound: &InstrumentSound) -> Vec<Row> {
    let mut rows = vec![
        Row::new(NAME_ROW, &[(Field::Name, VALUE_COLUMN)]),
        Row::new(NAME_ROW + 1, &[(Field::Kind, VALUE_COLUMN)]),
    ];
    match sound {
        InstrumentSound::Sfx(sfx) => {
            rows.push(Row::new(
                SETTINGS_ROW,
                &[
                    (Field::Speed, 4),
                    (Field::Length, 12),
                    (Field::LoopStart, 21),
                    (Field::LoopEnd, 27),
                ],
            ));
            rows.push(Row::new(
                FILTERS_ROW,
                &[
                    (Field::Noiz, 3),
                    (Field::Buzz, 9),
                    (Field::Detune, 15),
                    (Field::Reverb, 21),
                    (Field::Dampen, 27),
                ],
            ));
            for index in 0..sfx.steps.len() {
                let fields: Vec<_> = STEP_CELLS
                    .iter()
                    .map(|(cell, offset, _)| (Field::Step(index, *cell), *offset))
                    .collect();
                rows.push(Row::new(FIRST_STEP_ROW + index, &fields));
            }
        }
        InstrumentSound::Waveform(_) => {
            rows.push(Row::new(SETTINGS_ROW, &[(Field::Bass, VALUE_COLUMN)]));
            rows.push(Row::new(
                FILTERS_ROW,
                &[(Field::Detune, 3), (Field::Reverb, 9), (Field::Dampen, 15)],
            ));
            for row in 0..WAVEFORM_SAMPLES / SAMPLES_PER_ROW {
                let fields: Vec<_> = (0..SAMPLES_PER_ROW)
                    .map(|column| {
                        (
                            Field::Sample(row * SAMPLES_PER_ROW + column),
                            SAMPLE_COLUMN_WIDTH * (column + 1),
                        )
                    })
                    .collect();
                rows.push(Row::new(FIRST_SAMPLE_ROW + row, &fields));
            }
        }
    }
    rows
}

#[derive(Debug)]
pub struct InstrumentView {
    id: u8,
    row: usize,
    column: usize,
    first_digit: Option<u8>,
    instrument: u8,
    volume: u8,
}

impl InstrumentView {
    pub fn new(id: u8) -> Self {
        Self {
            id,
            row: 0,
            column: 0,
            first_digit: None,
            instrument: 2,
            volume: 5,
        }
    }

    pub fn id(&self) -> u8 {
        self.id
    }

    fn shown(&self, project: &Project) -> CustomInstrument {
        project
            .custom_instrument(self.id)
            .cloned()
            .unwrap_or_default()
    }

    fn field(&self, rows: &[Row]) -> Field {
        let row = &rows[self.row.min(rows.len() - 1)];
        row.fields[self.column.min(row.fields.len() - 1)].0
    }

    pub fn key_context(&self, project: &Project) -> KeyContext {
        match self.field(&rows(&self.shown(project).sound)) {
            Field::Name => KeyContext::InstrumentName,
            Field::Step(_, StepCell::Pitch) => KeyContext::PhrasePitch,
            _ => KeyContext::PhraseValue,
        }
    }

    pub fn apply(&mut self, action: Action, project: &mut Project, octave: u8) {
        if !matches!(action, Action::HexDigit(_)) {
            self.first_digit = None;
        }
        let before = self.shown(project);
        let mut instrument = before.clone();
        match action {
            Action::Move(direction) => self.move_cursor(direction, &instrument.sound),
            Action::TypeChar(ch) => self.type_char(&mut instrument, ch),
            Action::Backspace => self.backspace(&mut instrument),
            Action::HexDigit(digit) => self.type_digit(&mut instrument, digit),
            Action::Nudge { up, coarse } => self.nudge(&mut instrument, up, coarse),
            Action::PianoKey(key) => self.enter_note(&mut instrument, octave, key),
            Action::Clear => self.clear(&mut instrument),
            _ => {}
        }
        if instrument != before {
            project.set_custom_instrument(self.id, Some(instrument));
        }
        let rows = rows(&self.shown(project).sound);
        self.row = self.row.min(rows.len() - 1);
    }

    fn move_cursor(&mut self, direction: Direction, sound: &InstrumentSound) {
        let rows = rows(sound);
        let (columns, down) = direction.column_and_row_delta();
        self.row = stepped(self.row.min(rows.len() - 1), down, rows.len());
        let fields = rows[self.row].fields.len();
        self.column = stepped(self.column.min(fields - 1), columns, fields);
    }

    fn type_char(&self, instrument: &mut CustomInstrument, ch: char) {
        if self.field(&rows(&instrument.sound)) != Field::Name {
            return;
        }
        let mut name = instrument.name.clone().unwrap_or_default();
        if name.chars().count() < MAX_INSTRUMENT_NAME {
            name.push(ch);
        }
        instrument.name = Some(name);
    }

    fn backspace(&self, instrument: &mut CustomInstrument) {
        if self.field(&rows(&instrument.sound)) != Field::Name {
            return;
        }
        let mut name = instrument.name.clone().unwrap_or_default();
        name.pop();
        instrument.name = Some(name).filter(|name| !name.is_empty());
    }

    fn type_digit(&mut self, instrument: &mut CustomInstrument, digit: u8) {
        let field = self.field(&rows(&instrument.sound));
        let two_digits = matches!(
            field,
            Field::Speed | Field::Length | Field::LoopStart | Field::LoopEnd | Field::Sample(_)
        );
        if two_digits {
            let value = typed_value(&mut self.first_digit, digit);
            let value = match field {
                Field::Sample(_) => i32::from(value as i8),
                _ => i32::from(value),
            };
            self.set(instrument, field, value);
        } else {
            self.set(instrument, field, i32::from(digit));
        }
    }

    fn nudge(&mut self, instrument: &mut CustomInstrument, up: bool, coarse: bool) {
        let field = self.field(&rows(&instrument.sound));
        if field == Field::Kind {
            instrument.sound = match instrument.sound {
                InstrumentSound::Sfx(_) => InstrumentSound::Waveform(WaveformInstrument::default()),
                InstrumentSound::Waveform(_) => InstrumentSound::Sfx(SfxInstrument::default()),
            };
            return;
        }
        let amount = if coarse { coarse_amount(field) } else { 1 };
        let delta = if up { amount } else { -amount };
        if let Some(value) = value(instrument, field) {
            self.set(instrument, field, value + delta);
        }
    }

    fn enter_note(&mut self, instrument: &mut CustomInstrument, octave: u8, key: u8) {
        let Field::Step(index, StepCell::Pitch) = self.field(&rows(&instrument.sound)) else {
            return;
        };
        let InstrumentSound::Sfx(sfx) = &mut instrument.sound else {
            return;
        };
        let step = &mut sfx.steps[index];
        step.set_pitch(i32::from(octave) * SEMITONES_PER_OCTAVE + i32::from(key));
        step.set_instrument(i32::from(self.instrument));
        step.set_volume(i32::from(self.volume));
        self.row = stepped(self.row, 1, rows(&instrument.sound).len());
    }

    fn clear(&self, instrument: &mut CustomInstrument) {
        match (self.field(&rows(&instrument.sound)), &mut instrument.sound) {
            (Field::Name, _) => instrument.name = None,
            (Field::Step(index, StepCell::Effect), InstrumentSound::Sfx(sfx)) => {
                sfx.steps[index].set_effect(0);
            }
            (Field::Step(index, _), InstrumentSound::Sfx(sfx)) => sfx.steps[index].set_volume(0),
            (Field::Sample(index), InstrumentSound::Waveform(wave)) => wave.samples[index] = 0,
            _ => {}
        }
    }

    fn set(&mut self, instrument: &mut CustomInstrument, field: Field, value: i32) {
        match &mut instrument.sound {
            InstrumentSound::Sfx(sfx) => self.set_sfx(sfx, field, value),
            InstrumentSound::Waveform(wave) => set_waveform(wave, field, value),
        }
    }

    fn set_sfx(&mut self, sfx: &mut SfxInstrument, field: Field, value: i32) {
        let filters = &mut sfx.filters;
        match field {
            Field::Speed => sfx.speed = value.clamp(MIN_SPEED, MAX_SPEED) as u8,
            Field::Length => {
                let length = value.clamp(1, MAX_STEPS as i32) as usize;
                sfx.steps.resize(length, Step::default());
            }
            Field::LoopStart => sfx.loop_start = value.clamp(0, MAX_LOOP) as u8,
            Field::LoopEnd => sfx.loop_end = value.clamp(0, MAX_LOOP) as u8,
            Field::Noiz => filters.set_noiz(value > 0),
            Field::Buzz => filters.set_buzz(value > 0),
            Field::Detune => filters.set_detune(value),
            Field::Reverb => filters.set_reverb(value),
            Field::Dampen => filters.set_dampen(value),
            Field::Step(index, cell) => {
                let step = &mut sfx.steps[index];
                match cell {
                    StepCell::Pitch => step.set_pitch(value),
                    StepCell::Instrument => {
                        step.set_instrument(value.clamp(0, MAX_BUILT_IN_INSTRUMENT));
                        self.instrument = step.instrument();
                    }
                    StepCell::Volume => {
                        step.set_volume(value);
                        self.volume = step.volume();
                    }
                    StepCell::Effect => step.set_effect(value),
                }
            }
            Field::Name | Field::Kind | Field::Bass | Field::Sample(_) => {}
        }
    }

    pub fn finish_typing(&mut self) {
        self.first_digit = None;
    }

    pub fn draw(&self, grid: &mut Grid, project: &Project) {
        let instrument = self.shown(project);
        draw_title(grid, 0, 0, &format!("INSTRUMENT {:X}", self.id), true);
        let labels: Vec<(usize, &str)> = match &instrument.sound {
            InstrumentSound::Sfx(_) => vec![
                (SETTINGS_ROW, "SPD     LEN     LOOP    TO"),
                (FILTERS_ROW, "NZ    BZ    DT    RV    DP"),
                (STEP_HEADING_ROW, STEP_HEADING),
            ],
            InstrumentSound::Waveform(_) => {
                vec![(SETTINGS_ROW, "BASS"), (FILTERS_ROW, "DT    RV    DP")]
            }
        };
        grid.put(0, NAME_ROW, "NAME", HEADING);
        grid.put(0, NAME_ROW + 1, "KIND", HEADING);
        for (row, label) in labels {
            grid.put(0, row, label, HEADING);
        }
        let rows = rows(&instrument.sound);
        let cursor = (self.row.min(rows.len() - 1), self.column);
        for (index, row) in rows.iter().enumerate() {
            if let Some((Field::Step(step, _) | Field::Sample(step), _)) = row.fields.first() {
                draw_row_label(grid, 0, row.screen_row, &format!("{step:02X}"), false);
            }
            let cursor_column = cursor.1.min(row.fields.len() - 1);
            for (column, (field, offset)) in row.fields.iter().enumerate() {
                let text = field_text(&instrument, *field);
                let is_cursor = index == cursor.0 && column == cursor_column;
                draw_styled_value(grid, *offset, row.screen_row, &text, is_cursor, VALUE);
            }
        }
    }
}

fn set_waveform(wave: &mut WaveformInstrument, field: Field, value: i32) {
    let level = |value: i32| value.clamp(0, MAX_FILTER_LEVEL) as u8;
    match field {
        Field::Bass => wave.bass = value > 0,
        Field::Detune => wave.detune = level(value),
        Field::Reverb => wave.reverb = level(value),
        Field::Dampen => wave.dampen = level(value),
        Field::Sample(index) => {
            wave.samples[index] = value.clamp(i32::from(i8::MIN), i32::from(i8::MAX)) as i8;
        }
        _ => {}
    }
}

fn coarse_amount(field: Field) -> i32 {
    match field {
        Field::Step(_, StepCell::Pitch) => SEMITONES_PER_OCTAVE,
        Field::Speed => COARSE_SPEED,
        Field::Length => COARSE_LENGTH,
        Field::Sample(_) => COARSE_SAMPLE,
        _ => 1,
    }
}

fn value(instrument: &CustomInstrument, field: Field) -> Option<i32> {
    let value = match &instrument.sound {
        InstrumentSound::Sfx(sfx) => sfx_value(sfx, field)?,
        InstrumentSound::Waveform(wave) => match field {
            Field::Bass => u8::from(wave.bass),
            Field::Detune => wave.detune,
            Field::Reverb => wave.reverb,
            Field::Dampen => wave.dampen,
            Field::Sample(index) => return Some(i32::from(wave.samples[index])),
            _ => return None,
        },
    };
    Some(i32::from(value))
}

fn sfx_value(sfx: &SfxInstrument, field: Field) -> Option<u8> {
    let filters: Filters = sfx.filters;
    Some(match field {
        Field::Speed => sfx.speed,
        Field::Length => sfx.steps.len() as u8,
        Field::LoopStart => sfx.loop_start,
        Field::LoopEnd => sfx.loop_end,
        Field::Noiz => u8::from(filters.noiz()),
        Field::Buzz => u8::from(filters.buzz()),
        Field::Detune => filters.detune(),
        Field::Reverb => filters.reverb(),
        Field::Dampen => filters.dampen(),
        Field::Step(index, cell) => {
            let step = sfx.steps[index];
            match cell {
                StepCell::Pitch => step.pitch(),
                StepCell::Instrument => step.instrument(),
                StepCell::Volume => step.volume(),
                StepCell::Effect => step.effect(),
            }
        }
        Field::Name | Field::Kind | Field::Bass | Field::Sample(_) => return None,
    })
}

fn field_text(instrument: &CustomInstrument, field: Field) -> String {
    match (field, &instrument.sound) {
        (Field::Name, _) => {
            let name = instrument.name.clone().unwrap_or_default();
            let padding = MAX_INSTRUMENT_NAME.saturating_sub(name.chars().count());
            name + &EMPTY_NAME.to_string().repeat(padding)
        }
        (Field::Kind, InstrumentSound::Sfx(_)) => "SFX".to_owned(),
        (Field::Kind, InstrumentSound::Waveform(_)) => "WAVE".to_owned(),
        (Field::Step(index, cell), InstrumentSound::Sfx(sfx)) => {
            let column = STEP_CELLS
                .iter()
                .position(|(step_cell, _, _)| *step_cell == cell)
                .unwrap_or(0);
            step_texts(sfx.steps[index])[column].clone()
        }
        (Field::Sample(index), InstrumentSound::Waveform(wave)) => {
            format!("{:02X}", wave.samples[index] as u8)
        }
        (Field::Speed | Field::Length | Field::LoopStart | Field::LoopEnd, _) => {
            format!("{:02X}", value(instrument, field).unwrap_or(0))
        }
        _ => value(instrument, field).unwrap_or(0).to_string(),
    }
}
