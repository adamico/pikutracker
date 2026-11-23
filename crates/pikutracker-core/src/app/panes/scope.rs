use super::draw_title;
use crate::app::layout::Area;
use crate::player::{SCOPE_COLUMNS, SCOPE_RANGE};
use crate::screen::grid::{Grid, Style};
use crate::screen::palette::Colour;

const TRACE: Style = Style::fg(Colour::Green);
const FIRST_TRACE_ROW: usize = 1;

pub fn draw(grid: &mut Grid, area: Area, focused: bool, scope: &[i8; SCOPE_COLUMNS]) {
    draw_title(grid, area.column, 0, "SCOPE", focused);
    if scope.iter().all(|value| *value == 0) {
        return;
    }
    let half_height = (area.rows - FIRST_TRACE_ROW - 1) as i32 / 2;
    let middle = FIRST_TRACE_ROW as i32 + half_height;
    for (column, value) in scope.iter().enumerate() {
        let offset = i32::from(*value) * half_height / i32::from(SCOPE_RANGE);
        grid.put(area.column + column, (middle - offset) as usize, "*", TRACE);
    }
}
