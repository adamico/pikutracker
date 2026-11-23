use super::phrase::step_texts;
use super::{
    DIMMED, EMPTY, HEADING, LOOP_MARK, OUT_OF_RANGE, VALUE, draw_row_label, draw_title, mark_text,
    stepped,
};
use crate::app::layout::Area;
use crate::multitrack::{TimedStep, Timeline};
use crate::project::{Project, Step, TRACKS};
use crate::screen::grid::{Grid, Style};

const HEADER_ROWS: usize = 2;
const MARK_COLUMN: usize = 2;
const FIRST_TRACK_COLUMN: usize = 4;
const TRACK_WIDTH: usize = 12;
const TAIL: &str = "|";
const OUT_OF_RANGE_PITCH: &str = "!!!";

#[derive(Debug, Default)]
pub struct MultitrackPane {
    line: usize,
}

pub struct MultitrackView<'a> {
    pub project: &'a Project,
    pub timeline: &'a Timeline,
    pub playhead_tick: Option<u32>,
}

impl MultitrackPane {
    pub fn scroll(&mut self, lines: isize, timeline: &Timeline) {
        self.line = stepped(self.line, lines, timeline.line_starts().len());
    }

    pub fn follow(&mut self, tick: u32, timeline: &Timeline) {
        let starts = timeline.line_starts();
        self.line = starts
            .partition_point(|&start| start <= tick)
            .saturating_sub(1);
    }

    fn top_line(&self, visible: usize, lines: usize) -> usize {
        self.line
            .saturating_sub(visible / 2)
            .min(lines.saturating_sub(visible))
    }

    pub fn draw(&self, grid: &mut Grid, area: Area, focused: bool, view: &MultitrackView) {
        let starts = view.timeline.line_starts();
        let line = self.line.min(starts.len().saturating_sub(1));
        let song_row = starts
            .get(line)
            .and_then(|&tick| view.timeline.chain_row_at(tick))
            .map_or(0, |span| span.song_row);
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
        let top = self.top_line(visible, starts.len());
        for (offset, &tick) in starts.iter().skip(top).take(visible).enumerate() {
            let row = area.row + HEADER_ROWS + offset;
            let starting_song_row = view
                .timeline
                .song_row_starts()
                .find(|&(_, start)| start == tick)
                .map(|(song_row, _)| song_row);
            let label = starting_song_row.map_or_else(String::new, |row| format!("{row:02X}"));
            let playing = view.playhead_tick == Some(tick);
            draw_row_label(grid, area.column, row, &label, playing);
            if let Some(song_row) = starting_song_row {
                let mark = mark_text(view.project.song_row_marks(song_row));
                grid.put(area.column + MARK_COLUMN, row, mark, LOOP_MARK);
            }
            for track in 0..TRACKS {
                let column = area.column + FIRST_TRACK_COLUMN + track * TRACK_WIDTH;
                match view.timeline.step_sounding_at(track, tick) {
                    Some(step) if step.start == tick => draw_step(grid, column, row, step),
                    Some(_) => grid.put(column, row, TAIL, DIMMED),
                    None => {}
                }
            }
        }
    }
}

fn draw_step(grid: &mut Grid, column: usize, row: usize, step: &TimedStep) {
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
        grid.put(at, row, text, style(base));
        at += text.chars().count() + 1;
    }
}

fn transposed(step: Step, pitch: u8) -> Step {
    let mut step = step;
    step.set_pitch(i32::from(pitch));
    step
}
