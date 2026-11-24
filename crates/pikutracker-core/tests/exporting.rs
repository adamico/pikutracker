mod support;

use pikutracker_core::app::layout;
use pikutracker_core::app::{Action, App, Direction, DirectoryEntry, FileRequest, FileResponse};
use pikutracker_core::cart::Cart;
use pikutracker_core::player::AudioRequest;
use pikutracker_core::project::{Claim, PhraseId, Project, TargetCart};
use pikutracker_core::project_file;
use std::path::{Path, PathBuf};
use support::{HEADER, TestCart, TestPattern, TestSfx};
use support::{M8_STYLE, launched, pane_text, status_line};

const DOWN: Action = Action::Move(Direction::Down);
const CART_NAME: &str = "game.p8";

fn right_slot(app: &App, index: usize) -> String {
    pane_text(app, layout::RIGHT_SLOT, index)
}

fn cart_path() -> PathBuf {
    Path::new(".").join(CART_NAME)
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

fn imported(contents: Vec<u8>) -> App {
    let mut app = App::new();
    app.apply(Action::Import);
    app.take_file_requests();
    app.receive_file(FileResponse::Listed {
        directory: PathBuf::from("."),
        entries: vec![DirectoryEntry {
            name: CART_NAME.to_owned(),
            is_directory: false,
        }],
    });
    app.apply(DOWN);
    app.apply(Action::Confirm);
    app.take_file_requests();
    app.receive_file(FileResponse::BytesRead {
        path: cart_path(),
        contents,
    });
    app
}

fn export_with(app: &mut App, contents: Vec<u8>) -> Vec<FileRequest> {
    app.apply(Action::Export);
    assert_eq!(
        app.take_file_requests(),
        [FileRequest::ReadBytes { path: cart_path() }]
    );
    app.receive_file(FileResponse::BytesRead {
        path: cart_path(),
        contents,
    });
    app.take_file_requests()
}

fn export_ok(app: &mut App, contents: Vec<u8>) -> Vec<u8> {
    let requests = export_with(app, contents);
    let [FileRequest::WriteBytes { path, contents }] = &requests[..] else {
        panic!("expected one write, got {requests:?}");
    };
    assert_eq!(*path, cart_path());
    app.receive_file(FileResponse::Written { path: cart_path() });
    contents.clone()
}

fn stored_hash(app: &App) -> Option<String> {
    app.project()
        .target_cart()
        .unwrap()
        .claimed_bytes_hash
        .clone()
}

fn focus_right_slot(app: &mut App) {
    app.apply(Action::FocusNext);
}

fn with_more_song_rows_than_the_claim_holds(app: &App) -> App {
    let mut project = app.project().clone();
    project.set_song_cell(2, 0, project.song_cell(0, 0));
    App::with_project(project)
}

#[test]
fn escape_in_the_right_slot_moves_focus_to_the_phrase_pane() {
    let mut app = App::new();
    focus_right_slot(&mut app);

    app.apply(Action::Cancel);

    assert_eq!(
        app.key_context(),
        pikutracker_core::app::KeyContext::PhrasePitch
    );
}

#[test]
fn the_song_pane_shows_tracks_1_to_4_only() {
    let app = App::new();

    assert_eq!(pane_text(&app, layout::SONG, 1), "    T1 T2 T3 T4");
}

#[test]
fn export_writes_the_cart_and_says_what_it_wrote() {
    let original = simple_cart().p8();
    let mut app = imported(original.clone());

    let written = export_ok(&mut app, original);

    let cart = Cart::read(&written).unwrap();
    assert_eq!(cart.pattern(0).channel(0), Some(0));
    assert_eq!(cart.pattern(0).channel(2), Some(1));
    assert!(cart.pattern(1).stop());
    assert!(status_line(&app).starts_with("EXPORTED TO GAME.P8: 2 SFX, 2 PATTERNS."));
}

#[test]
fn mute_and_solo_change_nothing_export_writes() {
    let original = simple_cart().p8();
    let mut plain = imported(original.clone());
    let mut mixed = imported(original.clone());
    mixed.apply(Action::Mute { track_number: 1 });
    mixed.apply(Action::Solo { track_number: 2 });

    let written = export_ok(&mut mixed, original.clone());

    assert_eq!(written, export_ok(&mut plain, original));
}

#[test]
fn export_stores_the_new_claimed_bytes_hash() {
    let original = simple_cart().p8();
    let mut app = imported(original.clone());
    let claim = app.project().target_cart().unwrap().claim.clone();

    let written = export_ok(&mut app, original);

    let expected = Cart::read(&written).unwrap().claimed_bytes_hash(&claim);
    assert_eq!(stored_hash(&app), Some(expected));
}

#[test]
fn export_does_not_touch_the_undo_history() {
    let original = simple_cart().p8();
    let mut app = imported(original.clone());
    app.apply(Action::HexDigit(7));
    let edited = app.project().song_cell(0, 0);

    export_ok(&mut app, original);
    let hash = stored_hash(&app);
    app.apply(Action::Undo);

    assert_ne!(app.project().song_cell(0, 0), edited);
    assert_eq!(stored_hash(&app), hash);
    app.apply(Action::Undo);
    assert!(status_line(&app).contains("NOTHING TO UNDO"));
}

#[test]
fn export_does_not_stop_playback() {
    let original = simple_cart().p8();
    let mut app = imported(original.clone());
    app.apply(Action::PlaySongFromCursor);
    app.take_audio_requests();

    export_ok(&mut app, original);

    assert!(app.take_audio_requests().is_empty());
    app.apply(Action::PlayFocusedPane);
    assert!(matches!(
        app.take_audio_requests()[..],
        [AudioRequest::Stop]
    ));
}

#[test]
fn an_export_leaves_the_project_to_be_saved_so_the_hash_is_kept() {
    let original = simple_cart().p8();
    let mut app = imported(original.clone());

    export_ok(&mut app, original);

    assert!(app.has_unsaved_changes());
}

#[test]
fn a_failed_export_writes_nothing_and_lists_every_problem() {
    let original = simple_cart().p8();
    let mut app = with_more_song_rows_than_the_claim_holds(&imported(original.clone()));
    let hash = stored_hash(&app);

    let requests = export_with(&mut app, original);

    assert!(requests.is_empty());
    assert_eq!(stored_hash(&app), hash);
    assert!(status_line(&app).contains("EXPORT FAILED: 1 PROBLEM"));
    assert_eq!(right_slot(&app, 0), "LOG");
}

#[test]
fn an_empty_claim_fails_with_the_no_claim_message() {
    let mut project = Project::new();
    project.set_target_cart(Some(TargetCart {
        path: CART_NAME.to_owned(),
        claim: Claim::default(),
        claimed_bytes_hash: None,
    }));
    let mut app = App::with_project(project);

    let requests = export_with(&mut app, HEADER.as_bytes().to_vec());

    assert!(requests.is_empty());
    assert!(status_line(&app).contains("EXPORT FAILED: NO CLAIM IN GAME.P8"));
}

#[test]
fn a_project_file_claiming_a_slot_twice_exports_without_crashing() {
    let original = simple_cart().p8();
    let app = imported(original.clone());
    let mut project = app.project().clone();
    let mut target = project.target_cart().unwrap().clone();
    target.claim.sfx = vec![0, 0, 1];
    project.set_target_cart(Some(target));
    let reopened = project_file::parse(&project_file::write(&project)).unwrap();
    let mut app = App::with_project(reopened);

    let written = export_ok(&mut app, original);

    assert_eq!(Cart::read(&written).unwrap().pattern(0).channel(0), Some(0));
}

#[test]
fn export_without_a_target_cart_opens_the_cart_view() {
    let mut app = App::new();

    app.apply(Action::Export);

    assert!(app.take_file_requests().is_empty());
    assert_eq!(right_slot(&app, 0), "CART");
    assert_eq!(right_slot(&app, 1), ">NO TARGET CART");
}

fn edited_in_pico8() -> Vec<u8> {
    let mut cart = simple_cart();
    cart.sfx[0] = TestSfx::with_pitches(8, &[40]);
    cart.patterns[1].stop = false;
    cart.p8()
}

#[test]
fn changed_claimed_bytes_ask_before_they_are_overwritten() {
    let mut app = imported(simple_cart().p8());

    let requests = export_with(&mut app, edited_in_pico8());

    assert!(requests.is_empty());
    assert_eq!(
        status_line(&app).trim_end(),
        "SFX 00 AND PATTERN 01 CHANGED IN GAME.P8  ENTER OVERWRITE  ESC CANCEL"
    );
}

#[test]
fn enter_overwrites_the_changed_claimed_bytes() {
    let mut app = imported(simple_cart().p8());
    export_with(&mut app, edited_in_pico8());

    app.apply(Action::Confirm);

    let requests = app.take_file_requests();
    let [FileRequest::WriteBytes { path, contents }] = &requests[..] else {
        panic!("expected one write, got {requests:?}");
    };
    assert_eq!(*path, cart_path());
    assert!(Cart::read(contents).unwrap().pattern(1).stop());
}

#[test]
fn esc_leaves_the_changed_cart_alone() {
    let mut app = imported(simple_cart().p8());
    let hash = stored_hash(&app);
    export_with(&mut app, edited_in_pico8());

    app.apply(Action::Cancel);

    assert!(app.take_file_requests().is_empty());
    assert_eq!(stored_hash(&app), hash);
}

#[test]
fn a_failing_export_reports_its_failures_before_asking_to_overwrite() {
    let mut app = with_more_song_rows_than_the_claim_holds(&imported(simple_cart().p8()));

    export_with(&mut app, edited_in_pico8());

    assert!(status_line(&app).contains("EXPORT FAILED: 1 PROBLEM"));
}

#[test]
fn a_moved_target_cart_asks_to_be_located() {
    let mut app = imported(simple_cart().p8());

    app.apply(Action::Export);
    app.take_file_requests();
    app.receive_file(FileResponse::Missing { path: cart_path() });

    assert_eq!(
        status_line(&app).trim_end(),
        "GAME.P8 NOT FOUND  ENTER LOCATE  ESC CANCEL"
    );
}

#[test]
fn locating_a_moved_cart_keeps_the_claim_and_hash_and_exports_to_it() {
    let original = simple_cart().p8();
    let mut app = imported(original.clone());
    let target = app.project().target_cart().unwrap().clone();
    app.apply(Action::Export);
    app.take_file_requests();
    app.receive_file(FileResponse::Missing { path: cart_path() });

    app.apply(Action::Confirm);
    answer_listing(
        &mut app,
        Path::new("."),
        vec![DirectoryEntry {
            name: "moved.p8".to_owned(),
            is_directory: false,
        }],
    );
    app.apply(DOWN);
    app.apply(Action::Confirm);

    let moved = Path::new(".").join("moved.p8");
    assert_eq!(
        app.take_file_requests(),
        [FileRequest::ReadBytes {
            path: moved.clone()
        }]
    );
    let located = app.project().target_cart().unwrap();
    assert_eq!(located.path, "moved.p8");
    assert_eq!(located.claim, target.claim);
    assert_eq!(located.claimed_bytes_hash, target.claimed_bytes_hash);
    app.receive_file(FileResponse::BytesRead {
        path: moved.clone(),
        contents: original,
    });
    assert!(matches!(
        &app.take_file_requests()[..],
        [FileRequest::WriteBytes { path, .. }] if *path == moved
    ));
}

#[test]
fn esc_does_not_locate_a_moved_cart() {
    let mut app = imported(simple_cart().p8());
    app.apply(Action::Export);
    app.take_file_requests();
    app.receive_file(FileResponse::Missing { path: cart_path() });

    app.apply(Action::Cancel);

    assert!(app.take_file_requests().is_empty());
    assert_eq!(app.project().target_cart().unwrap().path, CART_NAME);
}

#[test]
fn a_missing_project_file_cannot_be_opened() {
    let mut app = App::new();
    app.apply(Action::OpenProject);
    answer_listing(
        &mut app,
        Path::new("."),
        vec![DirectoryEntry {
            name: "song.ptrk".to_owned(),
            is_directory: false,
        }],
    );
    app.apply(DOWN);
    app.apply(Action::Confirm);
    let path = Path::new(".").join("song.ptrk");

    app.receive_file(FileResponse::Missing { path });

    assert!(status_line(&app).contains("CANNOT OPEN SONG.PTRK: NOT FOUND"));
}

#[test]
fn claim_failures_point_to_editing_the_claim() {
    let original = simple_cart().p8();
    let mut app = imported(original.clone());
    let mut project = app.project().clone();
    let mut target = project.target_cart().unwrap().clone();
    target.claim.sfx = vec![0];
    project.set_target_cart(Some(target));
    app = App::with_project(project);

    export_with(&mut app, original);

    assert_eq!(right_slot(&app, 1), " ERR NEEDS 2 SFX, THE");
    assert!(
        (2..6).any(|row| right_slot(&app, row) == "     ^T EDIT CLAIM"),
        "{:?}",
        (0..8).map(|row| right_slot(&app, row)).collect::<Vec<_>>()
    );
}

#[test]
fn claim_failures_name_the_m8_style_cart_and_log_keys() {
    let original = simple_cart().p8();
    let mut project = imported(original.clone()).project().clone();
    let mut target = project.target_cart().unwrap().clone();
    target.claim.sfx = vec![0];
    project.set_target_cart(Some(target));
    let mut app = launched(App::with_project(project), M8_STYLE);

    export_with(&mut app, original);

    let slots: Vec<String> = (0..8).map(|row| right_slot(&app, row)).collect();
    assert!(
        slots.iter().any(|slot| slot.trim() == "X+T EDIT CLAIM"),
        "{slots:?}"
    );
}

#[test]
fn a_target_cart_that_cannot_be_read_is_reported() {
    let mut app = imported(simple_cart().p8());
    let hash = stored_hash(&app);

    app.apply(Action::Export);
    app.take_file_requests();
    app.receive_file(FileResponse::Failed {
        path: cart_path(),
        reason: "gone".to_owned(),
    });

    assert!(status_line(&app).contains("CANNOT EXPORT TO GAME.P8: GONE"));
    assert_eq!(stored_hash(&app), hash);
}

#[test]
fn a_failed_write_keeps_the_old_hash() {
    let original = simple_cart().p8();
    let mut app = imported(original.clone());
    let hash = stored_hash(&app);

    export_with(&mut app, original);
    app.receive_file(FileResponse::Failed {
        path: cart_path(),
        reason: "disk full".to_owned(),
    });

    assert!(status_line(&app).contains("CANNOT EXPORT TO GAME.P8: DISK FULL"));
    assert_eq!(stored_hash(&app), hash);
}

fn answer_listing(app: &mut App, directory: &Path, entries: Vec<DirectoryEntry>) {
    assert_eq!(
        app.take_file_requests(),
        [FileRequest::List {
            directory: directory.to_path_buf()
        }]
    );
    app.receive_file(FileResponse::Listed {
        directory: directory.to_path_buf(),
        entries,
    });
}

fn imported_from_a_subfolder(contents: Vec<u8>) -> App {
    let mut app = App::new();
    app.apply(Action::Import);
    let carts = DirectoryEntry {
        name: "carts".to_owned(),
        is_directory: true,
    };
    answer_listing(&mut app, Path::new("."), vec![carts]);
    app.apply(DOWN);
    app.apply(Action::Confirm);
    let cart = DirectoryEntry {
        name: CART_NAME.to_owned(),
        is_directory: false,
    };
    answer_listing(&mut app, &Path::new(".").join("carts"), vec![cart]);
    app.apply(DOWN);
    app.apply(Action::Confirm);
    app.take_file_requests();
    app.receive_file(FileResponse::BytesRead {
        path: Path::new(".").join("carts").join(CART_NAME),
        contents,
    });
    app
}

fn save_as_one_folder_up(app: &mut App) -> String {
    app.apply(Action::SaveAs);
    answer_listing(app, &Path::new(".").join("carts"), Vec::new());
    app.apply(DOWN);
    app.apply(Action::Confirm);
    answer_listing(app, Path::new("."), Vec::new());
    app.apply(Action::TypeChar('s'));
    app.apply(Action::Confirm);
    let requests = app.take_file_requests();
    let [FileRequest::Write { path, contents }] = &requests[..] else {
        panic!("expected one write, got {requests:?}");
    };
    let (path, contents) = (path.clone(), contents.clone());
    app.receive_file(FileResponse::Written { path });
    contents
}

#[test]
fn saving_into_another_folder_keeps_the_target_cart_pointing_at_the_same_cart() {
    let mut app = imported_from_a_subfolder(simple_cart().p8());

    let saved = save_as_one_folder_up(&mut app);

    let target = project_file::parse(&saved).unwrap();
    assert_eq!(target.target_cart().unwrap().path, "carts/game.p8");
    app.apply(Action::Export);
    assert_eq!(
        app.take_file_requests(),
        [FileRequest::ReadBytes {
            path: Path::new(".").join("carts").join(CART_NAME)
        }]
    );
}

#[test]
fn saving_into_another_folder_leaves_nothing_unsaved() {
    let mut app = imported_from_a_subfolder(simple_cart().p8());

    save_as_one_folder_up(&mut app);

    assert!(!app.has_unsaved_changes());
}

#[test]
fn saving_again_in_the_same_folder_keeps_the_path_as_it_was() {
    let mut app = imported_from_a_subfolder(simple_cart().p8());
    save_as_one_folder_up(&mut app);

    app.apply(Action::Save);

    let requests = app.take_file_requests();
    let [FileRequest::Write { contents, .. }] = &requests[..] else {
        panic!("expected one write, got {requests:?}");
    };
    let project = project_file::parse(contents).unwrap();
    assert_eq!(project.target_cart().unwrap().path, "carts/game.p8");
}

#[test]
fn the_target_cart_path_is_relative_to_the_project_file() {
    let mut project = Project::new();
    project.set_target_cart(Some(TargetCart {
        path: "carts/game.p8".to_owned(),
        claim: Claim::default(),
        claimed_bytes_hash: None,
    }));
    let mut app = App::new();
    app.apply(Action::OpenProject);
    app.take_file_requests();
    app.receive_file(FileResponse::Listed {
        directory: PathBuf::from("."),
        entries: vec![DirectoryEntry {
            name: "song.ptrk".to_owned(),
            is_directory: false,
        }],
    });
    app.apply(DOWN);
    app.apply(Action::Confirm);
    app.take_file_requests();
    app.receive_file(FileResponse::Read {
        path: Path::new(".").join("song.ptrk"),
        contents: project_file::write(&project),
    });

    app.apply(Action::Export);

    assert_eq!(
        app.take_file_requests(),
        [FileRequest::ReadBytes {
            path: Path::new(".").join("carts/game.p8")
        }]
    );
}

#[test]
fn the_overwrite_question_names_only_slots_the_claim_still_holds() {
    let mut app = imported(simple_cart().p8());
    let mut project = app.project().clone();
    let mut target = project.target_cart().unwrap().clone();
    target.claim.sfx = vec![1, 5];
    project.set_target_cart(Some(target));
    app = App::with_project(project);

    export_with(&mut app, edited_in_pico8());

    assert_eq!(
        status_line(&app).trim_end(),
        "PATTERN 01 CHANGED IN GAME.P8  ENTER OVERWRITE  ESC CANCEL"
    );
}

fn looping_sfx(pitches: &[u8], loop_start: u8, loop_end: u8) -> TestSfx {
    let mut sfx = TestSfx::with_pitches(8, pitches);
    sfx.editor_and_filters = 1;
    sfx.loop_start = loop_start;
    sfx.loop_end = loop_end;
    sfx
}

fn loops_cart() -> TestCart {
    let mut only = TestPattern::playing([Some(0), Some(1), Some(2), None]);
    only.stop = true;
    TestCart {
        sfx: vec![
            looping_sfx(&(10..26).collect::<Vec<u8>>(), 16, 0),
            looping_sfx(&[30, 31, 32, 33, 34, 35], 2, 6),
            looping_sfx(&[40, 41, 42, 43], 0, 4),
            looping_sfx(&[50, 51, 52], 1, 3),
        ],
        patterns: vec![only],
    }
}

fn loop_pair(project: &Project, track: usize) -> (u8, u8) {
    let entry = project.chain_row_entry(0, track, 0).unwrap();
    let phrase = project.phrase(entry.phrase());
    (phrase.loop_start(), phrase.loop_end())
}

#[test]
fn sfx_loops_import_as_phrase_loops_and_export_back_byte_for_byte() {
    let original = loops_cart().p8();
    let mut app = imported(original.clone());

    let project = app.project();
    assert_eq!(loop_pair(project, 0), (16, 0));
    assert_eq!(loop_pair(project, 1), (2, 6));
    assert_eq!(loop_pair(project, 2), (0, 4));
    let loose = (0..=255)
        .map(|id| project.phrase(PhraseId::new(id)))
        .find(|phrase| phrase.step(0).pitch() == 50 && !phrase.is_silent())
        .unwrap();
    assert_eq!((loose.loop_start(), loose.loop_end()), (1, 3));

    let written = export_ok(&mut app, original.clone());

    let (before, after) = (
        Cart::read(&original).unwrap(),
        Cart::read(&written).unwrap(),
    );
    for slot in 0..4 {
        assert_eq!(after.sfx_bytes(slot), before.sfx_bytes(slot), "SFX {slot}");
    }
}
