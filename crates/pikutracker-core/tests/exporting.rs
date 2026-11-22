mod support;

use pikutracker_core::app::{Action, App, Direction, DirectoryEntry, FileRequest, FileResponse};
use pikutracker_core::cart::Cart;
use pikutracker_core::keymap::{Key, KeyPress, Keymap, Modifiers};
use pikutracker_core::player::AudioRequest;
use pikutracker_core::project::{ChainEntry, ChainId, Claim, PhraseId, Project, Step, TargetCart};
use pikutracker_core::project_file;
use std::path::{Path, PathBuf};
use support::status_line;
use support::{HEADER, TestCart, TestPattern, TestSfx};

const DOWN: Action = Action::Move(Direction::Down);
const CART_NAME: &str = "game.p8";

fn line(app: &App, index: usize) -> String {
    app.grid()
        .text_dump()
        .lines()
        .nth(index)
        .unwrap()
        .to_owned()
}

fn right_slot(app: &App, index: usize) -> String {
    let text: String = line(app, index).chars().skip(83).collect();
    text.trim_end().to_owned()
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

fn focus_overview(app: &mut App) {
    app.apply(Action::FocusPrevious);
}

#[test]
fn overview_shows_each_channels_track() {
    let mut project = Project::new();
    project.set_export_track(0, Some(3));
    project.set_export_track(2, Some(8));

    let app = App::with_project(project);

    assert_eq!(right_slot(&app, 2), "CH0  T3");
    assert_eq!(right_slot(&app, 3), "CH1  --");
    assert_eq!(right_slot(&app, 4), "CH2  T8");
    assert_eq!(right_slot(&app, 5), "CH3  --");
}

#[test]
fn typing_a_digit_in_overview_sets_the_channels_track() {
    let mut app = App::new();
    focus_overview(&mut app);

    app.apply(DOWN);
    app.apply(Action::HexDigit(6));

    assert_eq!(
        app.project().export_track_numbers(),
        [None, Some(6), None, None]
    );
}

#[test]
fn the_keys_of_the_overview_reach_its_actions() {
    let keymap = Keymap::pc_tracker();
    let mut app = App::new();
    focus_overview(&mut app);
    let context = app.key_context();
    let action = |key, modifiers| keymap.action_in(KeyPress::new(key, modifiers), context);

    assert_eq!(
        action(Key::Char('3'), Modifiers::NONE),
        Some(Action::HexDigit(3))
    );
    assert_eq!(action(Key::Delete, Modifiers::NONE), Some(Action::Clear));
    assert_eq!(
        action(Key::Char('e'), Modifiers::CTRL),
        Some(Action::Export)
    );
}

#[test]
fn digits_that_name_no_track_are_ignored() {
    let mut app = App::new();
    focus_overview(&mut app);

    for digit in [0, 9, 10, 15] {
        app.apply(Action::HexDigit(digit));
    }

    assert_eq!(app.project().export_track_numbers(), [None; 4]);
}

#[test]
fn clear_takes_the_track_off_a_channel() {
    let mut project = Project::new();
    project.set_export_track(0, Some(2));
    let mut app = App::with_project(project);
    focus_overview(&mut app);

    app.apply(Action::Clear);

    assert_eq!(app.project().export_track_numbers(), [None; 4]);
}

#[test]
fn choosing_a_track_used_elsewhere_swaps_the_two() {
    let mut project = Project::new();
    project.set_export_track(0, Some(1));
    project.set_export_track(1, Some(2));
    let mut app = App::with_project(project);
    focus_overview(&mut app);

    app.apply(Action::HexDigit(2));

    assert_eq!(
        app.project().export_track_numbers(),
        [Some(2), Some(1), None, None]
    );
}

#[test]
fn choosing_a_track_from_an_empty_channel_moves_it() {
    let mut project = Project::new();
    project.set_export_track(1, Some(2));
    let mut app = App::with_project(project);
    focus_overview(&mut app);

    app.apply(Action::HexDigit(2));

    assert_eq!(
        app.project().export_track_numbers(),
        [Some(2), None, None, None]
    );
}

#[test]
fn each_channel_edit_is_its_own_undo_step() {
    let mut app = App::new();
    focus_overview(&mut app);
    app.apply(Action::HexDigit(1));
    app.apply(DOWN);
    app.apply(Action::HexDigit(2));

    app.apply(Action::Undo);

    assert_eq!(
        app.project().export_track_numbers(),
        [Some(1), None, None, None]
    );
}

#[test]
fn escape_in_overview_moves_focus_to_the_phrase_pane() {
    let mut app = App::new();
    focus_overview(&mut app);

    app.apply(Action::Cancel);

    assert_eq!(
        app.key_context(),
        pikutracker_core::app::KeyContext::PhrasePitch
    );
}

#[test]
fn the_song_headers_show_each_exported_tracks_channel() {
    let mut project = Project::new();
    project.set_export_track(0, Some(3));
    project.set_export_track(3, Some(1));

    let app = App::with_project(project);

    assert_eq!(&line(&app, 1)[..28], "    T13T2 T30T4 T5 T6 T7 T8 ");
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
    assert!(status_line(&app).starts_with("EXPORTED TO GAME.P8: 2 SFX, 2 PATTERNS. T1-T4"));
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
    let mut app = imported(original.clone());
    let hash = stored_hash(&app);
    focus_overview(&mut app);
    for _ in 0..4 {
        app.apply(Action::Clear);
        app.apply(DOWN);
    }

    let requests = export_with(&mut app, original);

    assert!(requests.is_empty());
    assert_eq!(stored_hash(&app), hash);
    assert!(status_line(&app).contains("EXPORT FAILED: NO EXPORT TRACKS"));
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
    project.set_export_track(0, Some(1));
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
    let mut app = imported(simple_cart().p8());
    focus_overview(&mut app);
    for _ in 0..4 {
        app.apply(Action::Clear);
        app.apply(DOWN);
    }

    export_with(&mut app, edited_in_pico8());

    assert!(status_line(&app).contains("EXPORT FAILED: NO EXPORT TRACKS"));
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
fn export_warnings_are_counted_on_the_status_line_and_listed_in_the_log() {
    let mut project = Project::new();
    let phrase = project.phrase_mut(PhraseId::new(0));
    *phrase.step_mut(0) = Step::new(24, 0, 3, 0).unwrap();
    project.phrase_mut(PhraseId::new(1)).set_length(32);
    *project.chain_mut(ChainId::new(0)).entry_mut(0) = Some(ChainEntry::new(PhraseId::new(0)));
    *project.chain_mut(ChainId::new(1)).entry_mut(0) = Some(ChainEntry::new(PhraseId::new(1)));
    for track in 0..4 {
        project.set_song_cell(0, track, Some(ChainId::new(0)));
        project.set_export_track(track, Some(track as u8 + 1));
    }
    project.set_song_cell(0, 5, Some(ChainId::new(1)));
    project.set_target_cart(Some(TargetCart {
        path: CART_NAME.to_owned(),
        claim: Claim {
            sfx: vec![8],
            first_and_last_pattern: Some((0, 0)),
        },
        claimed_bytes_hash: None,
    }));
    let mut app = App::with_project(project);

    export_ok(&mut app, format!("{HEADER}__lua__\n").into_bytes());

    assert_eq!(
        status_line(&app),
        "! EXPORTED TO GAME.P8: 1 SFX, 1 PATTERNS. T1-T4 WITH 1 WARNING  ^L LOG"
    );
    let dump = app.grid().text_dump();
    assert!(dump.contains("LENGTH FROM T6,"), "{dump}");
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
