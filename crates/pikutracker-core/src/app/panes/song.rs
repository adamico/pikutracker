use super::{
    EMPTY, HEADING, SELECTED, VALUE, draw_row_label, draw_styled_value, draw_title, id_text,
    nudged_id, stepped, typed_value,
};
use crate::app::Direction;
use crate::app::layout::Area;
use crate::app::selection::{CellPosition, CellRange};
use crate::player::{Playhead, TrackMix};
use crate::project::{ChainId, Project, SONG_ROWS, TRACKS};
use crate::screen::grid::{Grid, Style};
use crate::screen::palette::Colour;

const SOLOED: Style = Style::fg(Colour::Yellow);
const EXPORT_CHANNEL_NUMBER: Style = Style::fg(Colour::Green);
const PLAYING_BAR: Colour = Colour::DarkGreen;
const WAITING_BAR: Colour = Colour::DarkGrey;

const HEADER_ROWS: usize = 2;
const FIRST_TRACK_COLUMN: usize = 4;
const TRACK_WIDTH: usize = 3;

fn track_header(track_mix: &TrackMix, track: usize) -> (char, Style) {
    if track_mix.is_soloed(track) {
        ('S', SOLOED)
    } else if track_mix.is_muted(track) {
        ('M', EMPTY)
    } else {
        ('T', HEADING)
    }
}

#[derive(Debug, Default)]
pub struct SongPane {
    row: usize,
    track: usize,
    first_digit: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SongCursorPlace {
    row: usize,
    track: usize,
}

impl SongPane {
    pub fn cursor_place(&self) -> SongCursorPlace {
        SongCursorPlace {
            row: self.row,
            track: self.track,
        }
    }

    pub fn return_to(&mut self, place: SongCursorPlace) {
        self.row = place.row;
        self.track = place.track;
    }

    pub fn row(&self) -> usize {
        self.row
    }

    pub fn track(&self) -> usize {
        self.track
    }

    pub fn jump_to(&mut self, row: usize, track: usize) {
        self.row = row.min(SONG_ROWS - 1);
        self.track = track.min(TRACKS - 1);
    }

    pub fn move_cursor(&mut self, direction: Direction) {
        let (columns, rows) = direction.column_and_row_delta();
        self.track = stepped(self.track, columns, TRACKS);
        self.row = stepped(self.row, rows, SONG_ROWS);
    }

    pub fn follow(&mut self, row: usize) {
        if self.row != row {
            self.row = row;
            self.finish_typing();
        }
    }

    pub fn finish_typing(&mut self) {
        self.first_digit = None;
    }

    pub fn type_digit(&mut self, project: &mut Project, digit: u8) {
        let number = typed_value(&mut self.first_digit, digit);
        self.set_chain(project, Some(ChainId::new(number)));
    }

    pub fn nudge(&mut self, project: &mut Project, up: bool, coarse: bool) {
        let number = project.song_cell(self.row, self.track).map(ChainId::number);
        self.set_chain(project, Some(ChainId::new(nudged_id(number, up, coarse))));
    }

    pub fn clear(&mut self, project: &mut Project) {
        self.set_chain(project, None);
    }

    fn set_chain(&self, project: &mut Project, chain: Option<ChainId>) {
        project.set_song_cell(self.row, self.track, chain);
    }

    pub fn cell(&self) -> CellPosition {
        CellPosition {
            column: self.track,
            row: self.row,
        }
    }

    pub fn nudge_cells(&self, project: &mut Project, cells: CellRange, up: bool, coarse: bool) {
        for row in cells.rows() {
            for track in cells.columns() {
                if let Some(chain) = project.song_cell(row, track) {
                    let number = nudged_id(Some(chain.number()), up, coarse);
                    project.set_song_cell(row, track, Some(ChainId::new(number)));
                }
            }
        }
    }

    pub fn clear_cells(&self, project: &mut Project, cells: CellRange) {
        for row in cells.rows() {
            for track in cells.columns() {
                project.set_song_cell(row, track, None);
            }
        }
    }

    pub fn copy(&self, project: &Project, cells: CellRange) -> Vec<Vec<Option<ChainId>>> {
        cells
            .rows()
            .map(|row| {
                cells
                    .columns()
                    .map(|track| project.song_cell(row, track))
                    .collect()
            })
            .collect()
    }

