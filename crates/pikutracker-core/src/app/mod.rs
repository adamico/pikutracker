mod action;
mod cell_edits;
mod change_speed;
mod claim_text;
mod claiming;
mod exporting;
mod files;
mod focus;
mod history;
mod importing;
mod launching;
mod layout;
mod messages;
mod panes;
mod paths;
mod restructure;
mod selection;
mod status_line;

pub use action::{Action, Direction, KeyContext};
pub use files::{DirectoryEntry, FileRequest, FileResponse};
pub use launching::{Launch, SettingsContents};
pub use messages::{LogItem, Report, Severity};

use crate::export;
use crate::keymap::{Key, KeyPress, Keyboard};
use crate::midi_import::MidiTable;
use crate::player::{AudioRequest, PlaybackReport, Playhead, SCOPE_COLUMNS, TrackMix};
use crate::project::{CHAIN_ENTRIES, FIRST_CUSTOM_INSTRUMENT, SONG_ROWS};
use crate::project::{Chain, ChainEntry, ChainId, Phrase, PhraseId, Project, Step};
use crate::screen::grid::Grid;
use crate::settings::AppState;
use cell_edits::Clipboard;
use change_speed::{SpeedChange, SpeedEntry};
use files::{Question, Waiting};
use focus::Focus;
use history::{CursorPlace, History, Merge};
use importing::PickingSong;
use messages::Notice;
use panes::cart_view::CartView;
use panes::chain::{ChainPane, ChainView};
use panes::file_list::{FileList, Purpose};
use panes::instrument::InstrumentView;
use panes::log::LogPane;
use panes::midi_dialog::MidiDialog;
use panes::overview::OverviewPane;
use panes::phrase::{PhrasePane, PhraseView};
use panes::song::SongPane;
use panes::song_picker::SONG_PICKER_KEYS;
use selection::CellPosition;
use status_line::StatusLine;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

const SONG_JUMP_ROWS: usize = 16;
const NOTHING_TO_PLAY: &str = "NOTHING TO PLAY";
const NO_CHAIN_HERE: &str = "NO CHAIN HERE";
const NO_PHRASE_HERE: &str = "NO PHRASE HERE";
const NOTHING_TO_UNDO: &str = "NOTHING TO UNDO";
const NOTHING_TO_REDO: &str = "NOTHING TO REDO";
const FOLLOW_ON: &str = "FOLLOW ON";
const FOLLOW_OFF: &str = "FOLLOW OFF";
const BUILT_IN_INSTRUMENT: &str = "BUILT-IN INSTRUMENTS HAVE NO SETTINGS";
const OPEN_LIST_KEYS: &str = "UP/DOWN PICK  ENTER CHOOSE  ESC CANCEL";
const SAVE_LIST_KEYS: &str = "TYPE A NAME  UP/DOWN PICK  ENTER CHOOSE  ESC CANCEL";

#[derive(Debug)]
pub struct App {
    project: Project,
    saved_project: Project,
    history: History,
    selection_start: Option<CellPosition>,
    clipboard: Option<Clipboard>,
    project_path: Option<PathBuf>,
    directory: PathBuf,
    quitting: bool,
    app_state: AppState,
    state_file: Option<PathBuf>,
    config_set_aside: Option<PathBuf>,
    keyboard: Keyboard,
    midi_table: MidiTable,
    focus: Focus,
    song: SongPane,
    chain: ChainPane,
    phrase: PhrasePane,
    overview: OverviewPane,
    played_pane: Focus,
    playing: bool,
    latest_play: u64,
    heard_project: Arc<Project>,
    track_mix: TrackMix,
    playhead: Option<Playhead>,
    follow: bool,
    scope: [i8; SCOPE_COLUMNS],
    previewing: bool,
    remark: Option<&'static str>,
    notice: Option<Notice>,
    question: Option<Question>,
    speed_entry: Option<SpeedEntry>,
    log: LogPane,
    log_open: bool,
    focus_before_log: Focus,
    file_list: Option<FileList>,
    midi_dialog: Option<Box<MidiDialog>>,
    picking_song: Option<Box<PickingSong>>,
    instrument_view: Option<InstrumentView>,
    cart_view: Option<CartView>,
    cart_visits: u32,
    waiting: Option<Waiting>,
    file_requests: Vec<FileRequest>,
    audio_requests: Vec<AudioRequest>,
}

