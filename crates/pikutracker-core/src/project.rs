mod change;
mod song_order;
mod usage;

pub(crate) use change::ProjectChange;
pub use song_order::{SongRowEnding, SongRowMarks};
pub use usage::UnusedItems;

use std::collections::BTreeMap;
use std::ops::Range;

pub const MAX_STEPS: usize = 32;
pub const TRACKS: usize = 4;
pub const CHAIN_ENTRIES: usize = 16;
pub const SONG_ROWS: usize = 256;
pub const WAVEFORM_SAMPLES: usize = 64;
pub const FIRST_CUSTOM_INSTRUMENT: u8 = 8;
pub const LAST_CUSTOM_INSTRUMENT: u8 = 0xF;
pub const MAX_INSTRUMENT_NAME: usize = 12;
pub const CART_SLOTS: u8 = 64;
pub const MAX_FILTER_LEVEL: i32 = 2;

pub const NOTE_NAMES: [&str; 12] = [
    "C-", "C#", "D-", "D#", "E-", "F-", "F#", "G-", "G#", "A-", "A#", "B-",
];

const ID_COUNT: usize = 256;
const MIN_TRANSPOSE: i32 = i8::MIN as i32;
const MAX_TRANSPOSE: i32 = i8::MAX as i32;

const DEFAULT_LENGTH: usize = 16;
const DEFAULT_SPEED: u8 = 16;
const DEFAULT_PITCH: u8 = 24;
pub(crate) const MAX_PITCH: i32 = 63;
const MAX_INSTRUMENT: i32 = LAST_CUSTOM_INSTRUMENT as i32;
const MAX_VOLUME: i32 = 7;
const MAX_EFFECT: i32 = 7;
const EFFECT_ARPEGGIO_FAST: u8 = 6;
const EFFECT_ARPEGGIO_SLOW: u8 = 7;
pub const ARPEGGIO_GROUP: usize = 4;

pub const fn arpeggio_group_start(index: usize) -> usize {
    index - index % ARPEGGIO_GROUP
}
const MIN_SPEED: i32 = 1;
const MAX_SPEED: i32 = 255;
const BPM_AT_SPEED_1: f32 = 1807.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    pitch: u8,
    instrument: u8,
    volume: u8,
    effect: u8,
}

pub const fn custom_instrument_of_slot(slot: u8) -> u8 {
    FIRST_CUSTOM_INSTRUMENT + slot
}

pub const fn slot_of_custom_instrument(instrument: u8) -> Option<u8> {
    if instrument >= FIRST_CUSTOM_INSTRUMENT && instrument <= LAST_CUSTOM_INSTRUMENT {
        Some(instrument - FIRST_CUSTOM_INSTRUMENT)
    } else {
        None
    }
}

impl Default for Step {
    fn default() -> Self {
        Self {
            pitch: DEFAULT_PITCH,
            instrument: 0,
            volume: 0,
            effect: 0,
        }
    }
}

impl Step {
    pub const fn new(pitch: u8, instrument: u8, volume: u8, effect: u8) -> Option<Self> {
        if pitch as i32 > MAX_PITCH
            || instrument as i32 > MAX_INSTRUMENT
            || volume as i32 > MAX_VOLUME
            || effect as i32 > MAX_EFFECT
        {
            return None;
        }
        Some(Self {
            pitch,
            instrument,
            volume,
            effect,
        })
    }

