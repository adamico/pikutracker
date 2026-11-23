use crate::project::{MAX_EFFECT, MAX_INSTRUMENT, MAX_VOLUME, SEMITONES_PER_OCTAVE, Step};

const DEFAULT_OCTAVE: u8 = 2;
const MAX_OCTAVE: u8 = 4;
const DEFAULT_INSTRUMENT: u8 = 2;
const DEFAULT_VOLUME: u8 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pen {
    octave: u8,
    instrument: u8,
    volume: u8,
    effect: u8,
}

impl Default for Pen {
    fn default() -> Self {
        Self {
            octave: DEFAULT_OCTAVE,
            instrument: DEFAULT_INSTRUMENT,
            volume: DEFAULT_VOLUME,
            effect: 0,
        }
    }
}

impl Pen {
    pub fn new(octave: u8, instrument: u8, volume: u8, effect: u8) -> Option<Self> {
        let fits = |value: u8, max: i32| i32::from(value) <= max;
        (fits(octave, i32::from(MAX_OCTAVE))
            && fits(instrument, MAX_INSTRUMENT)
            && fits(volume, MAX_VOLUME)
            && fits(effect, MAX_EFFECT))
        .then_some(Self {
            octave,
            instrument,
            volume,
            effect,
        })
    }

    pub const fn octave(self) -> u8 {
        self.octave
    }

    pub const fn instrument(self) -> u8 {
        self.instrument
    }

    pub const fn volume(self) -> u8 {
        self.volume
    }

    pub const fn effect(self) -> u8 {
        self.effect
    }

    pub fn set_octave(&mut self, octave: i32) {
        self.octave = octave.clamp(0, i32::from(MAX_OCTAVE)) as u8;
    }

    pub fn change_octave(&mut self, delta: i32) {
        self.set_octave(i32::from(self.octave) + delta);
    }

    pub fn set_instrument(&mut self, instrument: i32) {
        self.instrument = instrument.clamp(0, MAX_INSTRUMENT) as u8;
    }

    pub fn set_volume(&mut self, volume: i32) {
        self.volume = volume.clamp(0, MAX_VOLUME) as u8;
    }

    pub fn set_effect(&mut self, effect: i32) {
        self.effect = effect.clamp(0, MAX_EFFECT) as u8;
    }

    pub fn write_note(self, step: &mut Step, piano_key: u8) {
        step.set_pitch(i32::from(self.octave) * SEMITONES_PER_OCTAVE + i32::from(piano_key));
        step.set_instrument(i32::from(self.instrument));
        step.set_volume(i32::from(self.volume));
        step.set_effect(i32::from(self.effect));
    }
}