    pub fn paste(&self, project: &mut Project, rows: &[Vec<Option<ChainId>>]) -> usize {
        let fitting_rows = rows.len().min(SONG_ROWS - self.row);
        for (row, cells) in (self.row..).zip(&rows[..fitting_rows]) {
            for (track, chain) in (self.track..TRACKS).zip(cells) {
                project.set_song_cell(row, track, *chain);
            }
        }
        let width = rows.first().map_or(0, Vec::len);
        if self.track + width > TRACKS {
            rows.len()
        } else {
            rows.len() - fitting_rows
        }
    }

    fn top_row(&self, visible: usize) -> usize {
        self.row
            .saturating_sub(visible / 2)
            .min(SONG_ROWS - visible)
    }

    pub fn draw_selection(&self, grid: &mut Grid, area: Area, cells: CellRange) {
        let visible = area.rows - HEADER_ROWS;
        let top = self.top_row(visible);
        for song_row in cells
            .rows()
            .filter(|row| (top..top + visible).contains(row))
        {
            for track in cells.columns() {
                if (song_row, track) != (self.row, self.track) {
                    let column = area.column + FIRST_TRACK_COLUMN + track * TRACK_WIDTH;
                    grid.restyle(column, HEADER_ROWS + song_row - top, 2, SELECTED);
                }
            }
        }
    }

    pub fn toggle_loop_row(&self, project: &mut Project) {
        let row = (project.song_loop_row() != Some(self.row)).then_some(self.row);
        project.set_song_loop_row(row);
    }

    pub fn draw(
        &self,
        grid: &mut Grid,
        area: Area,
        focused: bool,
        project: &Project,
        track_mix: &TrackMix,
        playhead: Option<&Playhead>,
    ) {
        let playing_row = playhead
            .and_then(|playhead| playhead.chain_row)
            .map(|(row, _)| row);
        let loop_row = project
            .song_loop_row()
            .map_or_else(|| "--".to_owned(), |row| format!("{row:02X}"));
        let title = format!("SONG  LOOP {loop_row}");
        draw_title(grid, area.column, 0, &title, focused);
        let export_tracks = project.export_track_numbers();
        for track in 0..TRACKS {
            let column = area.column + FIRST_TRACK_COLUMN + track * TRACK_WIDTH;
            let (letter, style) = track_header(track_mix, track);
            grid.put(column, 1, &format!("{letter}{}", track + 1), style);
            let number = track as u8 + 1;
            let channel = export_tracks
                .iter()
                .position(|exported| *exported == Some(number));
            if let Some(channel) = channel {
                grid.put(column + 2, 1, &channel.to_string(), EXPORT_CHANNEL_NUMBER);
            }
        }

        let visible = area.rows - HEADER_ROWS;
        let top = self.top_row(visible);
        for offset in 0..visible {
            let row = HEADER_ROWS + offset;
            let song_row = top + offset;
            let label = format!("{song_row:02X}");
            draw_row_label(
                grid,
                area.column,
                row,
                &label,
                playing_row == Some(song_row),
            );
            for track in 0..TRACKS {
                let column = area.column + FIRST_TRACK_COLUMN + track * TRACK_WIDTH;
                let is_cursor = focused && song_row == self.row && track == self.track;
                let chain = project.song_cell(song_row, track).map(ChainId::number);
                let style = if chain.is_some() { VALUE } else { EMPTY };
                draw_styled_value(grid, column, row, &id_text(chain), is_cursor, style);
            }
            if let Some(playhead) = playhead.filter(|_| playing_row == Some(song_row)) {
                draw_progress_bars(grid, area.column, row, project, playhead);
            }
        }
    }
}

/// Lays a bar over each Track's Chain number showing how far it is through
/// its Phrase: Steps played out of the Phrase's length, or full and dimmed
/// once it has ended and waits for the Chain row to end.
fn draw_progress_bars(
    grid: &mut Grid,
    area_column: usize,
    row: usize,
    project: &Project,
    playhead: &Playhead,
) {
    for track in 0..TRACKS {
        let column = area_column + FIRST_TRACK_COLUMN + track * TRACK_WIDTH;
        let (progress, colour) = match playhead.tracks[track] {
            Some((phrase, step)) => ((step + 1, project.phrase(phrase).length()), PLAYING_BAR),
            None if playhead.waiting[track] => ((1, 1), WAITING_BAR),
            None => continue,
        };
        grid.put_bar(column, row, 2, progress, colour);
    }
}
