use super::{HEADING, VALUE, draw_title, stepped};
use crate::app::Direction;
use crate::app::layout::Area;
use crate::app::messages::{Location, LogItem, Severity};
use crate::screen::grid::{Grid, Style};
use crate::screen::palette::Colour;

const MAX_ITEMS: usize = 32;
const FIRST_ITEM_ROW: usize = 1;
pub(super) const TAG_COLUMN: usize = 1;
pub(super) const TEXT_COLUMN: usize = 5;
pub(super) const TEXT_WIDTH: usize = 18;

const INFO_TAG: Style = Style::fg(Colour::Green);
const WARNING_TAG: Style = Style::fg(Colour::Orange);
const FAILURE_TAG: Style = Style::fg(Colour::Red);
const POINTER: Style = Style::fg(Colour::Yellow);

#[derive(Debug, Default)]
pub struct LogPane {
    items: Vec<LogItem>,
    picked: usize,
}

impl LogPane {
    pub fn add(&mut self, items: Vec<LogItem>) {
        self.items.splice(0..0, items);
        self.items.truncate(MAX_ITEMS);
        self.picked = 0;
    }

    pub fn move_cursor(&mut self, direction: Direction) {
        let (_, rows) = direction.column_and_row_delta();
        self.picked = stepped(self.picked, rows, self.items.len());
    }

    pub fn picked_location(&self) -> Option<Location> {
        self.items.get(self.picked)?.location
    }

    pub fn draw(&self, grid: &mut Grid, area: Area, focused: bool) {
        draw_title(grid, area.column, 0, "LOG", focused);

        let blocks: Vec<Vec<String>> = self.items.iter().map(item_lines).collect();
        let rows = area.rows - FIRST_ITEM_ROW;
        let mut row = FIRST_ITEM_ROW;
        let first = first_visible(&blocks, self.picked, rows);
        for (index, lines) in blocks.iter().enumerate().skip(first) {
            if row + lines.len() > area.rows {
                break;
            }
            let item = &self.items[index];
            if focused && index == self.picked {
                grid.put(area.column, row, ">", POINTER);
            }
            grid.put(
                area.column + TAG_COLUMN,
                row,
                tag(item.severity),
                tag_style(item.severity),
            );
            for (offset, line) in lines.iter().enumerate() {
                let is_hint = item.hint.is_some() && offset + 1 == lines.len();
                let style = if is_hint { HEADING } else { VALUE };
                grid.put(area.column + TEXT_COLUMN, row + offset, line, style);
            }
            row += lines.len();
        }
    }
}

fn first_visible(blocks: &[Vec<String>], picked: usize, rows: usize) -> usize {
    let mut first = 0;
    while first < picked && blocks[first..=picked].iter().map(Vec::len).sum::<usize>() > rows {
        first += 1;
    }
    first
}

fn item_lines(item: &LogItem) -> Vec<String> {
    let mut lines = wrapped(&item.text, TEXT_WIDTH);
    lines.extend(item.hint.clone());
    lines
}

pub(super) const fn tag(severity: Severity) -> &'static str {
    match severity {
        Severity::Info => "OK",
        Severity::Warning => "WRN",
        Severity::Failure => "ERR",
    }
}

pub(super) const fn tag_style(severity: Severity) -> Style {
    match severity {
        Severity::Info => INFO_TAG,
        Severity::Warning => WARNING_TAG,
        Severity::Failure => FAILURE_TAG,
    }
}

pub(super) fn wrapped(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let mut word: Vec<char> = word.chars().collect();
        while word.len() > width {
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            lines.push(word.drain(..width).collect());
        }
        let needed = line.chars().count() + 1 + word.len();
        if !line.is_empty() && needed > width {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.extend(word);
    }
    if !line.is_empty() || lines.is_empty() {
        lines.push(line);
    }
    lines
}
