use super::read::{Metre, Note, Score, Unused};
use super::table::drum_row;
use super::{Analysis, Choices, Voice};
use crate::project::{
    Chain, ChainEntry, ChainId, ChainRowPlace, MAX_STEPS, Phrase, PhraseId, Project, SONG_ROWS,
    SongPlace, Step, TRACKS,
};
use crate::wording::counted;

const MIDI_KEY_OF_PITCH_0: i16 = 36;
const SPEED_FACTOR: f64 = 7230.0;
const MICROS_PER_MINUTE: f64 = 60_000_000.0;
const MAX_VELOCITY: u32 = 127;
const MAX_VOLUME: u32 = 7;
const OCTAVE: i16 = 12;
const PITCHES: i16 = 64;
const HIGHEST_PITCH: i16 = PITCHES - 1;
const CHAIN_ROWS_PER_SONG_ROW: usize = 4;
const IDS: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Warning {
    pub text: String,
    pub place: Option<SongPlace>,
}

impl Warning {
    fn new(text: String) -> Self {
        Self { text, place: None }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Imported {
    pub project: Project,
    pub warnings: Vec<Warning>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridOption {
    pub steps_per_beat: u8,
    pub speed: u8,
    pub phrase_length: usize,
    pub bars: usize,
    pub collisions: usize,
}

fn ticks_to_steps(ticks: u32, steps_per_beat: u8, ticks_per_beat: u32) -> usize {
    let scaled = u64::from(ticks) * u64::from(steps_per_beat);
    let beat = u64::from(ticks_per_beat);
    ((scaled * 2 + beat) / (beat * 2)) as usize
}

fn speed_for(micros_per_beat: u32, steps_per_beat: u8) -> u8 {
    let bpm = MICROS_PER_MINUTE / f64::from(micros_per_beat.max(1));
    (SPEED_FACTOR / (bpm * f64::from(steps_per_beat)))
        .round()
        .clamp(1.0, 255.0) as u8
}

fn volume_for(velocity: u8) -> u8 {
    (u32::from(velocity) * MAX_VOLUME).div_ceil(MAX_VELOCITY) as u8
}

fn bar_steps(metre: Metre, steps_per_beat: u8) -> usize {
    let quarter_beats =
        f64::from(metre.numerator) * 4.0 / 2_f64.powi(metre.denominator_power.into());
    ((quarter_beats * f64::from(steps_per_beat)).round() as usize).max(1)
}

fn phrase_length(bar: usize) -> usize {
    if bar > MAX_STEPS {
        MAX_STEPS
    } else {
        MAX_STEPS / bar * bar
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Section {
    start: usize,
    speed: u8,
    bar: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Slot {
    start: usize,
    length: usize,
    speed: u8,
}

struct Timeline {
    sections: Vec<Section>,
    close_tempos: usize,
    first_close_step: Option<usize>,
}

fn timeline(score: &Score, steps_per_beat: u8) -> Timeline {
    let beat = score.ticks_per_beat;
    let to_step = |tick| ticks_to_steps(tick, steps_per_beat, beat);

    let mut tempos: Vec<(u32, u8)> = Vec::new();
    let mut close_tempos = 0;
    let mut first_close_step = None;
    for &(tick, micros) in &score.tempos {
        let speed = speed_for(micros, steps_per_beat);
        match tempos.last_mut() {
            Some(last)
                if u64::from(tick - last.0) * u64::from(steps_per_beat) < u64::from(beat) =>
            {
                if last.1 != speed {
                    close_tempos += 1;
                    first_close_step.get_or_insert(to_step(tick));
                }
                *last = (tick, speed);
            }
            _ => tempos.push((tick, speed)),
        }
    }

    let mut speed = speed_for(Score::default_micros_per_beat(), steps_per_beat);
    let mut metre = Metre::default();
    let mut changes: Vec<(usize, Option<u8>, Option<Metre>)> = tempos
        .iter()
        .map(|&(tick, speed)| (to_step(tick), Some(speed), None))
        .chain(
            score
                .metres
                .iter()
                .map(|&(tick, metre)| (to_step(tick), None, Some(metre))),
        )
        .collect();
    changes.sort_by_key(|change| change.0);

    let mut sections = vec![Section {
        start: 0,
        speed,
        bar: bar_steps(metre, steps_per_beat),
    }];
    for (step, new_speed, new_metre) in changes {
        speed = new_speed.unwrap_or(speed);
        metre = new_metre.unwrap_or(metre);
        let section = Section {
            start: step,
            speed,
            bar: bar_steps(metre, steps_per_beat),
        };
        let last = sections
            .last_mut()
            .expect("there is always a first section");
        if last.start == step {
            *last = section;
            let count = sections.len();
            if count > 1 && same_timing(sections[count - 2], section) {
                sections.pop();
            }
        } else if !same_timing(*last, section) {
            sections.push(section);
        }
    }
    Timeline {
        sections,
        close_tempos,
        first_close_step,
    }
}

fn same_timing(a: Section, b: Section) -> bool {
    (a.speed, a.bar) == (b.speed, b.bar)
}

fn slots(sections: &[Section], total: usize) -> Vec<Slot> {
    let mut slots = Vec::new();
    for (index, section) in sections.iter().enumerate() {
        let next = sections.get(index + 1).map(|next| next.start);
        let length = phrase_length(section.bar);
        let mut start = section.start;
        while start < next.unwrap_or(total).min(total) {
            let room = next.map_or(length, |next| next - start);
            slots.push(Slot {
                start,
                length: length.min(room),
                speed: section.speed,
            });
            start += length.min(room);
        }
    }
    slots
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Painted {
    key: u8,
    velocity: u8,
}

struct Painting {
    cells: Vec<Option<Painted>>,
    collisions: usize,
    first_collision: Option<usize>,
}

fn paint(notes: &[Note], steps_per_beat: u8, ticks_per_beat: u32) -> Painting {
    let to_steps = |ticks| ticks_to_steps(ticks, steps_per_beat, ticks_per_beat);
    let span = |note: &Note| {
        let start = to_steps(note.start);
        (
            start,
            start + to_steps(note.end.saturating_sub(note.start)).max(1),
        )
    };
    let total = notes.iter().map(|note| span(note).1).max().unwrap_or(0);
    let mut cells = vec![None; total];
    let mut struck = vec![false; total];
    let mut collisions = 0;
    let mut first_collision = None;
    let mut painted_to = 0;
    for note in notes {
        let (start, end) = span(note);
        if struck[start] {
            collisions += 1;
            first_collision.get_or_insert(start);
        }
        struck[start] = true;
        if start < painted_to {
            cells[start..painted_to].fill(None);
        }
        cells[start..end].fill(Some(Painted {
            key: note.key,
            velocity: note.velocity,
        }));
        painted_to = end;
    }
    Painting {
        cells,
        collisions,
        first_collision,
    }
}

fn program_steps(score: &Score, channel: u8, steps_per_beat: u8) -> Vec<(usize, u8)> {
    score.programs[usize::from(channel)]
        .iter()
        .map(|&(tick, program)| {
            (
                ticks_to_steps(tick, steps_per_beat, score.ticks_per_beat),
                program,
            )
        })
        .collect()
}

fn program_on(changes: &[(usize, u8)], step: usize) -> u8 {
    changes
        .iter()
        .take_while(|(at, _)| *at <= step)
        .last()
        .map_or(0, |(_, program)| *program)
}

struct PitchPlan {
    transpose: i8,
    lowest_pitch: i16,
    folded: usize,
    span: i16,
}

impl PitchPlan {
    const UNCHANGED: Self = Self {
        transpose: 0,
        lowest_pitch: 0,
        folded: 0,
        span: 0,
    };
}

fn sounding_pitch(key: u8) -> i16 {
    i16::from(key) - MIDI_KEY_OF_PITCH_0
}

fn pitch_plan(voice: &Voice) -> PitchPlan {
    let pitches = voice.notes.iter().map(|note| sounding_pitch(note.key));
    let lowest = pitches.clone().min().unwrap_or(0);
    let highest = pitches.max().unwrap_or(0);
    let span = highest - lowest + 1;
    let folded = voice
        .notes
        .iter()
        .filter(|note| sounding_pitch(note.key) > lowest + HIGHEST_PITCH)
        .count();
    let top = highest.min(lowest + HIGHEST_PITCH);
    PitchPlan {
        transpose: transpose_for(lowest, top),
        lowest_pitch: lowest,
        folded,
        span,
    }
}

fn transpose_for(lowest: i16, highest: i16) -> i8 {
    let least = highest - HIGHEST_PITCH;
    let most = lowest;
    let (nearest, octave) = if most < 0 {
        (most, most.div_euclid(OCTAVE) * OCTAVE)
    } else if least > 0 {
        (least, (least + OCTAVE - 1).div_euclid(OCTAVE) * OCTAVE)
    } else {
        return 0;
    };
    let offset = if (least..=most).contains(&octave) {
        octave
    } else {
        nearest
    };
    offset as i8
}

fn phrase_pitch(pitch: i16, plan: &PitchPlan) -> u8 {
    let mut pitch = pitch;
    while pitch > plan.lowest_pitch + HIGHEST_PITCH {
        pitch -= OCTAVE;
    }
    (pitch - i16::from(plan.transpose)).clamp(0, HIGHEST_PITCH) as u8
}

struct Part<'a> {
    voice: &'a Voice,
    painting: Painting,
    programs: Vec<(usize, u8)>,
    plan: PitchPlan,
    is_drums: bool,
}

impl Part<'_> {
    fn step(&self, index: usize, choices: &Choices) -> Step {
        let Some(Some(cell)) = self.painting.cells.get(index) else {
            return Step::default();
        };
        let volume = volume_for(cell.velocity);
        let built = if self.is_drums {
            let row = drum_row(cell.key).unwrap_or(0);
            let hit = choices.table.hit(row);
            let pitch = choices.table.drum_pitch(row, cell.key);
            Step::new(pitch, hit.instrument, volume, hit.effect)
        } else {
            let program = program_on(&self.programs, index);
            let pitch = phrase_pitch(sounding_pitch(cell.key), &self.plan);
            Step::new(pitch, choices.table.instrument(program), volume, 0)
        };
        built.unwrap_or_default()
    }

    fn phrase(&self, slot: Slot, choices: &Choices) -> Phrase {
        let mut phrase = silent_phrase(slot);
        for offset in 0..slot.length {
            *phrase.step_mut(offset) = self.step(slot.start + offset, choices);
        }
        phrase
    }
}

fn silent_phrase(slot: Slot) -> Phrase {
    let mut phrase = Phrase::default();
    phrase.set_length(slot.length as i32);
    phrase.set_speed(i32::from(slot.speed));
    phrase
}

fn kept_voices<'a>(analysis: &'a Analysis, choices: &Choices) -> Vec<&'a Voice> {
    analysis
        .voices
        .iter()
        .zip(choices.tracks())
        .filter_map(|(voice, track)| track.map(|_| voice))
        .collect()
}

pub fn compare_grids(analysis: &Analysis, choices: &Choices) -> Vec<GridOption> {
    let kept = kept_voices(analysis, choices);
    super::STEPS_PER_BEAT
        .into_iter()
        .map(|steps_per_beat| {
            let timing = timeline(&analysis.score, steps_per_beat);
            let first = timing.sections[0];
            let collisions = kept
                .iter()
                .map(|voice| {
                    paint(&voice.notes, steps_per_beat, analysis.score.ticks_per_beat).collisions
                })
                .sum();
            GridOption {
                steps_per_beat,
                speed: first.speed,
                phrase_length: phrase_length(first.bar),
                bars: if first.bar > MAX_STEPS {
                    0
                } else {
                    MAX_STEPS / first.bar
                },
                collisions,
            }
        })
        .collect()
}

fn index_of_or_push<T: PartialEq>(list: &mut Vec<T>, value: T) -> usize {
    list.iter()
        .position(|held| *held == value)
        .unwrap_or_else(|| {
            list.push(value);
            list.len() - 1
        })
}

pub fn build(analysis: &Analysis, choices: &Choices) -> Imported {
    let score = &analysis.score;
    let spb = choices.steps_per_beat;
    let kept = kept_voices(analysis, choices);
    let parts: Vec<Part> = kept
        .iter()
        .map(|voice| {
            let is_drums = score.sources[voice.source].is_drums();
            Part {
                voice,
                painting: paint(&voice.notes, spb, score.ticks_per_beat),
                programs: program_steps(score, voice.channel - 1, spb),
                plan: if is_drums {
                    PitchPlan::UNCHANGED
                } else {
                    pitch_plan(voice)
                },
                is_drums,
            }
        })
        .collect();

    let total = parts
        .iter()
        .map(|part| part.painting.cells.len())
        .max()
        .unwrap_or(0);
    let timing = timeline(score, spb);
    let slots = slots(&timing.sections, total);

    let mut project = Project::new();
    project.set_default_speed(i32::from(timing.sections[0].speed));
    let mut phrases: Vec<Phrase> = Vec::new();
    let mut chains: Vec<Chain> = Vec::new();
    let mut song_rows = 0;
    for (row, row_slots) in slots.chunks(CHAIN_ROWS_PER_SONG_ROW).enumerate() {
        let kept_phrases = phrases.len();
        let kept_chains = chains.len();
        let cells = song_row_cells(&parts, row_slots, choices, &mut phrases, &mut chains);
        if row >= SONG_ROWS || phrases.len() > IDS || chains.len() > IDS {
            phrases.truncate(kept_phrases);
            chains.truncate(kept_chains);
            break;
        }
        for (track, cell) in cells.into_iter().enumerate() {
            project.set_song_cell(row, track, cell.map(|id| ChainId::new(id as u8)));
        }
        song_rows += 1;
    }
    for (id, phrase) in phrases.iter().enumerate() {
        *project.phrase_mut(PhraseId::new(id as u8)) = *phrase;
    }
    for (id, chain) in chains.iter().enumerate() {
        *project.chain_mut(ChainId::new(id as u8)) = *chain;
    }

    let warnings = warnings(
        analysis, choices, &parts, &slots, &timing, &project, song_rows,
    );
    Imported { project, warnings }
}

fn song_row_cells(
    parts: &[Part],
    slots: &[Slot],
    choices: &Choices,
    phrases: &mut Vec<Phrase>,
    chains: &mut Vec<Chain>,
) -> [Option<usize>; TRACKS] {
    let mut rows: [Chain; TRACKS] = [Chain::default(); TRACKS];
    for (k, slot) in slots.iter().enumerate() {
        let mut sounding = false;
        for (track, part) in parts.iter().enumerate() {
            let phrase = part.phrase(*slot, choices);
            if phrase.is_silent() {
                continue;
            }
            sounding = true;
            let id = index_of_or_push(phrases, phrase);
            let mut entry = ChainEntry::new(PhraseId::new(id as u8));
            entry.set_transpose(i32::from(part.plan.transpose));
            *rows[track].entry_mut(k) = Some(entry);
        }
        if !sounding && !parts.is_empty() {
            let id = index_of_or_push(phrases, silent_phrase(*slot));
            *rows[0].entry_mut(k) = Some(ChainEntry::new(PhraseId::new(id as u8)));
        }
    }
    let mut cells = [None; TRACKS];
    for (track, chain) in rows.into_iter().enumerate() {
        if chain != Chain::default() {
            cells[track] = Some(index_of_or_push(chains, chain));
        }
    }
    cells
}

fn song_place_of(slots: &[Slot], track: usize, step: usize) -> Option<SongPlace> {
    let index = slots
        .iter()
        .position(|slot| (slot.start..slot.start + slot.length).contains(&step))?;
    Some(SongPlace {
        chain_row: ChainRowPlace {
            song_row: index / CHAIN_ROWS_PER_SONG_ROW,
            entry: index % CHAIN_ROWS_PER_SONG_ROW,
        },
        track,
        step_index: step - slots[index].start,
    })
}

fn warnings(
    analysis: &Analysis,
    choices: &Choices,
    parts: &[Part],
    slots: &[Slot],
    timing: &Timeline,
    project: &Project,
    song_rows: usize,
) -> Vec<Warning> {
    let score = &analysis.score;
    let mut warnings = Vec::new();

    let tracks = choices.tracks();
    let dropped: Vec<&str> = analysis
        .voices
        .iter()
        .zip(choices.ticked.iter().zip(&tracks))
        .filter(|(_, (ticked, track))| **ticked && track.is_none())
        .map(|(voice, _)| voice.name())
        .collect();
    if !dropped.is_empty() {
        let ticked = choices.ticked.iter().filter(|ticked| **ticked).count();
        warnings.push(Warning::new(format!(
            "{ticked} VOICES, {TRACKS} TRACKS: DROPPED {}.",
            dropped.join(", ")
        )));
    }

    for (track, part) in parts.iter().enumerate() {
        if !part.is_drums && part.plan.folded > 0 {
            warnings.push(Warning::new(format!(
                "{} VOICE SPANS {} SEMITONES: {} FOLDED BY AN OCTAVE. (T{})",
                part.voice.name(),
                part.plan.span,
                counted(part.plan.folded, "NOTE"),
                track + 1
            )));
        }
    }

    let collisions: usize = parts.iter().map(|part| part.painting.collisions).sum();
    if collisions > 0 {
        warnings.push(collision_warning(collisions, parts, slots, project));
    }

    match (score.volume_ignored, score.expression_ignored) {
        (true, true) => warnings.push(Warning::new(
            "CC7 AND CC11 (VOLUME, EXPRESSION) IGNORED.".to_owned(),
        )),
        (true, false) => warnings.push(Warning::new("CC7 (VOLUME) IGNORED.".to_owned())),
        (false, true) => warnings.push(Warning::new("CC11 (EXPRESSION) IGNORED.".to_owned())),
        (false, false) => {}
    }

    if !score.unused.is_empty() {
        let names: Vec<String> = score.unused.iter().map(unused_name).collect();
        warnings.push(Warning::new(format!("DROPPED: {}.", names.join(", "))));
    }

    if timing.close_tempos > 0 {
        let row = timing
            .first_close_step
            .and_then(|step| song_place_of(slots, 0, step))
            .map_or(0, |place| place.chain_row.song_row);
        warnings.push(Warning::new(format!(
            "{} CLOSER THAN ONE STEP: THE LAST ONE WON. (ROW {row:02X})",
            counted(timing.close_tempos, "TEMPO CHANGE")
        )));
    }

    if !analysis.unmapped_drums.is_empty() {
        let keys: Vec<String> = analysis.unmapped_drums.iter().map(u8::to_string).collect();
        warnings.push(Warning::new(format!(
            "{} OUTSIDE THE KIT DROPPED: {}.",
            counted(analysis.unmapped_drum_count, "DRUM NOTE"),
            keys.join(", ")
        )));
    }

    if song_rows * CHAIN_ROWS_PER_SONG_ROW < slots.len() {
        warnings.push(Warning::new(format!(
            "SONG TOO LONG FOR A PROJECT: STOPPED AFTER {song_rows} SONG ROWS."
        )));
    }
    warnings
}

fn collision_warning(
    collisions: usize,
    parts: &[Part],
    slots: &[Slot],
    project: &Project,
) -> Warning {
    let first = parts.iter().enumerate().find_map(|(track, part)| {
        let step = part.painting.first_collision?;
        song_place_of(slots, track, step)
    });
    let named = first.and_then(|place| {
        let ChainRowPlace { song_row, entry } = place.chain_row;
        let entry = project.chain_row_entry(song_row, place.track, entry)?;
        Some(format!(
            " FIRST: T{} PHRASE {:02X} STEP {:02X}",
            place.track + 1,
            entry.phrase().number(),
            place.step_index
        ))
    });
    Warning {
        text: format!(
            "{} LANDED ON A STEP ALREADY TAKEN: THE LATER NOTE WON.{}",
            counted(collisions, "NOTE"),
            named.as_deref().unwrap_or("")
        ),
        place: named.as_ref().and(first),
    }
}

fn unused_name(unused: &Unused) -> String {
    match unused {
        Unused::PitchBend => "PITCH BEND".to_owned(),
        Unused::Aftertouch => "AFTERTOUCH".to_owned(),
        Unused::Controller(number) => format!("CC{number}"),
    }
}
