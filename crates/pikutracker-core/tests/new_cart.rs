mod support;

use pikutracker_core::app::layout;
use pikutracker_core::app::{
    Action, App, Direction, DirectoryEntry, FileRequest, FileResponse, KeyContext,
};
use pikutracker_core::cart::{Cart, SLOTS};
use pikutracker_core::project::{Claim, Project, TargetCart};
use pikutracker_core::project_file;
use std::path::{Path, PathBuf};
use support::{TestCart, TestSfx, pane_text, status_line};

const DOWN: Action = Action::Move(Direction::Down);

fn right_slot(app: &App, index: usize) -> String {
    pane_text(app, layout::RIGHT_SLOT, index)
}

fn in_folder(name: &str) -> PathBuf {
    Path::new(".").join(name)
}

fn one_phrase_project() -> Project {
    let mut app = App::new();
    app.apply(Action::HexDigit(0));
    app.apply(Action::FocusPrevious);
    app.apply(Action::HexDigit(0));
    app.apply(Action::FocusPrevious);
    app.apply(Action::PianoKey(0));
    app.project().clone()
}

fn answer_listing(app: &mut App, entries: &[&str]) {
    answer_listing_of(app, PathBuf::from("."), entries, &[]);
}

fn answer_listing_of(app: &mut App, directory: PathBuf, files: &[&str], folders: &[&str]) {
    assert_eq!(
        app.take_file_requests(),
        [FileRequest::List {
            directory: directory.clone()
        }]
    );
    let entry = |name: &&str, is_directory| DirectoryEntry {
        name: (*name).to_owned(),
        is_directory,
    };
    app.receive_file(FileResponse::Listed {
        directory,
        entries: folders
            .iter()
            .map(|name| entry(name, true))
            .chain(files.iter().map(|name| entry(name, false)))
            .collect(),
    });
}

fn type_name(app: &mut App, name: &str) {
    for ch in name.chars() {
        app.apply(Action::TypeChar(ch));
    }
}

fn open_cart_field(app: &mut App, entries: &[&str]) {
    app.apply(Action::ToggleCart);
    app.apply(Action::Confirm);
    answer_listing(app, entries);
}

fn written_cart(app: &mut App) -> (PathBuf, Vec<u8>) {
    let requests = app.take_file_requests();
    let [FileRequest::WriteBytes { path, contents }] = &requests[..] else {
        panic!("expected one write, got {requests:?}");
    };
    (path.clone(), contents.clone())
}

fn unsaved_app() -> App {
    App::with_project(one_phrase_project())
}

#[test]
fn a_typed_name_in_the_cart_field_writes_a_new_cart() {
    let mut app = unsaved_app();
    open_cart_field(&mut app, &[]);

    type_name(&mut app, "tune.p8");
    app.apply(Action::Confirm);

    let (path, contents) = written_cart(&mut app);
    assert_eq!(path, in_folder("tune.p8"));
    assert_eq!(
        String::from_utf8(contents).unwrap(),
        "pico-8 cartridge // http://www.pico-8.com\nversion 43\n__lua__\n"
    );
}

fn saved_app(project: &Project) -> App {
    let mut app = App::new();
    app.apply(Action::OpenProject);
    answer_listing(&mut app, &["mysong.ptrk"]);
    app.apply(DOWN);
    app.apply(Action::Confirm);
    app.take_file_requests();
    app.receive_file(FileResponse::Read {
        path: in_folder("mysong.ptrk"),
        contents: project_file::write(project),
    });
    app
}

fn make_new_cart(app: &mut App, name: &str) {
    app.apply(Action::ToggleCart);
    make_new_cart_in_open_view(app, name);
}

fn make_new_cart_in_open_view(app: &mut App, name: &str) {
    app.apply(Action::Confirm);
    answer_listing(app, &[]);
    type_name(app, name);
    app.apply(Action::Confirm);
    let (path, _) = written_cart(app);
    app.receive_file(FileResponse::Written { path });
}

fn target(app: &App) -> TargetCart {
    app.project().target_cart().unwrap().clone()
}

fn export_reading(app: &mut App, contents: Vec<u8>) {
    app.apply(Action::Export);
    assert_eq!(
        app.take_file_requests(),
        [FileRequest::ReadBytes {
            path: in_folder("tune.p8")
        }]
    );
    app.receive_file(FileResponse::BytesRead {
        path: in_folder("tune.p8"),
        contents,
    });
}

