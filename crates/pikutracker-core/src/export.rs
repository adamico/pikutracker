use crate::cart::{
    Cart, CartError, EDITOR_MODE_BIT, FILTERS_BYTE, LOOP_END_BYTE, LOOP_START_BYTE, SFX_BYTES,
    SLOTS, SPEED_BYTE, WAVEFORM_INSTRUMENT_BIT, note_word,
};
use crate::project::{
    ChainEntry, Claim, CustomInstrument, Filters, InstrumentSound, MAX_PITCH, MAX_STEPS, Phrase,
    Project, SongRowEnding, SongRowMarks, TRACKS, custom_instrument_of_slot,
    slot_of_custom_instrument,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

const FULL_LENGTH: usize = 32;

const NOIZ_BIT: u8 = 0x02;
const BUZZ_BIT: u8 = 0x04;
const DETUNE_UNIT: u8 = 8;
const REVERB_UNIT: u8 = 24;
const DAMPEN_UNIT: u8 = 72;

const WAVEFORM_SPEED: u8 = 16;

const LOOP_START_CHANNEL: usize = 0;
const LOOP_BACK_CHANNEL: usize = 1;
const STOP_CHANNEL: usize = 2;
const FLAG: u8 = 0x80;
const FIRST_DISABLED_CHANNEL_BYTE: u8 = 0x41;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFailure {
    EmptyClaim,
    BadClaim,
    EmptySong,
    TooFewSfx {
        needed: usize,
        claimed: usize,
    },
    TooFewPatterns {
        needed: usize,
        claimed: usize,
    },
    NoteOutOfRange {
        phrase: u8,
        step: usize,
        track: usize,
        song_row: usize,
        chain_row: usize,
    },
    MissingInstrumentSlot {
        instrument: u8,
        slot: u8,
    },
    UnreadableCart(CartError),
}

impl fmt::Display for ExportFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyClaim => write!(
                formatter,
                "THIS PROJECT CLAIMS NO SFX OR PATTERNS, SO THERE IS NOWHERE TO WRITE"
            ),
            Self::BadClaim => write!(formatter, "THE CLAIM NAMES SLOTS A CART DOES NOT HAVE"),
            Self::EmptySong => write!(formatter, "THE SONG IS EMPTY"),
            Self::TooFewSfx { needed, claimed } => {
                write!(formatter, "NEEDS {needed} SFX, THE CLAIM HAS {claimed}")
            }
            Self::TooFewPatterns { needed, claimed } => {
                write!(
                    formatter,
                    "NEEDS {needed} PATTERNS, THE CLAIM HAS {claimed}"
                )
            }
            Self::NoteOutOfRange {
                phrase,
                step,
                track,
                ..
            } => write!(
                formatter,
                "PHRASE {phrase:02X} STEP {step:02X} ON T{} IS OUTSIDE C-0 TO D#5",
                track + 1
            ),
            Self::MissingInstrumentSlot { instrument, slot } => write!(
                formatter,
                "INSTRUMENT {instrument:X} NEEDS SFX {slot:02X}, WHICH THE CLAIM LACKS"
            ),
            Self::UnreadableCart(error) => write!(formatter, "CANNOT READ THE CART: {error}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Exported {
    pub cart_bytes: Vec<u8>,
    pub claimed_bytes_hash: String,
    pub sfx_count: usize,
    pub pattern_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ending {
    Len,
    Loop { start: u8, end: u8 },
}

type CompiledPattern = [Option<usize>; TRACKS];

#[derive(Default)]
struct CompiledSong {
    instruments: BTreeMap<u8, [u8; SFX_BYTES]>,
    sfx: Vec<[u8; SFX_BYTES]>,
    patterns: Vec<CompiledPattern>,
    marks: Vec<SongRowMarks>,
    loops_back_to_its_start: bool,
}

impl CompiledSong {
    fn sfx_index(&mut self, sfx: [u8; SFX_BYTES]) -> usize {
        self.sfx
            .iter()
            .position(|known| *known == sfx)
            .unwrap_or_else(|| {
                self.sfx.push(sfx);
                self.sfx.len() - 1
            })
    }
}

fn claim_is_valid(claim: &Claim) -> bool {
    let exists = |slot: u8| usize::from(slot) < SLOTS;
    claim.sfx.iter().all(|slot| exists(*slot))
        && claim
            .first_and_last_pattern
            .is_none_or(|(first, last)| first <= last && exists(last))
}

fn without_repeats(claim: &Claim) -> Claim {
    let mut seen = BTreeSet::new();
    Claim {
        sfx: claim
            .sfx
            .iter()
            .copied()
            .filter(|slot| seen.insert(*slot))
            .collect(),
        first_and_last_pattern: claim.first_and_last_pattern,
    }
}

struct ClaimedSlots {
    sfx: Vec<u8>,
    patterns: Vec<u8>,
}

impl ClaimedSlots {
    fn of(claim: &Claim) -> Self {
        let sfx: Vec<u8> = claim
            .sfx
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let patterns = claim
            .first_and_last_pattern
            .map_or(Vec::new(), |(first, last)| (first..=last).collect());
        Self { sfx, patterns }
    }
}

pub fn export(
    project: &Project,
    claim: &Claim,
    original: &[u8],
) -> Result<Exported, Vec<ExportFailure>> {
    let mut failures = Vec::new();
    let claim = &without_repeats(claim);
    let claim_is_empty = claim.sfx.is_empty() && claim.first_and_last_pattern.is_none();
    if claim_is_empty {
        failures.push(ExportFailure::EmptyClaim);
    }
    let claim_is_valid = claim_is_valid(claim);
    if !claim_is_valid {
        failures.push(ExportFailure::BadClaim);
    }
    let song_patterns = song_patterns(project);
    if song_patterns.is_empty() {
        failures.push(ExportFailure::EmptySong);
    }
    let slots = ClaimedSlots::of(claim);
    let song = compile_song(project, &song_patterns, &mut failures);
    if !claim_is_empty && claim_is_valid {
        check_room(&song, &slots, &mut failures);
    }

    let cart = Cart::read(original);
    if let Err(error) = cart {
        failures.push(ExportFailure::UnreadableCart(error));
    }
    let (Ok(mut cart), true) = (cart, failures.is_empty()) else {
        return Err(failures);
    };

    place_song(&song, &slots, &mut cart);
    let cart_bytes = cart
        .write_claimed(original, claim)
        .map_err(|error| vec![ExportFailure::UnreadableCart(error)])?;
    Ok(Exported {
        cart_bytes,
        claimed_bytes_hash: cart.claimed_bytes_hash(claim),
        sfx_count: song.instruments.len() + song.sfx.len(),
        pattern_count: song.patterns.len(),
    })
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExportNeeds {
    pub sfx: usize,
    pub instrument_slots: Vec<u8>,
    pub patterns: usize,
}

pub fn needs(project: &Project) -> ExportNeeds {
    let song = compile_song(project, &song_patterns(project), &mut Vec::new());
    ExportNeeds {
        sfx: song.instruments.len() + song.sfx.len(),
        instrument_slots: song.instruments.keys().copied().collect(),
        patterns: song.patterns.len(),
    }
}

#[derive(Clone, Copy)]
struct SongPattern {
    song_row: usize,
    chain_row: Option<usize>,
    marks: SongRowMarks,
}

fn song_patterns(project: &Project) -> Vec<SongPattern> {
    let Some(last_row) = project.last_played_song_row() else {
        return Vec::new();
    };
    (0..=last_row)
        .flat_map(|song_row| song_row_patterns(project, song_row))
        .collect()
}

fn song_row_patterns(project: &Project, song_row: usize) -> Vec<SongPattern> {
    let mut chain_rows: Vec<Option<usize>> =
        project.played_chain_rows(song_row).map(Some).collect();
    if chain_rows.is_empty() {
        chain_rows.push(None);
    }
    let marks = project.song_row_marks(song_row);
    let last = chain_rows.len() - 1;
    chain_rows
        .into_iter()
        .enumerate()
        .map(|(index, chain_row)| SongPattern {
            song_row,
            chain_row,
            marks: SongRowMarks {
                loop_start: marks.loop_start && index == 0,
                ending: marks.ending.filter(|_| index == last),
            },
        })
        .collect()
}

fn compile_song(
    project: &Project,
    song_patterns: &[SongPattern],
    failures: &mut Vec<ExportFailure>,
) -> CompiledSong {
    let rows: Vec<Option<ExportedChainRow>> = song_patterns
        .iter()
        .map(|place| {
            let chain_row = place.chain_row?;
            Some(ExportedChainRow::of(project, place.song_row, chain_row))
        })
        .collect();
    let mut song = CompiledSong {
        instruments: rows
            .iter()
            .flatten()
            .flat_map(|row| row.channels.iter().flatten())
            .flat_map(|entry| custom_instrument_slots_played(project.phrase(entry.phrase())))
            .map(|slot| {
                let instrument = project.custom_instrument(custom_instrument_of_slot(slot));
                (slot, instrument_sfx(instrument))
            })
            .collect(),
        ..CompiledSong::default()
    };
    for (row, place) in rows.iter().zip(song_patterns) {
        let mut pattern = [None; TRACKS];
        song.marks.push(place.marks);
        let (Some(row), Some(chain_row)) = (row, place.chain_row) else {
            song.patterns.push(pattern);
            continue;
        };
        let song_row = place.song_row;
        for (track, played) in row.channels.iter().enumerate() {
            let Some(entry) = *played else {
                continue;
            };
            let phrase = project.phrase(entry.phrase());
            for step in phrase.out_of_range_steps(entry.transpose()) {
                failures.push(ExportFailure::NoteOutOfRange {
                    phrase: entry.phrase().number(),
                    step,
                    track,
                    song_row,
                    chain_row,
                });
            }
            let ending = row.ending(project, track);
            let sfx = bake_sfx(phrase, entry.transpose(), ending);
            pattern[track] = Some(song.sfx_index(sfx));
        }
        song.patterns.push(pattern);
    }
    song.loops_back_to_its_start = song_patterns.iter().any(|place| {
        place.marks.loops_back() && project.loop_start_at_or_before(place.song_row).is_none()
    });
    song
}

struct ExportedChainRow {
    channels: [Option<ChainEntry>; TRACKS],
    length_channel: usize,
}

impl ExportedChainRow {
    fn of(project: &Project, song_row: usize, chain_row: usize) -> Self {
        let channels =
            std::array::from_fn(|track| project.chain_row_entry(song_row, track, chain_row));
        let ticks = project.chain_row_ticks(song_row, chain_row);
        let loops = |entry: &ChainEntry| project.phrase(entry.phrase()).loops();
        let all_loop = channels.iter().flatten().all(loops);
        let length_channel = channels
            .iter()
            .position(|played| {
                played.is_some_and(|entry| {
                    (all_loop || !loops(&entry)) && project.phrase(entry.phrase()).ticks() == ticks
                })
            })
            .expect("a Chain row that plays has an entry that sets its length");
        Self {
            channels,
            length_channel,
        }
    }

    fn ending(&self, project: &Project, channel: usize) -> Ending {
        let Some(entry) = self.channels[channel] else {
            return Ending::Len;
        };
        let phrase = project.phrase(entry.phrase());
        let length = phrase.length() as u8;
        if self.length_channel == channel {
            Ending::Len
        } else if phrase.loops() {
            Ending::Loop {
                start: phrase.loop_start(),
                end: phrase.loop_end(),
            }
        } else if channel < self.length_channel {
            Ending::Loop {
                start: length,
                end: length + 1,
            }
        } else {
            Ending::Len
        }
    }
}

fn custom_instrument_slots_played(phrase: &Phrase) -> impl Iterator<Item = u8> + '_ {
    let under_reverb = phrase.filters().reverb() > 0;
    (0..phrase.length())
        .map(|index| phrase.step(index))
        .filter(move |step| under_reverb || !step.is_silent())
        .filter_map(|step| slot_of_custom_instrument(step.instrument()))
}

fn instrument_sfx(instrument: Option<&CustomInstrument>) -> [u8; SFX_BYTES] {
    let mut sfx = *Cart::default().sfx_bytes(0);
    match instrument.map(|instrument| &instrument.sound) {
        None => {}
        Some(InstrumentSound::Sfx(instrument)) => {
            for (index, step) in instrument.steps.iter().take(MAX_STEPS).enumerate() {
                let word = note_word(
                    u16::from(step.pitch()),
                    u16::from(step.instrument() & 7),
                    u16::from(step.volume()),
                    u16::from(step.effect()),
                    0,
                );
                set_note(&mut sfx, index, word);
            }
            sfx[FILTERS_BYTE] = filters_byte(instrument.filters);
            sfx[SPEED_BYTE] = instrument.speed;
            if instrument.loop_start < instrument.loop_end {
                sfx[LOOP_START_BYTE] = instrument.loop_start;
                sfx[LOOP_END_BYTE] = instrument.loop_end;
            }
        }
        Some(InstrumentSound::Waveform(wave)) => {
            for (byte, sample) in sfx.iter_mut().zip(wave.samples) {
                *byte = sample as u8;
            }
            let mut filters = Filters::default();
            filters.set_detune(i32::from(wave.detune));
            filters.set_reverb(i32::from(wave.reverb));
            filters.set_dampen(i32::from(wave.dampen));
            sfx[FILTERS_BYTE] = filters_byte(filters);
            sfx[SPEED_BYTE] = WAVEFORM_SPEED + u8::from(wave.bass);
            sfx[LOOP_START_BYTE] = WAVEFORM_INSTRUMENT_BIT;
            sfx[LOOP_END_BYTE] = 0;
        }
    }
    sfx
}

fn bake_sfx(phrase: &Phrase, transpose: i8, ending: Ending) -> [u8; SFX_BYTES] {
    let mut sfx = [0; SFX_BYTES];
    let read = phrase.steps_read_by_arpeggios();
    for (index, read) in read.into_iter().enumerate() {
        let step = phrase.step(index);
        let pitch = (i32::from(step.pitch()) + i32::from(transpose)).clamp(0, MAX_PITCH) as u16;
        let word = if index < phrase.length() {
            let (instrument, custom) = slot_of_custom_instrument(step.instrument())
                .map_or((step.instrument(), 0), |slot| (slot, 1));
            note_word(
                pitch,
                u16::from(instrument),
                u16::from(step.volume()),
                u16::from(step.effect()),
                custom,
            )
        } else if read {
            pitch
        } else {
            0
        };
        set_note(&mut sfx, index, word);
    }
    sfx[FILTERS_BYTE] = filters_byte(phrase.filters());
    sfx[SPEED_BYTE] = phrase.speed();
    match ending {
        Ending::Len if !phrase.loops() => {
            sfx[LOOP_START_BYTE] = phrase.loop_start();
            sfx[LOOP_END_BYTE] = phrase.loop_end();
        }
        Ending::Len if phrase.length() < FULL_LENGTH => {
            sfx[LOOP_START_BYTE] = phrase.length() as u8;
        }
        Ending::Len => {}
        Ending::Loop { start, end } => {
            sfx[LOOP_START_BYTE] = start;
            sfx[LOOP_END_BYTE] = end;
        }
    }
    sfx
}

fn set_note(sfx: &mut [u8; SFX_BYTES], index: usize, word: u16) {
    sfx[index * 2..index * 2 + 2].copy_from_slice(&word.to_le_bytes());
}

fn filters_byte(filters: Filters) -> u8 {
    let switch = |on: bool, bit: u8| if on { bit } else { 0 };
    EDITOR_MODE_BIT
        + switch(filters.noiz(), NOIZ_BIT)
        + switch(filters.buzz(), BUZZ_BIT)
        + filters.detune() * DETUNE_UNIT
        + filters.reverb() * REVERB_UNIT
        + filters.dampen() * DAMPEN_UNIT
}

fn check_room(song: &CompiledSong, slots: &ClaimedSlots, failures: &mut Vec<ExportFailure>) {
    let (sfx_slots, pattern_slots) = (&slots.sfx, &slots.patterns);
    for &slot in song.instruments.keys() {
        if !sfx_slots.contains(&slot) {
            failures.push(ExportFailure::MissingInstrumentSlot {
                instrument: custom_instrument_of_slot(slot),
                slot,
            });
        }
    }
    let needed = song.instruments.len() + song.sfx.len();
    if needed > sfx_slots.len() {
        failures.push(ExportFailure::TooFewSfx {
            needed,
            claimed: sfx_slots.len(),
        });
    }
    if song.patterns.len() > pattern_slots.len() {
        failures.push(ExportFailure::TooFewPatterns {
            needed: song.patterns.len(),
            claimed: pattern_slots.len(),
        });
    }
}

fn place_song(song: &CompiledSong, slots: &ClaimedSlots, cart: &mut Cart) {
    let blank = Cart::default();
    let phrase_slots: Vec<u8> = slots
        .sfx
        .iter()
        .copied()
        .filter(|slot| !song.instruments.contains_key(slot))
        .collect();
    for (&slot, sfx) in &song.instruments {
        cart.set_sfx_bytes(slot, *sfx);
    }
    for (index, &slot) in phrase_slots.iter().enumerate() {
        let sfx = song.sfx.get(index).unwrap_or(blank.sfx_bytes(slot));
        cart.set_sfx_bytes(slot, *sfx);
    }
    for (index, &slot) in slots.patterns.iter().enumerate() {
        let bytes = song.patterns.get(index).map_or_else(
            || blank.pattern_bytes(slot),
            |pattern| pattern_bytes(song, index, slot, pattern, &phrase_slots),
        );
        cart.set_pattern_bytes(slot, bytes);
    }
}

fn pattern_bytes(
    song: &CompiledSong,
    index: usize,
    slot: u8,
    pattern: &CompiledPattern,
    sfx_slots: &[u8],
) -> [u8; TRACKS] {
    let mut bytes = [0; TRACKS];
    for (channel, byte) in bytes.iter_mut().enumerate() {
        *byte = pattern[channel].map_or(disabled_channel_byte(channel), |sfx| sfx_slots[sfx]);
    }
    let marks = song.marks[index];
    let starts_the_song_off_pattern_0 = index == 0 && slot != 0 && song.loops_back_to_its_start;
    if marks.loop_start || starts_the_song_off_pattern_0 {
        bytes[LOOP_START_CHANNEL] |= FLAG;
    }
    match marks.ending {
        Some(SongRowEnding::LoopBack) => bytes[LOOP_BACK_CHANNEL] |= FLAG,
        Some(SongRowEnding::Stop) => bytes[STOP_CHANNEL] |= FLAG,
        None => {}
    }
    bytes
}

const fn disabled_channel_byte(channel: usize) -> u8 {
    FIRST_DISABLED_CHANNEL_BYTE + channel as u8
}
