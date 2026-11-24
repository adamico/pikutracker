mod midi_support;
mod support;

use midi_support::{Song, TICKS_PER_STEP};
use pikutracker_core::app::{Action, App, DirectoryEntry, FileResponse};
use pikutracker_core::project::{ChainEntry, ChainId, PhraseId, Project};
use std::path::PathBuf;
use support::{M8_STYLE, apply, launched, status_line};

const REBOUND_ANSWERS: &str =
    r#"(version: 1, keys: {"Confirm": "F12", "Cancel": "F11", "MoveUp": "Ctrl+W"})"#;
const KICK: u8 = 36;

fn status(app: &App) -> String {
    status_line(app).trim().to_owned()
}

fn with_unsaved_changes(app: &mut App) {
    app.apply(Action::HexDigit(1));
}

fn one_phrase_in_the_song() -> Project {
    let mut project = Project::new();
    project.set_song_cell(0, 0, Some(ChainId::new(0)));
    *project.chain_mut(ChainId::new(0)).entry_mut(0) = Some(ChainEntry::new(PhraseId::new(0)));
    project
}

fn list_one_project(app: &mut App) {
    app.take_file_requests();
    app.receive_file(FileResponse::Listed {
        directory: PathBuf::from("."),
        entries: vec![DirectoryEntry {
            name: "song.ptrk".to_owned(),
            is_directory: false,
        }],
    });
}

#[test]
fn the_idle_status_line_shows_only_the_project_name() {
    let mut app = App::new();
    assert_eq!(status(&app), "UNTITLED");

    with_unsaved_changes(&mut app);

    assert_eq!(status(&app), "UNTITLED*");
}

#[test]
fn a_question_names_the_keys_that_answer_it() {
    let mut app = launched(App::new(), REBOUND_ANSWERS);
    with_unsaved_changes(&mut app);

    app.apply(Action::NewProject);

    assert_eq!(
        status(&app),
        "UNSAVED CHANGES WILL BE LOST  F12 DISCARD  F11 CANCEL"
    );
}

#[test]
fn the_speed_entry_names_the_keys_that_answer_it() {
    let mut app = launched(App::with_project(one_phrase_in_the_song()), REBOUND_ANSWERS);

    app.apply(Action::ChangeSpeed);

    assert!(
        status(&app).ends_with("F12 APPLY  F11 CANCEL"),
        "{}",
        status(&app)
    );
}

#[test]
fn the_open_list_names_its_keys() {
    let mut app = launched(App::new(), REBOUND_ANSWERS);

    app.apply(Action::OpenProject);
    list_one_project(&mut app);

    assert_eq!(status(&app), "^W/DOWN PICK  F12 CHOOSE  F11 CANCEL");
}

#[test]
fn the_save_list_names_its_keys() {
    let mut app = launched(App::new(), M8_STYLE);

    app.apply(Action::SaveAs);
    list_one_project(&mut app);

    assert_eq!(
        status(&app),
        "TYPE A NAME  UP/DOWN PICK  ENTER CHOOSE  ESC CANCEL"
    );
}

#[test]
fn the_midi_import_drum_kit_names_the_m8_style_keys() {
    let mut app = launched(App::new(), M8_STYLE);
    let drums = Song::new().note(0, TICKS_PER_STEP, 10, KICK, 127);
    app.start_midi_import("song.mid", &drums.bytes());

    apply(
        &mut app,
        &[Action::Confirm, Action::Confirm, Action::SwitchMidiTable],
    );

    assert_eq!(
        status(&app),
        "TAB TABLE  LEFT/RIGHT INSTRUMENT  Z+DOWN/Z+UP NOTE  F EFFECT  Z+X RESET  \
         SHIFT+R RESET ALL  ESC BACK"
    );
}

#[test]
fn the_midi_import_voices_page_names_rebound_keys() {
    let mut app = launched(App::new(), REBOUND_ANSWERS);
    let melody = Song::new().note(0, TICKS_PER_STEP, 1, 60, 127);
    app.start_midi_import("song.mid", &melody.bytes());

    assert_eq!(
        status(&app),
        "^W/DOWN PICK  SPACE TICK  S TICK SOURCE  F12 NEXT  F11 CANCEL"
    );
}

#[test]
fn the_cart_view_names_the_key_that_picks_a_target_cart() {
    let mut app = launched(App::new(), REBOUND_ANSWERS);

    app.apply(Action::ToggleCart);

    let screen = app.grid().text_dump();
    assert!(screen.contains("│     F12 PICKS ONE"), "{screen}");
}