#[test]
fn a_new_cart_reads_as_64_empty_sfx_and_64_empty_patterns() {
    let mut app = unsaved_app();
    open_cart_field(&mut app, &[]);
    type_name(&mut app, "tune.p8");
    app.apply(Action::Confirm);

    let (_, contents) = written_cart(&mut app);

    let cart = Cart::read(&contents).unwrap();
    for slot in 0..SLOTS as u8 {
        assert!(!cart.sfx(slot).is_used(), "SFX {slot}");
        assert!(cart.pattern(slot).is_empty(), "Pattern {slot}");
    }
}

#[test]
fn the_cart_field_takes_a_typed_name() {
    let mut app = unsaved_app();

    open_cart_field(&mut app, &[]);

    assert_eq!(app.key_context(), KeyContext::FileName);
}

#[test]
fn the_name_starts_from_the_project_file_name() {
    let mut app = saved_app(&one_phrase_project());

    open_cart_field(&mut app, &[]);

    assert_eq!(right_slot(&app, 2), ">NAME mysong.p8_");
    app.apply(Action::Confirm);
    assert_eq!(written_cart(&mut app).0, in_folder("mysong.p8"));
}

#[test]
fn an_unsaved_project_starts_with_an_empty_name_that_writes_nothing() {
    let mut app = unsaved_app();
    open_cart_field(&mut app, &[]);

    assert_eq!(right_slot(&app, 2), ">NAME _");
    app.apply(Action::Confirm);

    assert!(app.take_file_requests().is_empty());
}

#[test]
fn p8_is_added_to_a_name_without_it() {
    let mut app = unsaved_app();
    open_cart_field(&mut app, &[]);

    type_name(&mut app, "tune");
    app.apply(Action::Confirm);

    assert_eq!(written_cart(&mut app).0, in_folder("tune.p8"));
}

#[test]
fn a_p8_png_name_is_refused() {
    let mut app = unsaved_app();
    open_cart_field(&mut app, &[]);

    type_name(&mut app, "tune.p8.png");
    app.apply(Action::Confirm);

    assert!(app.take_file_requests().is_empty());
    assert!(
        status_line(&app).contains("A NEW CART MUST BE A .P8 FILE, NOT .P8.PNG"),
        "{}",
        status_line(&app)
    );
}

#[test]
fn the_name_of_an_existing_cart_picks_that_cart_without_writing() {
    let mut app = unsaved_app();
    open_cart_field(&mut app, &["tune.p8"]);

    type_name(&mut app, "tune");
    app.apply(Action::Confirm);

    assert_eq!(
        app.take_file_requests(),
        [FileRequest::ReadBytes {
            path: in_folder("tune.p8")
        }]
    );
}

#[test]
fn a_written_new_cart_becomes_the_target_cart_with_the_suggested_claim() {
    let mut app = unsaved_app();

    make_new_cart(&mut app, "tune");

    let target = target(&app);
    assert_eq!(target.path, "tune.p8");
    assert_eq!(
        target.claim,
        Claim {
            sfx: vec![0],
            first_and_last_pattern: Some((0, 0)),
        }
    );
    assert!(target.claimed_bytes_hash.is_some());
    assert_eq!(right_slot(&app, 1), " tune.p8");
    assert_eq!(right_slot(&app, 4), ">00_");
    assert!(status_line(&app).contains("CREATED NEW CART TUNE.P8"));
}

fn earlier_target_cart() -> TargetCart {
    TargetCart {
        path: "game.p8".to_owned(),
        claim: Claim {
            sfx: vec![5],
            first_and_last_pattern: Some((9, 9)),
        },
        claimed_bytes_hash: None,
    }
}

fn cart_field_of_earlier_target_cart() -> App {
    let mut project = one_phrase_project();
    project.set_target_cart(Some(earlier_target_cart()));
    let mut app = App::with_project(project);
    app.apply(Action::ToggleCart);
    app.take_file_requests();
    app.apply(Action::Move(Direction::Up));
    app
}

