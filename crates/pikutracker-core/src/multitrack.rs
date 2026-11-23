use crate::project::{
    ChainEntry, ChainRowPlace, PhraseId, Project, SongPlace, Step, TRACKS, Track,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimedStep {
    pub start: u32,
    pub ticks: u32,
    pub place: SongPlace,
    pub phrase: PhraseId,
    pub step: Step,
    pub transpose: i8,
}

impl TimedStep {
    pub fn end(&self) -> u32 {
        self.start + self.ticks
    }

    pub fn sounds_at(&self, tick: u32) -> bool {
        (self.start..self.end()).contains(&tick)
    }

    pub fn pitch(&self) -> Option<u8> {
        self.step.transposed_pitch(self.transpose)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChainRowSpan {
    pub place: ChainRowPlace,
    pub start: u32,
    pub ticks: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SongRowStart {
    pub song_row: usize,
    pub tick: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MultitrackLine {
    pub tick: u32,
    pub song_row: Option<usize>,
    pub holds_steps: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Timeline {
    chain_rows: Vec<ChainRowSpan>,
    empty_song_rows: Vec<SongRowStart>,
    tracks: [Vec<TimedStep>; TRACKS],
}

impl Timeline {
    pub fn of(project: &Project) -> Self {
        let mut timeline = Self::default();
        let Some(last_row) = project.last_played_song_row() else {
            return timeline;
        };
        let mut start = 0;
        for song_row in 0..=last_row {
            if project.song_row_is_empty(song_row) {
                timeline.empty_song_rows.push(SongRowStart {
                    song_row,
                    tick: start,
                });
            }
            for chain_row in project.played_chain_rows(song_row) {
                let span = ChainRowSpan {
                    place: ChainRowPlace {
                        song_row,
                        entry: chain_row,
                    },
                    start,
                    ticks: project.chain_row_ticks(song_row, chain_row),
                };
                for (track, steps) in Track::all().zip(timeline.tracks.iter_mut()) {
                    if let Some(entry) = project.chain_row_entry(song_row, track.index(), chain_row)
                    {
                        steps.extend(steps_of_entry(project, span, track, entry));
                    }
                }
                timeline.chain_rows.push(span);
                start += span.ticks;
            }
        }
        timeline
    }

    pub fn steps(&self, track: Track) -> &[TimedStep] {
        &self.tracks[track.index()]
    }

    pub fn step_sounding_at(&self, track: Track, tick: u32) -> Option<&TimedStep> {
        let steps = self.steps(track);
        let after = steps.partition_point(|step| step.start <= tick);
        steps[..after].last().filter(|step| step.sounds_at(tick))
    }

    pub fn chain_row_at(&self, tick: u32) -> Option<&ChainRowSpan> {
        let after = self.chain_rows.partition_point(|span| span.start <= tick);
        self.chain_rows[..after].last()
    }

    pub fn chain_row_start(&self, place: ChainRowPlace) -> Option<u32> {
        self.chain_rows
            .iter()
            .find(|span| span.place == place)
            .map(|span| span.start)
    }

    pub fn tick_of(&self, place: ChainRowPlace, chain_row_tick: u32) -> Option<u32> {
        self.chain_row_start(place)
            .map(|start| start + chain_row_tick)
    }

    pub fn song_row_starts(&self) -> impl Iterator<Item = SongRowStart> + '_ {
        self.chain_rows
            .iter()
            .enumerate()
            .filter(|(index, span)| {
                *index == 0 || self.chain_rows[index - 1].place.song_row != span.place.song_row
            })
            .map(|(_, span)| SongRowStart {
                song_row: span.place.song_row,
                tick: span.start,
            })
    }

    pub fn lines(&self) -> Vec<MultitrackLine> {
        let mut starts: Vec<u32> = self
            .tracks
            .iter()
            .flatten()
            .map(|step| step.start)
            .chain(self.song_row_starts().map(|start| start.tick))
            .collect();
        starts.sort_unstable();
        starts.dedup();
        let lines_with_steps = starts.into_iter().map(|tick| MultitrackLine {
            tick,
            song_row: self
                .song_row_starts()
                .find(|start| start.tick == tick)
                .map(|start| start.song_row),
            holds_steps: true,
        });
        let empty_lines = self.empty_song_rows.iter().map(|empty| MultitrackLine {
            tick: empty.tick,
            song_row: Some(empty.song_row),
            holds_steps: false,
        });
        let mut lines: Vec<MultitrackLine> = empty_lines.chain(lines_with_steps).collect();
        lines.sort_by_key(|line| (line.tick, line.holds_steps, line.song_row));
        lines
    }
}

fn steps_of_entry(
    project: &Project,
    span: ChainRowSpan,
    track: Track,
    entry: ChainEntry,
) -> impl Iterator<Item = TimedStep> + '_ {
    let phrase = project.phrase(entry.phrase());
    let speed = u32::from(phrase.speed());
    let mut next = Some(0);
    let mut at = 0;
    std::iter::from_fn(move || {
        let step_index = next.filter(|_| at < span.ticks)?;
        let timed = TimedStep {
            start: span.start + at,
            ticks: speed.min(span.ticks - at),
            place: SongPlace {
                chain_row: span.place,
                track: track.index(),
                step_index,
            },
            phrase: entry.phrase(),
            step: phrase.step(step_index),
            transpose: entry.transpose(),
        };
        next = phrase.step_after(step_index);
        at += speed;
        Some(timed)
    })
}
