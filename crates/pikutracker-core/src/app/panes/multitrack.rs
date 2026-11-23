use super::phrase::step_texts;
use super::{
    DIMMED, EMPTY, HEADING, LOOP_MARK, OUT_OF_RANGE, VALUE, draw_row_label, draw_styled_value,
    draw_title, mark_text,
};
use crate::app::Direction;
use crate::app::layout::Area;
use crate::multitrack::{MultitrackLine, TimedStep, Timeline};
use crate::project::{Project, Step, TRACKS};
use crate::screen::grid::{Grid, Style};

const HEADER_ROWS: usize = 2;
const MARK_COLUMN: usize = 2;
const FIRST_TRACK_COLUMN: usize = 4;
const TRACK_WIDTH: usize = 12;
const TAIL: &str = "|";
const OUT_OF_RANGE_PITCH: &str = "!!!";
const SILENCE: &str = "   ";

#[derive(Debug, Default)]
pub struct MultitrackPane {
    track: usize,
    tick: u32,
}

pub struct MultitrackView<'a> {
    pub project: &'a Project,
    pub timeline: &'a Timeline,
    pub playhead_tick: Option<u32>,
}

impl MultitrackPane {
    pub fn step<'a>(&self, timeline: &'a Timeline) -> Option<&'a TimedStep> {
        timeline.step_sounding_at(self.track, self.tick)
    }

    pub fn move_cursor(&mut self, direction: Direction, timeline: &Timeline) {
        let steps = timeline.steps(self.track);
        let tick = self.tick;
        match direction {
            Direction::Up => {
                if let Some(step) = steps.iter().rev().find(|step| step.start < tick) {
                    self.tick = step.start;
                }
            }
            Direction::Down => {
                if let Some(step) = steps.iter().find(|step| step.start > tick) {
                    self.tick = step.start;
                }
            }
            Direction::Left => self.go_to_the_first_step_sounding((0..self.track).rev(), timeline),
            Direction::Right => {
                self.go_to_the_first_step_sounding(self.track + 1..TRACKS, timeline);
            }
        }
    }

    fn go_to_the_first_step_sounding(
        &mut self,
        mut tracks: impl Iterator<Item = usize>,
        timeline: &Timeline,
    ) {
        let tick = self.tick;
        let found = tracks.find_map(|track| Some((track, timeline.step_sounding_at(track, tick)?)));
        if let Some((track, step)) = found {
            self.track = track;
            self.tick = step.start;
        }
    }

    pub fn follow(&mut self, tick: u32, timeline: &Timeline) {
        self.tick = timeline
            .step_sounding_at(self.track, tick)
            .map_or(tick, |step| step.start);
    }

    fn line(&self, lines: &[MultitrackLine]) -> usize {
        lines
            .partition_point(|line| line.tick <= self.tick)
            .saturating_sub(1)
    }

    fn top_line(line: usize, visible: usize, lines: usize) -> usize {
        line.saturating_sub(visible / 2)
            .min(lines.saturating_sub(visible))
    }

    pub fn draw(&self, grid: &mut Grid, area: Area, focused: bool, view: &MultitrackView) {
        let lines = view.timeline.lines();
        let line = self.line(&lines);
        let song_row = lines
            .get(line)
            .and_then(|line| view.timeline.chain_row_at(line.tick))
            .map_or(0, |span| span.place.song_row);
        let title = format!("MULTITRACK  ROW {song_row:02X}");
        draw_title(grid, area.column, area.row, &title, focused);
        for track in 0..TRACKS {
            let column = area.column + FIRST_TRACK_COLUMN + track * TRACK_WIDTH;
            grid.put(
                column,
                area.row + 1,
                &format!("TRACK {}", track + 1),
                HEADING,
            );
        }

        let visible = area.rows - HEADER_ROWS;
        let top = Self::top_line(line, visible, lines.len());
        for (offset, line) in lines.iter().skip(top).take(visible).enumerate() {
            let row = area.row + HEADER_ROWS + offset;
            let tick = line.tick;
            let label = line
                .song_row
                .map_or_else(String::new, |row| format!("{row:02X}"));
            let playing = line.holds_steps && view.playhead_tick == Some(tick);
            draw_row_label(grid, area.column, row, &label, playing);
            if let Some(song_row) = line.song_row {
                let mark = mark_text(view.project.song_row_marks(song_row));
                grid.put(area.column + MARK_COLUMN, row, mark, LOOP_MARK);
            }
            if !line.holds_steps {
                continue;
            }
            for track in 0..TRACKS {
                let column = area.column + FIRST_TRACK_COLUMN + track * TRACK_WIDTH;
                let is_cursor = focused && track == self.track && tick == self.tick;
                match view.timeline.step_sounding_at(track, tick) {
                    Some(step) if step.start == tick => {
                        draw_step(grid, column, row, step, is_cursor);
                    }
                    Some(_) => draw_styled_value(grid, column, row, TAIL, is_cursor, DIMMED),
                    None if is_cursor => draw_styled_value(grid, column, row, SILENCE, true, EMPTY),
                    None => {}
                }
            }
        }
    }
}

fn draw_step(grid: &mut Grid, column: usize, row: usize, step: &TimedStep, is_cursor: bool) {
    let (texts, pitch_style) = match step.pitch() {
        Some(pitch) => (step_texts(transposed(step.step, pitch)), VALUE),
        None => {
            let [_, instrument, volume, effect] = step_texts(step.step);
            let pitch = OUT_OF_RANGE_PITCH.to_owned();
            ([pitch, instrument, volume, effect], OUT_OF_RANGE)
        }
    };
    let style = |base: Style| if step.step.is_silent() { EMPTY } else { base };
    let mut at = column;
    for (index, text) in texts.iter().enumerate() {
        let base = if index == 0 { pitch_style } else { VALUE };
        draw_styled_value(grid, at, row, text, is_cursor && index == 0, style(base));
        at += text.chars().count() + 1;
    }
}

fn transposed(step: Step, pitch: u8) -> Step {
    let mut step = step;
    step.set_pitch(i32::from(pitch));
    step
}