#[test]
fn a_new_cart_replaces_the_claim_of_the_earlier_target_cart() {
    let mut app = cart_field_of_earlier_target_cart();

    make_new_cart_in_open_view(&mut app, "tune");

    assert_eq!(target(&app).claim.sfx, [0]);
    assert_eq!(target(&app).claim.first_and_last_pattern, Some((0, 0)));
}

#[test]
fn esc_after_making_a_new_cart_restores_the_earlier_target_cart() {
    let mut app = cart_field_of_earlier_target_cart();
    make_new_cart_in_open_view(&mut app, "tune");

    app.apply(Action::Cancel);

    assert_eq!(target(&app), earlier_target_cart());
    assert!(app.take_file_requests().is_empty());
}

#[test]
fn a_failed_write_leaves_no_target_cart() {
    let mut app = unsaved_app();
    open_cart_field(&mut app, &[]);
    type_name(&mut app, "tune");
    app.apply(Action::Confirm);
    let (path, _) = written_cart(&mut app);

    app.receive_file(FileResponse::Failed {
        path,
        reason: "read-only file system".to_owned(),
    });

    assert_eq!(app.project().target_cart(), None);
    assert!(
        status_line(&app).contains("CANNOT CREATE TUNE.P8: READ-ONLY FILE SYSTEM"),
        "{}",
        status_line(&app)
    );
}

#[test]
fn the_first_export_writes_the_claimed_sfx_and_patterns_into_the_new_cart() {
    let mut app = unsaved_app();
    make_new_cart(&mut app, "tune");
    app.apply(Action::Confirm);

    export_reading(&mut app, Cart::new_cart_bytes());

    let (path, contents) = written_cart(&mut app);
    assert_eq!(path, in_folder("tune.p8"));
    let cart = Cart::read(&contents).unwrap();
    assert!(cart.sfx(0).is_used());
    assert_eq!(cart.pattern(0).channel(0), Some(0));
    assert!(!cart.sfx(1).is_used());
    assert!(cart.pattern(1).is_empty());
}

#[test]
fn a_claimed_slot_changed_before_the_first_export_is_asked_about() {
    let mut app = unsaved_app();
    make_new_cart(&mut app, "tune");
    app.apply(Action::Confirm);
    let edited_in_pico8 = TestCart {
        sfx: vec![TestSfx::with_pitches(8, &[30])],
        patterns: Vec::new(),
    }
    .p8();

    export_reading(&mut app, edited_in_pico8);

    assert!(app.take_file_requests().is_empty());
    assert!(
        status_line(&app).contains("SFX 00 CHANGED IN TUNE.P8"),
        "{}",
        status_line(&app)
    );
}

#[test]
fn save_as_into_another_folder_rebases_the_new_carts_link() {
    let mut app = unsaved_app();
    make_new_cart(&mut app, "tune");
    app.apply(Action::Confirm);

    app.apply(Action::SaveAs);
    answer_listing_of(&mut app, PathBuf::from("."), &[], &["sub"]);
    app.apply(DOWN);
    app.apply(DOWN);
    app.apply(Action::Confirm);
    answer_listing_of(&mut app, in_folder("sub"), &[], &[]);
    type_name(&mut app, "s");
    app.apply(Action::Confirm);

    let requests = app.take_file_requests();
    let [FileRequest::Write { contents, .. }] = &requests[..] else {
        panic!("expected one write, got {requests:?}");
    };
    let saved = project_file::parse(contents).unwrap();
    assert_eq!(saved.target_cart().unwrap().path, "../tune.p8");
}

#[test]
fn a_name_confirmed_before_the_folder_is_listed_writes_nothing() {
    let mut app = saved_app(&one_phrase_project());
    app.apply(Action::ToggleCart);
    app.apply(Action::Confirm);
    app.take_file_requests();

    app.apply(Action::Confirm);

    assert!(app.take_file_requests().is_empty());
}

#[test]
fn an_existing_cart_named_in_other_letter_case_is_picked_without_writing() {
    let mut app = unsaved_app();
    open_cart_field(&mut app, &["Tune.p8"]);

    type_name(&mut app, "TUNE");
    app.apply(Action::Confirm);

    assert_eq!(
        app.take_file_requests(),
        [FileRequest::ReadBytes {
            path: in_folder("Tune.p8")
        }]
    );
}
