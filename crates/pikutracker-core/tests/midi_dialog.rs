mod midi_support;

use midi_support::{Song, TICKS_PER_STEP};
use pikutracker_core::app::{
    Action, App, Direction, DirectoryEntry, FileRequest, FileResponse, KeyContext,
};
use pikutracker_core::keymap::{Key, KeyPress, Keymap, Modifiers};
use pikutracker_core::project::Project;
use pikutracker_core::screen::palette::Colour;
use std::path::PathBuf;

const MIDDLE_C: u8 = 60;
const STATUS_ROW: usize = 44;
const DOWN: Action = Action::Move(Direction::Down);
const LEFT: Action = Action::Move(Direction::Left);
const RIGHT: Action = Action::Move(Direction::Right);
const UNSAVED_QUESTION: &str = "UNSAVED CHANGES WILL BE LOST  ENTER DISCARD  ESC CANCEL";

fn screen(app: &App) -> String {
    app.grid().text_dump()
}

fn status_line(app: &App) -> String {
    screen(app).lines().last().unwrap().to_owned()
}

fn shows(app: &App, text: &str) -> bool {
    screen(app).contains(text)
}

fn melody() -> Song {
    Song::new().note(0, TICKS_PER_STEP, 1, MIDDLE_C, 127)
}

fn band() -> Song {
    Song::new()
        .note(0, TICKS_PER_STEP, 1, MIDDLE_C, 127)
        .note(0, TICKS_PER_STEP, 1, MIDDLE_C + 4, 127)
        .note(0, TICKS_PER_STEP, 10, 36, 127)
}

fn dialog_on(song: &Song) -> App {
    let mut app = App::new();
    app.start_midi_import("song.mid", &song.bytes());
    app
}

fn press(app: &mut App, actions: &[Action]) {
    for action in actions {
        app.apply(*action);
    }
}

fn import(app: &mut App) {
    press(
        app,
        &[
            Action::Confirm,
            Action::Confirm,
            Action::Confirm,
            Action::Confirm,
        ],
    );
}

fn first_step_instrument(app: &App, track: usize) -> u8 {
    let project = app.project();
    let entry = project.chain_row_entry(0, track, 0).unwrap();
    project.phrase(entry.phrase()).step(0).instrument()
}

#[test]
fn the_dialog_opens_on_voices_with_the_pages_in_the_header() {
    let app = dialog_on(&melody());

    assert!(shows(
        &app,
        "1 VOICES > 2 STEPS PER BEAT > 3 INSTRUMENTS > 4 REVIEW"
    ));
    assert!(shows(&app, "PIANO"));
    assert!(!shows(&app, "CHAIN"), "the dialog replaces the panes");
}

#[test]
fn the_header_highlights_the_current_page() {
    let mut app = dialog_on(&melody());
    let header = screen(&app);
    let voices = header.lines().next().unwrap().find("1 VOICES").unwrap();
    assert_eq!(app.grid().cell(voices, 0).style.fg, Colour::Black);

    app.apply(Action::Confirm);

    assert_ne!(app.grid().cell(voices, 0).style.fg, Colour::Black);
}

#[test]
fn enter_moves_forward_and_esc_moves_back() {
    let mut app = dialog_on(&melody());

    app.apply(Action::Confirm);
    assert!(shows(&app, "STEPS/BEAT"));
    app.apply(Action::Confirm);
    assert!(shows(&app, "DRUM KIT"));
    app.apply(Action::Confirm);
    assert!(shows(&app, "WARNINGS"));
    app.apply(Action::Cancel);
    assert!(shows(&app, "DRUM KIT"));
    app.apply(Action::Cancel);
    assert!(shows(&app, "STEPS/BEAT"));
}

#[test]
fn esc_on_voices_cancels_and_changes_nothing() {
    let mut app = App::new();
    app.apply(Action::HexDigit(1));
    let before = app.project().clone();
    app.start_midi_import("song.mid", &melody().bytes());

    app.apply(Action::Cancel);

    assert_eq!(*app.project(), before);
    assert!(shows(&app, "CHAIN"));
    assert!(app.has_unsaved_changes());
    app.apply(Action::Undo);
    assert_eq!(*app.project(), Project::new(), "the undo history survives");
}

#[test]
fn enter_on_review_imports_the_file() {
    let mut app = dialog_on(&melody());

    import(&mut app);

    assert!(app.project().chain_row_entry(0, 0, 0).is_some());
    assert!(shows(&app, "CHAIN"), "the dialog has closed");
}

#[test]
fn each_voice_shows_its_track_and_ticks_decide_what_is_kept() {
    let mut app = dialog_on(&band());
    assert!(shows(&app, "T1"));
    assert!(shows(&app, "CH10 KICK"));

    app.apply(Action::ToggleVoice);
    import(&mut app);

    let entry = app.project().chain_row_entry(0, 0, 0).unwrap();
    assert_eq!(app.project().phrase(entry.phrase()).step(0).pitch(), 24);
    assert!(app.project().chain_row_entry(0, 2, 0).is_none());
}

