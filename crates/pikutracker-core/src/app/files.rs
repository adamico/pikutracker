use super::file_conversations::Asked;
use super::messages::Report;
use super::overlays::Overlay;
use super::panes::file_list::{Choice, FileList, Purpose};
use super::paths::{relative_path, resolved};
use super::{Action, App, Focus};
use crate::export::Exported;
use crate::project::Project;
use crate::project_file;
use std::path::{Path, PathBuf};

const UNTITLED: &str = "Untitled";
const ONLY_P8_NEW_CARTS: &str = "A NEW CART MUST BE A .P8 FILE, NOT .P8.PNG";
const NOT_FOUND: &str = "not found";
pub(super) const OVERWRITE: &str = "OVERWRITE";
const UNSAVED_QUESTION: &str = "UNSAVED CHANGES WILL BE LOST";
const DISCARD: &str = "DISCARD";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileRequest {
    List { directory: PathBuf },
    Read { path: PathBuf },
    ReadBytes { path: PathBuf },
    Write { path: PathBuf, contents: String },
    WriteBytes { path: PathBuf, contents: Vec<u8> },
    Rename { from: PathBuf, to: PathBuf },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectoryEntry {
    pub name: String,
    pub is_directory: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileResponse {
    Listed {
        directory: PathBuf,
        entries: Vec<DirectoryEntry>,
    },
    Read {
        path: PathBuf,
        contents: String,
    },
    BytesRead {
        path: PathBuf,
        contents: Vec<u8>,
    },
    Written {
        path: PathBuf,
    },
    Renamed {
        path: PathBuf,
    },
    Missing {
        path: PathBuf,
    },
    Failed {
        path: PathBuf,
        reason: String,
    },
}

#[derive(Debug)]
pub(super) enum Accepted {
    NewProject,
    BrowseToOpen,
    BrowseToImport,
    Quit,
    SaveOver(PathBuf),
    PurgeUnused,
    OverwriteCart(PathBuf, Box<Exported>),
    LocateCart,
}

#[derive(Debug)]
pub(super) struct Question {
    pub(super) text: String,
    pub(super) answer: &'static str,
    accepted: Accepted,
}

impl Question {
    pub(super) fn purge_unused(text: String, answer: &'static str) -> Self {
        Self {
            text,
            answer,
            accepted: Accepted::PurgeUnused,
        }
    }
}

pub(super) fn cannot_use(path: &Path, reason: &str) -> String {
    format!("CANNOT USE {}: {reason}", shown_name(path))
}

pub(super) fn shown_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .to_uppercase()
}

pub(super) fn folder_of(path: &Path) -> PathBuf {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

impl App {
    pub fn has_unsaved_changes(&self) -> bool {
        self.project != self.saved_project
    }

    pub fn take_file_requests(&mut self) -> Vec<FileRequest> {
        self.files.take_requests()
    }

    pub fn receive_file(&mut self, response: FileResponse) {
        if let Some(asked) = self.files.answer(&response) {
            self.hear_back(response, asked);
        }
        self.announce_help_when_free();
    }

    fn hear_back(&mut self, response: FileResponse, asked: Asked) {
        match (response, asked) {
            (FileResponse::Listed { directory, entries }, Asked::Listing) => {
                if let Some(list) = self.overlays.file_list() {
                    list.show(directory, entries);
                }
            }
            (FileResponse::Read { path, contents }, Asked::Opening) => {
                self.finish_opening(path, &contents);
            }
            (FileResponse::BytesRead { path, contents }, asked) => match asked {
                Asked::Importing => self.finish_importing(path, &contents),
                Asked::ExportReading => self.finish_reading_target(path, &contents),
                Asked::CartViewReading => self.show_cart_in_view(&contents),
                Asked::PickingCart => self.finish_picking_cart(&path, &contents),
                _ => {}
            },
            (FileResponse::Written { path }, asked) => match asked {
                Asked::Saving(project) => self.finish_saving(path, *project),
                Asked::ExportWriting(finished) => self.finish_writing_target(finished),
                Asked::MakingCart => self.finish_making_cart(&path),
                _ => {}
            },
            (FileResponse::Renamed { .. }, Asked::SettingAsideState { state_file }) => {
                self.state_file = Some(state_file);
            }
            (FileResponse::Missing { path }, Asked::ExportReading) => self.ask_to_locate(&path),
            (FileResponse::Missing { path }, asked) => self.file_failed(&path, NOT_FOUND, asked),
            (FileResponse::Failed { path, reason }, asked) => {
                self.file_failed(&path, &reason, asked);
            }
            _ => {}
        }
    }

    fn finish_opening(&mut self, path: PathBuf, contents: &str) {
        self.overlays.close_file_list();
        let name = shown_name(&path);
        match project_file::parse(contents) {
            Ok(project) => {
                self.directory = folder_of(&path);
                self.replace_project(project, Some(path.clone()));
                self.report(Report::info(format!("OPENED {name}")));
                self.remember_project(path);
            }
            Err(error) => self.report(Report::failure(format!(
                "CANNOT OPEN {name}: {}",
                error.to_string().to_uppercase()
            ))),
        }
    }

    fn finish_saving(&mut self, path: PathBuf, project: Project) {
        self.adopt_saved_cart_path(&project);
        self.saved_project = project;
        self.directory = folder_of(&path);
        self.report(Report::info(format!("SAVED {}", shown_name(&path))));
        self.project_path = Some(path.clone());
        self.remember_project(path);
    }

    fn file_failed(&mut self, path: &Path, reason: &str, asked: Asked) {
        let reason = reason.to_uppercase();
        let name = shown_name(path);
        let summary = match asked {
            Asked::Listing => format!("CANNOT LIST {}: {reason}", path.display()),
            Asked::Opening => {
                self.overlays.close_file_list();
                format!("CANNOT OPEN {name}: {reason}")
            }
            Asked::Importing => {
                self.overlays.close_file_list();
                format!("CANNOT IMPORT {name}: {reason}")
            }
            Asked::Saving(_) => format!("CANNOT SAVE {name}: {reason}"),
            Asked::ExportReading | Asked::ExportWriting(_) => {
                format!("CANNOT EXPORT TO {name}: {reason}")
            }
            Asked::CartViewReading => {
                self.show_unreadable_cart_in_view(reason);
                return;
            }
            Asked::PickingCart => cannot_use(path, &reason),
            Asked::MakingCart => format!("CANNOT CREATE {name}: {reason}"),
            Asked::SettingAsideState { .. } => format!(
                "CANNOT RENAME {name} TO {name}.BAD: {reason}. RECENT PROJECTS ARE NOT SAVED THIS SESSION"
            ),
            Asked::SettingAsideConfig => format!("CANNOT RENAME {name} TO {name}.BAD: {reason}"),
        };
        self.report(Report::failure(summary));
    }

    pub(super) fn ask(&mut self, text: String, answer: &'static str, accepted: Accepted) {
        self.overlays.open(Overlay::Question(Question {
            text,
            answer,
            accepted,
        }));
    }

    pub(super) fn replace_project(&mut self, project: Project, path: Option<PathBuf>) {
        self.transport.reset();
        self.saved_project = project.clone();
        self.project = project;
        self.history.clear();
        self.overlays.reset_for_new_project();
        self.selection_start = None;
        self.clipboard = None;
        self.project_path = path;
        self.song = Default::default();
        self.chain = Default::default();
        self.phrase.return_to_top();
        self.multitrack = Default::default();
        self.shown = Default::default();
        self.show_what_the_cursors_point_at();
        self.focus = Focus::Song;
        self.played_pane = Focus::Song;
    }

    pub(super) fn new_project(&mut self) {
        if self.has_unsaved_changes() {
            self.ask_about_unsaved_changes(Accepted::NewProject);
        } else {
            self.replace_project(Project::new(), None);
        }
    }

    pub(super) fn open_project(&mut self) {
        if self.has_unsaved_changes() {
            self.ask_about_unsaved_changes(Accepted::BrowseToOpen);
        } else {
            self.browse(Purpose::Open);
        }
    }

    pub(super) fn import(&mut self) {
        if self.has_unsaved_changes() {
            self.ask_about_unsaved_changes(Accepted::BrowseToImport);
        } else {
            self.browse(Purpose::Import);
        }
    }

    pub(super) fn quit(&mut self) {
        if self.has_unsaved_changes() {
            self.ask_about_unsaved_changes(Accepted::Quit);
        } else {
            self.quitting = true;
        }
    }

    pub fn wants_to_quit(&self) -> bool {
        self.quitting
    }

    fn ask_about_unsaved_changes(&mut self, accepted: Accepted) {
        self.ask(UNSAVED_QUESTION.to_owned(), DISCARD, accepted);
    }

    pub(super) fn save(&mut self) {
        match self.project_path.clone() {
            Some(path) => self.write_project(path),
            None => self.save_as(),
        }
    }

    pub(super) fn save_as(&mut self) {
        self.browse(Purpose::SaveAs);
    }

    pub(super) fn browse(&mut self, purpose: Purpose) {
        let name = match purpose {
            Purpose::TargetCart => self.new_cart_name(),
            _ => self
                .project_path
                .as_deref()
                .and_then(Path::file_name)
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned()),
        };
        let directory = match purpose {
            Purpose::TargetCart | Purpose::LocateCart => self.target_cart_folder().to_path_buf(),
            Purpose::Open | Purpose::SaveAs | Purpose::Import => self.directory.clone(),
        };
        let recent = match purpose {
            Purpose::Open => self
                .app_state
                .recent_projects()
                .iter()
                .filter(|recent| Some(recent.as_path()) != self.project_path.as_deref())
                .cloned()
                .collect(),
            Purpose::SaveAs | Purpose::Import | Purpose::TargetCart | Purpose::LocateCart => {
                Vec::new()
            }
        };
        let list = FileList::new(purpose, directory.clone(), name).with_recent(recent);
        self.overlays.open(Overlay::FileList(list));
        self.list_folder(directory);
    }

    fn list_folder(&mut self, directory: PathBuf) {
        self.files
            .ask(FileRequest::List { directory }, Asked::Listing);
    }

    fn project_saved_in(&self, folder: &Path) -> Project {
        let mut project = self.project.clone();
        let moved = resolved(self.target_cart_folder()) != resolved(folder);
        if let Some(mut target) = self.project.target_cart().cloned().filter(|_| moved) {
            let cart = self.target_cart_path(&target.path);
            if let Some(path) = relative_path(folder, &cart) {
                target.path = path.to_string_lossy().into_owned();
                project.set_target_cart(Some(target));
            }
        }
        project
    }

    fn adopt_saved_cart_path(&mut self, saved: &Project) {
        if let (Some(saved), Some(current)) = (saved.target_cart(), self.project.target_cart()) {
            let mut current = current.clone();
            current.path.clone_from(&saved.path);
            self.project.set_target_cart(Some(current));
        }
    }

    fn write_project(&mut self, path: PathBuf) {
        self.overlays.close_file_list();
        let project = self.project_saved_in(&folder_of(&path));
        let contents = project_file::write(&project);
        self.files.ask(
            FileRequest::Write { path, contents },
            Asked::Saving(Box::new(project)),
        );
    }

    pub(super) fn answer_question(&mut self, action: Action) {
        match action {
            Action::Cancel => {
                self.overlays.close_question();
            }
            Action::Confirm => {
                if let Some(question) = self.overlays.take_question() {
                    match question.accepted {
                        Accepted::NewProject => self.replace_project(Project::new(), None),
                        Accepted::BrowseToOpen => self.browse(Purpose::Open),
                        Accepted::BrowseToImport => self.browse(Purpose::Import),
                        Accepted::Quit => self.quitting = true,
                        Accepted::SaveOver(path) => self.write_project(path),
                        Accepted::PurgeUnused => self.purge_unused(),
                        Accepted::OverwriteCart(path, exported) => {
                            self.write_target(path, *exported);
                        }
                        Accepted::LocateCart => self.browse(Purpose::LocateCart),
                    }
                }
            }
            _ => {}
        }
    }

    pub(super) fn use_file_list(&mut self, action: Action) {
        let Some(list) = self.overlays.file_list() else {
            return;
        };
        match action {
            Action::Move(direction) => list.move_cursor(direction),
            Action::TypeChar(ch) => list.type_char(ch),
            Action::Backspace => list.backspace(),
            Action::Cancel => {
                self.overlays.close_file_list();
            }
            Action::Confirm => {
                let choice = list.choose();
                self.follow_choice(choice);
            }
            _ => {}
        }
    }

    fn follow_choice(&mut self, choice: Choice) {
        match choice {
            Choice::Stay => {}
            Choice::Folder(directory) => self.list_folder(directory),
            Choice::Open(path) => self.files.ask(FileRequest::Read { path }, Asked::Opening),
            Choice::Import(path) => self
                .files
                .ask(FileRequest::ReadBytes { path }, Asked::Importing),
            Choice::TargetCart(path) => self
                .files
                .ask(FileRequest::ReadBytes { path }, Asked::PickingCart),
            Choice::LocatedCart(path) => self.locate_target_cart(&path),
            Choice::NewCart(path) => self.make_new_cart(path),
            Choice::RefusedPngNewCart => self.remark = Some(ONLY_P8_NEW_CARTS.into()),
            Choice::Save { path, exists: true } => {
                let text = format!("{} EXISTS", shown_name(&path));
                self.ask(text, OVERWRITE, Accepted::SaveOver(path));
            }
            Choice::Save {
                path,
                exists: false,
            } => self.write_project(path),
        }
    }

    pub(super) fn project_name(&self) -> String {
        self.project_path
            .as_deref()
            .map_or_else(|| UNTITLED.to_uppercase(), shown_name)
    }

    pub fn window_title(&self) -> String {
        let name = self
            .project_path
            .as_deref()
            .and_then(Path::file_name)
            .map_or_else(
                || UNTITLED.to_owned(),
                |name| name.to_string_lossy().into_owned(),
            );
        let star = if self.has_unsaved_changes() { "*" } else { "" };
        format!("{name}{star} - pikutracker")
    }
}
