use super::{EMPTY, HEADING, INSTRUMENT_NAMES, VALUE, stepped};
use crate::app::{Action, Direction};
use crate::midi_import::{
    self, Analysis, Choices, DRUM_ROWS, GridOption, Imported, MidiTable, STEPS_PER_BEAT,
    family_name,
};
use crate::project::TRACKS;
use crate::screen::grid::{Grid, Style};
use crate::screen::palette::Colour;
use std::path::PathBuf;

const PAGE_NAMES: [&str; 4] = ["1 VOICES", "2 STEPS PER BEAT", "3 INSTRUMENTS", "4 REVIEW"];
const PAGE_GAP: &str = " > ";

const HEADER_ROW: usize = 0;
const FILE_ROW: usize = 1;
const TITLE_ROW: usize = 3;
const FIRST_ROW: usize = 5;
const LIST_ROWS: usize = 38;
const SECOND_COLUMN: usize = 58;

const CURRENT_PAGE: Style = Style {
    fg: Colour::Black,
    bg: Colour::Yellow,
};
const OTHER_PAGE: Style = Style::fg(Colour::Lavender);
const POINTER: Style = Style::fg(Colour::Yellow);
const DROPPED: Style = Style::fg(Colour::Red);
const WARNING: Style = Style::fg(Colour::Orange);

const VOICES_KEYS: &str = "UP/DOWN PICK  SPACE TICK  S TICK SOURCE  ENTER NEXT  ESC CANCEL";
const GRID_KEYS: &str = "ARROWS PICK  ENTER NEXT  ESC BACK";
const PROGRAM_KEYS: &str =
    "TAB TABLE  LEFT/RIGHT INSTRUMENT  R RESET  SHIFT+R RESET ALL  ENTER NEXT  ESC BACK";
const DRUM_KEYS: &str =
    "TAB TABLE  LEFT/RIGHT INSTRUMENT  [ ] NOTE  F EFFECT  R RESET  SHIFT+R RESET ALL  ESC BACK";
const REVIEW_KEYS: &str = "UP/DOWN SCROLL  ENTER IMPORT  ESC BACK";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Page {
    Voices,
    Grid,
    Instruments,
    Review,
}

