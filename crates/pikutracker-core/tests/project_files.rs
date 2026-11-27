mod support;

use pikutracker_core::app::layout;
use pikutracker_core::app::{
    Action, App, Direction, DirectoryEntry, FileRequest, FileResponse, KeyContext,
};
use pikutracker_core::keymap::{Key, KeyPress, Keymap, Modifiers};
use pikutracker_core::player::{AudioRequest, Player};
use pikutracker_core::project::{ChainEntry, ChainId, PhraseId, Project, Step};
use pikutracker_core::project_file;
use pikutracker_core::screen::palette::Colour;
use std::path::{Path, PathBuf};
use support::{pane_text, status_line};

const STATUS_ROW: usize = 44;
const DOWN: Action = Action::Move(Direction::Down);
const UNSAVED_QUESTION: &str = "UNSAVED CHANGES WILL BE LOST  ENTER DISCARD  ESC CANCEL";

fn right_slot(app: &App, line: usize) -> String {
    pane_text(app, layout::RIGHT_SLOT, line)
}

fn file(name: &str) -> DirectoryEntry {
    DirectoryEntry {
        name: name.to_owned(),
        is_directory: false,
    }
}

fn folder(name: &str) -> DirectoryEntry {
    DirectoryEntry {
        name: name.to_owned(),
        is_directory: true,
    }
}

fn answer_listing(app: &mut App, directory: &str, entries: Vec<DirectoryEntry>) {
    assert_eq!(
        app.take_file_requests(),
        [FileRequest::List {
            directory: PathBuf::from(directory)
        }]
    );
    app.receive_file(FileResponse::Listed {
        directory: PathBuf::from(directory),
        entries,
    });
}

fn type_name(app: &mut App, name: &str) {
    for ch in name.chars() {
        app.apply(Action::TypeChar(ch));
    }
}

fn place_chain_01_on_the_first_song_row(app: &mut App) {
    app.apply(Action::HexDigit(1));
}

fn written(app: &mut App, path: &str) -> String {
    let mut requests = app.take_file_requests();
    assert_eq!(requests.len(), 1, "{requests:?}");
    match requests.remove(0) {
        FileRequest::Write {
            path: written,
            contents,
        } => {
            assert_eq!(written, Path::new(path));
            contents
        }
        other => panic!("expected a write, got {other:?}"),
    }
}

fn saved_app() -> App {
    let mut app = App::new();
    place_chain_01_on_the_first_song_row(&mut app);
    app.apply(Action::SaveAs);
    answer_listing(&mut app, ".", vec![]);
    type_name(&mut app, "demo");
    app.apply(Action::Confirm);
    written(&mut app, "./demo.ptrk");
    app.receive_file(FileResponse::Written {
        path: PathBuf::from("./demo.ptrk"),
    });
    app
}

fn other_project_text() -> String {
    let mut project = Project::new();
    project.set_song_cell(2, 1, Some(ChainId::new(5)));
    project_file::write(&project)
}

fn open_file(app: &mut App, name: &str, contents: &str) {
    app.apply(Action::OpenProject);
    answer_listing(app, ".", vec![file(name)]);
    app.apply(DOWN);
    app.apply(Action::Confirm);
    let path = Path::new(".").join(name);
    assert_eq!(
        app.take_file_requests(),
        [FileRequest::Read { path: path.clone() }]
    );
    app.receive_file(FileResponse::Read {
        path,
        contents: contents.to_owned(),
    });
}

#[test]
fn saving_a_new_project_asks_for_a_name_on_the_file_list() {
    let mut app = App::new();
    place_chain_01_on_the_first_song_row(&mut app);

    app.apply(Action::Save);
    answer_listing(&mut app, ".", vec![file("old.ptrk")]);

    assert_eq!(right_slot(&app, 0), "SAVE PROJECT AS");
    assert_eq!(right_slot(&app, 2), ">NAME _");
    assert_eq!(right_slot(&app, 3), " ../");
    assert_eq!(right_slot(&app, 4), " old.ptrk");
    assert_eq!(app.key_context(), KeyContext::FileName);

    type_name(&mut app, "demo");
    assert_eq!(right_slot(&app, 2), ">NAME demo_");
    app.apply(Action::Confirm);

    let contents = written(&mut app, "./demo.ptrk");
    assert_eq!(&project_file::parse(&contents).unwrap(), app.project());

    app.receive_file(FileResponse::Written {
        path: PathBuf::from("./demo.ptrk"),
    });
    assert_eq!(status_line(&app), "SAVED DEMO.PTRK");
    assert!(!app.has_unsaved_changes());
    assert!(right_slot(&app, 0).starts_with("MULTITRACK"));
}

