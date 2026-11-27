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
mod key_hints;
mod launching;
pub mod layout;
mod messages;
mod panes;
mod paths;
mod restructure;
mod selection;
mod shown;
mod status_line;
mod transport;

pub use action::{Action, Direction, KeyContext};
pub use files::{DirectoryEntry, FileRequest, FileResponse};
use launching::HelpAnnouncement;
pub use launching::{Launch, SettingsContents};
pub use messages::{LogItem, Report, Severity};

use crate::export;
use crate::keymap::{Key, KeyPress, Keyboard};
use crate::midi_import::MidiTable;
use crate::multitrack::Timeline;
use crate::player::{AudioRequest, PlaybackReport, Playhead};
use crate::project::{CHAIN_ENTRIES, FIRST_CUSTOM_INSTRUMENT, LAST_CUSTOM_INSTRUMENT, SONG_ROWS};
use crate::project::{
    Chain, ChainEntry, ChainId, Phrase, PhraseId, Project, SongPlace, SongRowMarks, Step, Track,
};
use crate::screen::grid::Grid;
use crate::settings::AppState;
use cell_edits::Clipboard;
use change_speed::{SpeedChange, SpeedEntry};
use files::{Question, Waiting};
use focus::Focus;
use history::{CursorPlace, History, Merge};
use layout::Area;
use messages::Notice;
use panes::cart_view::CartView;
use panes::chain::{ChainPane, ChainView};
use panes::file_list::FileList;
use panes::help::HelpList;
use panes::instrument::InstrumentView;
use panes::log::LogPane;
use panes::midi_dialog::MidiDialog;
use panes::multitrack::{MultitrackPane, MultitrackView};
use panes::phrase::{PhrasePane, PhraseView};
use panes::song::SongPane;
use selection::CellPosition;
use shown::{CursorPlaces, Shown};
use status_line::StatusLine;
use std::borrow::Cow;
use std::path::PathBuf;
use std::time::Duration;
use transport::{PlayTarget, Transport};

