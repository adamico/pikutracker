use super::{
    EMPTY, HEADING, SELECTED, VALUE, draw_numbered_title, draw_out_of_range_flag, draw_row_label,
    draw_styled_value, nudged_id, stepped, typed_value,
};
use crate::app::Direction;
use crate::app::layout::Area;
use crate::app::selection::{CellPosition, CellRange};
use crate::project::{CHAIN_ENTRIES, Chain, ChainEntry, ChainId, PhraseId, Project};
use crate::screen::grid::{Grid, Style};
use std::ops::RangeInclusive;

const COARSE_TRANSPOSE: i32 = 12;
const HEADER_ROWS: usize = 2;
const OUT_OF_RANGE_COLUMN: usize = 14;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EntryCell {
    Phrase,
    Transpose,
}

const ENTRY_CELLS: [(EntryCell, usize, &str); 2] = [
    (EntryCell::Phrase, 3, "--"),
    (EntryCell::Transpose, 10, "---"),
];

#[derive(Clone, Copy, Debug)]
pub struct ChainView {
    pub id: Option<ChainId>,
    pub playing_entry: Option<usize>,
}

#[derive(Debug, Default)]
pub struct ChainPane {
    entry: usize,
    on_title: bool,
    column: usize,
    first_digit: Option<u8>,
    typing_negative: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChainCursorPlace {
    entry: usize,
    on_title: bool,
    column: usize,
}

impl ChainPane {
    pub fn cursor_place(&self) -> ChainCursorPlace {
        ChainCursorPlace {
            entry: self.entry,
            on_title: self.on_title,
            column: self.column,
        }
    }

    pub fn return_to(&mut self, place: ChainCursorPlace) {
        self.entry = place.entry;
        self.on_title = place.on_title;
        self.column = place.column;
    }

    pub fn jump_to(&mut self, entry: usize) {
        self.entry = entry.min(CHAIN_ENTRIES - 1);
        self.on_title = false;
    }

    pub fn on_title(&self) -> bool {
        self.on_title
    }

    pub fn move_cursor(&mut self, direction: Direction) {
        let (columns, rows) = direction.column_and_row_delta();
        if self.on_title {
            self.on_title = rows <= 0;
            return;
        }
        if rows < 0 && self.entry == 0 {
            self.on_title = true;
            return;
        }
        self.column = stepped(self.column, columns, ENTRY_CELLS.len());
        self.entry = stepped(self.entry, rows, CHAIN_ENTRIES);
    }

    pub fn entry(&self) -> usize {
        self.entry
    }

    fn cell(&self) -> EntryCell {
        ENTRY_CELLS[self.column].0
    }

    pub fn edits_empty_entries(&self) -> bool {
        self.cell() == EntryCell::Phrase
    }

    pub fn finish_typing(&mut self) {
        self.first_digit = None;
    }

    pub fn type_title_digit(&mut self, digit: u8) -> ChainId {
        ChainId::new(typed_value(&mut self.first_digit, digit))
    }

    pub fn nudged_title(&self, shown: Option<ChainId>, up: bool, coarse: bool) -> ChainId {
        ChainId::new(nudged_id(shown.map(ChainId::number), up, coarse))
    }

    pub fn type_digit(&mut self, chain: &mut Chain, digit: u8) {
        let is_first_digit = self.first_digit.is_none();
        let value = typed_value(&mut self.first_digit, digit);
        let slot = chain.entry_mut(self.entry);
        match (self.cell(), slot.as_mut()) {
            (EntryCell::Phrase, _) => set_phrase(slot, PhraseId::new(value)),
            (EntryCell::Transpose, Some(entry)) => {
                if is_first_digit {
                    self.typing_negative = entry.transpose() < 0;
                }
                let sign = if self.typing_negative { -1 } else { 1 };
                entry.set_transpose(sign * i32::from(value));
            }
            _ => {}
        }
    }

    pub fn nudge(&mut self, chain: &mut Chain, up: bool, coarse: bool) {
        nudge_cell(chain.entry_mut(self.entry), self.cell(), up, coarse);
    }

    pub fn clear(&mut self, chain: &mut Chain) {
        clear_cell(chain.entry_mut(self.entry), self.cell());
    }

    pub fn position(&self) -> Option<CellPosition> {
        (!self.on_title).then_some(CellPosition {
            column: self.column,
            row: self.entry,
        })
    }

    pub fn nudge_cells(&self, chain: &mut Chain, cells: CellRange, up: bool, coarse: bool) {
        for entry in cells.rows() {
            let slot = chain.entry_mut(entry);
            if slot.is_some() {
                for column in cells.columns() {
                    nudge_cell(slot, ENTRY_CELLS[column].0, up, coarse);
                }
            }
        }
    }

    pub fn clear_cells(&self, chain: &mut Chain, cells: CellRange) {
        for entry in cells.rows() {
            for column in cells.columns() {
                clear_cell(chain.entry_mut(entry), ENTRY_CELLS[column].0);
            }
        }
    }