#[test]
fn saving_again_writes_to_the_same_file_without_asking() {
    let mut app = saved_app();
    place_chain_01_on_the_first_song_row(&mut app);

    app.apply(Action::Save);

    written(&mut app, "./demo.ptrk");
}

#[test]
fn backspace_takes_the_last_character_off_the_name() {
    let mut app = App::new();
    app.apply(Action::SaveAs);
    answer_listing(&mut app, ".", vec![]);
    type_name(&mut app, "demos");

    app.apply(Action::Backspace);
    app.apply(Action::Confirm);

    written(&mut app, "./demo.ptrk");
}

#[test]
fn a_name_that_already_ends_in_the_extension_keeps_it() {
    let mut app = App::new();
    app.apply(Action::SaveAs);
    answer_listing(&mut app, ".", vec![]);

    type_name(&mut app, "demo.ptrk");
    app.apply(Action::Confirm);

    written(&mut app, "./demo.ptrk");
}

#[test]
fn characters_a_file_name_cannot_hold_are_not_typed() {
    let mut app = App::new();
    app.apply(Action::SaveAs);
    answer_listing(&mut app, ".", vec![]);

    type_name(&mut app, "../de/mo");
    app.apply(Action::Confirm);

    written(&mut app, "./..demo.ptrk");
}

#[test]
fn an_empty_name_saves_nothing() {
    let mut app = App::new();
    app.apply(Action::SaveAs);
    answer_listing(&mut app, ".", vec![]);

    app.apply(Action::Confirm);

    assert_eq!(app.take_file_requests(), []);
    assert_eq!(right_slot(&app, 0), "SAVE PROJECT AS");
}

#[test]
fn save_as_starts_from_the_current_name_and_saves_under_a_new_one() {
    let mut app = saved_app();

    app.apply(Action::SaveAs);
    answer_listing(&mut app, ".", vec![file("demo.ptrk")]);
    assert_eq!(right_slot(&app, 2), ">NAME demo.ptrk_");

    for _ in 0.."demo.ptrk".len() {
        app.apply(Action::Backspace);
    }
    type_name(&mut app, "take2");
    app.apply(Action::Confirm);
    written(&mut app, "./take2.ptrk");
    app.receive_file(FileResponse::Written {
        path: PathBuf::from("./take2.ptrk"),
    });

    place_chain_01_on_the_first_song_row(&mut app);
    app.apply(Action::Save);
    written(&mut app, "./take2.ptrk");
}

#[test]
fn save_as_asks_before_replacing_a_file_that_exists() {
    let mut app = App::new();
    app.apply(Action::SaveAs);
    answer_listing(&mut app, ".", vec![file("demo.ptrk")]);
    type_name(&mut app, "demo");

    app.apply(Action::Confirm);
    assert_eq!(app.take_file_requests(), []);
    assert_eq!(
        status_line(&app),
        "DEMO.PTRK EXISTS  ENTER OVERWRITE  ESC CANCEL"
    );
    assert_eq!(app.grid().cell(0, STATUS_ROW).style.bg, Colour::Blue);

    app.apply(Action::Cancel);
    assert_eq!(app.take_file_requests(), []);
    assert_eq!(right_slot(&app, 0), "SAVE PROJECT AS");

    app.apply(Action::Confirm);
    app.apply(Action::Confirm);
    written(&mut app, "./demo.ptrk");
}

#[test]
fn enter_on_a_file_in_save_as_takes_its_name() {
    let mut app = App::new();
    app.apply(Action::SaveAs);
    answer_listing(&mut app, ".", vec![file("demo.ptrk")]);
    app.apply(DOWN);
    app.apply(DOWN);

    app.apply(Action::Confirm);

    assert_eq!(right_slot(&app, 2), ">NAME demo.ptrk_");
    assert_eq!(app.take_file_requests(), []);
}

#[test]
fn a_failed_save_says_why_and_stays_unsaved() {
    let mut app = App::new();
    place_chain_01_on_the_first_song_row(&mut app);
    app.apply(Action::SaveAs);
    answer_listing(&mut app, ".", vec![]);
    type_name(&mut app, "demo");
    app.apply(Action::Confirm);
    written(&mut app, "./demo.ptrk");

    app.receive_file(FileResponse::Failed {
        path: PathBuf::from("./demo.ptrk"),
        reason: "disk full".to_owned(),
    });

    assert_eq!(status_line(&app), "! CANNOT SAVE DEMO.PTRK: DISK FULL");
    assert_eq!(app.grid().cell(0, STATUS_ROW).style.bg, Colour::Red);
    assert!(app.has_unsaved_changes());
}

