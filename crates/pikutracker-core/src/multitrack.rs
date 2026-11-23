use crate::project::{ChainEntry, PhraseId, Project, Step, TRACKS};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepPlace {
    pub song_row: usize,
    pub chain_row: usize,
    pub phrase: PhraseId,
    pub step: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimedStep {
    pub start: u32,
    pub ticks: u32,
    pub place: StepPlace,
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
    pub song_row: usize,
    pub chain_row: usize,
    pub start: u32,
    pub ticks: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Line {
    pub tick: u32,
    pub song_row: Option<usize>,
    pub holds_steps: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Timeline {
    chain_rows: Vec<ChainRowSpan>,
    empty_song_rows: Vec<(usize, u32)>,
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
                timeline.empty_song_rows.push((song_row, start));
            }
            for chain_row in project.played_chain_rows(song_row) {
                let span = ChainRowSpan {
                    song_row,
                    chain_row,
                    start,
                    ticks: project.chain_row_ticks(song_row, chain_row),
                };
                for (track, steps) in timeline.tracks.iter_mut().enumerate() {
                    if let Some(entry) = project.chain_row_entry(song_row, track, chain_row) {
                        steps.extend(steps_of_entry(project, span, entry));
                    }
                }
                timeline.chain_rows.push(span);
                start += span.ticks;
            }
        }
        timeline
    }

    pub fn chain_rows(&self) -> &[ChainRowSpan] {
        &self.chain_rows
    }

    pub fn steps(&self, track: usize) -> &[TimedStep] {
        &self.tracks[track]
    }

    pub fn step_sounding_at(&self, track: usize, tick: u32) -> Option<&TimedStep> {
        let steps = self.steps(track);
        let after = steps.partition_point(|step| step.start <= tick);
        steps[..after].last().filter(|step| step.sounds_at(tick))
    }

    pub fn chain_row_at(&self, tick: u32) -> Option<&ChainRowSpan> {
        let after = self.chain_rows.partition_point(|span| span.start <= tick);
        self.chain_rows[..after].last()
    }

    pub fn chain_row_start(&self, song_row: usize, chain_row: usize) -> Option<u32> {
        self.chain_rows
            .iter()
            .find(|span| (span.song_row, span.chain_row) == (song_row, chain_row))
            .map(|span| span.start)
    }

    pub fn tick_of(&self, song_row: usize, chain_row: usize, chain_row_tick: u32) -> Option<u32> {
        self.chain_row_start(song_row, chain_row)
            .map(|start| start + chain_row_tick)
    }

    pub fn song_row_starts(&self) -> impl Iterator<Item = (usize, u32)> + '_ {
        self.chain_rows
            .iter()
            .enumerate()
            .filter(|(index, span)| {
                *index == 0 || self.chain_rows[index - 1].song_row != span.song_row
            })
            .map(|(_, span)| (span.song_row, span.start))
    }

    pub fn lines(&self) -> Vec<Line> {
        let mut starts: Vec<u32> = self
            .tracks
            .iter()
            .flatten()
            .map(|step| step.start)
            .chain(self.song_row_starts().map(|(_, start)| start))
            .collect();
        starts.sort_unstable();
        starts.dedup();
        let lines_with_steps = starts.into_iter().map(|tick| Line {
            tick,
            song_row: self
                .song_row_starts()
                .find(|&(_, start)| start == tick)
                .map(|(song_row, _)| song_row),
            holds_steps: true,
        });
        let empty_lines = self.empty_song_rows.iter().map(|&(song_row, tick)| Line {
            tick,
            song_row: Some(song_row),
            holds_steps: false,
        });
        let mut lines: Vec<Line> = empty_lines.chain(lines_with_steps).collect();
        lines.sort_by_key(|line| (line.tick, line.holds_steps, line.song_row));
        lines
    }
}

fn steps_of_entry(
    project: &Project,
    span: ChainRowSpan,
    entry: ChainEntry,
) -> impl Iterator<Item = TimedStep> + '_ {
    let phrase = project.phrase(entry.phrase());
    let speed = u32::from(phrase.speed());
    let mut next = Some(0);
    let mut at = 0;
    std::iter::from_fn(move || {
        let step = next.filter(|_| at < span.ticks)?;
        let timed = TimedStep {
            start: span.start + at,
            ticks: speed.min(span.ticks - at),
            place: StepPlace {
                song_row: span.song_row,
                chain_row: span.chain_row,
                phrase: entry.phrase(),
                step,
            },
            step: phrase.step(step),
            transpose: entry.transpose(),
        };
        next = phrase.step_after(step);
        at += speed;
        Some(timed)
    })
}