impl Page {
    const fn index(self) -> usize {
        match self {
            Self::Voices => 0,
            Self::Grid => 1,
            Self::Instruments => 2,
            Self::Review => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Table {
    Programs,
    Drums,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Stay,
    Cancel,
    Import,
}

#[derive(Debug)]
pub struct MidiDialog {
    file_name: String,
    folder: PathBuf,
    analysis: Analysis,
    choices: Choices,
    page: Page,
    voice: usize,
    grids: Vec<GridOption>,
    table: Table,
    programs: Vec<u8>,
    program_row: usize,
    drum_row: usize,
    preview: Option<Imported>,
    warning_scroll: usize,
}

impl MidiDialog {
    pub fn new(file_name: String, folder: PathBuf, analysis: Analysis, table: MidiTable) -> Self {
        let choices = Choices {
            table,
            ..Choices::defaults_for(&analysis)
        };
        let grids = midi_import::compare_grids(&analysis, &choices);
        let programs = analysis.programs();
        Self {
            file_name,
            folder,
            analysis,
            choices,
            page: Page::Voices,
            voice: 0,
            grids,
            table: Table::Programs,
            programs,
            program_row: 0,
            drum_row: 0,
            preview: None,
            warning_scroll: 0,
        }
    }

    pub fn finish(self) -> (String, PathBuf, Imported, MidiTable) {
        let imported = self
            .preview
            .unwrap_or_else(|| midi_import::build(&self.analysis, &self.choices));
        (self.file_name, self.folder, imported, self.choices.table)
    }

    pub fn hints(&self) -> &'static str {
        match (self.page, self.table) {
            (Page::Voices, _) => VOICES_KEYS,
            (Page::Grid, _) => GRID_KEYS,
            (Page::Instruments, Table::Programs) => PROGRAM_KEYS,
            (Page::Instruments, Table::Drums) => DRUM_KEYS,
            (Page::Review, _) => REVIEW_KEYS,
        }
    }

    pub fn apply(&mut self, action: Action) -> Outcome {
        match action {
            Action::Confirm => return self.forward(),
            Action::Cancel => return self.back(),
            _ => {}
        }
        match self.page {
            Page::Voices => self.use_voices(action),
            Page::Grid => self.use_grid(action),
            Page::Instruments => self.use_instruments(action),
            Page::Review => self.use_review(action),
        }
        Outcome::Stay
    }

    fn forward(&mut self) -> Outcome {
        self.page = match self.page {
            Page::Voices => Page::Grid,
            Page::Grid => Page::Instruments,
            Page::Instruments => {
                self.preview = Some(midi_import::build(&self.analysis, &self.choices));
                self.warning_scroll = 0;
                Page::Review
            }
            Page::Review => return Outcome::Import,
        };
        Outcome::Stay
    }

    fn back(&mut self) -> Outcome {
        self.page = match self.page {
            Page::Voices => return Outcome::Cancel,
            Page::Grid => Page::Voices,
            Page::Instruments => Page::Grid,
            Page::Review => Page::Instruments,
        };
        Outcome::Stay
    }

    fn use_voices(&mut self, action: Action) {
        match action {
            Action::Move(direction) => {
                let (_, rows) = direction.column_and_row_delta();
                self.voice = stepped(self.voice, rows, self.choices.ticked.len());
            }
            Action::ToggleVoice => {
                self.choices.ticked[self.voice] = !self.choices.ticked[self.voice];
                self.ticks_changed();
            }
            Action::ToggleSource => {
                let source = self.analysis.voices()[self.voice].source();
                let members: Vec<usize> = (0..self.choices.ticked.len())
                    .filter(|index| self.analysis.voices()[*index].source() == source)
                    .collect();
                let tick = !members.iter().all(|index| self.choices.ticked[*index]);
                for index in members {
                    self.choices.ticked[index] = tick;
                }
                self.ticks_changed();
            }
            _ => {}
        }
    }

    fn ticks_changed(&mut self) {
        self.grids = midi_import::compare_grids(&self.analysis, &self.choices);
    }

    fn use_grid(&mut self, action: Action) {
        let Action::Move(direction) = action else {
            return;
        };
        let delta = match direction {
            Direction::Up | Direction::Left => -1,
            Direction::Down | Direction::Right => 1,
        };
        let index = STEPS_PER_BEAT
            .iter()
            .position(|option| *option == self.choices.steps_per_beat)
            .unwrap_or(0);
        self.choices.steps_per_beat = STEPS_PER_BEAT[stepped(index, delta, STEPS_PER_BEAT.len())];
    }

    fn use_instruments(&mut self, action: Action) {
        match action {
            Action::SwitchMidiTable => {
                self.table = match self.table {
                    Table::Programs => Table::Drums,
                    Table::Drums => Table::Programs,
                };
            }
            Action::ResetMidiTable => self.choices.table = Default::default(),
            _ => match self.table {
                Table::Programs => self.use_programs(action),
                Table::Drums => self.use_drums(action),
            },
        }
    }

    fn use_programs(&mut self, action: Action) {
        if let Action::Move(direction @ (Direction::Up | Direction::Down)) = action {
            let (_, rows) = direction.column_and_row_delta();
            self.program_row = stepped(self.program_row, rows, self.programs.len());
            return;
        }
        let Some(program) = self.programs.get(self.program_row).copied() else {
            return;
        };
        match action {
            Action::Move(Direction::Left) => self.choices.table.change_program(program, false),
            Action::Move(Direction::Right) => self.choices.table.change_program(program, true),
            Action::ResetMidiRow => self.choices.table.reset_program(program),
            _ => {}
        }
    }

    fn use_drums(&mut self, action: Action) {
        let row = self.drum_row;
        let table = &mut self.choices.table;
        match action {
            Action::Move(Direction::Up) => self.drum_row = stepped(row, -1, DRUM_ROWS),
            Action::Move(Direction::Down) => self.drum_row = stepped(row, 1, DRUM_ROWS),
            Action::Move(Direction::Left) => table.change_hit_instrument(row, false),
            Action::Move(Direction::Right) => table.change_hit_instrument(row, true),
            Action::MidiDrumNote { up } => table.change_hit_note(row, up),
            Action::CycleDrumEffect => table.cycle_hit_effect(row),
            Action::ResetMidiRow => table.reset_hit(row),
            _ => {}
        }
    }

    fn use_review(&mut self, action: Action) {
        let Action::Move(direction) = action else {
            return;
        };
        let count = self.preview.as_ref().map_or(0, |p| p.warnings.len());
        let (_, rows) = direction.column_and_row_delta();
        self.warning_scroll = stepped(self.warning_scroll, rows, count);
    }

    pub fn draw(&self, grid: &mut Grid) {
        self.draw_header(grid);
        match self.page {
            Page::Voices => self.draw_voices(grid),
            Page::Grid => self.draw_grids(grid),
            Page::Instruments => self.draw_instruments(grid),
            Page::Review => self.draw_review(grid),
        }
    }

    fn draw_header(&self, grid: &mut Grid) {
        let mut column = 0;
        for (index, name) in PAGE_NAMES.into_iter().enumerate() {
            if index > 0 {
                grid.put(column, HEADER_ROW, PAGE_GAP, OTHER_PAGE);
                column += PAGE_GAP.len();
            }
            let style = if index == self.page.index() {
                CURRENT_PAGE
            } else {
                OTHER_PAGE
            };
            grid.put(column, HEADER_ROW, name, style);
            column += name.len();
        }
        let file = format!("MIDI IMPORT: {}", self.file_name);
        grid.put(0, FILE_ROW, &file, HEADING);
    }

    fn track_names(&self) -> [Option<&str>; TRACKS] {
        let mut names = [None; TRACKS];
        for (voice, track) in self.analysis.voices().iter().zip(self.choices.tracks()) {
            if let Some(track) = track {
                names[track] = Some(voice.name());
            }
        }
        names
    }

    fn draw_track_list(&self, grid: &mut Grid, column: usize, first_row: usize) {
        for (track, name) in self.track_names().into_iter().enumerate() {
            let row = first_row + track;
            let text = format!("T{} {}", track + 1, name.unwrap_or("(EMPTY)"));
            grid.put(
                column,
                row,
                &text,
                if name.is_some() { VALUE } else { EMPTY },
            );
        }
    }

    fn draw_voices(&self, grid: &mut Grid) {
        grid.put(
            2,
            TITLE_ROW,
            "TICK VOICE            CH NOTES PROG TRACK",
            HEADING,
        );
        grid.put(SECOND_COLUMN, TITLE_ROW, "TRACKS WILL HOLD", HEADING);
        let tracks = self.choices.tracks();
        let first = (self.voice + 1).saturating_sub(LIST_ROWS);
        let voices = self.analysis.voices().iter().enumerate();
        for (row, (index, voice)) in voices.skip(first).take(LIST_ROWS).enumerate() {
            let row = FIRST_ROW + row;
            if index == self.voice {
                grid.put(0, row, ">", POINTER);
            }
            let ticked = self.choices.ticked[index];
            let program = voice
                .program()
                .map_or_else(|| "DRUM".to_owned(), |program| format!("{program:03}"));
            let text = format!(
                "[{}]  {:<16} {:>2} {:>5} {:<4} ",
                if ticked { "X" } else { " " },
                voice.name(),
                voice.channel(),
                voice.note_count(),
                program,
            );
            grid.put(2, row, &text, if ticked { VALUE } else { EMPTY });
            let track_column = 2 + text.chars().count();
            match tracks[index] {
                Some(track) => grid.put(track_column, row, &format!("T{}", track + 1), VALUE),
                None if ticked => grid.put(track_column, row, "DROP", DROPPED),
                None => grid.put(track_column, row, "--", EMPTY),
            }
        }
        self.draw_track_list(grid, SECOND_COLUMN, FIRST_ROW);

        let dropped = self
            .choices
            .ticked
            .iter()
            .zip(&tracks)
            .filter(|(ticked, track)| **ticked && track.is_none())
            .count();
        if dropped > 0 {
            let text = if dropped == 1 {
                "1 VOICE DROPPED: UNTICK OTHERS TO KEEP IT".to_owned()
            } else {
                format!("{dropped} VOICES DROPPED: UNTICK OTHERS TO KEEP THEM")
            };
            grid.put(SECOND_COLUMN, FIRST_ROW + TRACKS + 1, &text, DROPPED);
        }
    }

    fn draw_grids(&self, grid: &mut Grid) {
        grid.put(
            2,
            TITLE_ROW,
            "STEPS/BEAT  SPEED  PHRASE  BARS  COLLIDING NOTES",
            HEADING,
        );
        for (index, option) in self.grids.iter().enumerate() {
            let picked = option.steps_per_beat == self.choices.steps_per_beat;
            let pointer = if picked { ">" } else { " " };
            let bars = if option.bars == 0 {
                "<1".to_owned()
            } else {
                option.bars.to_string()
            };
            let text = format!(
                "{pointer} {:<10}  {:02X}     {:<6}  {:<4}  {}",
                option.steps_per_beat, option.speed, option.phrase_length, bars, option.collisions
            );
            let row = FIRST_ROW + index * 2;
            grid.put(0, row, &text, if picked { VALUE } else { HEADING });
            if picked {
                grid.put(0, row, pointer, POINTER);
            }
        }
    }

    fn draw_instruments(&self, grid: &mut Grid) {
        let table = &self.choices.table;
        grid.put(2, TITLE_ROW, "PROGRAMS IN THIS FILE", HEADING);
        for (index, program) in self.programs.iter().take(LIST_ROWS).enumerate() {
            let row = FIRST_ROW + index;
            if self.table == Table::Programs && index == self.program_row {
                grid.put(0, row, ">", POINTER);
            }
            let name = INSTRUMENT_NAMES[usize::from(table.instrument(*program))];
            let mark = if table.program_is_edited(*program) {
                " *"
            } else {
                ""
            };
            let text = format!("{program:03} {:<11}{name}{mark}", family_name(*program));
            grid.put(2, row, &text, VALUE);
        }

        grid.put(SECOND_COLUMN + 2, TITLE_ROW, "DRUM KIT", HEADING);
        let columns = "HIT      NOTE FX INSTRUMENT";
        grid.put(SECOND_COLUMN + 2, TITLE_ROW + 1, columns, HEADING);
        for index in 0..DRUM_ROWS {
            let row = FIRST_ROW + index;
            if self.table == Table::Drums && index == self.drum_row {
                grid.put(SECOND_COLUMN, row, ">", POINTER);
            }
            let hit = table.hit(index);
            let name = INSTRUMENT_NAMES[usize::from(hit.instrument)];
            let mark = if table.hit_is_edited(index) { " *" } else { "" };
            let text = format!(
                "{:<8} {:02X}   {}  {name}{mark}",
                hit.name, hit.note, hit.effect
            );
            grid.put(SECOND_COLUMN + 2, row, &text, VALUE);
        }
    }

    fn draw_review(&self, grid: &mut Grid) {
        grid.put(0, TITLE_ROW, "TRACKS", HEADING);
        self.draw_track_list(grid, 0, FIRST_ROW);

        grid.put(SECOND_COLUMN, TITLE_ROW, "SETTINGS", HEADING);
        let chosen = self
            .grids
            .iter()
            .find(|option| option.steps_per_beat == self.choices.steps_per_beat);
        if let Some(option) = chosen {
            let beats = format!("STEPS PER BEAT {}", option.steps_per_beat);
            grid.put(SECOND_COLUMN, FIRST_ROW, &beats, VALUE);
            grid.put(SECOND_COLUMN, FIRST_ROW + 1, &readout(option), VALUE);
        }
        let table = if self.choices.table.is_edited() {
            "TABLE EDITED"
        } else {
            "TABLE FACTORY"
        };
        grid.put(SECOND_COLUMN, FIRST_ROW + 2, table, VALUE);

        let warnings = self.preview.as_ref().map_or(&[][..], |p| &p.warnings);
        let title_row = FIRST_ROW + TRACKS + 1;
        let title = format!("WARNINGS ({})", warnings.len());
        grid.put(0, title_row, &title, HEADING);
        let rows = LIST_ROWS.saturating_sub(TRACKS + 2);
        let shown = warnings.iter().skip(self.warning_scroll).take(rows);
        for (offset, warning) in shown.enumerate() {
            grid.put(0, title_row + 1 + offset, &warning.text, WARNING);
        }
    }
}

fn readout(option: &GridOption) -> String {
    let bars = match option.bars {
        0 => "LESS THAN A BAR".to_owned(),
        1 => "1 BAR".to_owned(),
        bars => format!("{bars} BARS"),
    };
    format!(
        "SPEED {:02X}  {} STEPS = {bars} A PHRASE",
        option.speed, option.phrase_length
    )
}
