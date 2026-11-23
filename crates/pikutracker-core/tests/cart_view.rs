mod support;

use pikutracker_core::app::layout;
use pikutracker_core::app::{Action, App, Direction, DirectoryEntry, FileRequest, FileResponse};
use pikutracker_core::keymap::{Key, KeyPress, Keymap, Modifiers};
use pikutracker_core::project::{Claim, Project, TargetCart};
use std::path::{Path, PathBuf};
use support::{TestCart, TestPattern, TestSfx};
use support::{pane_text, status_line};

const UP: Action = Action::Move(Direction::Up);
const DOWN: Action = Action::Move(Direction::Down);
const CART_NAME: &str = "game.p8";

fn right_slot(app: &App, index: usize) -> String {
    pane_text(app, layout::RIGHT_SLOT, index)
}

fn right_slot_rows(app: &App) -> Vec<String> {
    let mut rows: Vec<String> = (0..44).map(|row| right_slot(app, row)).collect();
    while rows.last().is_some_and(String::is_empty) {
        rows.pop();
    }
    rows
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

fn busy_cart() -> TestCart {
    let sound = TestSfx::with_pitches(8, &[30]);
    let silent = TestSfx::with_pitches(8, &[]);
    let mut stop = TestPattern::playing([Some(3), None, None, None]);
    stop.stop = true;
    TestCart {
        sfx: vec![sound, sound, silent, sound],
        patterns: vec![
            TestPattern::playing([Some(0), None, None, None]),
            TestPattern::default(),
            stop,
        ],
    }
}

fn answer_listing(app: &mut App, entries: &[&str]) {
    assert_eq!(
        app.take_file_requests(),
        [FileRequest::List {
            directory: PathBuf::from(".")
        }]
    );
    app.receive_file(FileResponse::Listed {
        directory: PathBuf::from("."),
        entries: entries
            .iter()
            .map(|name| DirectoryEntry {
                name: (*name).to_owned(),
                is_directory: false,
            })
            .collect(),
    });
}

fn imported(contents: Vec<u8>) -> App {
    let mut app = App::new();
    app.apply(Action::Import);
    answer_listing(&mut app, &[CART_NAME]);
    app.apply(DOWN);
    app.apply(Action::Confirm);
    app.take_file_requests();
    app.receive_file(FileResponse::BytesRead {
        path: cart_path(),
        contents,
    });
    app
}

fn open_cart_view(app: &mut App, contents: Vec<u8>) {
    app.apply(Action::ToggleCart);
    assert_eq!(
        app.take_file_requests(),
        [FileRequest::ReadBytes { path: cart_path() }]
    );
    app.receive_file(FileResponse::BytesRead {
        path: cart_path(),
        contents,
    });
}

fn claim(app: &App) -> Claim {
    app.project().target_cart().unwrap().claim.clone()
}

fn type_text(app: &mut App, text: &str) {
    for ch in text.chars() {
        let action = match ch {
            '-' => Action::TypeChar('-'),
            _ => Action::HexDigit(ch.to_digit(16).unwrap() as u8),
        };
        app.apply(action);
    }
}

fn clear_field(app: &mut App) {
    for _ in 0..20 {
        app.apply(Action::Backspace);
    }
}

fn one_phrase_project() -> Project {
    let mut app = App::new();
    app.apply(Action::HexDigit(0));
    app.apply(Action::FocusNext);
    app.apply(Action::HexDigit(0));
    app.apply(Action::FocusNext);
    app.apply(Action::PianoKey(0));
    app.project().clone()
}

#[test]
fn ctrl_t_opens_the_cart_view_with_the_claim_its_maps_and_checks() {
    let original = simple_cart().p8();
    let mut app = imported(original.clone());

    open_cart_view(&mut app, original);

    assert_eq!(
        right_slot_rows(&app),
        [
            "CART",
            " game.p8",
            "",
            "SFX 2/2",
            ">00 01_",
            "PATTERNS 2/2",
            " 00-01",
            "",
            "SFX",
            "0 ##..............",
            "1 ................",
            "2 ................",
            "3 ................",
            "PATTERNS",
            "0 ##..............",
            "1 ................",
            "2 ................",
            "3 ................",
            "",
            " OK  NEED 2 SFX, HAVE 2",
            " OK  NEED 2 PATTERNS,",
            "     HAVE 2",
        ]
    );
}

#[test]
fn ctrl_t_again_closes_the_cart_view_and_returns_focus() {
    let original = simple_cart().p8();
    let mut app = imported(original.clone());

    open_cart_view(&mut app, original);
    app.apply(Action::ToggleCart);

    assert_eq!(right_slot(&app, 0), "");
    app.apply(Action::HexDigit(1));
    assert_eq!(app.project().song_cell(0, 0).map(|id| id.number()), Some(1));
}

#[test]
fn the_claim_fields_take_hex_digits_dashes_backspace_and_equals() {
    let original = simple_cart().p8();
    let mut app = imported(original.clone());
    open_cart_view(&mut app, original);
    let keymap = Keymap::pc_tracker();
    let context = app.key_context();
    let press = |ch| KeyPress::new(Key::Char(ch), Modifiers::NONE);

    assert_eq!(
        keymap.action_in(press('a'), context),
        Some(Action::HexDigit(10))
    );
    assert_eq!(
        keymap.action_in(press('-'), context),
        Some(Action::TypeChar('-'))
    );
    assert_eq!(
        keymap.action_in(press('='), context),
        Some(Action::SuggestClaim)
    );
    assert_eq!(
        keymap.action_in(KeyPress::new(Key::Backspace, Modifiers::NONE), context),
        Some(Action::Backspace)
    );
}

#[test]
fn sfx_are_typed_as_slots_and_ranges_with_separators_added() {
    let original = simple_cart().p8();
    let mut app = imported(original.clone());
    open_cart_view(&mut app, original);

    clear_field(&mut app);
    type_text(&mut app, "0304-07");

    assert_eq!(right_slot(&app, 4), ">03 04-07_");
    assert_eq!(claim(&app).sfx, [3, 4, 5, 6, 7]);
    assert_eq!(right_slot(&app, 9), "0 OO.#####........");
}

#[test]
fn incomplete_text_shows_err_and_keeps_the_last_valid_claim() {
    let original = simple_cart().p8();
    let mut app = imported(original.clone());
    open_cart_view(&mut app, original);

    clear_field(&mut app);
    type_text(&mut app, "05-");

    assert_eq!(claim(&app).sfx, [5]);
    assert!(right_slot_rows(&app).contains(&" ERR SFX TEXT IS".to_owned()));
}

#[test]
fn a_slot_past_3f_is_not_a_claim() {
    let original = simple_cart().p8();
    let mut app = imported(original.clone());
    open_cart_view(&mut app, original);

    clear_field(&mut app);
    type_text(&mut app, "40");

    assert_eq!(claim(&app).sfx, Vec::<u8>::new());
}

#[test]
fn patterns_are_typed_as_one_range() {
    let original = simple_cart().p8();
    let mut app = imported(original.clone());
    open_cart_view(&mut app, original);
    app.apply(DOWN);

    clear_field(&mut app);
    type_text(&mut app, "181d");

    assert_eq!(right_slot(&app, 6), ">18-1D_");
    assert_eq!(claim(&app).first_and_last_pattern, Some((0x18, 0x1d)));
    app.apply(Action::HexDigit(4));
    assert_eq!(right_slot(&app, 6), ">18-1D_");
}

#[test]
fn a_single_pattern_is_a_range_of_one() {
    let original = simple_cart().p8();
    let mut app = imported(original.clone());
    open_cart_view(&mut app, original);
    app.apply(DOWN);

    clear_field(&mut app);
    type_text(&mut app, "05");

    assert_eq!(claim(&app).first_and_last_pattern, Some((5, 5)));
}

#[test]
fn claimed_slots_that_hold_other_sounds_show_as_overwritten_with_a_warning() {
    let original = busy_cart().p8();
    let mut project = one_phrase_project();
    project.set_target_cart(Some(TargetCart {
        path: CART_NAME.to_owned(),
        claim: Claim {
            sfx: vec![2, 3],
            first_and_last_pattern: Some((1, 2)),
        },
        claimed_bytes_hash: None,
    }));
    let mut app = App::with_project(project);

    open_cart_view(&mut app, original);

    let rows = right_slot_rows(&app);
    assert_eq!(rows[9], "0 OO#!............");
    assert_eq!(rows[14], "0 O#!.............");
    assert!(
        rows.contains(&" WRN OVERWRITES SFX 03".to_owned()),
        "{rows:?}"
    );
    assert!(
        rows.contains(&" WRN OVERWRITES PATTERN".to_owned()),
        "{rows:?}"
    );
    assert!(rows.contains(&"     02".to_owned()), "{rows:?}");
}

#[test]
fn too_small_a_claim_shows_err() {
    let original = simple_cart().p8();
    let mut app = imported(original.clone());
    open_cart_view(&mut app, original);

    clear_field(&mut app);
    type_text(&mut app, "01");

    let rows = right_slot_rows(&app);
    assert_eq!(rows[3], "SFX 1/2");
    assert!(
        rows.contains(&" ERR NEED 2 SFX, HAVE 1".to_owned()),
        "{rows:?}"
    );
}

#[test]
fn up_and_down_move_between_the_cart_sfx_and_pattern_fields() {
    let original = simple_cart().p8();
    let mut app = imported(original.clone());
    open_cart_view(&mut app, original);

    app.apply(UP);
    assert_eq!(right_slot(&app, 1), ">game.p8");
    app.apply(DOWN);
    app.apply(DOWN);
    assert_eq!(right_slot(&app, 6), ">00-01_");
}

#[test]
fn enter_on_the_patterns_field_closes_the_view() {
    let original = simple_cart().p8();
    let mut app = imported(original.clone());
    open_cart_view(&mut app, original);

    app.apply(DOWN);
    app.apply(Action::Confirm);

    assert_eq!(right_slot(&app, 0), "");
}

#[test]
fn esc_restores_the_claim_from_when_the_view_opened() {
    let original = simple_cart().p8();
    let mut app = imported(original.clone());
    let before = claim(&app);
    open_cart_view(&mut app, original);

    clear_field(&mut app);
    type_text(&mut app, "0809");
    app.apply(Action::Cancel);

    assert_eq!(claim(&app), before);
    assert_eq!(right_slot(&app, 0), "");
}

#[test]
fn each_visit_to_the_cart_view_is_one_undo_step() {
    let original = simple_cart().p8();
    let mut app = imported(original.clone());
    let first = claim(&app);
    open_cart_view(&mut app, original.clone());
    clear_field(&mut app);
    type_text(&mut app, "0809");
    app.apply(Action::ToggleCart);
    let second = claim(&app);
    open_cart_view(&mut app, original);
    clear_field(&mut app);
    type_text(&mut app, "0a");
    app.apply(Action::ToggleCart);

    app.apply(Action::Undo);
    assert_eq!(claim(&app), second);
    app.apply(Action::Undo);
    assert_eq!(claim(&app), first);
}

#[test]
fn a_cancelled_visit_leaves_no_undo_step() {
    let original = simple_cart().p8();
    let mut app = imported(original.clone());
    open_cart_view(&mut app, original);
    clear_field(&mut app);
    type_text(&mut app, "0809");
    app.apply(Action::Cancel);

    app.apply(Action::Undo);

    assert!(status_line(&app).contains("NOTHING TO UNDO"));
}

fn pick_cart(app: &mut App, contents: Vec<u8>) {
    app.apply(Action::ToggleCart);
    assert!(app.take_file_requests().is_empty());
    assert_eq!(right_slot(app, 1), ">NO TARGET CART");
    app.apply(Action::Confirm);
    answer_listing(app, &[CART_NAME, "song.mid", "other.ptrk"]);
    assert_eq!(right_slot(app, 0), "TARGET CART");
    app.apply(DOWN);
    app.apply(Action::Confirm);
    assert_eq!(
        app.take_file_requests(),
        [FileRequest::ReadBytes { path: cart_path() }]
    );
    app.receive_file(FileResponse::BytesRead {
        path: cart_path(),
        contents,
    });
}

#[test]
fn the_cart_file_list_shows_only_carts() {
    let mut app = App::with_project(one_phrase_project());
    app.apply(Action::ToggleCart);
    app.apply(Action::Confirm);
    answer_listing(&mut app, &[CART_NAME, "song.mid", "other.ptrk"]);

    assert_eq!(right_slot(&app, 3), " game.p8");
    assert_eq!(right_slot(&app, 4), "");
}

#[test]
fn a_newly_picked_cart_gets_a_suggested_claim_of_the_lowest_free_slots() {
    let mut app = App::with_project(one_phrase_project());

    pick_cart(&mut app, busy_cart().p8());

    let target = app.project().target_cart().unwrap();
    assert_eq!(target.path, CART_NAME);
    assert_eq!(target.claimed_bytes_hash, None);
    assert_eq!(
        target.claim,
        Claim {
            sfx: vec![2],
            first_and_last_pattern: Some((1, 1)),
        }
    );
    assert_eq!(right_slot(&app, 1), " game.p8");
    assert_eq!(right_slot(&app, 4), ">02_");
    assert_eq!(right_slot(&app, 6), " 01");
}

fn playing_custom_instrument(instrument: u8) -> App {
    let mut project = one_phrase_project();
    let id = app_phrase(&project);
    project
        .phrase_mut(id)
        .step_mut(0)
        .set_instrument(i32::from(instrument));
    App::with_project(project)
}

#[test]
fn the_suggestion_claims_sfx_k_for_custom_instrument_8_plus_k() {
    let mut app = playing_custom_instrument(0x0C);

    pick_cart(&mut app, busy_cart().p8());

    assert_eq!(claim(&app).sfx, [2, 4]);
}

#[test]
fn a_claim_without_sfx_k_shows_err_naming_the_instrument_and_the_slot() {
    let mut app = playing_custom_instrument(0x08);

    pick_cart(&mut app, busy_cart().p8());

    let checks = right_slot_rows(&app)
        .iter()
        .map(|row| row.trim())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        checks.contains("ERR INSTRUMENT 8 NEEDS SFX 00, WHICH THE CLAIM LACKS"),
        "{checks}"
    );
}

