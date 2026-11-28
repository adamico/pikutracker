use super::edit_target::{EditTarget, NO_STEP_HERE};
use super::focus::EditPane;
use super::messages::Report;
use super::selection::{CellPosition, CellRange};
use super::{App, Chain, ChainPane, Direction, Focus};
use crate::project::{ChainEntry, ChainId, Step};
use crate::screen::grid::Grid;
use std::ops::RangeInclusive;

const NOTHING_TO_PASTE: &str = "NOTHING TO PASTE";

#[derive(Clone, Debug)]
pub(super) enum Clipboard {
    Song(Vec<Vec<Option<ChainId>>>),
    Chain {
        columns: RangeInclusive<usize>,
        rows: Vec<Option<ChainEntry>>,
    },
    Phrase {
        columns: RangeInclusive<usize>,
        rows: Vec<Step>,
    },
}

impl Clipboard {
    const fn contents(&self) -> &'static str {
        match self {
            Clipboard::Song(_) => "SONG CELLS",
            Clipboard::Chain { .. } => "CHAIN ENTRIES",
            Clipboard::Phrase { .. } => "STEPS",
        }
    }
}

const fn pane_name(focus: Focus) -> &'static str {
    match focus {
        Focus::Song => "SONG",
        Focus::Chain => "CHAIN",
        Focus::Phrase => "PHRASE",
        Focus::RightSlot => "RIGHT-HAND",
    }
}

impl App {
    fn cursor_cell(&self) -> Option<CellPosition> {
        match self.focus.edit_pane()? {
            EditPane::Song => Some(self.song.cell()),
            EditPane::Chain => self.chain.position(),
            EditPane::Phrase => self.phrase.cell(),
        }
    }

    pub(super) fn selected_cells(&self) -> Option<CellRange> {
        Some(CellRange::between(
            self.selection_start?,
            self.cursor_cell()?,
        ))
    }

    pub(super) fn target_cells(&self) -> Option<CellRange> {
        self.selected_cells()
            .or_else(|| self.cursor_cell().map(CellRange::at))
    }

    pub(super) fn start_selection(&mut self) {
        match self.cursor_cell() {
            Some(cell) => {
                self.stop_following();
                self.selection_start = Some(cell);
            }
            None if self.focus == Focus::Phrase => self.remark = Some(NO_STEP_HERE.into()),
            None => {}
        }
    }

    pub(super) fn start_selection_unless_started(&mut self) {
        if self.selection_start.is_none() {
            self.selection_start = self.cursor_cell();
        }
    }

    fn edit_selected_chain(&mut self, edit: impl FnOnce(&ChainPane, &mut Chain)) {
        if let Some(EditTarget::Chain(id)) = self.edit_target_or_remark() {
            edit(&self.chain, self.project.chain_mut(id));
        }
    }

    pub(super) fn nudge_selection(&mut self, up: bool, coarse: bool) {
        let Some(cells) = self.selected_cells() else {
            return;
        };
        match self.focus.edit_pane() {
            Some(EditPane::Song) => self.song.nudge_cells(&mut self.project, cells, up, coarse),
            Some(EditPane::Chain) => {
                self.edit_selected_chain(|pane, chain| pane.nudge_cells(chain, cells, up, coarse));
            }
            Some(EditPane::Phrase) => {
                self.edit_phrase(|pane, phrase| pane.nudge_cells(phrase, cells, up, coarse));
            }
            None => {}
        }
    }

    pub(super) fn move_rows(&mut self, up: bool) {
        let Some(cells) = self.selected_cells() else {
            return;
        };
        let moved = match self.edit_target_or_remark() {
            Some(EditTarget::Song) => {
                self.project
                    .move_song_cells(cells.rows(), cells.columns(), up)
            }
            Some(EditTarget::Chain(id)) => {
                self.project.chain_mut(id).move_entries(cells.rows(), up)
            }
            Some(EditTarget::Phrase(id)) => {
                self.project.phrase_mut(id).move_steps(cells.rows(), up)
            }
            None => false,
        };
        if moved {
            self.move_selection_with_rows(up);
        }
    }

    fn move_selection_with_rows(&mut self, up: bool) {
        let direction = if up { Direction::Up } else { Direction::Down };
        if let Some(start) = self.selection_start.as_mut() {
            start.row = if up { start.row - 1 } else { start.row + 1 };
        }
        match self.focus.edit_pane() {
            Some(EditPane::Song) => self.song.move_cursor(direction),
            Some(EditPane::Chain) => self.chain.move_cursor(direction),
            Some(EditPane::Phrase) => self.phrase.move_cursor(direction),
            None => {}
        }
    }

