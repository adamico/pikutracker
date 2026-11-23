use crate::cart::{Cart, CartNote, Pattern, SFX_NOTES, Sfx, WAVEFORM_INSTRUMENT_BIT};
use crate::project::{
    ARPEGGIO_GROUP, Filters, MAX_STEPS, Phrase, Step, TRACKS, arpeggio_group_start,
    custom_instrument_of_slot,
};
use std::collections::BTreeSet;

const NOIZ_BIT: u8 = 0x02;
const BUZZ_BIT: u8 = 0x04;
const DETUNE_DIVISOR: u8 = 8;
const REVERB_DIVISOR: u8 = 24;
const DAMPEN_DIVISOR: u8 = 72;
const FILTER_LEVELS: u8 = 3;

pub(super) struct RebuiltEntry {
    pub(super) phrase: Phrase,
    pub(super) instruments: BTreeSet<u8>,
    pub(super) arpeggio_changed: Option<usize>,
}

pub(super) struct RebuiltChannel {
    pub(super) entry: Option<RebuiltEntry>,
    pub(super) lost_step: bool,
    pub(super) early_ticks: u32,
}

pub(super) struct RebuiltPattern {
    pub(super) chain_rows: Vec<[Option<RebuiltChannel>; TRACKS]>,
    pub(super) only_bit_7: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Playback {
    Once,
    Loop { start: usize, end: usize },
    Bit7,
}

impl Playback {
    fn of(sfx: Sfx<'_>) -> Self {
        let (start, end) = (sfx.loop_start(), sfx.loop_end());
        if has_bit_7(sfx) {
            Self::Bit7
        } else if start < end {
            Self::Loop {
                start: usize::from(start),
                end: usize::from(end),
            }
        } else {
            Self::Once
        }
    }

