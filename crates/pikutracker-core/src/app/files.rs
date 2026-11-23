use super::exporting::FinishedExport;
use super::messages::Report;
use super::panes::file_list::{Choice, FileList, Purpose};
use super::paths::{relative_path, resolved};
use super::{Action, App, Focus};
use crate::export::Exported;
use crate::player::{AudioRequest, TrackMix};
use crate::project::Project;
use crate::project_file;
use std::path::{Path, PathBuf};

const UNTITLED: &str = "Untitled";
const NOT_FOUND: &str = "not found";
pub(super) const OVERWRITE_ANSWERS: &str = "ENTER OVERWRITE  ESC CANCEL";
const UNSAVED_QUESTION: &str = "UNSAVED CHANGES WILL BE LOST  ENTER DISCARD  ESC CANCEL";

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
    accepted: Accepted,
}

impl Question {
    pub(super) fn purge_unused(text: String) -> Self {
        Self {
            text,
            accepted: Accepted::PurgeUnused,
        }
    }
}

#[derive(Debug)]
pub(super) enum Waiting {
    Listing(PathBuf),
    Reading(PathBuf),
    Importing(PathBuf),
    Writing(PathBuf, Box<Project>),
    ExportReading(PathBuf),
    ExportWriting(PathBuf, FinishedExport),
    CartViewReading(PathBuf),
    PickingCart(PathBuf),
    SettingAside {
        state_file: PathBuf,
        bad_file: PathBuf,
    },
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
        std::mem::take(&mut self.file_requests)
    }

    pub fn receive_file(&mut self, response: FileResponse) {
        match response {
            FileResponse::Listed { directory, entries } => {
                if matches!(&self.waiting, Some(Waiting::Listing(listed)) if *listed == directory) {
                    self.waiting = None;
                    if let Some(list) = self.file_list.as_mut() {
                        list.show(directory, entries);
                    }
                }
            }
            FileResponse::Read { path, contents } => self.finish_reading(path, &contents),
            FileResponse::BytesRead { path, contents } => {
                self.finish_reading_bytes(path, &contents)
            }
            FileResponse::Written { path } => self.finish_writing(&path),
            FileResponse::Renamed { path } => self.finish_setting_aside(&path),
            FileResponse::Missing { path } => self.file_missing(&path),
            FileResponse::Failed { path, reason } => self.file_failed(&path, &reason),
        }
    }

    fn finish_reading(&mut self, path: PathBuf, contents: &str) {
        if !matches!(&self.waiting, Some(Waiting::Reading(read)) if *read == path) {
            return;
        }
        self.waiting = None;
        self.file_list = None;
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

    fn finish_reading_bytes(&mut self, path: PathBuf, contents: &[u8]) {
        match self.waiting.take() {
            Some(Waiting::Importing(read)) if read == path => self.finish_importing(path, contents),
            Some(Waiting::ExportReading(read)) if read == path => {
                self.finish_reading_target(path, contents);
            }
            Some(Waiting::CartViewReading(read)) if read == path => {
                self.show_cart_in_view(contents);
            }
            Some(Waiting::PickingCart(read)) if read == path => {
                self.finish_picking_cart(&path, contents)
            }
            other => self.waiting = other,
        }
    }

    pub(super) fn request_bytes(&mut self, path: PathBuf) {
        self.file_requests.push(FileRequest::ReadBytes { path });
    }

    pub(super) fn request_write_bytes(&mut self, path: PathBuf, contents: Vec<u8>) {
        self.file_requests
            .push(FileRequest::WriteBytes { path, contents });
    }

    fn finish_writing(&mut self, path: &Path) {
        match self.waiting.take() {
            Some(Waiting::Writing(written, project)) if written == path => {
                self.adopt_saved_cart_path(&project);
                self.saved_project = *project;
                self.directory = folder_of(&written);
                self.report(Report::info(format!("SAVED {}", shown_name(&written))));
                self.project_path = Some(written.clone());
                self.remember_project(written);
            }
            Some(Waiting::ExportWriting(written, finished)) if written == path => {
                self.finish_writing_target(finished);
            }
            other => self.waiting = other,
        }
    }

    fn file_failed(&mut self, path: &Path, reason: &str) {
        if self.config_not_set_aside(path, reason) {
            return;
        }
        let reason = reason.to_uppercase();
        let name = shown_name(path);
        let summary = match self.waiting.take() {
            Some(Waiting::Listing(listed)) if listed == path => {
                format!("CANNOT LIST {}: {reason}", path.display())
            }
            Some(Waiting::Reading(read)) if read == path => {
                self.file_list = None;
                format!("CANNOT OPEN {name}: {reason}")
            }
            Some(Waiting::Importing(read)) if read == path => {
                self.file_list = None;
                format!("CANNOT IMPORT {name}: {reason}")
            }
            Some(Waiting::Writing(written, _)) if written == path => {
                format!("CANNOT SAVE {name}: {reason}")
            }
            Some(Waiting::ExportReading(target) | Waiting::ExportWriting(target, _))
                if target == path =>
            {
                format!("CANNOT EXPORT TO {name}: {reason}")
            }
            Some(Waiting::CartViewReading(target)) if target == path => {
                self.show_unreadable_cart_in_view(reason);
                return;
            }
            Some(Waiting::PickingCart(picked)) if picked == path => cannot_use(path, &reason),
            Some(Waiting::SettingAside { state_file, .. }) if state_file == path => format!(
                "CANNOT RENAME {name} TO {name}.BAD: {reason}. RECENT PROJECTS ARE NOT SAVED THIS SESSION"
            ),
            other => {
                self.waiting = other;
                return;
            }
        };
        self.report(Report::failure(summary));
    }

    fn file_missing(&mut self, path: &Path) {
        if matches!(&self.waiting, Some(Waiting::ExportReading(target)) if target == path) {
            self.waiting = None;
            self.ask_to_locate(path);
        } else {
            self.file_failed(path, NOT_FOUND);
        }
    }

    pub(super) fn ask(&mut self, text: String, accepted: Accepted) {
        self.question = Some(Question { text, accepted });
    }

    pub(super) fn replace_project(&mut self, project: Project, path: Option<PathBuf>) {
        if self.playing || self.previewing {
            self.playing = false;
            self.previewing = false;
            self.audio_requests.push(AudioRequest::Stop);
        }
        self.playhead = None;
        if self.track_mix != TrackMix::default() {
            self.track_mix = TrackMix::default();
            self.audio_requests
                .push(AudioRequest::SetTrackMix(self.track_mix));
        }
        self.saved_project = project.clone();
        self.project = project;
        self.history.clear();
        self.speed_entry = None;
        self.selection_start = None;
        self.clipboard = None;
        self.project_path = path;
        self.song = Default::default();
        self.chain = Default::default();
        self.phrase.return_to_top();
        self.multitrack = Default::default();
        self.shown = Default::default();
        self.show_what_the_cursors_point_at();
        self.cart_view = None;
        self.focus = Focus::Song;
        self.played_pane = Focus::Song;
        self.focus_before_log = Focus::Song;
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
        self.midi_dialog = None;
        self.speed_entry = None;
        self.file_list = None;
        self.question = None;
        self.selection_start = None;
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
        self.question = Some(Question {
            text: UNSAVED_QUESTION.to_owned(),
            accepted,
        });
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
        let name = self
            .project_path
            .as_deref()
            .and_then(Path::file_name)
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
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
        self.file_list = Some(list);
        self.list_folder(directory);
    }

    fn list_folder(&mut self, directory: PathBuf) {
        self.file_requests.push(FileRequest::List {
            directory: directory.clone(),
        });
        self.waiting = Some(Waiting::Listing(directory));
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
        self.file_list = None;
        let project = self.project_saved_in(&folder_of(&path));
        self.file_requests.push(FileRequest::Write {
            path: path.clone(),
            contents: project_file::write(&project),
        });
        self.waiting = Some(Waiting::Writing(path, Box::new(project)));
    }

    pub(super) fn answer_question(&mut self, action: Action) {
        match action {
            Action::Cancel => self.question = None,
            Action::Confirm => {
                if let Some(question) = self.question.take() {
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
        let Some(list) = self.file_list.as_mut() else {
            return;
        };
        match action {
            Action::Move(direction) => list.move_cursor(direction),
            Action::TypeChar(ch) => list.type_char(ch),
            Action::Backspace => list.backspace(),
            Action::Cancel => self.file_list = None,
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
            Choice::Open(path) => {
                self.file_requests
                    .push(FileRequest::Read { path: path.clone() });
                self.waiting = Some(Waiting::Reading(path));
            }
            Choice::Import(path) => {
                self.file_requests
                    .push(FileRequest::ReadBytes { path: path.clone() });
                self.waiting = Some(Waiting::Importing(path));
            }
            Choice::TargetCart(path) => {
                self.request_bytes(path.clone());
                self.waiting = Some(Waiting::PickingCart(path));
            }
            Choice::LocatedCart(path) => self.locate_target_cart(&path),
            Choice::Save { path, exists: true } => {
                self.question = Some(Question {
                    text: format!("{} EXISTS  {OVERWRITE_ANSWERS}", shown_name(&path)),
                    accepted: Accepted::SaveOver(path),
                });
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
