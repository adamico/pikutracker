mod support;

use pikutracker_core::app::{
    Action, App, Direction, DirectoryEntry, FileRequest, FileResponse, KeyContext,
};
use pikutracker_core::keymap::{Key, KeyPress, Keymap, Modifiers};
use pikutracker_core::player::AudioRequest;
use pikutracker_core::project::Project;
use std::path::{Path, PathBuf};
use support::status_line;
use support::{TestCart, TestPattern, TestSfx};

const DOWN: Action = Action::Move(Direction::Down);
const PICKER_KEYS: &str = "UP/DOWN PICK  P LISTEN  ENTER IMPORT  ESC BACK";
const UNSAVED_QUESTION: &str = "UNSAVED CHANGES WILL BE LOST  ENTER DISCARD  ESC CANCEL";

fn right_slot(app: &App, line: usize) -> String {
    let text: String = app
        .grid()
        .text_dump()
        .lines()
        .nth(line)
        .unwrap()
        .chars()
        .skip(83)
        .collect();
    text.trim_end().to_owned()
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

fn answer_listing(app: &mut App, entries: Vec<DirectoryEntry>) {
    assert_eq!(
        app.take_file_requests(),
        [FileRequest::List {
            directory: PathBuf::from(".")
        }]
    );
    app.receive_file(FileResponse::Listed {
        directory: PathBuf::from("."),
        entries,
    });
}

fn simple_cart() -> TestCart {
    let mut last = TestPattern::playing([Some(1), Some(0), None, None]);
    last.stop = true;
    TestCart {
        sfx: vec![
            TestSfx::with_pitches(8, &[12, 14]),
            TestSfx::with_pitches(8, &[19, 21]),
        ],
        patterns: vec![TestPattern::playing([Some(0), None, Some(1), None]), last],
    }
}

fn import_file(app: &mut App, name: &str, contents: Vec<u8>) {
    app.apply(Action::Import);
    answer_listing(app, vec![file(name)]);
    app.apply(DOWN);
    app.apply(Action::Confirm);
    let path = Path::new(".").join(name);
    assert_eq!(
        app.take_file_requests(),
        [FileRequest::ReadBytes { path: path.clone() }]
    );
    app.receive_file(FileResponse::BytesRead { path, contents });
}

fn place_chain_01_on_the_first_song_row(app: &mut App) {
    app.apply(Action::HexDigit(1));
}

#[test]
fn import_opens_a_file_list_of_folders_and_carts() {
    let mut app = App::new();

    app.apply(Action::Import);
    answer_listing(
        &mut app,
        vec![
            file("song.ptrk"),
            file("demo.p8"),
            file("notes.txt"),
            file("picture.png"),
            file("LABEL.P8.PNG"),
            folder("carts"),
        ],
    );

    assert_eq!(right_slot(&app, 0), "IMPORT FILE");
    assert_eq!(right_slot(&app, 2), ">../");
    assert_eq!(right_slot(&app, 3), " carts/");
    assert_eq!(right_slot(&app, 4), " demo.p8");
    assert_eq!(right_slot(&app, 5), " LABEL.P8.PNG");
    assert_eq!(right_slot(&app, 6), "");
}

#[test]
fn selecting_a_p8_cart_opens_it_as_a_new_project() {
    let mut app = App::new();

    import_file(&mut app, "demo.p8", simple_cart().p8());

    let project = app.project();
    assert!(project.chain_row_entry(0, 0, 0).is_some());
    assert!(project.chain_row_entry(1, 1, 0).is_some());
    assert_eq!(project.target_cart().unwrap().path, "demo.p8");
    assert_eq!(status_line(&app), "IMPORTED SONG 1 OF DEMO.P8");
    assert_eq!(right_slot(&app, 0), "SCOPE");
}

#[test]
fn selecting_a_png_cart_gives_the_same_project_as_its_text_twin() {
    let mut from_text = App::new();
    let mut from_pixels = App::new();

    import_file(&mut from_text, "demo.p8", simple_cart().p8());
    import_file(&mut from_pixels, "demo.p8.png", simple_cart().png());

    let mut expected = from_text.project().clone();
    let mut target = expected.target_cart().unwrap().clone();
    target.path = "demo.p8.png".to_owned();
    expected.set_target_cart(Some(target));
    assert_eq!(from_pixels.project(), &expected);
    assert_eq!(status_line(&from_pixels), "IMPORTED SONG 1 OF DEMO.P8.PNG");
}

#[test]
fn an_imported_project_starts_with_an_empty_undo_history() {
    let mut app = App::new();
    place_chain_01_on_the_first_song_row(&mut app);
    app.apply(Action::Import);
    app.apply(Action::Confirm);
    answer_listing(&mut app, vec![file("demo.p8")]);
    app.apply(DOWN);
    app.apply(Action::Confirm);
    let path = PathBuf::from("./demo.p8");
    app.take_file_requests();
    app.receive_file(FileResponse::BytesRead {
        path,
        contents: simple_cart().p8(),
    });
    let imported = app.project().clone();

    app.apply(Action::Undo);

    assert_eq!(status_line(&app), "NOTHING TO UNDO");
    assert_eq!(app.project(), &imported);
}

#[test]
fn import_asks_first_when_there_are_unsaved_changes() {
    let mut app = App::new();
    place_chain_01_on_the_first_song_row(&mut app);
    let edited = app.project().clone();

    app.apply(Action::Import);

    assert_eq!(status_line(&app), UNSAVED_QUESTION);
    assert_eq!(app.take_file_requests(), []);

    app.apply(Action::Cancel);
    assert_eq!(app.take_file_requests(), []);
    assert_eq!(app.project(), &edited);

    app.apply(Action::Import);
    app.apply(Action::Confirm);
    answer_listing(&mut app, vec![file("demo.p8")]);
    assert_eq!(right_slot(&app, 0), "IMPORT FILE");
}

#[test]
fn leaving_the_file_list_changes_nothing() {
    let mut app = App::new();
    place_chain_01_on_the_first_song_row(&mut app);
    let edited = app.project().clone();

    app.apply(Action::Import);
    app.apply(Action::Confirm);
    answer_listing(&mut app, vec![file("demo.p8")]);
    app.apply(Action::Cancel);

    assert_eq!(right_slot(&app, 0), "SCOPE");
    assert_eq!(app.project(), &edited);
}

#[test]
fn import_stops_playback() {
    let mut app = App::new();
    import_file(&mut app, "first.p8", simple_cart().p8());
    app.apply(Action::PlayFocusedPane);
    app.take_audio_requests();

    import_file(&mut app, "second.p8", simple_cart().p8());

    assert!(app.take_audio_requests().contains(&AudioRequest::Stop));
}

#[test]
fn warnings_are_counted_on_the_status_line_and_listed_in_the_log() {
    let cart = TestCart {
        sfx: vec![TestSfx::with_pitches(0, &[12])],
        patterns: vec![TestPattern::playing([Some(0), None, None, None])],
    };
    let mut app = App::new();

    import_file(&mut app, "demo.p8", cart.p8());

    assert_eq!(
        status_line(&app),
        "! IMPORTED SONG 1 OF DEMO.P8 WITH 1 WARNING  ^L LOG"
    );
    let dump = app.grid().text_dump();
    assert!(dump.contains("SFX 00 HAS SPD 0"), "{dump}");
}

#[test]
fn enter_on_a_timing_warning_jumps_to_the_step_it_is_about() {
    let mut setter = TestSfx::with_pitches(8, &[12]);
    setter.loop_start = 5;
    let cart = TestCart {
        sfx: vec![
            TestSfx::with_pitches(8, &[20]),
            setter,
            TestSfx::with_pitches(3, &[24]),
        ],
        patterns: vec![
            TestPattern::playing([Some(0), None, None, None]),
            TestPattern::playing([Some(1), Some(2), None, None]),
        ],
    };
    let mut app = App::new();
    import_file(&mut app, "demo.p8", cart.p8());
    let cut_off = app.project().chain_row_entry(1, 1, 0).unwrap().phrase();

    app.apply(Action::FocusPrevious);
    app.apply(Action::Confirm);

    assert_eq!(app.phrase_under_cursor(), Some(cut_off));
    let dump = app.grid().text_dump();
    assert!(dump.contains("0C[...]"), "{dump}");
}

#[test]
fn a_damaged_cart_is_refused_and_the_project_stays() {
    let mut app = App::new();
    place_chain_01_on_the_first_song_row(&mut app);
    let edited = app.project().clone();

    app.apply(Action::Import);
    app.apply(Action::Confirm);
    answer_listing(&mut app, vec![file("demo.p8")]);
    app.apply(DOWN);
    app.apply(Action::Confirm);
    app.take_file_requests();
    app.receive_file(FileResponse::BytesRead {
        path: PathBuf::from("./demo.p8"),
        contents: b"not a cart".to_vec(),
    });

    assert_eq!(
        status_line(&app),
        "! CANNOT IMPORT DEMO.P8: NOT A PICO-8 CART"
    );
    assert_eq!(app.project(), &edited);
    assert_eq!(right_slot(&app, 0), "SCOPE");
}

#[test]
fn a_cart_that_cannot_be_read_says_why() {
    let mut app = App::new();

    app.apply(Action::Import);
    answer_listing(&mut app, vec![file("demo.p8")]);
    app.apply(DOWN);
    app.apply(Action::Confirm);
    app.take_file_requests();
    app.receive_file(FileResponse::Failed {
        path: PathBuf::from("./demo.p8"),
        reason: "permission denied".to_owned(),
    });

    assert_eq!(
        status_line(&app),
        "! CANNOT IMPORT DEMO.P8: PERMISSION DENIED"
    );
}

fn two_song_cart() -> TestCart {
    let mut melody = TestSfx::with_pitches(8, &[12, 14]);
    melody.notes[0].custom = true;
    melody.notes[0].instrument = 2;
    let mut stop = TestPattern::playing([Some(1), None, None, None]);
    stop.stop = true;
    let mut looping = TestPattern::playing([Some(3), None, None, None]);
    looping.loop_start = true;
    looping.loop_back = true;
    TestCart {
        sfx: vec![
            melody,
            TestSfx::with_pitches(8, &[19, 21]),
            TestSfx::with_pitches(4, &[24]),
            TestSfx::with_pitches(16, &[30]),
        ],
        patterns: vec![
            TestPattern::playing([Some(0), Some(1), None, None]),
            stop,
            looping,
        ],
    }
}

fn is_song_2(project: &Project) -> bool {
    let first = project
        .chain_row_entry(0, 0, 0)
        .map(|entry| project.phrase(entry.phrase()));
    first.is_some_and(|phrase| phrase.step(0).pitch() == 30)
        && project.song_row_is_empty(1)
        && project.song_loop_row() == Some(0)
        && project.target_cart().unwrap().claim.first_and_last_pattern == Some((2, 2))
}

fn song_title(app: &App) -> String {
    let dump = app.grid().text_dump();
    dump.lines()
        .next()
        .unwrap()
        .chars()
        .take(28)
        .collect::<String>()
        .trim_end()
        .to_owned()
}

#[test]
fn a_cart_with_several_songs_opens_the_song_picker() {
    let mut app = App::new();
    let before = app.project().clone();

    import_file(&mut app, "demo.p8", two_song_cart().p8());

    assert_eq!(app.project(), &before);
    let lines: Vec<String> = (0..10).map(|line| right_slot(&app, line)).collect();
    assert_eq!(
        lines,
        [
            "IMPORT SONG",
            "DEMO.P8",
            ">SONG 1 PATTERNS 00-01",
            "  3 SFX  0:04  STOP",
            "  INSTRUMENTS 0A",
            "",
            " SONG 2 PATTERN 02",
            "  1 SFX  0:04  LOOP",
            "  NO INSTRUMENTS",
            "",
        ]
    );
    assert_eq!(status_line(&app), PICKER_KEYS);
}

#[test]
fn the_song_pane_previews_the_highlighted_song() {
    let mut app = App::new();
    import_file(&mut app, "demo.p8", two_song_cart().p8());
    assert_eq!(song_title(&app), "SONG  LOOP --");

    app.apply(DOWN);

    assert_eq!(right_slot(&app, 6), ">SONG 2 PATTERN 02");
    assert_eq!(song_title(&app), "SONG  LOOP 00");
    let dump = app.grid().text_dump();
    assert!(dump.contains("F#2"), "{dump}");
}

#[test]
fn p_listens_to_the_highlighted_song_and_stops_it_again() {
    let mut app = App::new();
    import_file(&mut app, "demo.p8", two_song_cart().p8());
    app.take_audio_requests();
    app.apply(DOWN);
    let p = KeyPress::new(Key::Char('p'), Modifiers::NONE);
    let listen = Keymap::pc_tracker()
        .action_in(p, app.key_context())
        .unwrap();

    app.apply(listen);

    assert_eq!(app.key_context(), KeyContext::SongPicker);
    let requests = app.take_audio_requests();
    let [AudioRequest::PlaySong { project, from_row }] = requests.as_slice() else {
        panic!("expected the song to play, got {requests:?}");
    };
    assert!(is_song_2(project));
    assert_eq!(*from_row, 0);
    app.apply(listen);
    assert_eq!(app.take_audio_requests(), [AudioRequest::Stop]);
}

#[test]
fn enter_imports_the_highlighted_song() {
    let mut app = App::new();
    import_file(&mut app, "demo.p8", two_song_cart().p8());
    app.apply(DOWN);

    app.apply(Action::Confirm);

    assert!(is_song_2(app.project()));
    assert_eq!(
        status_line(&app),
        "! IMPORTED SONG 2 OF DEMO.P8 WITH 1 WARNING  ^L LOG"
    );
    assert_eq!(right_slot(&app, 0), "LOG");
}

#[test]
fn esc_leaves_the_song_picker_and_keeps_the_project() {
    let mut app = App::new();
    place_chain_01_on_the_first_song_row(&mut app);
    let edited = app.project().clone();
    app.apply(Action::Import);
    app.apply(Action::Confirm);
    answer_listing(&mut app, vec![file("demo.p8")]);
    app.apply(DOWN);
    app.apply(Action::Confirm);
    let path = PathBuf::from("./demo.p8");
    app.take_file_requests();
    app.receive_file(FileResponse::BytesRead {
        path,
        contents: two_song_cart().p8(),
    });
    app.apply(Action::PlayFocusedPane);
    app.take_audio_requests();

    app.apply(Action::Cancel);

    assert_eq!(app.project(), &edited);
    assert_eq!(right_slot(&app, 0), "SCOPE");
    assert_eq!(app.take_audio_requests(), [AudioRequest::Stop]);
    assert_eq!(app.chain_under_cursor(), edited.song_cell(0, 0));
}

#[test]
fn a_cart_with_no_music_skips_the_song_picker() {
    let cart = TestCart {
        sfx: vec![TestSfx::with_pitches(8, &[12])],
        patterns: vec![],
    };
    let mut app = App::new();

    import_file(&mut app, "demo.p8", cart.p8());

    assert_eq!(right_slot(&app, 0), "LOG");
    assert_eq!(
        status_line(&app),
        "! IMPORTED DEMO.P8 WITH 1 WARNING  ^L LOG"
    );
    assert!(app.project().song_row_is_empty(0));
}