#[test]
fn s_ticks_a_whole_source() {
    let mut app = dialog_on(&band());

    app.apply(Action::ToggleSource);
    import(&mut app);

    assert_eq!(first_step_instrument(&app, 0), 0);
    assert!(app.project().chain_row_entry(0, 1, 0).is_none());
}

#[test]
fn more_than_four_ticked_voices_show_as_dropped_and_say_how_to_keep_them() {
    let song = (0..5).fold(Song::new(), |song, voice| {
        song.note(0, TICKS_PER_STEP, 1, MIDDLE_C + voice, 127)
    });

    let app = dialog_on(&song);

    assert!(shows(&app, "DROP"));
    assert!(shows(&app, "1 VOICE DROPPED: UNTICK OTHERS TO KEEP IT"));
}

#[test]
fn the_steps_per_beat_page_compares_every_choice_and_starts_on_4() {
    let mut app = dialog_on(&melody());
    app.apply(Action::Confirm);

    for row in ["  2", "  3", "> 4", "  6", "  8"] {
        assert!(shows(&app, row), "{row}");
    }
    assert!(shows(&app, "SPEED"));
}

#[test]
fn up_down_and_left_right_pick_the_steps_per_beat() {
    let mut app = dialog_on(&melody());
    app.apply(Action::Confirm);

    press(&mut app, &[DOWN, RIGHT, LEFT]);
    assert!(shows(&app, "> 6"));
    app.apply(DOWN);
    press(
        &mut app,
        &[Action::Confirm, Action::Confirm, Action::Confirm],
    );

    let entry = app.project().chain_row_entry(0, 0, 0).unwrap();
    assert_eq!(app.project().phrase(entry.phrase()).speed(), 8);
}

#[test]
fn the_instruments_page_edits_the_program_table_for_this_import() {
    let mut app = dialog_on(&melody());
    press(&mut app, &[Action::Confirm, Action::Confirm]);
    assert!(shows(&app, "TILTED SAW"));

    app.apply(RIGHT);

    assert!(shows(&app, "SAW *"));
    press(&mut app, &[Action::Confirm, Action::Confirm]);
    assert_eq!(first_step_instrument(&app, 0), 2);
}

#[test]
fn the_drum_kit_edits_instrument_note_and_effect() {
    let mut app = dialog_on(&Song::new().note(0, TICKS_PER_STEP, 10, 36, 127));
    press(&mut app, &[Action::Confirm, Action::Confirm]);

    press(
        &mut app,
        &[
            Action::SwitchMidiTable,
            RIGHT,
            Action::MidiDrumNote { up: true },
            Action::CycleDrumEffect,
        ],
    );
    press(&mut app, &[Action::Confirm, Action::Confirm]);

    let entry = app.project().chain_row_entry(0, 0, 0).unwrap();
    let step = app.project().phrase(entry.phrase()).step(0);
    assert_eq!((step.instrument(), step.pitch(), step.effect()), (1, 13, 4));
}

#[test]
fn r_resets_a_row_and_shift_r_resets_everything() {
    let mut app = dialog_on(&melody());
    press(&mut app, &[Action::Confirm, Action::Confirm, RIGHT]);

    app.apply(Action::ResetMidiRow);
    assert!(!shows(&app, "*"));

    press(
        &mut app,
        &[
            RIGHT,
            Action::SwitchMidiTable,
            RIGHT,
            Action::ResetMidiTable,
        ],
    );
    assert!(!shows(&app, "*"));
}

#[test]
fn cancelling_discards_table_edits() {
    let mut app = dialog_on(&melody());
    press(&mut app, &[Action::Confirm, Action::Confirm, RIGHT]);
    press(&mut app, &[Action::Cancel, Action::Cancel, Action::Cancel]);

    app.start_midi_import("song.mid", &melody().bytes());
    import(&mut app);

    assert_eq!(first_step_instrument(&app, 0), 1);
}

#[test]
fn review_shows_the_tracks_the_readout_the_table_and_every_warning() {
    let song = melody().controller(0, 1, 7, 100);
    let mut app = dialog_on(&song);

    press(
        &mut app,
        &[Action::Confirm, Action::Confirm, RIGHT, Action::Confirm],
    );

    assert!(shows(&app, "T1 PIANO"));
    assert!(shows(&app, "SPEED 0F  32 STEPS = 2 BARS A PHRASE"));
    assert!(shows(&app, "TABLE EDITED"));
    assert!(shows(&app, "CC7 (VOLUME) IGNORED."));
}

#[test]
fn warnings_go_to_the_status_line_and_the_log_after_import() {
    let song = melody().controller(0, 1, 7, 100);
    let mut app = dialog_on(&song);

    import(&mut app);

    assert_eq!(
        status_line(&app).trim_end(),
        "! IMPORTED SONG.MID WITH 1 WARNING  ^L LOG"
    );
    assert_eq!(app.grid().cell(0, STATUS_ROW).style.bg, Colour::Orange);
    assert!(shows(&app, "LOG"));
    assert!(shows(&app, "CC7 (VOLUME)"));
}

