use super::{draw_styled_value, draw_title, id_text, nudged_id, title_style, typed_value};
use crate::app::layout::Area;
use crate::screen::grid::Grid;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TitleNumber {
    has_cursor: bool,
    first_digit: Option<u8>,
}

impl TitleNumber {
    pub const fn has_cursor(self) -> bool {
        self.has_cursor
    }

    pub fn set_cursor(&mut self, has_cursor: bool) {
        self.has_cursor = has_cursor;
    }

    pub fn finish_typing(&mut self) {
        self.first_digit = None;
    }

    pub fn type_digit(&mut self, digit: u8) -> u8 {
        typed_value(&mut self.first_digit, digit)
    }

    pub fn nudged(self, shown: Option<u8>, up: bool, coarse: bool) -> u8 {
        nudged_id(shown, up, coarse)
    }

    pub fn draw(self, grid: &mut Grid, area: Area, name: &str, number: Option<u8>, focused: bool) {
        draw_title(grid, area.column, area.row, name, focused);
        let number_column = area.column + name.len() + 1;
        let text = id_text(number);
        let is_cursor = focused && self.has_cursor;
        draw_styled_value(
            grid,
            number_column,
            area.row,
            &text,
            is_cursor,
            title_style(focused),
        );
    }
}