#[test]
fn edits_made_while_a_save_is_written_stay_unsaved() {
    let mut app = App::new();
    place_chain_01_on_the_first_song_row(&mut app);
    app.apply(Action::SaveAs);
    answer_listing(&mut app, ".", vec![]);
    type_name(&mut app, "demo");
    app.apply(Action::Confirm);
    written(&mut app, "./demo.ptrk");

    app.apply(Action::HexDigit(2));
    app.receive_file(FileResponse::Written {
        path: PathBuf::from("./demo.ptrk"),
    });

    assert!(app.has_unsaved_changes());
}

#[test]
fn the_open_list_shows_folders_then_project_files_in_order() {
    let mut app = App::new();
    app.apply(Action::OpenProject);
    answer_listing(
        &mut app,
        ".",
        vec![
            file("zebra.ptrk"),
            file("notes.txt"),
            folder("songs"),
            file("Alpha.PTRK"),
            folder("Carts"),
        ],
    );

    assert_eq!(right_slot(&app, 0), "OPEN PROJECT");
    assert_eq!(right_slot(&app, 2), ">../");
    assert_eq!(right_slot(&app, 3), " Carts/");
    assert_eq!(right_slot(&app, 4), " songs/");
    assert_eq!(right_slot(&app, 5), " Alpha.PTRK");
    assert_eq!(right_slot(&app, 6), " zebra.ptrk");
    assert_eq!(right_slot(&app, 7), "");
}

#[test]
fn enter_on_a_folder_lists_it_and_enter_on_dots_goes_back_up() {
    let mut app = App::new();
    app.apply(Action::OpenProject);
    answer_listing(&mut app, ".", vec![folder("songs")]);

    app.apply(DOWN);
    app.apply(Action::Confirm);
    answer_listing(&mut app, "./songs", vec![]);
    assert_eq!(right_slot(&app, 1), "./songs");

    app.apply(Action::Confirm);
    answer_listing(&mut app, ".", vec![]);
}

#[test]
fn esc_closes_the_file_list_and_brings_back_the_right_hand_view() {
    let mut app = App::new();
    app.apply(Action::ToggleLog);
    app.apply(Action::OpenProject);
    answer_listing(&mut app, ".", vec![]);

    app.apply(Action::Cancel);

    assert_eq!(right_slot(&app, 0), "LOG");
    assert_eq!(app.take_file_requests(), []);
}

#[test]
fn a_folder_that_cannot_be_listed_is_reported() {
    let mut app = App::new();
    app.apply(Action::OpenProject);
    app.take_file_requests();

    app.receive_file(FileResponse::Failed {
        path: PathBuf::from("."),
        reason: "permission denied".to_owned(),
    });

    assert_eq!(status_line(&app), "! CANNOT LIST .: PERMISSION DENIED");
    assert_eq!(right_slot(&app, 0), "OPEN PROJECT");
}

#[test]
fn a_folder_inside_the_list_that_cannot_be_listed_is_reported() {
    let mut app = App::new();
    app.apply(Action::OpenProject);
    answer_listing(&mut app, ".", vec![folder("locked")]);
    app.apply(DOWN);
    app.apply(Action::Confirm);
    app.take_file_requests();

    app.receive_file(FileResponse::Failed {
        path: PathBuf::from("./locked"),
        reason: "permission denied".to_owned(),
    });

    assert_eq!(
        status_line(&app),
        "! CANNOT LIST ./locked: PERMISSION DENIED"
    );
    assert_eq!(right_slot(&app, 1), ".");
}

#[test]
fn opening_a_file_replaces_the_project_and_says_so() {
    let mut app = App::new();

    open_file(&mut app, "song.ptrk", &other_project_text());

    assert_eq!(
        app.project(),
        &project_file::parse(&other_project_text()).unwrap()
    );
    assert_eq!(status_line(&app), "OPENED SONG.PTRK");
    assert!(!app.has_unsaved_changes());
    assert!(right_slot(&app, 0).starts_with("MULTITRACK"));

    place_chain_01_on_the_first_song_row(&mut app);
    app.apply(Action::Save);
    written(&mut app, "./song.ptrk");
}