    pub const fn pitch(self) -> u8 {
        self.pitch
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

    pub const fn is_silent(self) -> bool {
        self.volume == 0
    }

    pub const fn plays_arpeggio(self) -> bool {
        !self.is_silent() && matches!(self.effect, EFFECT_ARPEGGIO_FAST | EFFECT_ARPEGGIO_SLOW)
    }

    pub fn set_pitch(&mut self, pitch: i32) {
        self.pitch = pitch.clamp(0, MAX_PITCH) as u8;
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
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Filters {
    noiz: bool,
    buzz: bool,
    detune: u8,
    reverb: u8,
    dampen: u8,
}

impl Filters {
    pub const fn noiz(self) -> bool {
        self.noiz
    }

    pub const fn buzz(self) -> bool {
        self.buzz
    }

    pub const fn detune(self) -> u8 {
        self.detune
    }

    pub const fn reverb(self) -> u8 {
        self.reverb
    }

    pub const fn dampen(self) -> u8 {
        self.dampen
    }

    pub fn set_noiz(&mut self, on: bool) {
        self.noiz = on;
    }

    pub fn set_buzz(&mut self, on: bool) {
        self.buzz = on;
    }

    pub fn set_detune(&mut self, level: i32) {
        self.detune = level.clamp(0, MAX_FILTER_LEVEL) as u8;
    }

    pub fn set_reverb(&mut self, level: i32) {
        self.reverb = level.clamp(0, MAX_FILTER_LEVEL) as u8;
    }

    pub fn set_dampen(&mut self, level: i32) {
        self.dampen = level.clamp(0, MAX_FILTER_LEVEL) as u8;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Phrase {
    steps: [Step; MAX_STEPS],
    loop_start: u8,
    loop_end: u8,
    speed: u8,
    filters: Filters,
}

impl Default for Phrase {
    fn default() -> Self {
        Self {
            steps: [Step::default(); MAX_STEPS],
            loop_start: DEFAULT_LENGTH as u8,
            loop_end: 0,
            speed: DEFAULT_SPEED,
            filters: Filters::default(),
        }
    }
}

impl Phrase {
    pub fn step(&self, index: usize) -> Step {
        self.steps[index]
    }

    pub fn step_mut(&mut self, index: usize) -> &mut Step {
        &mut self.steps[index]
    }

    pub const fn length(&self) -> usize {
        let (start, end) = (self.loop_start as usize, self.loop_end as usize);
        if end == 0 && start > 0 {
            start
        } else if start < end {
            end
        } else {
            MAX_STEPS
        }
    }

    pub const fn phrase_loop(&self) -> Option<Range<usize>> {
        let (start, end) = (self.loop_start as usize, self.loop_end as usize);
        if start < end { Some(start..end) } else { None }
    }

    pub const fn loops(&self) -> bool {
        self.phrase_loop().is_some()
    }

    pub fn step_after(&self, step: usize) -> Option<usize> {
        if step + 1 < self.length() {
            Some(step + 1)
        } else {
            self.phrase_loop().map(|steps| steps.start)
        }
    }

    pub const fn loop_start(&self) -> u8 {
        self.loop_start
    }

    pub const fn loop_end(&self) -> u8 {
        self.loop_end
    }

    pub fn set_loop_start(&mut self, start: i32) {
        self.loop_start = start.clamp(0, MAX_STEPS as i32) as u8;
    }

    pub fn set_loop_end(&mut self, end: i32) {
        self.loop_end = end.clamp(0, MAX_STEPS as i32) as u8;
    }

    pub fn set_loop(&mut self, start: i32, end: i32) {
        self.set_loop_start(start);
        self.set_loop_end(end);
    }

    pub fn set_length(&mut self, length: i32) {
        match length.clamp(1, MAX_STEPS as i32) {
            full if full == MAX_STEPS as i32 => self.set_loop(0, 0),
            length => self.set_loop(length, 0),
        }
    }

    pub const fn speed(&self) -> u8 {
        self.speed
    }

    pub fn set_speed(&mut self, speed: i32) {
        self.speed = speed.clamp(MIN_SPEED, MAX_SPEED) as u8;
    }

    pub fn bpm(&self) -> u16 {
        (BPM_AT_SPEED_1 / f32::from(self.speed)).round() as u16
    }

    pub const fn filters(&self) -> Filters {
        self.filters
    }

    pub fn filters_mut(&mut self) -> &mut Filters {
        &mut self.filters
    }

    pub fn ticks(&self) -> u32 {
        self.length() as u32 * u32::from(self.speed)
    }

    pub fn step_is_out_of_range(&self, index: usize, transpose: i8) -> bool {
        let step = self.steps[index];
        let pitch = i32::from(step.pitch) + i32::from(transpose);
        let heard =
            (index < self.length() && !step.is_silent()) || self.steps_read_by_arpeggios()[index];
        heard && !(0..=MAX_PITCH).contains(&pitch)
    }

    pub fn out_of_range_steps(&self, transpose: i8) -> impl Iterator<Item = usize> + '_ {
        (0..MAX_STEPS).filter(move |&index| self.step_is_out_of_range(index, transpose))
    }

    pub fn steps_read_by_arpeggios(&self) -> [bool; MAX_STEPS] {
        let mut read = [false; MAX_STEPS];
        for index in (0..self.length()).filter(|&index| self.steps[index].plays_arpeggio()) {
            let group = arpeggio_group_start(index);
            read[group..group + ARPEGGIO_GROUP].fill(true);
        }
        read
    }

    pub fn is_out_of_range(&self, transpose: i8) -> bool {
        self.out_of_range_steps(transpose).next().is_some()
    }

    pub fn is_silent(&self) -> bool {
        self.steps[..self.length()]
            .iter()
            .all(|step| step.is_silent())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PhraseId(u8);

impl PhraseId {
    pub const fn new(id: u8) -> Self {
        Self(id)
    }

    pub const fn number(self) -> u8 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChainId(u8);

impl ChainId {
    pub const fn new(id: u8) -> Self {
        Self(id)
    }

    pub const fn number(self) -> u8 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChainEntry {
    phrase: PhraseId,
    transpose: i8,
}

impl ChainEntry {
    pub const fn new(phrase: PhraseId) -> Self {
        Self {
            phrase,
            transpose: 0,
        }
    }

    pub const fn phrase(self) -> PhraseId {
        self.phrase
    }

    pub const fn transpose(self) -> i8 {
        self.transpose
    }

    pub fn set_phrase(&mut self, phrase: PhraseId) {
        self.phrase = phrase;
    }

    pub fn set_transpose(&mut self, semitones: i32) {
        self.transpose = semitones.clamp(MIN_TRANSPOSE, MAX_TRANSPOSE) as i8;
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Chain {
    entries: [Option<ChainEntry>; CHAIN_ENTRIES],
}

impl Chain {
    pub fn entry(&self, index: usize) -> Option<ChainEntry> {
        self.entries[index]
    }

    pub fn entry_mut(&mut self, index: usize) -> &mut Option<ChainEntry> {
        &mut self.entries[index]
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SfxInstrument {
    pub steps: Vec<Step>,
    pub speed: u8,
    pub filters: Filters,
    pub loop_start: u8,
    pub loop_end: u8,
}

impl Default for SfxInstrument {
    fn default() -> Self {
        Self {
            steps: vec![Step::default(); DEFAULT_LENGTH],
            speed: DEFAULT_SPEED,
            filters: Filters::default(),
            loop_start: 0,
            loop_end: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaveformInstrument {
    pub samples: [i8; WAVEFORM_SAMPLES],
    pub bass: bool,
    pub detune: u8,
    pub reverb: u8,
    pub dampen: u8,
}

impl Default for WaveformInstrument {
    fn default() -> Self {
        Self {
            samples: [0; WAVEFORM_SAMPLES],
            bass: false,
            detune: 0,
            reverb: 0,
            dampen: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstrumentSound {
    Sfx(SfxInstrument),
    Waveform(WaveformInstrument),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomInstrument {
    pub name: Option<String>,
    pub sound: InstrumentSound,
}

impl Default for CustomInstrument {
    fn default() -> Self {
        Self {
            name: None,
            sound: InstrumentSound::Sfx(SfxInstrument::default()),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Claim {
    pub sfx: Vec<u8>,
    pub first_and_last_pattern: Option<(u8, u8)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TargetCart {
    pub path: String,
    pub claim: Claim,
    pub claimed_bytes_hash: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SongPlace {
    pub song_row: usize,
    pub track: usize,
    pub chain_row: usize,
    pub step: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Project {
    phrases: Vec<Phrase>,
    chains: Vec<Chain>,
    song: Vec<[Option<ChainId>; TRACKS]>,
    song_marks: Vec<SongRowMarks>,
    default_speed: u8,
    target_cart: Option<TargetCart>,
    custom_instruments: BTreeMap<u8, CustomInstrument>,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            phrases: vec![Phrase::default(); ID_COUNT],
            chains: vec![Chain::default(); ID_COUNT],
            song: vec![[None; TRACKS]; SONG_ROWS],
            song_marks: vec![SongRowMarks::default(); SONG_ROWS],
            default_speed: DEFAULT_SPEED,
            target_cart: None,
            custom_instruments: BTreeMap::new(),
        }
    }
}

impl Project {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn phrase(&self, id: PhraseId) -> &Phrase {
        &self.phrases[usize::from(id.0)]
    }

    pub fn phrase_mut(&mut self, id: PhraseId) -> &mut Phrase {
        &mut self.phrases[usize::from(id.0)]
    }

    pub fn chain(&self, id: ChainId) -> &Chain {
        &self.chains[usize::from(id.0)]
    }

    pub fn chain_mut(&mut self, id: ChainId) -> &mut Chain {
        &mut self.chains[usize::from(id.0)]
    }

    pub fn song_cell(&self, row: usize, track: usize) -> Option<ChainId> {
        self.song.get(row)?[track]
    }

    pub fn set_song_cell(&mut self, row: usize, track: usize, chain: Option<ChainId>) {
        self.song[row][track] = chain;
    }

    pub fn chain_row_entry(
        &self,
        row: usize,
        track: usize,
        chain_row: usize,
    ) -> Option<ChainEntry> {
        self.chain(self.song_cell(row, track)?).entry(chain_row)
    }

    pub fn chain_row_ticks(&self, row: usize, chain_row: usize) -> u32 {
        let mut longest = 0;
        let mut longest_looping = 0;
        for track in 0..TRACKS {
            if let Some(entry) = self.chain_row_entry(row, track, chain_row) {
                let phrase = self.phrase(entry.phrase());
                let slot = if phrase.loops() {
                    &mut longest_looping
                } else {
                    &mut longest
                };
                *slot = phrase.ticks().max(*slot);
            }
        }
        if longest > 0 {
            longest
        } else {
            longest_looping
        }
    }

    pub fn song_row_is_empty(&self, row: usize) -> bool {
        self.played_chain_rows(row).next().is_none()
    }

    pub const fn default_speed(&self) -> u8 {
        self.default_speed
    }

    pub fn set_default_speed(&mut self, speed: i32) {
        self.default_speed = speed.clamp(MIN_SPEED, MAX_SPEED) as u8;
    }

    pub const fn target_cart(&self) -> Option<&TargetCart> {
        self.target_cart.as_ref()
    }

    pub fn set_target_cart(&mut self, target_cart: Option<TargetCart>) {
        self.target_cart = target_cart;
    }

    pub fn custom_instruments(&self) -> impl Iterator<Item = (u8, &CustomInstrument)> {
        self.custom_instruments
            .iter()
            .map(|(id, instrument)| (*id, instrument))
    }

    pub fn custom_instrument(&self, id: u8) -> Option<&CustomInstrument> {
        self.custom_instruments.get(&id)
    }

    pub fn set_custom_instrument(&mut self, id: u8, instrument: Option<CustomInstrument>) {
        if slot_of_custom_instrument(id).is_none() {
            return;
        }
        match instrument {
            Some(instrument) => self.custom_instruments.insert(id, instrument),
            None => self.custom_instruments.remove(&id),
        };
    }
}
