use crate::project::{SEMITONES_PER_OCTAVE, Step};

const DEFAULT_OCTAVE: u8 = 2;
const MAX_OCTAVE: u8 = 4;
const DEFAULT_INSTRUMENT: u8 = 2;
const DEFAULT_VOLUME: u8 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PenField {
    Octave,
    Instrument,
    Volume,
    Effect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pen {
    octave: u8,
    written_step: Step,
}

impl Default for Pen {
    fn default() -> Self {
        let mut pen = Self {
            octave: DEFAULT_OCTAVE,
            written_step: Step::default(),
        };
        pen.set(PenField::Instrument, DEFAULT_INSTRUMENT);
        pen.set(PenField::Volume, DEFAULT_VOLUME);
        pen
    }
}

impl Pen {
    pub fn new(octave: u8, instrument: u8, volume: u8, effect: u8) -> Option<Self> {
        let written_step = Step::new(0, instrument, volume, effect)?;
        (octave <= MAX_OCTAVE).then_some(Self {
            octave,
            written_step,
        })
    }

    pub const fn octave(self) -> u8 {
        self.octave
    }

    pub const fn instrument(self) -> u8 {
        self.written_step.instrument()
    }

    pub const fn volume(self) -> u8 {
        self.written_step.volume()
    }

    pub const fn effect(self) -> u8 {
        self.written_step.effect()
    }

    pub const fn value(self, field: PenField) -> u8 {
        match field {
            PenField::Octave => self.octave(),
            PenField::Instrument => self.instrument(),
            PenField::Volume => self.volume(),
            PenField::Effect => self.effect(),
        }
    }

    pub fn set(&mut self, field: PenField, value: u8) {
        let value = i32::from(value);
        match field {
            PenField::Octave => self.octave = value.min(i32::from(MAX_OCTAVE)) as u8,
            PenField::Instrument => self.written_step.set_instrument(value),
            PenField::Volume => self.written_step.set_volume(value),
            PenField::Effect => self.written_step.set_effect(value),
        }
    }

    pub fn nudge(&mut self, field: PenField, up: bool) {
        let value = self.value(field);
        let nudged = if up {
            value.saturating_add(1)
        } else {
            value.saturating_sub(1)
        };
        self.set(field, nudged);
    }

    pub fn write_note(self, step: &mut Step, piano_key: u8) {
        let pitch = i32::from(self.octave) * SEMITONES_PER_OCTAVE + i32::from(piano_key);
        *step = self.written_step;
        step.set_pitch(pitch);
    }
}
