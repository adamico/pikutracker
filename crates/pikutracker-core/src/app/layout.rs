use crate::screen::grid::{GRID_COLUMNS, GRID_ROWS, Grid, Style};
use crate::screen::palette::Colour;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Area {
    pub column: usize,
    pub row: usize,
    pub width: usize,
    pub rows: usize,
}

pub const STATUS_ROW: usize = GRID_ROWS - 1;

const PANE_ROWS: usize = STATUS_ROW;
const LEFT_SEPARATOR_COLUMN: usize = 30;
const RIGHT_SEPARATOR_COLUMN: usize = 50;
const MIDDLE_COLUMN: usize = LEFT_SEPARATOR_COLUMN + 1;
const MIDDLE_WIDTH: usize = RIGHT_SEPARATOR_COLUMN - MIDDLE_COLUMN;
const CHAIN_ROWS: usize = 18;
const MIDDLE_SEPARATOR_ROW: usize = CHAIN_ROWS;

pub const PHRASE: Area = Area {
    column: 0,
    row: 0,
    width: LEFT_SEPARATOR_COLUMN,
    rows: PANE_ROWS,
};
pub const CHAIN: Area = Area {
    column: MIDDLE_COLUMN,
    row: 0,
    width: MIDDLE_WIDTH,
    rows: CHAIN_ROWS,
};
pub const SONG: Area = Area {
    column: MIDDLE_COLUMN,
    row: MIDDLE_SEPARATOR_ROW + 1,
    width: MIDDLE_WIDTH,
    rows: PANE_ROWS - MIDDLE_SEPARATOR_ROW - 1,
};
pub const RIGHT_SLOT: Area = Area {
    column: RIGHT_SEPARATOR_COLUMN + 1,
    row: 0,
    width: GRID_COLUMNS - RIGHT_SEPARATOR_COLUMN - 1,
    rows: PANE_ROWS,
};

const SEPARATOR: Style = Style::fg(Colour::DarkBlue);

pub(super) fn draw_separators(grid: &mut Grid) {
    for column in [LEFT_SEPARATOR_COLUMN, RIGHT_SEPARATOR_COLUMN] {
        for row in 0..PANE_ROWS {
            grid.put(column, row, "│", SEPARATOR);
        }
    }
    let line = "─".repeat(MIDDLE_WIDTH);
    grid.put(MIDDLE_COLUMN, MIDDLE_SEPARATOR_ROW, &line, SEPARATOR);
    grid.put(LEFT_SEPARATOR_COLUMN, MIDDLE_SEPARATOR_ROW, "├", SEPARATOR);
    grid.put(RIGHT_SEPARATOR_COLUMN, MIDDLE_SEPARATOR_ROW, "┤", SEPARATOR);
}