#[test]
fn each_project_opens_at_the_top_of_the_song_with_the_song_pane_focused() {
    let mut app = App::new();
    for _ in 0..5 {
        app.apply(DOWN);
    }
    app.apply(Action::Move(Direction::Right));
    app.apply(Action::FocusPrevious);
    app.apply(DOWN);
    app.apply(Action::FocusPrevious);

    open_file(&mut app, "song.ptrk", &other_project_text());

    assert_eq!(pane_text(&app, layout::SONG, 2), "00 [--]-- -- --");
    assert_eq!(pane_text(&app, layout::CHAIN, 2), "00 --     ---");
}

#[test]
fn a_new_project_keeps_the_keyboard_octave() {
    let mut app = App::new();
    app.apply(Action::OctaveUp);

    app.apply(Action::NewProject);

    assert!(app.grid().text_dump().contains("OCT 3"));
}

#[test]
fn a_newer_file_is_refused_with_a_clear_message_and_changes_nothing() {
    let mut app = saved_app();
    let before = app.project().clone();

    open_file(&mut app, "new.ptrk", "(version: 99)");

    assert_eq!(
        status_line(&app),
        "! CANNOT OPEN NEW.PTRK: MADE BY A NEWER PIKUTRACKER: FORMAT 99, THIS VERSION READS UP TO 1"
    );
    assert_eq!(app.project(), &before);

    app.apply(Action::Cancel);
    place_chain_01_on_the_first_song_row(&mut app);
    app.apply(Action::Save);
    written(&mut app, "./demo.ptrk");
}

#[test]
fn a_damaged_file_is_refused_and_changes_nothing() {
    let mut app = saved_app();
    let before = app.project().clone();

    open_file(&mut app, "bad.ptrk", "this is not a project");

    assert!(status_line(&app).starts_with("! CANNOT OPEN BAD.PTRK: NOT A READABLE PROJECT"));
    assert_eq!(app.project(), &before);
}

#[test]
fn a_file_that_cannot_be_read_is_reported() {
    let mut app = App::new();
    app.apply(Action::OpenProject);
    answer_listing(&mut app, ".", vec![file("song.ptrk")]);
    app.apply(DOWN);
    app.apply(Action::Confirm);
    app.take_file_requests();

    app.receive_file(FileResponse::Failed {
        path: PathBuf::from("./song.ptrk"),
        reason: "no such file".to_owned(),
    });

    assert_eq!(status_line(&app), "! CANNOT OPEN SONG.PTRK: NO SUCH FILE");
}

#[test]
fn moving_the_cursor_never_marks_a_project_as_unsaved() {
    let mut app = App::new();

    for action in [
        DOWN,
        Action::Move(Direction::Right),
        Action::FocusPrevious,
        DOWN,
        Action::FocusPrevious,
        DOWN,
        Action::FocusPrevious,
        Action::ToggleLog,
    ] {
        app.apply(action);
        assert!(!app.has_unsaved_changes(), "{action:?}");
    }
}

#[test]
fn changing_the_music_marks_a_project_as_unsaved_and_shows_it() {
    let mut app = App::new();
    assert!(status_line(&app).ends_with("UNTITLED"));

    place_chain_01_on_the_first_song_row(&mut app);

    assert!(app.has_unsaved_changes());
    assert!(status_line(&app).ends_with("UNTITLED*"));
}

#[test]
fn putting_the_music_back_clears_the_unsaved_mark() {
    let mut app = App::new();
    place_chain_01_on_the_first_song_row(&mut app);

    app.apply(Action::Clear);

    assert!(!app.has_unsaved_changes());
}

#[test]
fn a_new_project_asks_before_replacing_unsaved_changes() {
    let mut app = App::new();
    place_chain_01_on_the_first_song_row(&mut app);

    app.apply(Action::NewProject);
    assert_eq!(status_line(&app), UNSAVED_QUESTION);
    assert_eq!(app.grid().cell(0, STATUS_ROW).style.bg, Colour::Blue);

    app.apply(Action::Cancel);
    assert!(app.has_unsaved_changes());
    assert_eq!(app.project().song_cell(0, 0), Some(ChainId::new(1)));

    app.apply(Action::NewProject);
    app.apply(Action::Confirm);
    assert_eq!(app.project(), &Project::new());
    assert!(!app.has_unsaved_changes());
}