const SONG_JUMP_ROWS: usize = 16;
const NOTHING_TO_PLAY: &str = "NOTHING TO PLAY";
const NO_CHAIN_HERE: &str = "NO CHAIN HERE";
const NO_PHRASE_HERE: &str = "NO PHRASE HERE";
const NOTHING_TO_UNDO: &str = "NOTHING TO UNDO";
const NOTHING_TO_REDO: &str = "NOTHING TO REDO";
const FOLLOW_ON: &str = "FOLLOW ON";
const FOLLOW_OFF: &str = "FOLLOW OFF";
const BUILT_IN_INSTRUMENT: &str = "BUILT-IN INSTRUMENTS HAVE NO SETTINGS";
const CUSTOM_INSTRUMENTS_END_AT_F: &str = "CUSTOM INSTRUMENTS END AT F";

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
    multitrack: MultitrackPane,
    shown: Shown,
    played_pane: Focus,
    transport: Transport,
    follow: bool,
    follow_set_by_key: bool,
    remark: Option<Cow<'static, str>>,
    notice: Option<Notice>,
    question: Option<Question>,
    speed_entry: Option<SpeedEntry>,
    log: LogPane,
    log_open: bool,
    help_open: bool,
    help_announcement: HelpAnnouncement,
    focus_before_log: Focus,
    file_list: Option<FileList>,
    midi_dialog: Option<Box<MidiDialog>>,
    instrument_view: Option<InstrumentView>,
    cart_view: Option<CartView>,
    cart_visits: u32,
    waiting: Option<Waiting>,
    file_requests: Vec<FileRequest>,
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
        let mut app = Self {
            saved_project: project.clone(),
            transport: Transport::new(&project),
            follow: false,
            follow_set_by_key: false,
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
            multitrack: MultitrackPane::default(),
            shown: Shown::default(),
            played_pane: Focus::default(),
            remark: None,
            notice: None,
            question: None,
            speed_entry: None,
            log: LogPane::default(),
            log_open: false,
            help_open: false,
            help_announcement: HelpAnnouncement::default(),
            focus_before_log: Focus::default(),
            file_list: None,
            midi_dialog: None,
            instrument_view: None,
            cart_view: None,
            cart_visits: 0,
            waiting: None,
            file_requests: Vec::new(),
        };
        app.show_what_the_cursors_point_at();
        app
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    pub fn shown_chain_id(&self) -> Option<ChainId> {
        self.shown.chain()
    }

    pub fn shown_phrase_id(&self) -> Option<PhraseId> {
        self.shown.phrase()
    }

    fn entry_under_chain_cursor(&self) -> Option<ChainEntry> {
        let chain = self.shown_chain_id()?;
        self.project.chain(chain).entry(self.chain.entry())
    }

    fn shown_transpose(&self) -> i8 {
        self.entry_under_chain_cursor()
            .filter(|entry| Some(entry.phrase()) == self.shown_phrase_id())
            .map_or(0, ChainEntry::transpose)
    }

    fn shown_phrase(&self) -> Phrase {
        self.shown_phrase_id()
            .map_or_else(Phrase::default, |id| *self.project.phrase(id))
    }

    fn show_song_place(&mut self, place: SongPlace) {
        self.song.jump_to(place.chain_row.song_row, place.track);
        self.chain.jump_to(place.chain_row.entry);
        self.phrase.jump_to(place.step_index);
        self.show_what_the_cursors_point_at();
    }

    fn cursor_places(&self) -> CursorPlaces {
        CursorPlaces {
            song_row: self.song.row(),
            track: Track::from_index(self.song.track()),
            chain_entry: self.chain.entry(),
        }
    }

    fn show_what_the_cursors_point_at(&mut self) {
        let cursors = self.cursor_places();
        self.shown.follow_cursors(&self.project, cursors);
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
        if let Some(list) = &self.file_list {
            return if list.purpose().takes_a_name() {
                KeyContext::FileName
            } else {
                KeyContext::Navigation
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
            Focus::Chain => KeyContext::ReferenceValue,
            Focus::Phrase => self.phrase.key_context(),
            _ => KeyContext::Navigation,
        }
    }

    pub fn take_audio_requests(&mut self) -> Vec<AudioRequest> {
        self.transport.take_requests()
    }

    pub fn show_playback(&mut self, report: &PlaybackReport) -> bool {
        let moved = self.transport.receive(report);
        if moved && self.selection_start.is_none() {
            self.follow_playhead();
        }
        moved
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
        self.transport.end_preview();
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
        if step.is_silent() || !(entered || changed) {
            return;
        }
        let transpose = self.shown_transpose();
        self.transport
            .preview(&self.project, phrase, index, transpose);
    }

    fn follow_playhead(&mut self) {
        let Some(playhead) = self.transport.playhead() else {
            return;
        };
        if let Some(place) = playhead.chain_row {
            self.song.follow(place.song_row);
            let timeline = Timeline::of(&self.project);
            if let Some(tick) = playhead_tick(&timeline, &playhead) {
                self.multitrack.follow(tick, &timeline);
            }
        }
        if !self.follow {
            let cursors = self.cursor_places();
            self.shown
                .keep_on_show_past_the_song_cursor(&self.project, cursors);
            return;
        }
        if let Some(place) = playhead.chain_row {
            self.chain.follow(place.entry);
        }
        if let Some(place) = playhead.chain_alone {
            self.shown.open_chain(place.chain);
            self.chain.follow(place.entry);
        }
        self.show_what_the_cursors_point_at();
        let step = self
            .shown_phrase_id()
            .and_then(|phrase| playhead.step_in(phrase, self.song.track()));
        if let Some(step) = step {
            self.phrase.follow(step);
        }
    }

    fn show_the_song_cursor_in_the_multitrack(&mut self) {
        let timeline = Timeline::of(&self.project);
        if let Some(tick) = timeline.song_row_start(self.song.row()) {
            let track = Track::from_index(self.song.track());
            self.multitrack.show_song_row(track, tick, &timeline);
        }
    }

    fn toggle_follow(&mut self) {
        self.follow = !self.follow;
        self.follow_set_by_key = self.follow;
        self.remark = Some(if self.follow { FOLLOW_ON } else { FOLLOW_OFF }.into());
    }

    fn stop_following(&mut self) {
        if self.follow {
            self.follow = false;
            self.remark = Some(FOLLOW_OFF.into());
        }
    }

    pub fn report(&mut self, report: Report) {
        let log_pointer = key_hints::log_pointer(self.keyboard.keymap());
        let text = report.status_text(&log_pointer);
        self.notice = Some(Notice::new(report.severity, text));
        if self.help_announcement == HelpAnnouncement::Showing {
            self.help_announcement = HelpAnnouncement::Over;
        }
        if !report.items.is_empty() && !self.log_open {
            self.log_open = true;
            self.help_open = false;
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
        self.end_help_announcement(action);
        let merge = match action {
            _ if self.cart_view_has_focus() => Some(Merge::CartVisit(self.cart_visits)),
            Action::Nudge { .. } if self.selection_start.is_none() => Some(Merge::Nudge),
            Action::HexDigit(_) => Some(Merge::Digit),
            _ => None,
        };
        self.history.begin(&self.project, self.cursor_place());
        let song_cursor = (self.song.row(), self.song.track());
        let from_the_multitrack = self.focus == Focus::RightSlot;
        self.perform(action);
        if !from_the_multitrack && song_cursor != (self.song.row(), self.song.track()) {
            self.show_the_song_cursor_in_the_multitrack();
        }
        self.show_what_the_cursors_point_at();
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
            shown: self.shown,
        }
    }

    fn return_to(&mut self, place: CursorPlace) {
        self.selection_start = None;
        self.stop_following();
        self.set_focus(place.focus);
        self.song.return_to(place.song);
        self.chain.return_to(place.chain);
        self.phrase.return_to(place.phrase);
        self.shown = place.shown;
    }

    fn undo(&mut self) {
        match self.history.undo(&mut self.project) {
            Some(place) => self.return_to(place),
            None => self.remark = Some(NOTHING_TO_UNDO.into()),
        }
        self.show_claim_in_cart_view();
    }

    fn redo(&mut self) {
        match self.history.redo(&mut self.project) {
            Some(place) => self.return_to(place),
            None => self.remark = Some(NOTHING_TO_REDO.into()),
        }
        self.show_claim_in_cart_view();
    }

    fn perform(&mut self, action: Action) {
        if action == Action::ToggleHelp {
            self.help_open = !self.help_open;
            return;
        }
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
        if self.use_title(action) {
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
            Action::Move(direction) => {
                self.selection_start = None;
                self.move_cursor(direction);
            }
            Action::Jump(direction) => {
                self.selection_start = None;
                self.jump(direction);
            }
            Action::Select(direction) => {
                self.start_selection_unless_started();
                self.move_cursor(direction);
            }
            Action::SelectJump(direction) => {
                self.start_selection_unless_started();
                self.jump(direction);
            }
            Action::FocusNext => self.set_focus(self.focus.next()),
            Action::FocusPrevious => self.set_focus(self.focus.previous()),
            Action::PlayFocusedPane => self.play_or_stop(),
            Action::PlaySongFromCursor => self.play_song(),
            Action::ToggleFollow => self.toggle_follow(),
            Action::Mute { track_number } => self.transport.change_track_mix(|mix| {
                mix.toggle_mute(usize::from(track_number) - 1);
            }),
            Action::Solo { track_number } => self.transport.change_track_mix(|mix| {
                mix.toggle_solo(usize::from(track_number) - 1);
            }),
            Action::ClearMutesAndSolos => self.transport.clear_track_mix(),
            Action::OctaveUp => self.phrase.nudge_octave(true),
            Action::OctaveDown => self.phrase.nudge_octave(false),
            Action::ToggleLoopStart => {
                self.song
                    .change_marks(&mut self.project, SongRowMarks::toggle_loop_start);
            }
            Action::ToggleLoopBack => {
                self.song
                    .change_marks(&mut self.project, SongRowMarks::toggle_loop_back);
            }
            Action::ToggleStop => {
                self.song
                    .change_marks(&mut self.project, SongRowMarks::toggle_stop);
            }
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
        self.transport.project_changed(&self.project);
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
        if self.focus == Focus::Phrase && self.shown_phrase_id().is_some() {
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
            self.show_song_place(location);
            self.set_focus(Focus::Phrase);
        }
    }

    fn persistent_notice_showing(&self) -> bool {
        self.notice.as_ref().is_some_and(Notice::is_persistent)
    }

    fn cancel(&mut self) {
        if self.persistent_notice_showing() {
            self.notice = None;
            self.announce_help_when_free();
        } else if self.log_has_focus() {
            self.close_log();
        } else if self.focus == Focus::RightSlot {
            self.set_focus(Focus::Phrase);
        }
    }

    fn open_instrument_view(&mut self, instrument: u8) {
        if instrument < FIRST_CUSTOM_INSTRUMENT {
            self.remark = Some(BUILT_IN_INSTRUMENT.into());
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
                if self.transport.is_playing() {
                    self.transport.stop();
                }
            }
            Action::PlayFocusedPane if self.transport.is_playing() => self.transport.stop(),
            Action::PlayFocusedPane => {
                let instrument = view.id();
                self.transport
                    .play(PlayTarget::Audition { instrument }, &self.project);
            }
            Action::Undo => self.undo(),
            Action::Redo => self.redo(),
            Action::Save => self.save(),
            Action::OctaveUp => self.phrase.nudge_octave(true),
            Action::OctaveDown => self.phrase.nudge_octave(false),
            Action::Jump(Direction::Right) if view.id() == LAST_CUSTOM_INSTRUMENT => {
                self.remark = Some(CUSTOM_INSTRUMENTS_END_AT_F.into());
            }
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
            _ => view.apply(action, &mut self.project, self.phrase.pen().octave()),
        }
    }

    fn use_title(&mut self, action: Action) -> bool {
        let (title, shown, open): (_, _, fn(&mut Shown, u8)) = match self.focus {
            Focus::Chain => (
                self.chain.title_mut(),
                self.shown.chain().map(ChainId::number),
                |shown, number| shown.open_chain(ChainId::new(number)),
            ),
            Focus::Phrase => (
                self.phrase.title_mut(),
                self.shown.phrase().map(PhraseId::number),
                |shown, number| shown.open_phrase(PhraseId::new(number)),
            ),
            Focus::Song | Focus::RightSlot => return false,
        };
        if !title.has_cursor() {
            return false;
        }
        let number = match action {
            Action::HexDigit(digit) => title.type_digit(digit),
            Action::Nudge { up, coarse } => title.nudged(shown, up, coarse),
            _ => return action.acts_on_cells(),
        };
        open(&mut self.shown, number);
        true
    }

    fn type_digit(&mut self, digit: u8) {
        match self.focus {
            Focus::Song => self.song.type_digit(&mut self.project, digit),
            Focus::Chain => self.edit_chain(|pane, chain| pane.type_digit(chain, digit)),
            Focus::Phrase if self.phrase.is_on_the_pen() => {
                self.phrase.type_pen_digit(digit);
                self.remark_an_undefined_instrument();
            }
            Focus::Phrase => {
                self.edit_phrase(|pane, phrase| pane.type_digit(phrase, digit));
                self.remark_an_undefined_instrument();
            }
            Focus::RightSlot => {}
        }
    }

    fn remark_an_undefined_instrument(&mut self) {
        let Some(instrument) = self.phrase.edited_instrument(&self.shown_phrase()) else {
            return;
        };
        let undefined = instrument >= FIRST_CUSTOM_INSTRUMENT
            && self.project.custom_instrument(instrument).is_none();
        if undefined {
            self.remark = Some(format!("CUSTOM INSTRUMENT {instrument:X} IS NOT DEFINED").into());
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
            Focus::Phrase if up && self.goes_past_the_last_instrument() => {
                self.remark = Some(CUSTOM_INSTRUMENTS_END_AT_F.into());
            }
            Focus::Phrase if self.phrase.is_on_the_pen() => {
                self.phrase.nudge_pen(up);
                self.remark_an_undefined_instrument();
            }
            Focus::Phrase => {
                self.edit_phrase(|pane, phrase| pane.nudge(phrase, up, coarse));
                self.remark_an_undefined_instrument();
            }
            Focus::RightSlot => {}
        }
    }

    fn goes_past_the_last_instrument(&self) -> bool {
        self.phrase.is_on_the_last_instrument(&self.shown_phrase())
    }

    fn edit_chain(&mut self, edit: impl FnOnce(&mut ChainPane, &mut Chain)) {
        let Some(id) = self.shown_chain_id() else {
            self.remark = Some(NO_CHAIN_HERE.into());
            return;
        };
        if self.entry_under_chain_cursor().is_none() && !self.chain.edits_empty_entries() {
            self.remark = Some(NO_PHRASE_HERE.into());
            return;
        }
        edit(&mut self.chain, self.project.chain_mut(id));
    }

    fn edit_phrase(&mut self, edit: impl FnOnce(&mut PhrasePane, &mut Phrase)) {
        match self.shown_phrase_id() {
            Some(id) => edit(&mut self.phrase, self.project.phrase_mut(id)),
            None => self.remark = Some(NO_PHRASE_HERE.into()),
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
        if self.transport.is_playing() {
            self.transport.stop();
            return;
        }
        match self.played_pane {
            Focus::Song | Focus::RightSlot => self.play_song(),
            Focus::Chain => self.play_chain(),
            Focus::Phrase => self.play_phrase(),
        }
    }

    fn play_song(&mut self) {
        let from_row = self.song.row();
        if self.project.song_row_is_empty(from_row) {
            self.remark = Some(NOTHING_TO_PLAY.into());
            return;
        }
        self.transport
            .play(PlayTarget::Song { from_row }, &self.project);
    }

    fn play_chain(&mut self) {
        let Some(chain) = self.shown_chain_id().filter(|_| self.shown_chain_sounds()) else {
            self.remark = Some(NOTHING_TO_PLAY.into());
            return;
        };
        let in_song_row = self.chain_in_song_row_shown();
        let from_entry = self.chain.entry();
        let target = if in_song_row {
            PlayTarget::ChainInSongRow {
                song_row: self.song.row(),
                track: self.song.track(),
                from_entry,
            }
        } else {
            PlayTarget::ChainAlone { chain, from_entry }
        };
        self.transport.play(target, &self.project);
    }

    fn shown_chain_sounds(&self) -> bool {
        self.shown_chain_id().is_some_and(|id| {
            let chain = self.project.chain(id);
            (0..CHAIN_ENTRIES)
                .filter_map(|index| chain.entry(index))
                .any(|entry| !self.project.phrase(entry.phrase()).is_silent())
        })
    }

    fn play_phrase(&mut self) {
        let Some(phrase) = self
            .shown_phrase_id()
            .filter(|id| !self.project.phrase(*id).is_silent())
        else {
            self.remark = Some(NOTHING_TO_PLAY.into());
            return;
        };
        let target = PlayTarget::Phrase {
            phrase,
            transpose: self.shown_transpose(),
            from_step: self.phrase.step(),
        };
        self.transport.play(target, &self.project);
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
                let Some(chain) = self.shown_chain_id() else {
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
            Focus::RightSlot if self.shows_multitrack() => {
                self.stop_following();
                let timeline = Timeline::of(&self.project);
                self.multitrack.move_cursor(direction, &timeline);
                self.show_what_the_multitrack_cursor_points_at(&timeline);
            }
            Focus::RightSlot => {}
        }
    }

    fn show_what_the_multitrack_cursor_points_at(&mut self, timeline: &Timeline) {
        let Some(step) = self.multitrack.step(timeline) else {
            return;
        };
        let place = step.place;
        self.show_song_place(place);
        if let Some(chain) = self
            .project
            .song_cell(place.chain_row.song_row, place.track)
        {
            self.shown.open_chain(chain);
        }
        self.shown.open_phrase(step.phrase);
    }

    fn shown_help(&self) -> Option<(HelpList, Area)> {
        if !self.help_open {
            return None;
        }
        if self.speed_entry.is_some() {
            return Some((HelpList::SpeedEntry, self.help_area_away_from_focus()));
        }
        if self.question.is_some() {
            return None;
        }
        if let Some(dialog) = &self.midi_dialog {
            let keys = dialog.help_list();
            let area = if keys == HelpList::MidiDrumKit {
                layout::LEFT_AREA
            } else {
                layout::RIGHT_SLOT
            };
            return Some((keys, area));
        }
        if let Some(list) = &self.file_list {
            let keys = if list.purpose().takes_a_name() {
                HelpList::SaveList
            } else {
                HelpList::OpenList
            };
            return Some((keys, layout::LEFT_AREA));
        }
        let keys = match self.focus {
            Focus::Phrase => HelpList::Phrase,
            Focus::Chain => HelpList::Chain,
            Focus::Song => HelpList::Song,
            Focus::RightSlot if self.log_open => HelpList::Log,
            Focus::RightSlot if self.cart_view.is_some() => HelpList::CartView,
            Focus::RightSlot => return None,
        };
        Some((keys, self.help_area_away_from_focus()))
    }

    fn help_area_away_from_focus(&self) -> Area {
        if self.focus == Focus::RightSlot {
            layout::LEFT_AREA
        } else {
            layout::RIGHT_SLOT
        }
    }

    fn shows_multitrack(&self) -> bool {
        self.file_list.is_none() && !self.log_open && self.cart_view.is_none()
    }

    fn playing_entry_of_shown_chain(&self) -> Option<usize> {
        let playhead = self.transport.playhead()?;
        let shown = self.shown_chain_id()?;
        if let Some(place) = playhead.chain_alone {
            return (place.chain == shown).then_some(place.entry);
        }
        let place = playhead.chain_row?;
        let in_song_row = place.song_row == self.song.row() && self.chain_in_song_row_shown();
        in_song_row.then_some(place.entry)
    }

    fn chain_in_song_row_shown(&self) -> bool {
        self.shown_chain_id().is_some()
            && self.shown_chain_id() == self.project.song_cell(self.song.row(), self.song.track())
    }

    fn selection_would_reach_phrase_header(&self, direction: Direction) -> bool {
        self.selection_start.is_some() && direction == Direction::Up && self.phrase.step() == 0
    }

    pub fn grid(&self) -> Grid {
        let mut grid = Grid::new();
        if let Some(dialog) = &self.midi_dialog {
            dialog.draw(&mut grid);
            if let Some((keys, area)) = self.shown_help() {
                panes::help::draw(&mut grid, area, self.keyboard.keymap(), keys);
                layout::draw_the_right_separator(&mut grid);
            }
            let hints = dialog.hints(self.keyboard.keymap());
            status_line::draw(&mut grid, &StatusLine::Hints(hints));
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
        let takes_keys = self.file_list.is_some();
        let focus = (!takes_keys).then_some(self.focus);
        let project = &self.project;
        self.song.draw(
            &mut grid,
            layout::SONG,
            focus == Some(Focus::Song),
            project,
            self.transport.track_mix(),
            self.transport.playhead().as_ref(),
        );
        self.chain.draw(
            &mut grid,
            layout::CHAIN,
            focus == Some(Focus::Chain),
            project,
            ChainView {
                id: self.shown_chain_id(),
                playing_entry: self.playing_entry_of_shown_chain(),
            },
        );
        let phrase = self.shown_phrase_id();
        self.phrase.draw(
            &mut grid,
            layout::PHRASE,
            focus == Some(Focus::Phrase),
            PhraseView {
                id: phrase,
                phrase: &self.shown_phrase(),
                transpose: self.shown_transpose(),
                playing_step: self
                    .transport
                    .playhead()
                    .zip(phrase)
                    .and_then(|(playhead, phrase)| playhead.step_in(phrase, self.song.track())),
                pen_instrument_name: panes::instrument_name(
                    &self.project,
                    self.phrase.pen().instrument(),
                ),
            },
        );
        self.draw_selection(&mut grid);
        let right_slot_focused = focus == Some(Focus::RightSlot);
        let help = self.shown_help();
        if let Some(list) = &self.file_list {
            list.draw(&mut grid, layout::RIGHT_SLOT);
        } else if let Some((keys, area)) = help.filter(|(_, area)| *area == layout::RIGHT_SLOT) {
            panes::help::draw(&mut grid, area, self.keyboard.keymap(), keys);
        } else if self.log_open {
            self.log
                .draw(&mut grid, layout::RIGHT_SLOT, right_slot_focused);
        } else if let Some(view) = &self.cart_view {
            view.draw(
                &mut grid,
                layout::RIGHT_SLOT,
                right_slot_focused,
                self.project.target_cart(),
                &export::needs(&self.project),
                &key_hints::picks_one(self.keyboard.keymap()),
            );
        } else {
            let timeline = Timeline::of(&self.project);
            self.multitrack.draw(
                &mut grid,
                layout::RIGHT_SLOT,
                right_slot_focused,
                &MultitrackView {
                    project: &self.project,
                    timeline: &timeline,
                    playhead_tick: self
                        .transport
                        .playhead()
                        .and_then(|playhead| playhead_tick(&timeline, &playhead)),
                },
            );
        }
        layout::draw_separators(&mut grid);
        if let Some((keys, area)) = help.filter(|(_, area)| *area == layout::LEFT_AREA) {
            panes::help::draw(&mut grid, area, self.keyboard.keymap(), keys);
            layout::draw_the_right_separator(&mut grid);
        }
        let project = self.project_name();
        status_line::draw(&mut grid, &self.status_line(&project));
        grid
    }

    fn status_line<'a>(&'a self, project: &'a str) -> StatusLine<'a> {
        let keymap = self.keyboard.keymap();
        if let Some(entry) = &self.speed_entry {
            let text = key_hints::asked(
                keymap,
                KeyContext::SpeedEntry,
                entry.text(),
                change_speed::APPLY,
            );
            return StatusLine::Question(text);
        }
        if let Some(question) = &self.question {
            let context = KeyContext::Navigation;
            let text = key_hints::asked(keymap, context, &question.text, question.answer);
            return StatusLine::Question(text);
        }
        let notice = self.notice.as_ref();
        if let Some(notice) = notice.filter(|notice| notice.is_persistent()) {
            return StatusLine::Notice(notice);
        }
        if let Some(remark) = &self.remark {
            return StatusLine::Remark(remark);
        }
        if let Some(notice) = notice {
            return StatusLine::Notice(notice);
        }
        match self.file_list.as_ref().map(FileList::purpose) {
            Some(purpose) if purpose.takes_a_name() => {
                StatusLine::Hints(key_hints::save_list_keys(keymap))
            }
            Some(_) => StatusLine::Hints(key_hints::open_list_keys(keymap)),
            None => StatusLine::Idle {
                project,
                unsaved: self.has_unsaved_changes(),
            },
        }
    }
}

fn playhead_tick(timeline: &Timeline, playhead: &Playhead) -> Option<u32> {
    timeline.tick_of(playhead.chain_row?, playhead.chain_row_tick)
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