    pub(super) fn clear_cells(&mut self) {
        match (self.focus.edit_pane(), self.selected_cells()) {
            (Some(EditPane::Song), Some(cells)) => self.song.clear_cells(&mut self.project, cells),
            (Some(EditPane::Song), None) => self.song.clear(&mut self.project),
            (Some(EditPane::Chain), Some(cells)) => {
                self.edit_selected_chain(|pane, chain| pane.clear_cells(chain, cells));
            }
            (Some(EditPane::Chain), None) => self.edit_chain(|pane, chain| pane.clear(chain)),
            (Some(EditPane::Phrase), _) => {
                if let Some(cells) = self.target_cells() {
                    self.edit_phrase(|pane, phrase| pane.clear_cells(phrase, cells));
                }
            }
            (None, _) => {}
        }
    }

    fn copied_cells(&mut self) -> Option<Clipboard> {
        let cells = self.target_cells();
        match self.edit_target_or_remark()? {
            EditTarget::Song => Some(Clipboard::Song(self.song.copy(&self.project, cells?))),
            EditTarget::Chain(id) => {
                let cells = cells?;
                Some(Clipboard::Chain {
                    columns: cells.columns(),
                    rows: self.chain.copy(self.project.chain(id), cells),
                })
            }
            EditTarget::Phrase(id) => {
                self.phrase_step_or_remark()?;
                let cells = cells?;
                Some(Clipboard::Phrase {
                    columns: cells.columns(),
                    rows: self.phrase.copy(self.project.phrase(id), cells),
                })
            }
        }
    }

    pub(super) fn copy(&mut self) {
        if let Some(copied) = self.copied_cells() {
            self.clipboard = Some(copied);
            self.selection_start = None;
        }
    }

    pub(super) fn cut(&mut self) {
        if let Some(copied) = self.copied_cells() {
            self.clipboard = Some(copied);
            self.clear_cells();
            self.selection_start = None;
        }
    }

    pub(super) fn paste(&mut self) {
        if self.focus == Focus::RightSlot {
            return;
        }
        let Some(clipboard) = self.clipboard.clone() else {
            self.remark = Some(NOTHING_TO_PASTE.into());
            return;
        };
        let lost = match (&clipboard, self.focus) {
            (Clipboard::Song(rows), Focus::Song) => Some(self.song.paste(&mut self.project, rows)),
            (Clipboard::Chain { columns, rows }, Focus::Chain) => {
                self.paste_into_chain(columns, rows)
            }
            (Clipboard::Phrase { columns, rows }, Focus::Phrase) => {
                self.paste_into_phrase(columns, rows)
            }
            _ => {
                self.report(Report::failure(format!(
                    "CANNOT PASTE {} INTO THE {} PANE",
                    clipboard.contents(),
                    pane_name(self.focus),
                )));
                return;
            }
        };
        let Some(lost) = lost else {
            return;
        };
        self.selection_start = None;
        if lost > 0 {
            let rows = if lost == 1 { "ROW" } else { "ROWS" };
            self.report(Report::warning(format!("PASTE LOST {lost} {rows}")));
        }
    }

    fn paste_into_chain(
        &mut self,
        columns: &RangeInclusive<usize>,
        rows: &[Option<ChainEntry>],
    ) -> Option<usize> {
        let Some(EditTarget::Chain(id)) = self.edit_target_or_remark() else {
            return None;
        };
        Some(self.chain.paste(self.project.chain_mut(id), columns, rows))
    }

    fn paste_into_phrase(
        &mut self,
        columns: &RangeInclusive<usize>,
        rows: &[Step],
    ) -> Option<usize> {
        let Some(EditTarget::Phrase(id)) = self.edit_target_or_remark() else {
            return None;
        };
        self.phrase_step_or_remark()?;
        Some(
            self.phrase
                .paste(self.project.phrase_mut(id), columns, rows),
        )
    }

    pub(super) fn draw_selection(&self, grid: &mut Grid) {
        let Some(cells) = self.selected_cells() else {
            return;
        };
        match self.focus.edit_pane() {
            Some(EditPane::Song) => self.song.draw_selection(grid, super::layout::SONG, cells),
            Some(EditPane::Chain) => self.chain.draw_selection(grid, super::layout::CHAIN, cells),
            Some(EditPane::Phrase) => {
                self.phrase
                    .draw_selection(grid, super::layout::PHRASE, cells)
            }
            None => {}
        }
    }
}
