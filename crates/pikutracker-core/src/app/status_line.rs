use super::layout::STATUS_ROW;
use super::messages::{Look, Notice};
use crate::screen::grid::{GRID_COLUMNS, Grid, Style};
use crate::screen::palette::Colour;

const BAR: Style = Style {
    fg: Colour::LightGrey,
    bg: Colour::DarkBlue,
};
const INFO: Style = Style {
    fg: Colour::Black,
    bg: Colour::Green,
};
const FADING_INFO: Style = Style {
    fg: Colour::LightGrey,
    bg: Colour::DarkGreen,
};
const WARNING: Style = Style {
    fg: Colour::Black,
    bg: Colour::Orange,
};
const FAILURE: Style = Style {
    fg: Colour::White,
    bg: Colour::Red,
};
const QUESTION: Style = Style {
    fg: Colour::White,
    bg: Colour::Blue,
};

pub enum StatusLine<'a> {
    Question(String),
    Notice(&'a Notice),
    Remark(&'a str),
    Hints(String),
    Idle { project: &'a str, unsaved: bool },
}

pub fn draw(grid: &mut Grid, status: &StatusLine<'_>) {
    match status {
        StatusLine::Question(text) => draw_bar(grid, text, QUESTION),
        StatusLine::Notice(notice) => draw_bar(grid, notice.text(), notice_style(notice)),
        StatusLine::Remark(text) => draw_bar(grid, text, BAR),
        StatusLine::Hints(text) => draw_bar(grid, text, BAR),
        StatusLine::Idle { project, unsaved } => {
            draw_bar(grid, "", BAR);
            let name = format!("{project}{}", if *unsaved { "*" } else { "" });
            let column = GRID_COLUMNS.saturating_sub(name.chars().count());
            grid.put(column, STATUS_ROW, &name, BAR);
        }
    }
}

fn draw_bar(grid: &mut Grid, text: &str, style: Style) {
    grid.fill_row(STATUS_ROW, style);
    grid.put(0, STATUS_ROW, text, style);
}

fn notice_style(notice: &Notice) -> Style {
    match notice.look() {
        Look::Info => INFO,
        Look::FadingInfo => FADING_INFO,
        Look::Warning => WARNING,
        Look::Failure => FAILURE,
    }
}