    pub fn copy(&self, chain: &Chain, cells: CellRange) -> Vec<Option<ChainEntry>> {
        cells.rows().map(|entry| chain.entry(entry)).collect()
    }

    pub fn paste(
        &self,
        chain: &mut Chain,
        columns: &RangeInclusive<usize>,
        rows: &[Option<ChainEntry>],
    ) -> usize {
        let fitting = rows.len().min(CHAIN_ENTRIES - self.entry);
        for (entry, source) in (self.entry..).zip(&rows[..fitting]) {
            paste_entry(chain.entry_mut(entry), *source, columns);
        }
        rows.len() - fitting
    }

    pub fn draw_selection(&self, grid: &mut Grid, area: Area, cells: CellRange) {
        for entry in cells.rows() {
            for column in cells.columns() {
                if (entry, column) != (self.entry, self.column) {
                    let (_, offset, empty) = ENTRY_CELLS[column];
                    let row = area.row + HEADER_ROWS + entry;
                    grid.restyle(area.column + offset, row, empty.len(), SELECTED);
                }
            }
        }
    }

    pub fn follow(&mut self, entry: usize) {
        if self.entry != entry {
            self.entry = entry;
            self.finish_typing();
        }
    }

    pub fn draw(
        &self,
        grid: &mut Grid,
        area: Area,
        focused: bool,
        project: &Project,
        view: ChainView,
    ) {
        let ChainView { id, playing_entry } = view;
        let number = id.map(ChainId::number);
        let on_number = focused && self.on_title;
        draw_numbered_title(grid, area, "CHAIN", number, focused, on_number);
        grid.put(area.column, area.row + 1, "   PHRASE TRANSP", HEADING);

        for index in 0..CHAIN_ENTRIES {
            let row = area.row + HEADER_ROWS + index;
            let label = format!("{index:02X}");
            draw_row_label(grid, area.column, row, &label, playing_entry == Some(index));
            let entry = id.and_then(|id| project.chain(id).entry(index));
            for (column, (cell, offset, empty)) in ENTRY_CELLS.iter().enumerate() {
                let is_cursor =
                    focused && !self.on_title && index == self.entry && column == self.column;
                let (text, style) = match entry {
                    Some(entry) => cell_text(*cell, entry),
                    None => ((*empty).to_owned(), EMPTY),
                };
                draw_styled_value(grid, area.column + offset, row, &text, is_cursor, style);
            }
            let out_of_range = entry.is_some_and(|entry| {
                project
                    .phrase(entry.phrase())
                    .is_out_of_range(entry.transpose())
            });
            if out_of_range {
                draw_out_of_range_flag(grid, area.column + OUT_OF_RANGE_COLUMN, row);
            }
        }
    }
}

fn nudge_cell(slot: &mut Option<ChainEntry>, cell: EntryCell, up: bool, coarse: bool) {
    match (cell, slot.as_mut()) {
        (EntryCell::Phrase, entry) => {
            let number = entry.map(|entry| entry.phrase().number());
            set_phrase(slot, PhraseId::new(nudged_id(number, up, coarse)));
        }
        (EntryCell::Transpose, Some(entry)) => {
            let amount = if coarse { COARSE_TRANSPOSE } else { 1 };
            let delta = if up { amount } else { -amount };
            entry.set_transpose(i32::from(entry.transpose()) + delta);
        }
        _ => {}
    }
}

fn clear_cell(slot: &mut Option<ChainEntry>, cell: EntryCell) {
    match (cell, slot.as_mut()) {
        (EntryCell::Phrase, _) => *slot = None,
        (EntryCell::Transpose, Some(entry)) => entry.set_transpose(0),
        _ => {}
    }
}

fn paste_entry(
    slot: &mut Option<ChainEntry>,
    source: Option<ChainEntry>,
    columns: &RangeInclusive<usize>,
) {
    let copied = |cell| {
        ENTRY_CELLS
            .iter()
            .position(|(column, _, _)| *column == cell)
            .is_some_and(|column| columns.contains(&column))
    };
    if copied(EntryCell::Phrase) {
        match source {
            Some(source) => set_phrase(slot, source.phrase()),
            None => *slot = None,
        }
    }
    let Some(target) = slot.as_mut() else {
        return;
    };
    if copied(EntryCell::Transpose) {
        target.set_transpose(source.map_or(0, |source| i32::from(source.transpose())));
    }
}

fn set_phrase(slot: &mut Option<ChainEntry>, phrase: PhraseId) {
    match slot {
        Some(entry) => entry.set_phrase(phrase),
        None => *slot = Some(ChainEntry::new(phrase)),
    }
}

fn cell_text(cell: EntryCell, entry: ChainEntry) -> (String, Style) {
    match cell {
        EntryCell::Phrase => (format!("{:02X}", entry.phrase().number()), VALUE),
        EntryCell::Transpose => {
            let transpose = i16::from(entry.transpose());
            let sign = if transpose < 0 { '-' } else { '+' };
            (format!("{sign}{:02X}", transpose.abs()), VALUE)
        }
    }
}
