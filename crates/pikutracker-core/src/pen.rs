use crate::project::{SEMITONES_PER_OCTAVE, Step};

const DEFAULT_OCTAVE: u8 = 2;
const MAX_OCTAVE: u8 = 4;
const DEFAULT_INSTRUMENT: i32 = 2;
const DEFAULT_VOLUME: i32 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pen {
    octave: u8,
    note: Step,
}

impl Default for Pen {
    fn default() -> Self {
        let mut note = Step::default();
        note.set_instrument(DEFAULT_INSTRUMENT);
        note.set_volume(DEFAULT_VOLUME);
        Self {
            octave: DEFAULT_OCTAVE,
            note,
        }
    }
}

impl Pen {
    pub fn new(octave: u8, instrument: u8, volume: u8, effect: u8) -> Option<Self> {
        let note = Step::new(0, instrument, volume, effect)?;
        (octave <= MAX_OCTAVE).then_some(Self { octave, note })
    }

    pub const fn octave(self) -> u8 {
        self.octave
    }

    pub const fn instrument(self) -> u8 {
        self.note.instrument()
    }

    pub const fn volume(self) -> u8 {
        self.note.volume()
    }

    pub const fn effect(self) -> u8 {
        self.note.effect()
    }

    pub fn set_octave(&mut self, octave: i32) {
        self.octave = octave.clamp(0, i32::from(MAX_OCTAVE)) as u8;
    }

    pub fn change_octave(&mut self, delta: i32) {
        self.set_octave(i32::from(self.octave) + delta);
    }

    pub fn set_instrument(&mut self, instrument: i32) {
        self.note.set_instrument(instrument);
    }

    pub fn set_volume(&mut self, volume: i32) {
        self.note.set_volume(volume);
    }

    pub fn set_effect(&mut self, effect: i32) {
        self.note.set_effect(effect);
    }

    pub fn write_note(self, step: &mut Step, piano_key: u8) {
        let pitch = i32::from(self.octave) * SEMITONES_PER_OCTAVE + i32::from(piano_key);
        *step = self.note;
        step.set_pitch(pitch);
    }
}
