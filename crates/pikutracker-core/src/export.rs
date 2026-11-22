use crate::cart::{
    Cart, CartError, EDITOR_MODE_BIT, FILTERS_BYTE, LOOP_END_BYTE, LOOP_START_BYTE, SFX_BYTES,
    SLOTS, SPEED_BYTE, WAVEFORM_INSTRUMENT_BIT, note_word,
};
use crate::project::{
    CHAIN_ENTRIES, ChainEntry, Claim, CustomInstrument, EXPORT_CHANNELS, FIRST_CUSTOM_INSTRUMENT,
    Filters, InstrumentSound, MAX_PITCH, MAX_STEPS, Phrase, Project, SONG_ROWS, Step, TRACKS,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

const FULL_LENGTH: usize = 32;

const NOIZ_BIT: u8 = 0x02;
const BUZZ_BIT: u8 = 0x04;
const DETUNE_UNIT: u8 = 8;
const REVERB_UNIT: u8 = 24;
const DAMPEN_UNIT: u8 = 72;

pub const INSTRUMENT_SLOTS: u8 = 8;
const WAVEFORM_SPEED: u8 = 16;

const LOOP_START_CHANNEL: usize = 0;
const LOOP_BACK_CHANNEL: usize = 1;
const STOP_CHANNEL: usize = 2;
const FLAG: u8 = 0x80;
const FIRST_DISABLED_CHANNEL_BYTE: u8 = 0x41;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFailure {
    NoExportTracks,
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
    TooFewInstrumentSlots {
        needed: usize,
        claimed: usize,
    },
    UnreadableCart(CartError),
}

impl fmt::Display for ExportFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoExportTracks => write!(formatter, "NO EXPORT TRACKS"),
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
            Self::TooFewInstrumentSlots { needed, claimed } => write!(
                formatter,
                "NEEDS {needed} SFX IN 00-07 FOR CUSTOM INSTRUMENTS, THE CLAIM HAS {claimed}"
            ),
            Self::UnreadableCart(error) => write!(formatter, "CANNOT READ THE CART: {error}"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportWarning {
    UnexportedTrackSetsLength {
        song_row: usize,
        chain_row: usize,
        track: usize,
    },
}

impl fmt::Display for ExportWarning {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexportedTrackSetsLength {
                song_row,
                chain_row,
                track,
            } => write!(
                formatter,
                "SONG ROW {song_row:02X} CHAIN ROW {chain_row:X} TAKES ITS LENGTH FROM T{}, WHICH \
                 IS NOT EXPORTED, AND NO CHANNEL IS FREE TO HOLD IT",
                track + 1
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Exported {
    pub cart_bytes: Vec<u8>,
    pub claimed_bytes_hash: String,
    pub sfx_count: usize,
    pub pattern_count: usize,
    pub warnings: Vec<ExportWarning>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ending {
    Len,
    Loop { start: u8, end: u8 },
}

type CompiledPattern = [Option<usize>; EXPORT_CHANNELS];

#[derive(Default)]
struct CompiledSong {
    instruments: Vec<[u8; SFX_BYTES]>,
    sfx: Vec<[u8; SFX_BYTES]>,
    patterns: Vec<CompiledPattern>,
    loop_target: Option<usize>,
    warnings: Vec<ExportWarning>,
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
    instruments: Vec<u8>,
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
        let instruments = sfx
            .iter()
            .copied()
            .filter(|slot| *slot < INSTRUMENT_SLOTS)
            .collect();
        let patterns = claim
            .first_and_last_pattern
            .map_or(Vec::new(), |(first, last)| (first..=last).collect());
        Self {
            sfx,
            instruments,
            patterns,
        }
    }
}

pub fn export(
    project: &Project,
    claim: &Claim,
    original: &[u8],
) -> Result<Exported, Vec<ExportFailure>> {
    let mut failures = Vec::new();
    let has_tracks = project.export_track_numbers().iter().any(Option::is_some);
    if !has_tracks {
        failures.push(ExportFailure::NoExportTracks);
    }
    let claim = &without_repeats(claim);
    let claim_is_empty = claim.sfx.is_empty() && claim.first_and_last_pattern.is_none();
    if claim_is_empty {
        failures.push(ExportFailure::EmptyClaim);
    }
    let claim_is_valid = claim_is_valid(claim);
    if !claim_is_valid {
        failures.push(ExportFailure::BadClaim);
    }
    let chain_rows = chain_rows(project);
    if chain_rows.is_empty() {
        failures.push(ExportFailure::EmptySong);
    }
    let slots = ClaimedSlots::of(claim);
    let song = compile_song(
        project,
        &chain_rows,
        has_tracks,
        &slots.instruments,
        &mut failures,
    );
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
        warnings: song.warnings,
    })
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExportNeeds {
    pub sfx: usize,
    pub instrument_sfx: usize,
    pub patterns: usize,
}

pub fn needs(project: &Project) -> ExportNeeds {
    let has_tracks = project.export_track_numbers().iter().any(Option::is_some);
    let every_instrument_slot: Vec<u8> = (0..INSTRUMENT_SLOTS).collect();
    let song = compile_song(
        project,
        &chain_rows(project),
        has_tracks,
        &every_instrument_slot,
        &mut Vec::new(),
    );
    ExportNeeds {
        sfx: song.instruments.len() + song.sfx.len(),
        instrument_sfx: song.instruments.len(),
        patterns: song.patterns.len(),
    }
}

fn song_rows(project: &Project) -> Vec<usize> {
    let rows_from =
        |first: usize| (first..SONG_ROWS).take_while(|&row| !project.song_row_is_empty(row));
    let mut rows: Vec<usize> = rows_from(0).collect();
    let past_a_gap = project
        .song_loop_row()
        .filter(|row| !rows.is_empty() && !rows.contains(row));
    if let Some(loop_row) = past_a_gap {
        rows.extend(rows_from(loop_row));
    }
    rows
}

fn chain_rows(project: &Project) -> Vec<(usize, usize)> {
    song_rows(project)
        .into_iter()
        .flat_map(|row| (0..CHAIN_ENTRIES).map(move |k| (row, k)))
        .filter(|&(row, k)| project.chain_row_ticks(row, k) > 0)
        .collect()
}

fn compile_song(
    project: &Project,
    chain_rows: &[(usize, usize)],
    has_tracks: bool,
    instrument_slots: &[u8],
    failures: &mut Vec<ExportFailure>,
) -> CompiledSong {
    let rows: Vec<ExportedChainRow> = chain_rows
        .iter()
        .map(|&(song_row, chain_row)| ExportedChainRow::of(project, song_row, chain_row))
        .collect();
    let instruments_played = rows
        .iter()
        .flat_map(|row| row.channels.iter().flatten())
        .flat_map(|(_, entry)| custom_instruments_played(project.phrase(entry.phrase())))
        .collect::<BTreeSet<u8>>();
    let slot_of_instrument: BTreeMap<u8, u8> = instruments_played
        .iter()
        .copied()
        .zip(instrument_slots.iter().copied())
        .collect();
    let mut song = CompiledSong {
        instruments: instruments_played
            .iter()
            .map(|id| instrument_sfx(project.custom_instrument(*id)))
            .collect(),
        ..CompiledSong::default()
    };
    for (row, &(song_row, chain_row)) in rows.iter().zip(chain_rows) {
        let mut pattern = [None; EXPORT_CHANNELS];
        for (channel, played) in row.channels.iter().enumerate() {
            let Some((track, entry)) = *played else {
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
            let ending = row.ending(project, channel);
            let sfx = bake_sfx(phrase, entry.transpose(), ending, &slot_of_instrument);
            pattern[channel] = Some(song.sfx_index(sfx));
        }
        match row.length {
            RowLength::PaddingOnChannel(channel) if has_tracks => {
                pattern[channel] = Some(song.sfx_index(silent_sfx(row.ticks)));
            }
            RowLength::UnexportedTrack(track) if has_tracks => {
                song.warnings
                    .push(ExportWarning::UnexportedTrackSetsLength {
                        song_row,
                        chain_row,
                        track,
                    });
            }
            _ => {}
        }
        song.patterns.push(pattern);
    }
    song.loop_target = project
        .song_loop_row()
        .and_then(|loop_row| chain_rows.iter().position(|(row, _)| *row == loop_row));
    song
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RowLength {
    EntryOnChannel(usize),
    PaddingOnChannel(usize),
    UnexportedTrack(usize),
}

impl RowLength {
    fn channel(self) -> Option<usize> {
        match self {
            Self::EntryOnChannel(channel) | Self::PaddingOnChannel(channel) => Some(channel),
            Self::UnexportedTrack(_) => None,
        }
    }
}

struct ExportedChainRow {
    channels: [Option<(usize, ChainEntry)>; EXPORT_CHANNELS],
    ticks: u32,
    length: RowLength,
}

impl ExportedChainRow {
    fn of(project: &Project, song_row: usize, chain_row: usize) -> Self {
        let channels = project.export_track_indices().map(|track| {
            let track = track?;
            Some((track, project.chain_row_entry(song_row, track, chain_row)?))
        });
        let ticks = project.chain_row_ticks(song_row, chain_row);
        let entries = || {
            (0..TRACKS).filter_map(|track| {
                let entry = project.chain_row_entry(song_row, track, chain_row)?;
                Some((track, entry))
            })
        };
        let loops = |entry: ChainEntry| project.phrase(entry.phrase()).loops();
        let all_loop = entries().all(|(_, entry)| loops(entry));
        let sets_length = |entry: ChainEntry| {
            (all_loop || !loops(entry)) && project.phrase(entry.phrase()).ticks() == ticks
        };
        let entry_channel = channels
            .iter()
            .position(|played| played.is_some_and(|(_, entry)| sets_length(entry)));
        let free_channel = channels.iter().position(Option::is_none);
        let length = match (entry_channel, free_channel) {
            (Some(channel), _) => RowLength::EntryOnChannel(channel),
            (None, Some(channel)) => RowLength::PaddingOnChannel(channel),
            (None, None) => RowLength::UnexportedTrack(
                entries()
                    .find(|(_, entry)| sets_length(*entry))
                    .map(|(track, _)| track)
                    .expect("a Chain row that plays has an entry that sets its length"),
            ),
        };
        Self {
            channels,
            ticks,
            length,
        }
    }

    fn ending(&self, project: &Project, channel: usize) -> Ending {
        let Some((_, entry)) = self.channels[channel] else {
            return Ending::Len;
        };
        let phrase = project.phrase(entry.phrase());
        let length = phrase.length() as u8;
        let length_channel = self.length.channel();
        let is_left_of_length =
            length_channel.is_some_and(|length_channel| channel < length_channel);
        if length_channel == Some(channel) {
            Ending::Len
        } else if phrase.loops() {
            Ending::Loop {
                start: phrase.loop_start(),
                end: phrase.loop_end(),
            }
        } else if is_left_of_length {
            Ending::Loop {
                start: length,
                end: length + 1,
            }
        } else {
            Ending::Len
        }
    }
}

fn custom_instruments_played(phrase: &Phrase) -> impl Iterator<Item = u8> + '_ {
    let under_reverb = phrase.filters().reverb() > 0;
    (0..phrase.length())
        .map(|index| phrase.step(index))
        .filter(move |step| {
            (under_reverb || !step.is_silent()) && step.instrument() >= FIRST_CUSTOM_INSTRUMENT
        })
        .map(Step::instrument)
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

fn bake_sfx(
    phrase: &Phrase,
    transpose: i8,
    ending: Ending,
    slot_of_instrument: &BTreeMap<u8, u8>,
) -> [u8; SFX_BYTES] {
    let mut sfx = [0; SFX_BYTES];
    let read = phrase.steps_read_by_arpeggios();
    for (index, read) in read.into_iter().enumerate() {
        let step = phrase.step(index);
        let pitch = (i32::from(step.pitch()) + i32::from(transpose)).clamp(0, MAX_PITCH) as u16;
        let word = if index < phrase.length() {
            let (instrument, custom) = slot_of_instrument
                .get(&step.instrument())
                .map_or((step.instrument() & 7, 0), |slot| (*slot, 1));
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

fn silent_sfx(ticks: u32) -> [u8; SFX_BYTES] {
    let notes = (1..=FULL_LENGTH as u32)
        .find(|notes| ticks.is_multiple_of(*notes) && ticks / notes <= u32::from(u8::MAX))
        .unwrap_or(FULL_LENGTH as u32);
    let mut sfx = [0; SFX_BYTES];
    sfx[FILTERS_BYTE] = EDITOR_MODE_BIT;
    sfx[SPEED_BYTE] = (ticks / notes) as u8;
    if notes < FULL_LENGTH as u32 {
        sfx[LOOP_START_BYTE] = notes as u8;
    }
    sfx
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
    let (sfx_slots, instrument_slots, pattern_slots) =
        (&slots.sfx, &slots.instruments, &slots.patterns);
    if song.instruments.len() > instrument_slots.len() {
        failures.push(ExportFailure::TooFewInstrumentSlots {
            needed: song.instruments.len(),
            claimed: instrument_slots.len(),
        });
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
    let taken = &slots.instruments[..song.instruments.len()];
    let phrase_slots: Vec<u8> = slots
        .sfx
        .iter()
        .copied()
        .filter(|slot| !taken.contains(slot))
        .collect();
    for (&slot, sfx) in taken.iter().zip(&song.instruments) {
        cart.set_sfx_bytes(slot, *sfx);
    }
    for (index, &slot) in phrase_slots.iter().enumerate() {
        let sfx = song.sfx.get(index).unwrap_or(blank.sfx_bytes(slot));
        cart.set_sfx_bytes(slot, *sfx);
    }
    for (index, &slot) in slots.patterns.iter().enumerate() {
        let bytes = song.patterns.get(index).map_or_else(
            || blank.pattern_bytes(slot),
            |pattern| pattern_bytes(song, index, pattern, &phrase_slots),
        );
        cart.set_pattern_bytes(slot, bytes);
    }
}

fn pattern_bytes(
    song: &CompiledSong,
    index: usize,
    pattern: &CompiledPattern,
    sfx_slots: &[u8],
) -> [u8; EXPORT_CHANNELS] {
    let mut bytes = [0; EXPORT_CHANNELS];
    for (channel, byte) in bytes.iter_mut().enumerate() {
        *byte = pattern[channel].map_or(disabled_channel_byte(channel), |sfx| sfx_slots[sfx]);
    }
    if song.loop_target == Some(index) {
        bytes[LOOP_START_CHANNEL] |= FLAG;
    }
    if index + 1 == song.patterns.len() {
        let ending = if song.loop_target.is_some() {
            LOOP_BACK_CHANNEL
        } else {
            STOP_CHANNEL
        };
        bytes[ending] |= FLAG;
    }
    bytes
}

const fn disabled_channel_byte(channel: usize) -> u8 {
    FIRST_DISABLED_CHANNEL_BYTE + channel as u8
}