    fn note_at(self, step: usize) -> Option<usize> {
        let note = match self {
            Self::Loop { start, end } if step >= end => start + (step - start) % (end - start),
            _ => step,
        };
        (note < SFX_NOTES).then_some(note)
    }
}

pub(super) const fn has_bit_7(sfx: Sfx<'_>) -> bool {
    sfx.loop_start() & WAVEFORM_INSTRUMENT_BIT != 0
}

pub(super) fn speed_of(sfx: Sfx<'_>) -> u32 {
    u32::from(sfx.speed().max(1))
}

fn notes_when_setting_length(sfx: Sfx<'_>) -> u32 {
    if sfx.loop_end() == 0 && sfx.loop_start() > 0 {
        u32::from(sfx.loop_start())
    } else {
        SFX_NOTES as u32
    }
}

pub(super) fn filters_of(sfx: Sfx<'_>) -> Filters {
    let byte = sfx.filters();
    let level = |divisor: u8| i32::from(byte / divisor % FILTER_LEVELS);
    let mut filters = Filters::default();
    filters.set_noiz(byte & NOIZ_BIT != 0);
    filters.set_buzz(byte & BUZZ_BIT != 0);
    filters.set_detune(level(DETUNE_DIVISOR));
    filters.set_reverb(level(REVERB_DIVISOR));
    filters.set_dampen(level(DAMPEN_DIVISOR));
    filters
}

fn carry_arpeggio_pitches(
    sfx: Sfx<'_>,
    played: &[Option<usize>],
    phrase: &mut Phrase,
) -> Option<usize> {
    let mut pitches = [None; MAX_STEPS];
    for (index, note) in played.iter().enumerate() {
        pitches[index] = note.map(|note| sfx.note(note).pitch);
    }
    let mut changed = None;
    for (index, note) in played.iter().enumerate() {
        let Some(note) = *note else { continue };
        if !phrase.step(index).plays_arpeggio() {
            continue;
        }
        let wanted: [u8; ARPEGGIO_GROUP] =
            std::array::from_fn(|offset| sfx.note(arpeggio_group_start(note) + offset).pitch);
        let mut keeps_sound = index == note || wanted.iter().all(|pitch| *pitch == wanted[0]);
        for (offset, pitch) in wanted.into_iter().enumerate() {
            let slot = &mut pitches[arpeggio_group_start(index) + offset];
            keeps_sound &= *slot.get_or_insert(pitch) == pitch;
        }
        if !keeps_sound {
            changed.get_or_insert(index);
        }
    }
    for (index, pitch) in pitches.into_iter().enumerate() {
        let plays_a_note = played.get(index).is_some_and(Option::is_some);
        if let Some(pitch) = pitch.filter(|_| !plays_a_note) {
            *phrase.step_mut(index) = Step::new(pitch, 0, 0, 0).unwrap_or_default();
        }
    }
    changed
}

pub(super) fn built_in_step(note: CartNote) -> Step {
    Step::new(note.pitch, note.instrument, note.volume, note.effect).unwrap_or_default()
}

fn step_of(note: CartNote) -> Step {
    let mut step = built_in_step(note);
    if note.custom {
        step.set_instrument(i32::from(custom_instrument_of_slot(note.instrument)));
    }
    step
}

pub(super) fn stored_length(sfx: Sfx<'_>) -> usize {
    (notes_when_setting_length(sfx) as usize).min(SFX_NOTES)
}

fn entry_of(sfx: Sfx<'_>, notes: &[Option<usize>]) -> RebuiltEntry {
    let mut phrase = Phrase::default();
    let mut instruments = BTreeSet::new();
    for (index, played) in notes.iter().enumerate() {
        let Some(played) = *played else { continue };
        let note = sfx.note(played);
        *phrase.step_mut(index) = step_of(note);
        if note.custom && note.volume > 0 {
            instruments.insert(note.instrument);
        }
    }
    phrase.set_length(notes.len() as i32);
    phrase.set_speed(speed_of(sfx) as i32);
    *phrase.filters_mut() = filters_of(sfx);
    let arpeggio_changed = carry_arpeggio_pitches(sfx, notes, &mut phrase);
    RebuiltEntry {
        phrase,
        instruments,
        arpeggio_changed,
    }
}

fn every_note() -> Vec<Option<usize>> {
    (0..SFX_NOTES).map(Some).collect()
}

pub(super) fn loose_phrase(sfx: Sfx<'_>) -> Option<(Phrase, BTreeSet<u8>)> {
    let entry = if has_bit_7(sfx) {
        let notes: Vec<Option<usize>> = (0..stored_length(sfx)).map(Some).collect();
        entry_of(sfx, &notes)
    } else {
        let mut entry = entry_of(sfx, &every_note());
        entry
            .phrase
            .set_loop(i32::from(sfx.loop_start()), i32::from(sfx.loop_end()));
        entry
    };
    (!entry.phrase.is_silent()).then_some((entry.phrase, entry.instruments))
}

fn loop_from(sfx: Sfx<'_>, first: usize, start: usize, end: usize) -> RebuiltEntry {
    if first == 0 {
        let mut entry = entry_of(sfx, &every_note());
        entry.phrase.set_loop(start as i32, end as i32);
        return entry;
    }
    if first < start {
        let notes: Vec<Option<usize>> = (first..SFX_NOTES).map(Some).collect();
        let mut entry = entry_of(sfx, &notes);
        entry
            .phrase
            .set_loop((start - first) as i32, (end - first) as i32);
        return entry;
    }
    let body = end - start;
    let offset = (first - start) % body;
    let notes: Vec<Option<usize>> = (0..body)
        .map(|index| Some(start + (offset + index) % body))
        .collect();
    let mut entry = entry_of(sfx, &notes);
    entry.phrase.set_loop(0, body as i32);
    entry
}

fn rebuild_channel(sfx: Sfx<'_>, from: u32, to: u32, sets_length: bool) -> RebuiltChannel {
    let playback = Playback::of(sfx);
    let speed = speed_of(sfx);
    let first = from.div_ceil(speed) as usize;
    let early_ticks = first as u32 * speed - from;
    if let Playback::Loop { start, end } = playback
        && (!sets_length || end == SFX_NOTES)
        && end <= SFX_NOTES
    {
        return RebuiltChannel {
            entry: Some(loop_from(sfx, first, start, end)),
            lost_step: false,
            early_ticks,
        };
    }
    let past = (to / speed) as usize;
    let needed = past.saturating_sub(first);
    let steps = needed.min(MAX_STEPS);
    let notes: Vec<Option<usize>> = (first..first + steps)
        .map(|step| playback.note_at(step))
        .collect();
    RebuiltChannel {
        entry: (steps > 0).then(|| entry_of(sfx, &notes)),
        lost_step: needed <= MAX_STEPS
            && !to.is_multiple_of(speed)
            && playback.note_at(past).is_some(),
        early_ticks,
    }
}

fn timing_of(sfx: &[Option<Sfx<'_>>; TRACKS]) -> Option<(usize, u32)> {
    let playing = || (0..TRACKS).filter_map(|c| sfx[c].map(|sfx| (c, sfx)));
    if let Some((channel, sfx)) = playing().find(|(_, sfx)| Playback::of(*sfx) == Playback::Once) {
        return Some((channel, notes_when_setting_length(sfx) * speed_of(sfx)));
    }
    let (channel, sfx) = left_most_slowest(playing().filter(|(_, sfx)| !has_bit_7(*sfx)))
        .or_else(|| left_most_slowest(playing()))?;
    Some((channel, SFX_NOTES as u32 * speed_of(sfx)))
}

fn left_most_slowest<'a>(
    channels: impl DoubleEndedIterator<Item = (usize, Sfx<'a>)>,
) -> Option<(usize, Sfx<'a>)> {
    channels.rev().max_by_key(|(_, sfx)| speed_of(*sfx))
}

pub(super) fn rebuild_pattern(cart: &Cart, pattern: Pattern) -> RebuiltPattern {
    let sfx = [0, 1, 2, 3].map(|channel| pattern.channel(channel).map(|slot| cart.sfx(slot)));
    let Some((timing_channel, ticks)) = timing_of(&sfx) else {
        return RebuiltPattern {
            chain_rows: Vec::new(),
            only_bit_7: false,
        };
    };
    let ticks_per_chain_row = sfx[timing_channel].map_or(1, speed_of) * SFX_NOTES as u32;
    let rows = ticks.div_ceil(ticks_per_chain_row);
    let chain_rows = (0..rows)
        .map(|row| {
            let from = row * ticks_per_chain_row;
            let to = ticks.min(from + ticks_per_chain_row);
            [0, 1, 2, 3].map(|channel| {
                let sets_length = channel == timing_channel;
                sfx[channel].map(|sfx| {
                    let mut rebuilt = rebuild_channel(sfx, from, to, sets_length);
                    let finished = rebuilt
                        .entry
                        .as_ref()
                        .is_some_and(|entry| entry.phrase.is_silent());
                    if row > 0 && !sets_length && finished {
                        rebuilt.entry = None;
                    }
                    rebuilt
                })
            })
        })
        .collect();
    RebuiltPattern {
        chain_rows,
        only_bit_7: sfx.iter().flatten().all(|sfx| has_bit_7(*sfx)),
    }
}
