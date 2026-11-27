use super::file_conversations::Asked;
use super::files::FileRequest;
use super::key_hints;
use super::messages::{LogItem, Notice, Report, Severity};
use super::{Action, App};
use crate::keymap::{Keyboard, Keymap, RebindProblem};
use crate::settings::{AppState, Config, SettingsRead, read_config, read_state, write_state};
use crate::wording::counted;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Launch {
    pub config_file: Option<PathBuf>,
    pub config: SettingsContents,
    pub state_file: Option<PathBuf>,
    pub state: SettingsContents,
    pub directory: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingsContents {
    Missing,
    Bytes(Vec<u8>),
    Unreadable { reason: String },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum HelpAnnouncement {
    Due,
    Showing,
    #[default]
    Over,
}

const CONFIG_RON: &str = "CONFIG.RON";
const STATE_RON: &str = "STATE.RON";

fn set_aside_path(path: &Path) -> PathBuf {
    let mut bad: OsString = path.as_os_str().to_owned();
    bad.push(".bad");
    PathBuf::from(bad)
}

fn read_contents<T: Default>(
    contents: SettingsContents,
    read: impl FnOnce(&str) -> SettingsRead<T>,
) -> Result<SettingsRead<T>, String> {
    match contents {
        SettingsContents::Missing => Ok(SettingsRead::Loaded(T::default())),
        SettingsContents::Bytes(bytes) => Ok(match String::from_utf8(bytes) {
            Ok(text) => read(&text),
            Err(_) => SettingsRead::Damaged,
        }),
        SettingsContents::Unreadable { reason } => Err(reason),
    }
}

fn cannot_read(name: &str, reason: &str) -> Report {
    let reason = reason.to_uppercase();
    Report::warning(format!(
        "CANNOT READ {name}: {reason}. USING DEFAULTS THIS SESSION"
    ))
}

fn damaged(name: &str) -> Report {
    Report::warning(format!(
        "{name} WAS DAMAGED: RENAMED TO {name}.BAD, USING DEFAULTS"
    ))
}

fn newer(name: &str) -> Report {
    Report::warning(format!(
        "{name} IS FROM A NEWER PIKUTRACKER: USING DEFAULTS THIS SESSION"
    ))
}

fn ignored_keys(problems: &[RebindProblem]) -> Report {
    let items = problems
        .iter()
        .map(|problem| {
            let text = match problem {
                RebindProblem::UnknownAction(name) => {
                    format!("NO ACTION IS CALLED {}", name.to_uppercase())
                }
                RebindProblem::UnreadableKey { action, key } => format!(
                    "CANNOT READ THE KEY {} FOR {}",
                    key.to_uppercase(),
                    action.to_uppercase()
                ),
            };
            LogItem::new(Severity::Warning, text)
        })
        .collect();
    let count = counted(problems.len(), "KEY");
    Report::warning(format!("{CONFIG_RON}: IGNORED {count}")).with_items(items)
}

impl App {
    pub fn launch(&mut self, launch: Launch) {
        self.directory = launch.directory;
        if let Some(config_file) = launch.config_file {
            self.launch_config(config_file, launch.config);
        }
        if let Some(state_file) = launch.state_file {
            self.launch_state(state_file, launch.state);
        }
        self.help_announcement = HelpAnnouncement::Due;
        self.announce_help_when_free();
    }

    pub(super) fn announce_help_when_free(&mut self) {
        let free = !self.persistent_notice_showing() && !self.files.waiting();
        if self.help_announcement != HelpAnnouncement::Due || !free {
            return;
        }
        self.help_announcement = HelpAnnouncement::Showing;
        let help_hint = key_hints::help_hint(self.keyboard.keymap());
        self.notice = Some(Notice::new(Severity::Info, help_hint));
    }

    pub(super) fn end_help_announcement(&mut self, action: Action) {
        let dismissing = action == Action::Cancel && self.persistent_notice_showing();
        match self.help_announcement {
            HelpAnnouncement::Due if dismissing => {}
            HelpAnnouncement::Showing => {
                self.notice = None;
                self.help_announcement = HelpAnnouncement::Over;
            }
            _ => self.help_announcement = HelpAnnouncement::Over,
        }
    }

    fn launch_config(&mut self, config_file: PathBuf, contents: SettingsContents) {
        match read_contents(contents, read_config) {
            Err(reason) => self.report(cannot_read(CONFIG_RON, &reason)),
            Ok(SettingsRead::Loaded(config)) => self.use_config(&config),
            Ok(SettingsRead::Damaged) => {
                let to = set_aside_path(&config_file);
                self.files.ask(
                    FileRequest::Rename {
                        from: config_file,
                        to,
                    },
                    Asked::SettingAsideConfig,
                );
                self.report(damaged(CONFIG_RON));
            }
            Ok(SettingsRead::Newer { .. }) => self.report(newer(CONFIG_RON)),
        }
    }

    fn use_config(&mut self, config: &Config) {
        let (keymap, problems) = Keymap::rebound(config.preset, &config.keys);
        self.keyboard = Keyboard::new(keymap);
        if !problems.is_empty() {
            self.report(ignored_keys(&problems));
        }
    }

    fn launch_state(&mut self, state_file: PathBuf, contents: SettingsContents) {
        match read_contents(contents, read_state) {
            Err(reason) => self.report(cannot_read(STATE_RON, &reason)),
            Ok(SettingsRead::Loaded(state)) => {
                self.follow = state.follow;
                self.follow_set_by_key = state.follow;
                self.phrase.set_pen(state.pen);
                self.midi_table = state.midi_table.clone();
                self.state_file = Some(state_file);
                if let Some(last) = state.last_project() {
                    let path = last.to_path_buf();
                    self.files.ask(FileRequest::Read { path }, Asked::Opening);
                }
                self.app_state = state;
            }
            Ok(SettingsRead::Damaged) => {
                self.files.ask(
                    FileRequest::Rename {
                        from: state_file.clone(),
                        to: set_aside_path(&state_file),
                    },
                    Asked::SettingAsideState { state_file },
                );
                self.report(damaged(STATE_RON));
            }
            Ok(SettingsRead::Newer { .. }) => self.report(newer(STATE_RON)),
        }
    }

    fn state_to_remember(&self) -> AppState {
        let mut state = self.app_state.clone();
        state.follow = self.follow_set_by_key;
        state.pen = self.phrase.pen();
        state.midi_table = self.midi_table.clone();
        state
    }

    pub(super) fn save_changed_state(&mut self) {
        if self.state_file.is_some() && self.state_to_remember() != self.app_state {
            self.write_state();
        }
    }

    pub(super) fn remember_project(&mut self, path: PathBuf) {
        if self.state_file.is_none() {
            return;
        }
        self.app_state.remember_project(path);
        self.write_state();
    }

    fn write_state(&mut self) {
        let Some(state_file) = self.state_file.clone() else {
            return;
        };
        self.app_state = self.state_to_remember();
        self.files.tell(FileRequest::Write {
            path: state_file,
            contents: write_state(&self.app_state),
        });
    }
}