fn app_phrase(project: &Project) -> pikutracker_core::project::PhraseId {
    project
        .chain(project.song_cell(0, 0).unwrap())
        .entry(0)
        .unwrap()
        .phrase()
}

#[test]
fn equals_reapplies_the_suggestion() {
    let mut app = App::with_project(one_phrase_project());
    pick_cart(&mut app, busy_cart().p8());
    clear_field(&mut app);
    type_text(&mut app, "3e");

    app.apply(Action::SuggestClaim);

    assert_eq!(claim(&app).sfx, [2]);
    assert_eq!(right_slot(&app, 4), ">02_");
}

#[test]
fn picking_a_cart_is_part_of_the_visits_undo_step() {
    let mut app = App::with_project(one_phrase_project());
    pick_cart(&mut app, busy_cart().p8());
    app.apply(Action::ToggleCart);

    app.apply(Action::Undo);

    assert_eq!(app.project().target_cart(), None);
}

#[test]
fn esc_after_picking_a_cart_restores_no_target_cart() {
    let mut app = App::with_project(one_phrase_project());
    pick_cart(&mut app, busy_cart().p8());

    app.apply(Action::Cancel);

    assert_eq!(app.project().target_cart(), None);
}

#[test]
fn a_file_that_is_not_a_cart_is_not_picked() {
    let mut app = App::with_project(one_phrase_project());

    pick_cart(&mut app, b"not a cart".to_vec());

    assert_eq!(app.project().target_cart(), None);
    assert!(status_line(&app).contains("CANNOT USE GAME.P8: NOT A PICO-8 CART"));
}

#[test]
fn claim_fields_need_a_target_cart_first() {
    let mut app = App::with_project(one_phrase_project());
    app.apply(Action::ToggleCart);
    app.apply(DOWN);

    app.apply(Action::HexDigit(3));

    assert_eq!(app.project().target_cart(), None);
    assert!(status_line(&app).contains("PICK A TARGET CART FIRST"));
}

#[test]
fn picking_the_target_cart_again_keeps_its_claimed_bytes_hash() {
    let original = simple_cart().p8();
    let mut app = imported(original.clone());
    let hash = app
        .project()
        .target_cart()
        .unwrap()
        .claimed_bytes_hash
        .clone();
    open_cart_view(&mut app, original.clone());
    app.apply(UP);
    app.apply(Action::Confirm);
    answer_listing(&mut app, &[CART_NAME]);
    app.apply(DOWN);
    app.apply(Action::Confirm);
    app.take_file_requests();
    app.receive_file(FileResponse::BytesRead {
        path: cart_path(),
        contents: original,
    });

    assert!(hash.is_some());
    assert_eq!(
        app.project().target_cart().unwrap().claimed_bytes_hash,
        hash
    );
}
