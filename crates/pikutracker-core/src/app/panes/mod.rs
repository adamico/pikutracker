pub mod cart_view;
pub mod chain;
pub mod file_list;
pub mod instrument;
pub mod log;
pub mod midi_dialog;
pub mod overview;
pub mod phrase;
pub mod song;
pub mod song_picker;

use crate::screen::grid::{Grid, Style};
use crate::screen::palette::Colour;

const TITLE: Style = Style::fg(Colour::Blue);
const FOCUSED_TITLE: Style = Style::fg(Colour::Yellow);
const HEADING: Style = Style::fg(Colour::Lavender);
const EMPTY: Style = Style::fg(Colour::DarkGrey);
const DIMMED: Style = Style::fg(Colour::DarkGrey);
const VALUE: Style = Style::fg(Colour::White);
const CURSOR: Style = Style {
    fg: Colour::Black,
    bg: Colour::Blue,
};
const CURSOR_BRACKET: Style = Style::fg(Colour::Yellow);
const SELECTED: Style = Style {
    fg: Colour::Black,
    bg: Colour::Lavender,
};
const LOOP_MARK: Style = Style::fg(Colour::Pink);
const OUT_OF_RANGE: Style = Style::fg(Colour::Red);
const PLAYHEAD: Style = Style::fg(Colour::Green);
const OUT_OF_RANGE_COLUMN: usize = 14;
const COARSE_ID: i32 = 16;
const MAX_ID: i32 = 255;

fn typed_value(first_digit: &mut Option<u8>, digit: u8) -> u8 {
    let value = first_digit.map_or(0, |high| high << 4) | digit;
    *first_digit = Some(digit);
    value
}

fn nudged_id(number: Option<u8>, up: bool, coarse: bool) -> u8 {
    let amount = if coarse { COARSE_ID } else { 1 };
    let delta = if up { amount } else { -amount };
    number.map_or(0, |number| {
        (i32::from(number) + delta).clamp(0, MAX_ID) as u8
    })
}

fn id_text(number: Option<u8>) -> String {
    number.map_or_else(|| "--".to_owned(), |number| format!("{number:02X}"))
}

fn stepped(position: usize, delta: isize, positions: usize) -> usize {
    position
        .saturating_add_signed(delta)
        .min(positions.saturating_sub(1))
}

fn draw_out_of_range_flag(grid: &mut Grid, column: usize, row: usize) {
    grid.put(column + OUT_OF_RANGE_COLUMN, row, "!", OUT_OF_RANGE);
}

fn draw_row_label(grid: &mut Grid, column: usize, row: usize, label: &str, playing: bool) {
    grid.put(column, row, label, HEADING);
    if playing {
        grid.put(column, row, ">", PLAYHEAD);
    }
}

fn draw_title(grid: &mut Grid, column: usize, row: usize, title: &str, focused: bool) {
    let style = if focused { FOCUSED_TITLE } else { TITLE };
    grid.put(column, row, title, style);
}

fn draw_styled_value(
    grid: &mut Grid,
    column: usize,
    row: usize,
    text: &str,
    is_cursor: bool,
    style: Style,
) {
    if is_cursor {
        grid.put(column - 1, row, "[", CURSOR_BRACKET);
        grid.put(column, row, text, CURSOR);
        grid.put(column + text.chars().count(), row, "]", CURSOR_BRACKET);
    } else {
        grid.put(column, row, text, style);
    }
}
