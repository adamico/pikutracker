use super::palette::Colour;

pub const GRID_COLUMNS: usize = 106;
pub const GRID_ROWS: usize = 45;
pub const CELL_WIDTH: usize = 6;
pub const CELL_HEIGHT: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Style {
    pub fg: Colour,
    pub bg: Colour,
}

impl Style {
    pub const fn fg(fg: Colour) -> Self {
        Self {
            fg,
            bg: Colour::Black,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub style: Style,
    pub bar: Option<ProgressBar>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProgressBar {
    pub colour: Colour,
    pub pixels: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grid {
    cells: Vec<Cell>,
}

impl Grid {
    pub fn new() -> Self {
        let blank = Cell {
            ch: ' ',
            style: Style::fg(Colour::LightGrey),
            bar: None,
        };
        Self {
            cells: vec![blank; GRID_COLUMNS * GRID_ROWS],
        }
    }

    pub fn cell(&self, column: usize, row: usize) -> Cell {
        assert!(column < GRID_COLUMNS && row < GRID_ROWS);
        self.cells[row * GRID_COLUMNS + column]
    }

    pub fn put(&mut self, column: usize, row: usize, text: &str, style: Style) {
        if row >= GRID_ROWS {
            return;
        }
        for (x, ch) in (column..GRID_COLUMNS).zip(text.chars()) {
            self.cells[row * GRID_COLUMNS + x] = Cell {
                ch,
                style,
                bar: None,
            };
        }
    }

    pub fn restyle(&mut self, column: usize, row: usize, width: usize, style: Style) {
        if row >= GRID_ROWS {
            return;
        }
        for x in column..(column + width).min(GRID_COLUMNS) {
            self.cells[row * GRID_COLUMNS + x].style = style;
        }
    }

    pub fn put_bar(
        &mut self,
        column: usize,
        row: usize,
        width: usize,
        (filled, of): (usize, usize),
        colour: Colour,
    ) {
        if row >= GRID_ROWS || of == 0 {
            return;
        }
        let pixels = width * CELL_WIDTH * filled.min(of) / of;
        for (index, x) in (column..(column + width).min(GRID_COLUMNS)).enumerate() {
            let in_cell = pixels.saturating_sub(index * CELL_WIDTH).min(CELL_WIDTH);
            self.cells[row * GRID_COLUMNS + x].bar = (in_cell > 0).then_some(ProgressBar {
                colour,
                pixels: in_cell as u8,
            });
        }
    }

    pub fn fill_row(&mut self, row: usize, style: Style) {
        self.put(0, row, &" ".repeat(GRID_COLUMNS), style);
    }

    pub fn text_dump(&self) -> String {
        let mut dump = String::new();
        for row in self.cells.chunks(GRID_COLUMNS) {
            let line: String = row.iter().map(|cell| cell.ch).collect();
            dump.push_str(line.trim_end());
            dump.push('\n');
        }
        dump
    }
}

impl Default for Grid {
    fn default() -> Self {
        Self::new()
    }
}