impl Default for App {
    fn default() -> Self {
        Self::with_project(Project::default())
    }
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_project(project: Project) -> Self {
        Self {
            saved_project: project.clone(),
            heard_project: Arc::new(project.clone()),
            track_mix: TrackMix::default(),
            playhead: None,
            follow: false,
            scope: [0; SCOPE_COLUMNS],
            previewing: false,
            project,
            history: History::default(),
            selection_start: None,
            clipboard: None,
            project_path: None,
            directory: PathBuf::from("."),
            quitting: false,
            app_state: AppState::default(),
            state_file: None,
            config_set_aside: None,
            keyboard: Keyboard::default(),
            midi_table: MidiTable::default(),
            focus: Focus::default(),
            song: SongPane::default(),
            chain: ChainPane::default(),
            phrase: PhrasePane::default(),
            overview: OverviewPane::default(),
            played_pane: Focus::default(),
            playing: false,
            latest_play: 0,
            remark: None,
            notice: None,
            question: None,
            speed_entry: None,
            log: LogPane::default(),
            log_open: false,
            focus_before_log: Focus::default(),
            file_list: None,
            midi_dialog: None,
            picking_song: None,
            instrument_view: None,
            cart_view: None,
            cart_visits: 0,
            waiting: None,
            file_requests: Vec::new(),
            audio_requests: Vec::new(),
        }
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    fn shown_project(&self) -> &Project {
        self.picking_song
            .as_ref()
            .map_or(&self.project, |picking| picking.picker.project())
    }

    pub fn chain_under_cursor(&self) -> Option<ChainId> {
        self.shown_project()
            .song_cell(self.song.row(), self.song.track())
    }

    pub fn phrase_under_cursor(&self) -> Option<PhraseId> {
        self.entry_under_cursor().map(ChainEntry::phrase)
    }

    fn entry_under_cursor(&self) -> Option<ChainEntry> {
        let chain = self.chain_under_cursor()?;
        self.shown_project().chain(chain).entry(self.chain.entry())
    }

    fn shown_phrase(&self) -> Phrase {
        self.phrase_under_cursor()
            .map_or_else(Phrase::default, |id| *self.shown_project().phrase(id))
    }

    pub fn key_context(&self) -> KeyContext {
        if self.speed_entry.is_some() {
            return KeyContext::SpeedEntry;
        }
        if self.question.is_some() {
            return KeyContext::Navigation;
        }
        if self.midi_dialog.is_some() {
            return KeyContext::MidiImport;
        }
        if self.picking_song.is_some() {
            return KeyContext::SongPicker;
        }
        if let Some(list) = &self.file_list {
            return match list.purpose() {
                Purpose::SaveAs => KeyContext::FileName,
                Purpose::Open | Purpose::Import | Purpose::TargetCart | Purpose::LocateCart => {
                    KeyContext::Navigation
                }
            };
        }
        if let Some(view) = &self.instrument_view {
            return view.key_context(&self.project);
        }
        if self.cart_view_has_focus() {
            return KeyContext::ClaimText;
        }
        match self.focus {
            Focus::Song => KeyContext::ReferenceValue,
            Focus::Chain if self.chain.takes_digits() => KeyContext::ReferenceValue,
            Focus::Phrase => self.phrase.key_context(),
            Focus::RightSlot if !self.log_open => KeyContext::ReferenceValue,
            _ => KeyContext::Navigation,
        }
    }

    pub fn take_audio_requests(&mut self) -> Vec<AudioRequest> {
        std::mem::take(&mut self.audio_requests)
    }

    pub fn playback_finished(&mut self, play: u64) {
        if play == self.latest_play {
            self.playing = false;
        }
    }

    pub fn show_playback(&mut self, report: &PlaybackReport) -> bool {
        self.playback_finished(report.finished_play);
        let playhead = report.playhead.filter(|_| self.playing);
        let moved = playhead != self.playhead;
        let changed = moved || report.scope != self.scope;
        self.playhead = playhead;
        self.scope = report.scope;
        if moved && self.follow && self.selection_start.is_none() {
            self.follow_playhead();
        }
        changed
    }

    pub fn key_down(&mut self, press: KeyPress, pressed_at: Duration) -> Option<Action> {
        let context = self.key_context();
        let selecting = self.selection_start.is_some();
        self.keyboard
            .key_down(press, context, selecting, pressed_at)
    }

    pub fn key_up(&mut self, key: Key) -> Option<Action> {
        self.key_released();
        self.keyboard.key_up(key)
    }

    pub fn key_released(&mut self) {
        if self.previewing {
            self.previewing = false;
            self.audio_requests.push(AudioRequest::EndPreview);
        }
    }

    fn step_before_edit(&self, action: Action) -> Option<(usize, Step)> {
        let edits_note = matches!(
            action,
            Action::PianoKey(_) | Action::HexDigit(_) | Action::Nudge { .. }
        );
        if !edits_note || self.focus != Focus::Phrase {
            return None;
        }
        let index = self.phrase.note_step()?;
        Some((index, self.shown_phrase().step(index)))
    }

    fn preview_edit(&mut self, action: Action, index: usize, before: Step) {
        let phrase = self.shown_phrase();
        let step = phrase.step(index);
        let changed = step.pitch() != before.pitch() || step.instrument() != before.instrument();
        let entered = matches!(action, Action::PianoKey(_));
        if self.playing || step.is_silent() || !(entered || changed) {
            return;
        }
        self.previewing = true;
        let project = self.project_for_player();
        self.audio_requests.push(AudioRequest::PreviewStep {
            project,
            phrase,
            step: index,
            transpose: self.entry_under_cursor().map_or(0, ChainEntry::transpose),
        });
    }

    fn follow_playhead(&mut self) {
        let Some(playhead) = self.playhead else {
            return;
        };
        if let Some((row, k)) = playhead.chain_row {
            self.song.follow(row);
            self.chain.follow(k);
        }
        let step = self
            .phrase_under_cursor()
            .and_then(|phrase| playhead.step_in(phrase, self.song.track()));
        if let Some(step) = step {
            self.phrase.follow(step);
        }
    }

    fn toggle_follow(&mut self) {
        self.follow = !self.follow;
        self.remark = Some(if self.follow { FOLLOW_ON } else { FOLLOW_OFF });
    }

    fn stop_following(&mut self) {
        if self.follow {
            self.follow = false;
            self.remark = Some(FOLLOW_OFF);
        }
    }

    pub fn report(&mut self, report: Report) {
        self.notice = Some(Notice::new(report.severity, report.status_text()));
        if !report.items.is_empty() && !self.log_open {
            self.log_open = true;
            self.focus_before_log = self.focus;
        }
        self.log.add(report.into_log_items());
    }

    pub fn advance_time(&mut self, elapsed: Duration) -> bool {
        let Some(notice) = self.notice.as_mut() else {
            return false;
        };
        let look = notice.look();
        if notice.grow_older(elapsed) {
            self.notice = None;
            return true;
        }
        notice.look() != look
    }

    pub fn time_until_status_change(&self) -> Option<Duration> {
        self.notice.as_ref()?.time_until_change()
    }

    pub fn apply(&mut self, action: Action) {
        let merge = match action {
            _ if self.cart_view_has_focus() => Some(Merge::CartVisit(self.cart_visits)),
            Action::Nudge { .. } if self.selection_start.is_none() => Some(Merge::Nudge),
            Action::HexDigit(_) => Some(Merge::Digit),
            _ => None,
        };
        self.history.begin(&self.project, self.cursor_place());
        self.perform(action);
        self.history
            .finish(&self.project, self.cursor_place(), merge);
        self.save_changed_state();
    }

    fn cursor_place(&self) -> CursorPlace {
        CursorPlace {
            focus: self.focus,
            song: self.song.cursor_place(),
            chain: self.chain.cursor_place(),
            phrase: self.phrase.cursor_place(),
            overview_channel: self.overview.channel(),
        }
    }

    fn return_to(&mut self, place: CursorPlace) {
        self.selection_start = None;
        self.stop_following();
        self.set_focus(place.focus);
        self.song.return_to(place.song);
        self.chain.return_to(place.chain);
        self.phrase.return_to(place.phrase);
        self.overview.return_to(place.overview_channel);
    }

    fn undo(&mut self) {
        match self.history.undo(&mut self.project) {
            Some(place) => self.return_to(place),
            None => self.remark = Some(NOTHING_TO_UNDO),
        }
        self.show_claim_in_cart_view();
    }

    fn redo(&mut self) {
        match self.history.redo(&mut self.project) {
            Some(place) => self.return_to(place),
            None => self.remark = Some(NOTHING_TO_REDO),
        }
        self.show_claim_in_cart_view();
    }

    fn perform(&mut self, action: Action) {
        self.remark = None;
        if !matches!(action, Action::HexDigit(_)) {
            self.song.finish_typing();
            self.chain.finish_typing();
            self.phrase.finish_typing();
            if let Some(view) = self.instrument_view.as_mut() {
                view.finish_typing();
            }
        }
        if action == Action::Quit {
            self.quit();
            return;
        }
        if self.midi_dialog.is_some() {
            self.use_midi_dialog(action);
            return;
        }
        if self.picking_song.is_some() {
            self.use_song_picker(action);
            return;
        }
        if self.speed_entry.is_some() {
            self.use_speed_entry(action);
            self.settle_after_edit();
            return;
        }
        if action == Action::Cancel && self.selection_start.take().is_some() {
            return;
        }
        if self.question.is_some() {
            self.answer_question(action);
            self.settle_after_edit();
            return;
        }
        if self.file_list.is_some() {
            self.use_file_list(action);
            return;
        }
        if self.instrument_view.is_some() {
            self.use_instrument_view(action);
            self.settle_after_edit();
            return;
        }
        if self.cart_view_has_focus() && self.use_cart_view(action) {
            self.settle_after_edit();
            return;
        }
        let before_edit = self.step_before_edit(action);
        match action {
            Action::NewProject => self.new_project(),
            Action::OpenProject => self.open_project(),
            Action::Save => self.save(),
            Action::SaveAs => self.save_as(),
            Action::Import => self.import(),
            Action::Export => self.export(),
            Action::ToggleLog => self.toggle_log(),
            Action::ToggleCart => self.toggle_cart(),
            Action::Undo => self.undo(),
            Action::Redo => self.redo(),
            Action::StartSelection => self.start_selection(),
            Action::Copy => self.copy(),
            Action::Cut => self.cut(),
            Action::Paste => self.paste(),
            Action::Clone => self.clone_items(false),
            Action::DeepClone => self.clone_items(true),
            Action::NewUnused => self.new_unused(),
            Action::InsertRow => self.insert_row(),
            Action::DeleteRow => self.delete_row(),
            Action::PurgeUnused => self.ask_to_purge(),
            Action::ChangeSpeed => self.ask_for_speed(SpeedChange::Set),
            Action::ScaleSpeed => self.ask_for_speed(SpeedChange::Scale),
            Action::Confirm => self.confirm(),
            Action::Cancel => self.cancel(),
            Action::Move(direction) => self.move_cursor(direction),
            Action::Jump(direction) => self.jump(direction),
            Action::FocusNext => self.set_focus(self.focus.next()),
            Action::FocusPrevious => self.set_focus(self.focus.previous()),
            Action::PlayFocusedPane => self.play_or_stop(),
            Action::PlaySongFromCursor => self.play_song(),
            Action::ToggleFollow => self.toggle_follow(),
            Action::Mute { track_number } => self.change_track_mix(|mix| {
                mix.toggle_mute(usize::from(track_number) - 1);
            }),
            Action::Solo { track_number } => self.change_track_mix(|mix| {
                mix.toggle_solo(usize::from(track_number) - 1);
            }),
            Action::ClearMutesAndSolos => {
                self.change_track_mix(|mix| *mix = TrackMix::default());
            }
            Action::OctaveUp => self.phrase.change_octave(1),
            Action::OctaveDown => self.phrase.change_octave(-1),
            Action::ToggleSongLoopRow => self.song.toggle_loop_row(&mut self.project),
            Action::ToggleFill if self.focus == Focus::Chain => self.toggle_fill(),
            Action::Clear => self.clear_cells(),
            Action::PianoKey(key) if self.focus == Focus::Phrase => {
                self.edit_phrase(|pane, phrase| pane.enter_note(phrase, key));
            }
            Action::HexDigit(digit) => self.type_digit(digit),
            Action::Nudge { up, coarse } => self.nudge(up, coarse),
            Action::MoveRows { up } => self.move_rows(up),
            _ => {}
        }
        if let Some((index, before)) = before_edit {
            self.preview_edit(action, index, before);
        }
        self.settle_after_edit();
    }

    fn settle_after_edit(&mut self) {
        self.send_edits_to_the_player();
    }

    fn change_track_mix(&mut self, change: impl FnOnce(&mut TrackMix)) {
        change(&mut self.track_mix);
        self.audio_requests
            .push(AudioRequest::SetTrackMix(self.track_mix));
    }

    fn send_edits_to_the_player(&mut self) {
        if self.playing && *self.heard_project != self.project {
            let project = self.project_for_player();
            self.audio_requests
                .push(AudioRequest::UpdateProject(project));
        }
    }

    fn project_for_player(&mut self) -> Arc<Project> {
        self.heard_project = Arc::new(self.project.clone());
        Arc::clone(&self.heard_project)
    }

    fn log_has_focus(&self) -> bool {
        self.log_open && self.focus == Focus::RightSlot
    }

    fn toggle_log(&mut self) {
        if self.log_open {
            self.close_log();
        } else {
            self.log_open = true;
            self.focus_before_log = self.focus;
            self.set_focus(Focus::RightSlot);
        }
    }

    fn close_log(&mut self) {
        self.log_open = false;
        if self.focus == Focus::RightSlot {
            self.set_focus(self.focus_before_log);
        }
    }

    fn confirm(&mut self) {
        if self.focus == Focus::Phrase && self.phrase_under_cursor().is_some() {
            let shown = self.shown_phrase();
            if let Some(instrument) = self.phrase.instrument_under_cursor(&shown) {
                self.open_instrument_view(instrument);
            }
            return;
        }
        if !self.log_has_focus() {
            return;
        }
        if let Some(location) = self.log.picked_location() {
            self.stop_following();
            self.song.jump_to(location.song_row, location.track);
            self.chain.jump_to(location.chain_row);
            self.phrase.jump_to(location.step);
            self.set_focus(Focus::Phrase);
        }
    }

    fn cancel(&mut self) {
        if self.notice.as_ref().is_some_and(Notice::is_persistent) {
            self.notice = None;
        } else if self.log_has_focus() {
            self.close_log();
        } else if self.focus == Focus::RightSlot {
            self.set_focus(Focus::Phrase);
        }
    }

    fn open_instrument_view(&mut self, instrument: u8) {
        if instrument < FIRST_CUSTOM_INSTRUMENT {
            self.remark = Some(BUILT_IN_INSTRUMENT);
            return;
        }
        self.selection_start = None;
        self.instrument_view = Some(InstrumentView::new(instrument));
    }

    fn use_instrument_view(&mut self, action: Action) {
        let Some(view) = self.instrument_view.as_mut() else {
            return;
        };
        match action {
            Action::Cancel => {
                self.instrument_view = None;
                if self.playing {
                    self.play_or_stop();
                }
            }
            Action::PlayFocusedPane if self.playing => self.play_or_stop(),
            Action::PlayFocusedPane => {
                let instrument = view.id();
                let project = self.project_for_player();
                self.start_playing(AudioRequest::AuditionInstrument {
                    project,
                    instrument,
                });
            }
            Action::Undo => self.undo(),
            Action::Redo => self.redo(),
            Action::Save => self.save(),
            Action::OctaveUp => self.phrase.change_octave(1),
            Action::OctaveDown => self.phrase.change_octave(-1),
            Action::Jump(direction) => {
                let id = match direction {
                    Direction::Left => view.id().saturating_sub(1),
                    Direction::Right => view.id().saturating_add(1),
                    Direction::Up | Direction::Down => view.id(),
                };
                if id >= FIRST_CUSTOM_INSTRUMENT && id != view.id() {
                    self.instrument_view = Some(InstrumentView::new(id));
                }
            }
            _ => view.apply(action, &mut self.project, self.phrase.octave()),
        }
    }

    fn type_digit(&mut self, digit: u8) {
        match self.focus {
            Focus::Song => self.song.type_digit(&mut self.project, digit),
            Focus::Chain => self.edit_chain(|pane, chain| pane.type_digit(chain, digit)),
            Focus::Phrase => self.edit_phrase(|pane, phrase| pane.type_digit(phrase, digit)),
            Focus::RightSlot if !self.log_open => {
                self.overview.type_digit(&mut self.project, digit);
            }
            Focus::RightSlot => {}
        }
    }

    fn nudge(&mut self, up: bool, coarse: bool) {
        if self.selection_start.is_some() {
            self.nudge_selection(up, coarse);
            return;
        }
        match self.focus {
            Focus::Song => self.song.nudge(&mut self.project, up, coarse),
            Focus::Chain => self.edit_chain(|pane, chain| pane.nudge(chain, up, coarse)),
            Focus::Phrase => self.edit_phrase(|pane, phrase| pane.nudge(phrase, up, coarse)),
            Focus::RightSlot => {}
        }
    }

    fn toggle_fill(&mut self) {
        if self.chain_under_cursor().is_some() && self.entry_under_cursor().is_none() {
            self.remark = Some(NO_PHRASE_HERE);
            return;
        }
        self.edit_chain(|pane, chain| pane.toggle_fill(chain));
    }

    fn edit_chain(&mut self, edit: impl FnOnce(&mut ChainPane, &mut Chain)) {
        let Some(id) = self.chain_under_cursor() else {
            self.remark = Some(NO_CHAIN_HERE);
            return;
        };
        if self.entry_under_cursor().is_none() && !self.chain.edits_empty_entries() {
            self.remark = Some(NO_PHRASE_HERE);
            return;
        }
        edit(&mut self.chain, self.project.chain_mut(id));
    }

    fn edit_phrase(&mut self, edit: impl FnOnce(&mut PhrasePane, &mut Phrase)) {
        match self.phrase_under_cursor() {
            Some(id) => edit(&mut self.phrase, self.project.phrase_mut(id)),
            None => self.remark = Some(NO_PHRASE_HERE),
        }
    }

    fn set_focus(&mut self, focus: Focus) {
        if focus != self.focus {
            self.selection_start = None;
        }
        self.focus = focus;
        if focus != Focus::RightSlot {
            self.played_pane = focus;
        }
    }

    fn play_or_stop(&mut self) {
        if self.playing {
            self.playing = false;
            self.playhead = None;
            self.audio_requests.push(AudioRequest::Stop);
            return;
        }
        match self.played_pane {
            Focus::Song | Focus::RightSlot => self.play_song(),
            Focus::Chain => self.play_chain(),
            Focus::Phrase => self.play_phrase(),
        }
    }

    fn start_playing(&mut self, request: AudioRequest) {
        self.playing = true;
        self.previewing = false;
        self.latest_play += 1;
        self.audio_requests.push(request);
    }

    fn play_song(&mut self) {
        let from_row = self.song.row();
        if self.project.song_row_is_empty(from_row) {
            self.remark = Some(NOTHING_TO_PLAY);
            return;
        }
        let project = self.project_for_player();
        self.start_playing(AudioRequest::PlaySong { project, from_row });
    }

    fn play_chain(&mut self) {
        if !self.chain_under_cursor_sounds() {
            self.remark = Some(NOTHING_TO_PLAY);
            return;
        }
        let project = self.project_for_player();
        self.start_playing(AudioRequest::PlayChain {
            project,
            song_row: self.song.row(),
            track: self.song.track(),
            from_entry: self.chain.entry(),
        });
    }

    fn chain_under_cursor_sounds(&self) -> bool {
        self.chain_under_cursor().is_some_and(|id| {
            let chain = self.project.chain(id);
            (0..CHAIN_ENTRIES)
                .filter_map(|index| chain.entry(index))
                .any(|entry| !self.project.phrase(entry.phrase()).is_silent())
        })
    }

    fn play_phrase(&mut self) {
        let Some(phrase) = self
            .phrase_under_cursor()
            .filter(|id| !self.project.phrase(*id).is_silent())
        else {
            self.remark = Some(NOTHING_TO_PLAY);
            return;
        };
        let project = self.project_for_player();
        self.start_playing(AudioRequest::PlayPhrase {
            project,
            phrase,
            transpose: self.entry_under_cursor().map_or(0, ChainEntry::transpose),
            from_step: self.phrase.step(),
        });
    }

    fn jump(&mut self, direction: Direction) {
        if self.focus == Focus::RightSlot {
            return;
        }
        self.stop_following();
        let (row, track) = (self.song.row(), self.song.track());
        let (_, rows) = direction.column_and_row_delta();
        match (self.focus, direction) {
            (Focus::Song, Direction::Up) => {
                self.song.jump_to(row.saturating_sub(SONG_JUMP_ROWS), track);
            }
            (Focus::Song, Direction::Down) => self.song.jump_to(row + SONG_JUMP_ROWS, track),
            (Focus::Song, _) => {}
            (_, Direction::Left | Direction::Right) => self.song.move_cursor(direction),
            (Focus::Chain, _) => {
                let next = next_held_index(row, rows, SONG_ROWS, |row| {
                    self.project.song_cell(row, track).is_some()
                });
                if let Some(next) = next {
                    self.song.jump_to(next, track);
                }
            }
            (_, _) => {
                let Some(chain) = self.chain_under_cursor() else {
                    return;
                };
                let chain = self.project.chain(chain);
                let next = next_held_index(self.chain.entry(), rows, CHAIN_ENTRIES, |entry| {
                    chain.entry(entry).is_some()
                });
                if let Some(next) = next {
                    self.chain.jump_to(next);
                }
            }
        }
    }

    fn move_cursor(&mut self, direction: Direction) {
        if self.focus != Focus::RightSlot {
            self.stop_following();
        }
        match self.focus {
            Focus::Song => self.song.move_cursor(direction),
            Focus::Chain => self.chain.move_cursor(direction),
            Focus::Phrase if self.selection_would_reach_phrase_header(direction) => {}
            Focus::Phrase => self.phrase.move_cursor(direction),
            Focus::RightSlot if self.log_open => self.log.move_cursor(direction),
            Focus::RightSlot => self.overview.move_cursor(direction),
        }
    }

    fn selection_would_reach_phrase_header(&self, direction: Direction) -> bool {
        self.selection_start.is_some() && direction == Direction::Up && self.phrase.step() == 0
    }

    pub fn grid(&self) -> Grid {
        let mut grid = Grid::new();
        if let Some(dialog) = &self.midi_dialog {
            dialog.draw(&mut grid);
            status_line::draw(&mut grid, &StatusLine::Hints(dialog.hints()));
            return grid;
        }
        if let Some(view) = self
            .instrument_view
            .as_ref()
            .filter(|_| self.file_list.is_none())
        {
            view.draw(&mut grid, &self.project);
            let project = self.project_name();
            status_line::draw(&mut grid, &self.status_line(&project));
            return grid;
        }
        let chain_row = self.playhead.and_then(|playhead| playhead.chain_row);
        let takes_keys = self.file_list.is_some() || self.picking_song.is_some();
        let focus = (!takes_keys).then_some(self.focus);
        let project = self.shown_project();
        self.song.draw(
            &mut grid,
            layout::SONG,
            focus == Some(Focus::Song),
            project,
            &self.track_mix,
            chain_row.map(|(row, _)| row),
        );
        self.chain.draw(
            &mut grid,
            layout::CHAIN,
            focus == Some(Focus::Chain),
            project,
            ChainView {
                song_row: self.song.row(),
                track: self.song.track(),
                playing_entry: chain_row
                    .filter(|(row, _)| *row == self.song.row())
                    .map(|(_, k)| k),
            },
        );
        let phrase = self.phrase_under_cursor();
        self.phrase.draw(
            &mut grid,
            layout::PHRASE,
            focus == Some(Focus::Phrase),
            PhraseView {
                id: phrase,
                phrase: &self.shown_phrase(),
                transpose: self.entry_under_cursor().map_or(0, ChainEntry::transpose),
                playing_step: self
                    .playhead
                    .zip(phrase)
                    .and_then(|(playhead, phrase)| playhead.step_in(phrase, self.song.track())),
            },
        );
        self.draw_selection(&mut grid);
        let right_slot_focused = focus == Some(Focus::RightSlot);
        if let Some(picking) = &self.picking_song {
            picking.picker.draw(&mut grid, layout::RIGHT_SLOT);
        } else if let Some(list) = &self.file_list {
            list.draw(&mut grid, layout::RIGHT_SLOT);
        } else if self.log_open {
            self.log
                .draw(&mut grid, layout::RIGHT_SLOT, right_slot_focused);
        } else if let Some(view) = &self.cart_view {
            view.draw(
                &mut grid,
                layout::RIGHT_SLOT,
                right_slot_focused,
                self.project.target_cart(),
                export::needs(&self.project),
            );
        } else {
            self.overview.draw(
                &mut grid,
                layout::RIGHT_SLOT,
                right_slot_focused,
                self.project.export_track_numbers(),
                &self.scope,
            );
        }
        layout::draw_separators(&mut grid);
        let project = self.project_name();
        status_line::draw(&mut grid, &self.status_line(&project));
        grid
    }

    fn status_line<'a>(&'a self, project: &'a str) -> StatusLine<'a> {
        if let Some(entry) = &self.speed_entry {
            return StatusLine::Question(entry.text());
        }
        if let Some(question) = &self.question {
            return StatusLine::Question(&question.text);
        }
        let notice = self.notice.as_ref();
        if let Some(notice) = notice.filter(|notice| notice.is_persistent()) {
            return StatusLine::Notice(notice);
        }
        if let Some(remark) = self.remark {
            return StatusLine::Remark(remark);
        }
        if let Some(notice) = notice {
            return StatusLine::Notice(notice);
        }
        if self.picking_song.is_some() {
            return StatusLine::Hints(SONG_PICKER_KEYS);
        }
        match self.file_list.as_ref().map(FileList::purpose) {
            Some(Purpose::SaveAs) => StatusLine::Hints(SAVE_LIST_KEYS),
            Some(Purpose::Open | Purpose::Import | Purpose::TargetCart | Purpose::LocateCart) => {
                StatusLine::Hints(OPEN_LIST_KEYS)
            }
            None => StatusLine::Idle {
                project,
                unsaved: self.has_unsaved_changes(),
            },
        }
    }
}

fn next_held_index(
    from: usize,
    step: isize,
    count: usize,
    holds: impl Fn(usize) -> bool,
) -> Option<usize> {
    let mut index = from;
    loop {
        index = index
            .checked_add_signed(step)
            .filter(|&index| index < count)?;
        if holds(index) {
            return Some(index);
        }
    }
}
