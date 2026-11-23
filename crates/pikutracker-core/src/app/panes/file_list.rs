use super::{HEADING, VALUE, draw_title, stepped};
use crate::app::Direction;
use crate::app::files::DirectoryEntry;
use crate::app::importing::{ImportKind, PNG_CART_EXTENSION, TEXT_CART_EXTENSION, import_kind};
use crate::app::layout::Area;
use crate::screen::grid::{Grid, Style};
use crate::screen::palette::Colour;
use std::path::{Component, Path, PathBuf};

pub const PROJECT_EXTENSION: &str = ".ptrk";
const PARENT: &str = "..";
const MAX_NAME: usize = 32;
const SLOT_WIDTH: usize = 22;
const NAME_LABEL: &str = "NAME ";
const RECENT_HEADING: &str = "RECENT";

const FOLDER: Style = Style::fg(Colour::Blue);
const POINTER: Style = Style::fg(Colour::Yellow);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Purpose {
    Open,
    SaveAs,
    Import,
    TargetCart,
    LocateCart,
}

impl Purpose {
    pub fn takes_a_name(self) -> bool {
        matches!(self, Self::SaveAs | Self::TargetCart)
    }

    fn lists(self, name: &str) -> bool {
        match self {
            Self::Open | Self::SaveAs => is_project_file(name),
            Self::Import => import_kind(name).is_some(),
            Self::TargetCart | Self::LocateCart => import_kind(name) == Some(ImportKind::Cart),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Choice {
    Stay,
    Folder(PathBuf),
    Open(PathBuf),
    Import(PathBuf),
    TargetCart(PathBuf),
    LocatedCart(PathBuf),
    NewCart(PathBuf),
    RefusedPngNewCart,
    Save { path: PathBuf, exists: bool },
}

#[derive(Debug)]
struct Row {
    name: String,
    is_directory: bool,
}

#[derive(Debug)]
pub struct FileList {
    purpose: Purpose,
    directory: PathBuf,
    rows: Vec<Row>,
    recent_projects: Vec<PathBuf>,
    picked: usize,
    name: String,
    listed: bool,
}

impl FileList {
    pub fn new(purpose: Purpose, directory: PathBuf, name: String) -> Self {
        Self {
            purpose,
            directory,
            rows: vec![parent_row()],
            recent_projects: Vec::new(),
            picked: 0,
            name,
            listed: false,
        }
    }

    pub fn with_recent(mut self, recent: Vec<PathBuf>) -> Self {
        self.recent_projects = recent;
        self
    }

    pub fn purpose(&self) -> Purpose {
        self.purpose
    }

    pub fn show(&mut self, directory: PathBuf, entries: Vec<DirectoryEntry>) {
        let mut rows: Vec<Row> = entries
            .into_iter()
            .filter(|entry| entry.is_directory || self.purpose.lists(&entry.name))
            .map(|entry| Row {
                name: entry.name,
                is_directory: entry.is_directory,
            })
            .collect();
        rows.sort_by_key(|row| (!row.is_directory, row.name.to_lowercase()));
        rows.insert(0, parent_row());
        self.directory = directory;
        self.rows = rows;
        let newest_recent_project = 0;
        let first_folder_row = self.recent_projects.len();
        self.picked = if self.listed {
            first_folder_row
        } else {
            newest_recent_project
        };
        self.listed = true;
    }

    fn name_rows(&self) -> usize {
        usize::from(self.purpose.takes_a_name())
    }

    fn top_rows(&self) -> usize {
        self.name_rows() + self.recent_projects.len()
    }

    pub fn move_cursor(&mut self, direction: Direction) {
        let (_, rows) = direction.column_and_row_delta();
        self.picked = stepped(self.picked, rows, self.rows.len() + self.top_rows());
    }

    pub fn type_char(&mut self, ch: char) {
        let allowed = ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.');
        if self.purpose.takes_a_name() && allowed && self.name.len() < MAX_NAME {
            self.name.push(ch);
            self.picked = 0;
        }
    }

    pub fn backspace(&mut self) {
        self.name.pop();
        self.picked = 0;
    }

    pub fn choose(&mut self) -> Choice {
        let Some(index) = self.picked.checked_sub(self.name_rows()) else {
            return match self.purpose {
                Purpose::TargetCart => self.new_cart_choice(),
                _ => self.save_choice(),
            };
        };
        if let Some(recent) = self.recent_projects.get(index) {
            return Choice::Open(recent.clone());
        }
        let row = &self.rows[index - self.recent_projects.len()];
        if row.name == PARENT {
            Choice::Folder(parent_of(&self.directory))
        } else if row.is_directory {
            Choice::Folder(self.directory.join(&row.name))
        } else {
            match self.purpose {
                Purpose::Open => Choice::Open(self.directory.join(&row.name)),
                Purpose::Import => Choice::Import(self.directory.join(&row.name)),
                Purpose::TargetCart => Choice::TargetCart(self.directory.join(&row.name)),
                Purpose::LocateCart => Choice::LocatedCart(self.directory.join(&row.name)),
                Purpose::SaveAs => {
                    self.name.clone_from(&row.name);
                    self.picked = 0;
                    Choice::Stay
                }
            }
        }
    }

    fn save_choice(&self) -> Choice {
        if self.name.is_empty() {
            return Choice::Stay;
        }
        let file_name = if is_project_file(&self.name) {
            self.name.clone()
        } else {
            format!("{}{PROJECT_EXTENSION}", self.name)
        };
        let exists = self.rows.iter().any(|row| row.name == file_name);
        Choice::Save {
            path: self.directory.join(file_name),
            exists,
        }
    }

    fn new_cart_choice(&self) -> Choice {
        if self.name.is_empty() || !self.listed {
            return Choice::Stay;
        }
        let lower = self.name.to_lowercase();
        if lower.ends_with(PNG_CART_EXTENSION) {
            return Choice::RefusedPngNewCart;
        }
        let file_name = if lower.ends_with(TEXT_CART_EXTENSION) {
            self.name.clone()
        } else {
            format!("{}{TEXT_CART_EXTENSION}", self.name)
        };
        let existing = self
            .rows
            .iter()
            .find(|row| !row.is_directory && row.name.eq_ignore_ascii_case(&file_name));
        match existing {
            Some(row) => Choice::TargetCart(self.directory.join(&row.name)),
            None => Choice::NewCart(self.directory.join(file_name)),
        }
    }

    pub fn draw(&self, grid: &mut Grid, area: Area) {
        let title = match self.purpose {
            Purpose::Open => "OPEN PROJECT",
            Purpose::SaveAs => "SAVE PROJECT AS",
            Purpose::Import => "IMPORT FILE",
            Purpose::TargetCart => "TARGET CART",
            Purpose::LocateCart => "LOCATE CART",
        };
        draw_title(grid, area.column, 0, title, true);
        let mut row = 1;
        if !self.recent_projects.is_empty() {
            grid.put(area.column, row, RECENT_HEADING, HEADING);
            row += 1;
            for (index, path) in self.recent_projects.iter().enumerate() {
                self.draw_pointer(grid, area, row, self.name_rows() + index);
                let name = path.file_name().unwrap_or(path.as_os_str());
                grid.put(area.column + 1, row, &name.to_string_lossy(), VALUE);
                row += 1;
            }
        }
        let directory = self.directory.to_string_lossy();
        grid.put(area.column, row, tail(&directory, SLOT_WIDTH + 1), HEADING);
        row += 1;

        if self.purpose.takes_a_name() {
            self.draw_pointer(grid, area, row, 0);
            let name = format!("{NAME_LABEL}{}_", self.name);
            grid.put(area.column + 1, row, tail(&name, SLOT_WIDTH), VALUE);
            row += 1;
        }

        let visible = area.rows - row;
        let picked_row = self.picked.saturating_sub(self.top_rows());
        let first = (picked_row + 1).saturating_sub(visible);
        for (index, entry) in self.rows.iter().enumerate().skip(first).take(visible) {
            self.draw_pointer(grid, area, row, index + self.top_rows());
            let (text, style) = if entry.is_directory {
                (format!("{}/", entry.name), FOLDER)
            } else {
                (entry.name.clone(), VALUE)
            };
            grid.put(area.column + 1, row, &text, style);
            row += 1;
        }
    }

    fn draw_pointer(&self, grid: &mut Grid, area: Area, row: usize, index: usize) {
        if self.picked == index {
            grid.put(area.column, row, ">", POINTER);
        }
    }
}

fn parent_row() -> Row {
    Row {
        name: PARENT.to_owned(),
        is_directory: true,
    }
}

fn is_project_file(name: &str) -> bool {
    name.to_lowercase().ends_with(PROJECT_EXTENSION)
}

fn parent_of(directory: &Path) -> PathBuf {
    match directory.components().next_back() {
        Some(Component::Normal(_)) => directory
            .parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf),
        _ => directory.join(PARENT),
    }
}

fn tail(text: &str, width: usize) -> &str {
    let extra = text.chars().count().saturating_sub(width);
    text.char_indices()
        .nth(extra)
        .map_or("", |(at, _)| &text[at..])
}