#[test]
fn a_clean_import_reports_the_file_name() {
    let mut app = dialog_on(&melody());

    import(&mut app);

    assert_eq!(status_line(&app).trim_end(), "IMPORTED SONG.MID");
}

#[test]
fn import_asks_before_replacing_unsaved_changes() {
    let mut app = App::new();
    app.apply(Action::HexDigit(1));
    let before = app.project().clone();

    app.apply(Action::Import);
    assert_eq!(status_line(&app).trim_end(), UNSAVED_QUESTION);
    assert!(app.take_file_requests().is_empty());

    app.apply(Action::Cancel);
    assert_eq!(*app.project(), before);
    assert!(app.take_file_requests().is_empty());

    app.apply(Action::Import);
    app.apply(Action::Confirm);
    assert_eq!(
        app.take_file_requests(),
        [FileRequest::List {
            directory: PathBuf::from(".")
        }]
    );
    assert_eq!(*app.project(), before, "only Import replaces the Project");
}

#[test]
fn saying_yes_to_the_prompt_leads_through_the_dialog_with_no_second_prompt() {
    let mut app = App::new();
    app.apply(Action::HexDigit(1));
    press(&mut app, &[Action::Import, Action::Confirm]);
    app.take_file_requests();
    app.receive_file(FileResponse::Listed {
        directory: PathBuf::from("."),
        entries: vec![DirectoryEntry {
            name: "song.mid".to_owned(),
            is_directory: false,
        }],
    });
    press(&mut app, &[DOWN, Action::Confirm]);
    app.take_file_requests();
    app.receive_file(FileResponse::BytesRead {
        path: PathBuf::from("./song.mid"),
        contents: melody().bytes(),
    });

    import(&mut app);

    assert!(app.project().chain_row_entry(0, 0, 0).is_some());
    assert_eq!(status_line(&app).trim_end(), "IMPORTED SONG.MID");
}

#[test]
fn import_starts_an_empty_undo_history() {
    let mut app = App::new();
    app.apply(Action::HexDigit(1));
    app.start_midi_import("song.mid", &melody().bytes());
    import(&mut app);
    let imported = app.project().clone();

    app.apply(Action::Undo);

    assert_eq!(*app.project(), imported);
    assert!(shows(&app, "NOTHING TO UNDO"));
}

#[test]
fn a_file_that_cannot_be_read_as_midi_reports_a_failure_and_opens_no_dialog() {
    let mut app = App::new();

    app.start_midi_import("broken.mid", b"nonsense");

    assert_eq!(
        status_line(&app).trim_end(),
        "! CANNOT IMPORT BROKEN.MID: NOT A STANDARD MIDI FILE"
    );
    assert!(shows(&app, "CHAIN"));
}

#[test]
fn ctrl_i_lists_midi_files_and_opens_the_selected_one() {
    let mut app = App::new();

    app.apply(Action::Import);
    assert_eq!(
        app.take_file_requests(),
        [FileRequest::List {
            directory: PathBuf::from(".")
        }]
    );
    let entries = ["song.mid", "notes.txt"].map(|name| DirectoryEntry {
        name: name.to_owned(),
        is_directory: false,
    });
    app.receive_file(FileResponse::Listed {
        directory: PathBuf::from("."),
        entries: entries.to_vec(),
    });
    assert!(shows(&app, "song.mid"));
    assert!(!shows(&app, "notes.txt"));

    press(&mut app, &[DOWN, Action::Confirm]);
    let path = PathBuf::from("./song.mid");
    assert_eq!(
        app.take_file_requests(),
        [FileRequest::ReadBytes { path: path.clone() }]
    );
    app.receive_file(FileResponse::BytesRead {
        path,
        contents: melody().bytes(),
    });

    assert!(shows(&app, "1 VOICES > 2 STEPS PER BEAT"));
}

#[test]
fn the_dialog_keys_reach_their_actions_through_the_keymap() {
    let app = dialog_on(&melody());
    let keymap = Keymap::pc_tracker();
    assert_eq!(app.key_context(), KeyContext::MidiImport);

    let none = Modifiers::NONE;
    let cases = [
        (Key::Space, none, Action::ToggleVoice),
        (Key::Char('s'), none, Action::ToggleSource),
        (Key::Tab, none, Action::SwitchMidiTable),
        (Key::Char('['), none, Action::MidiDrumNote { up: false }),
        (Key::Char(']'), none, Action::MidiDrumNote { up: true }),
        (Key::Char('f'), none, Action::CycleDrumEffect),
        (Key::Char('r'), none, Action::ResetMidiRow),
        (Key::Char('R'), Modifiers::SHIFT, Action::ResetMidiTable),
    ];
    for (key, modifiers, action) in cases {
        let press = KeyPress::new(key, modifiers);
        assert_eq!(
            keymap.action_in(press, KeyContext::MidiImport),
            Some(action),
            "{key:?}"
        );
    }
}