#[test]
fn opening_asks_before_replacing_unsaved_changes() {
    let mut app = App::new();
    place_chain_01_on_the_first_song_row(&mut app);

    app.apply(Action::OpenProject);
    assert_eq!(status_line(&app), UNSAVED_QUESTION);
    assert_eq!(app.take_file_requests(), []);

    app.apply(Action::Confirm);
    answer_listing(&mut app, ".", vec![]);
    assert_eq!(right_slot(&app, 0), "OPEN PROJECT");
}

#[test]
fn a_project_with_no_unsaved_changes_is_replaced_without_asking() {
    let mut app = saved_app();

    app.apply(Action::NewProject);

    assert_eq!(app.project(), &Project::new());
}

#[test]
fn other_keys_wait_while_a_question_is_asked() {
    let mut app = App::new();
    place_chain_01_on_the_first_song_row(&mut app);
    app.apply(Action::NewProject);

    app.apply(Action::HexDigit(9));
    app.apply(DOWN);

    assert_eq!(status_line(&app), UNSAVED_QUESTION);
    assert_eq!(app.project().song_cell(0, 0), Some(ChainId::new(1)));
    assert_eq!(app.project().song_cell(1, 0), None);
}

fn playing_app() -> App {
    let mut app = App::new();
    app.apply(Action::HexDigit(0));
    app.apply(Action::FocusPrevious);
    app.apply(Action::HexDigit(0));
    app.apply(Action::FocusPrevious);
    app.apply(Action::PianoKey(0));
    app.apply(Action::PlayFocusedPane);
    app.take_audio_requests();
    app
}

#[test]
fn opening_another_project_stops_playback() {
    let mut app = playing_app();
    app.apply(Action::NewProject);
    app.apply(Action::Confirm);

    assert!(matches!(
        app.take_audio_requests().as_slice(),
        [AudioRequest::Stop]
    ));
}

#[test]
fn saving_does_not_stop_playback() {
    let mut app = playing_app();
    app.apply(Action::SaveAs);
    answer_listing(&mut app, ".", vec![]);
    type_name(&mut app, "demo");
    app.apply(Action::Confirm);
    written(&mut app, "./demo.ptrk");
    app.receive_file(FileResponse::Written {
        path: PathBuf::from("./demo.ptrk"),
    });

    assert!(app.take_audio_requests().is_empty());
}

#[test]
fn a_project_holding_custom_instruments_opens_and_plays() {
    let mut project = Project::new();
    project.set_song_cell(0, 0, Some(ChainId::new(0)));
    *project.chain_mut(ChainId::new(0)).entry_mut(0) = Some(ChainEntry::new(PhraseId::new(0)));
    *project.phrase_mut(PhraseId::new(0)).step_mut(0) = Step::new(24, 8, 5, 0).unwrap();
    let mut app = App::new();

    open_file(&mut app, "song.ptrk", &project_file::write(&project));
    app.apply(Action::PlayFocusedPane);

    let mut player = Player::new();
    assert_eq!(support::play_along(&mut app, &mut player, 1000).len(), 1000);
}

#[test]
fn the_keys_for_dialogs_are_bound() {
    let keymap = Keymap::pc_tracker();
    let press = |key, modifiers| KeyPress::new(key, modifiers);

    assert_eq!(
        keymap.action_for(press(Key::Enter, Modifiers::NONE)),
        Some(Action::Confirm)
    );
    assert_eq!(
        keymap.action_for(press(Key::Escape, Modifiers::NONE)),
        Some(Action::Cancel)
    );
}

#[test]
fn letters_type_into_a_file_name_keeping_their_case() {
    let keymap = Keymap::pc_tracker();
    let in_name = |press| keymap.action_in(press, KeyContext::FileName);

    assert_eq!(
        in_name(KeyPress::new(Key::Char('d'), Modifiers::NONE)),
        Some(Action::TypeChar('d'))
    );
    assert_eq!(
        in_name(KeyPress::new(Key::Char('D'), Modifiers::SHIFT)),
        Some(Action::TypeChar('D'))
    );
    assert_eq!(
        in_name(KeyPress::new(Key::Char('_'), Modifiers::SHIFT)),
        Some(Action::TypeChar('_'))
    );
    assert_eq!(
        in_name(KeyPress::repeated(Key::Backspace, Modifiers::NONE)),
        Some(Action::Backspace)
    );
    assert_eq!(
        in_name(KeyPress::new(Key::Char('s'), Modifiers::CTRL)),
        Some(Action::Save)
    );
}
