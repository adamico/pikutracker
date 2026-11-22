use super::font::{GLYPH_HEIGHT, GLYPH_WIDTH, glyph};
use super::grid::{CELL_HEIGHT, CELL_WIDTH, Cell, GRID_COLUMNS, GRID_ROWS, Grid};
use super::palette::Colour;

pub const LOGICAL_WIDTH: u32 = 640;
pub const LOGICAL_HEIGHT: u32 = 360;

const WIDTH: usize = LOGICAL_WIDTH as usize;
const HEIGHT: usize = LOGICAL_HEIGHT as usize;
const SEPARATOR: char = '│';
const SEPARATOR_COLUMN: usize = 2;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    rgba: Vec<u8>,
}

impl Frame {
    pub fn from_grid(grid: &Grid) -> Self {
        let mut frame = Self {
            rgba: Colour::Black.rgba().repeat(WIDTH * HEIGHT),
        };
        for row in 0..GRID_ROWS {
            for column in 0..GRID_COLUMNS {
                frame.draw_cell(
                    column * CELL_WIDTH,
                    row * CELL_HEIGHT,
                    grid.cell(column, row),
                );
            }
        }
        frame
    }

    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    fn draw_cell(&mut self, left: usize, top: usize, cell: Cell) {
        for y in 0..CELL_HEIGHT {
            for x in 0..CELL_WIDTH {
                self.set_pixel(left + x, top + y, cell.style.bg);
            }
        }
        if let Some(bar) = cell.bar {
            for y in 0..CELL_HEIGHT {
                for x in 0..usize::from(bar.pixels).min(CELL_WIDTH) {
                    self.set_pixel(left + x, top + y, bar.colour);
                }
            }
        }

        if cell.ch == SEPARATOR {
            for y in 0..CELL_HEIGHT {
                self.set_pixel(left + SEPARATOR_COLUMN, top + y, cell.style.fg);
            }
            return;
        }

        for (y, bits) in glyph(cell.ch).into_iter().enumerate().take(GLYPH_HEIGHT) {
            for x in 0..GLYPH_WIDTH {
                if bits & (1 << (GLYPH_WIDTH - 1 - x)) != 0 {
                    self.set_pixel(left + x, top + y, cell.style.fg);
                }
            }
        }
    }

    fn set_pixel(&mut self, x: usize, y: usize, colour: Colour) {
        let offset = (y * WIDTH + x) * 4;
        self.rgba[offset..offset + 4].copy_from_slice(&colour.rgba());
    }
}
