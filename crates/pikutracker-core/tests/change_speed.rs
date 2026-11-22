mod support;

use pikutracker_core::app::{Action, App, Direction, KeyContext};
use pikutracker_core::keymap::{Key, KeyPress, Keymap, Modifiers};
use pikutracker_core::player::AudioRequest;
use pikutracker_core::project::{ChainEntry, ChainId, PhraseId, Project, Step};
use support::{apply, status_line};

const DOWN: Action = Action::Move(Direction::Down);
const RIGHT: Action = Action::Move(Direction::Right);
const TAB: Action = Action::FocusNext;
const SELECT: Action = Action::StartSelection;

fn change_speed(app: &mut App, high: u8, low: u8) {
    apply(
        app,
        &[
            Action::ChangeSpeed,
            Action::HexDigit(high),
            Action::HexDigit(low),
            Action::Confirm,
        ],
    );
}

fn speed(app: &App, phrase: u8) -> u8 {
    app.project().phrase(PhraseId::new(phrase)).speed()
}

fn put_entry(project: &mut Project, chain: u8, index: usize, phrase: u8) {
    *project.chain_mut(ChainId::new(chain)).entry_mut(index) =
        Some(ChainEntry::new(PhraseId::new(phrase)));
}

fn set_speed(project: &mut Project, phrase: u8, speed: i32) {
    project.phrase_mut(PhraseId::new(phrase)).set_speed(speed);
}

fn two_rows() -> Project {
    let mut project = Project::new();
    project.set_song_cell(0, 0, Some(ChainId::new(0)));
    project.set_song_cell(1, 0, Some(ChainId::new(1)));
    put_entry(&mut project, 0, 0, 0);
    put_entry(&mut project, 0, 1, 1);
    put_entry(&mut project, 1, 0, 2);
    set_speed(&mut project, 0, 16);
    set_speed(&mut project, 1, 10);
    set_speed(&mut project, 2, 20);
    project
}

#[test]
fn the_typed_speed_goes_to_the_phrase_on_show_and_the_others_scale_with_it() {
    let mut app = App::with_project(two_rows());

    change_speed(&mut app, 0x2, 0x0);

    assert_eq!(speed(&app, 0), 32);
    assert_eq!(speed(&app, 1), 20);
    assert_eq!(speed(&app, 2), 40);
}

#[test]
fn nothing_changes_until_enter() {
    let mut app = App::with_project(two_rows());
    let before = app.project().clone();

    apply(
        &mut app,
        &[
            Action::ChangeSpeed,
            Action::HexDigit(2),
            Action::HexDigit(0),
        ],
    );

    assert!(app.project() == &before);
    assert!(status_line(&app).contains("PHRASE 00 SPEED"));
}

#[test]
fn the_question_names_the_whole_song_when_nothing_is_selected() {
    let mut app = App::with_project(two_rows());

    app.apply(Action::ChangeSpeed);

    assert!(
        status_line(&app).contains("SCALES 3 PHRASES: WHOLE SONG"),
        "{}",
        status_line(&app)
    );
}

#[test]
fn the_question_names_the_selection_when_one_is_in_scope() {
    let mut app = App::with_project(two_rows());

    apply(&mut app, &[TAB, SELECT, Action::ChangeSpeed]);

    assert!(
        status_line(&app).contains("SCALES 1 PHRASE: SELECTION"),
        "{}",
        status_line(&app)
    );
}

#[test]
fn the_question_names_the_whole_song_for_a_selection_of_steps() {
    let mut app = App::with_project(two_rows());

    apply(&mut app, &[TAB, TAB, SELECT, DOWN, Action::ChangeSpeed]);

    assert!(
        status_line(&app).contains("WHOLE SONG"),
        "{}",
        status_line(&app)
    );
}

#[test]
fn the_question_shows_what_has_been_typed() {
    let mut app = App::with_project(two_rows());

    apply(&mut app, &[Action::ChangeSpeed, Action::HexDigit(0xA)]);

    assert!(status_line(&app).contains("A_"), "{}", status_line(&app));
}

#[test]
fn speeds_round_to_the_nearest_whole_speed() {
    let mut project = two_rows();
    set_speed(&mut project, 1, 5);
    set_speed(&mut project, 2, 7);
    let mut app = App::with_project(project);

    change_speed(&mut app, 0x1, 0x8);

    assert_eq!(speed(&app, 0), 24);
    assert_eq!(speed(&app, 1), 8);
    assert_eq!(speed(&app, 2), 11);
}

#[test]
fn speeds_stop_at_1_and_255() {
    let mut project = two_rows();
    set_speed(&mut project, 1, 200);
    let mut app = App::with_project(project);
    change_speed(&mut app, 0x4, 0x0);
    assert_eq!(speed(&app, 1), 255, "200 x 4 stops at 255");

    let mut project = two_rows();
    set_speed(&mut project, 1, 3);
    let mut app = App::with_project(project);
    change_speed(&mut app, 0x0, 0x1);
    assert_eq!(speed(&app, 0), 1);
    assert_eq!(speed(&app, 1), 1, "3 / 16 stops at 1");
}

#[test]
fn loose_phrases_and_chains_off_the_song_are_left_alone() {
    let mut project = two_rows();
    set_speed(&mut project, 9, 30);
    put_entry(&mut project, 7, 0, 8);
    set_speed(&mut project, 8, 30);
    let mut app = App::with_project(project);

    change_speed(&mut app, 0x2, 0x0);

    assert_eq!(speed(&app, 9), 30);
    assert_eq!(speed(&app, 8), 30);
}

#[test]
fn a_phrase_used_twice_scales_once() {
    let mut project = two_rows();
    put_entry(&mut project, 1, 1, 1);
    let mut app = App::with_project(project);

    change_speed(&mut app, 0x2, 0x0);

    assert_eq!(speed(&app, 1), 20);
}

#[test]
fn a_song_selection_scales_only_the_phrases_of_its_chains() {
    let mut project = two_rows();
    project.set_song_cell(0, 1, Some(ChainId::new(2)));
    put_entry(&mut project, 2, 0, 3);
    set_speed(&mut project, 3, 12);
    let mut app = App::with_project(project);

    apply(&mut app, &[SELECT, RIGHT]);
    app.apply(Action::Move(Direction::Left));
    change_speed(&mut app, 0x2, 0x0);

    assert_eq!(speed(&app, 0), 32);
    assert_eq!(speed(&app, 1), 20);
    assert_eq!(speed(&app, 2), 20, "row 1 is outside the selection");
    assert_eq!(speed(&app, 3), 12, "Track 2 is outside the selection");
}

#[test]
fn a_chain_selection_scales_only_the_phrases_of_its_entries() {
    let mut app = App::with_project(two_rows());

    apply(&mut app, &[TAB, SELECT]);
    change_speed(&mut app, 0x2, 0x0);

    assert_eq!(speed(&app, 0), 32);
    assert_eq!(speed(&app, 1), 10);
    assert_eq!(speed(&app, 2), 20);
}

#[test]
fn a_selection_of_steps_leaves_the_whole_song_in_scope() {
    let mut app = App::with_project(two_rows());

    apply(&mut app, &[TAB, TAB, SELECT, DOWN]);
    change_speed(&mut app, 0x2, 0x0);

    assert_eq!(speed(&app, 2), 40);
}

#[test]
fn the_report_counts_the_phrases_that_changed() {
    let mut app = App::with_project(two_rows());

    change_speed(&mut app, 0x2, 0x0);

    let status = status_line(&app);
    assert!(status.contains("3 PHRASES CHANGED"), "{status}");
    assert!(status.contains("0 INEXACT"), "{status}");
    assert!(status.contains("0 USED ELSEWHERE"), "{status}");
    assert!(status.contains("0 CROSSED SPEED 8"), "{status}");
}

#[test]
fn a_phrase_that_keeps_its_speed_has_not_changed() {
    let mut project = two_rows();
    set_speed(&mut project, 1, 255);
    set_speed(&mut project, 2, 255);
    let mut app = App::with_project(project);

    change_speed(&mut app, 0x2, 0x0);

    assert!(status_line(&app).contains("1 PHRASE CHANGED"));
}

#[test]
fn the_report_counts_rounded_and_stopped_speeds_as_inexact() {
    let mut project = two_rows();
    set_speed(&mut project, 1, 5);
    set_speed(&mut project, 2, 200);
    let mut app = App::with_project(project);

    change_speed(&mut app, 0x1, 0x8);

    assert!(status_line(&app).contains("2 INEXACT"));
}

#[test]
fn the_report_counts_a_phrase_another_chain_plays_as_used_elsewhere() {
    let mut project = two_rows();
    put_entry(&mut project, 7, 0, 1);
    let mut app = App::with_project(project);

    change_speed(&mut app, 0x2, 0x0);

    assert!(status_line(&app).contains("1 USED ELSEWHERE"));
}

#[test]
fn a_chain_also_placed_outside_a_song_selection_counts_as_used_elsewhere() {
    let mut project = two_rows();
    project.set_song_cell(5, 3, Some(ChainId::new(0)));
    let mut app = App::with_project(project);

    apply(&mut app, &[SELECT]);
    change_speed(&mut app, 0x2, 0x0);

    assert!(status_line(&app).contains("2 USED ELSEWHERE"));
}

#[test]
fn a_chain_selected_in_every_place_it_is_placed_is_not_used_elsewhere() {
    let mut project = two_rows();
    project.set_song_cell(1, 0, Some(ChainId::new(0)));
    let mut app = App::with_project(project);

    apply(&mut app, &[SELECT, DOWN]);
    change_speed(&mut app, 0x2, 0x0);

    assert!(status_line(&app).contains("0 USED ELSEWHERE"));
}

#[test]
fn an_entry_outside_a_chain_selection_counts_as_used_elsewhere() {
    let mut project = two_rows();
    put_entry(&mut project, 0, 5, 0);
    let mut app = App::with_project(project);

    apply(&mut app, &[TAB, SELECT]);
    change_speed(&mut app, 0x2, 0x0);

    assert!(status_line(&app).contains("1 USED ELSEWHERE"));
}

#[test]
fn the_report_counts_phrases_that_crossed_speed_8_in_either_direction() {
    let mut project = two_rows();
    set_speed(&mut project, 1, 8);
    set_speed(&mut project, 2, 4);
    let mut app = App::with_project(project);

    change_speed(&mut app, 0x3, 0x0);
    assert!(status_line(&app).contains("2 CROSSED SPEED 8"));

    let mut project = two_rows();
    set_speed(&mut project, 1, 9);
    set_speed(&mut project, 2, 8);
    let mut app = App::with_project(project);

    change_speed(&mut app, 0x0, 0x8);
    assert!(status_line(&app).contains("2 CROSSED SPEED 8"));
}

#[test]
fn a_report_with_something_to_notice_stays_until_esc() {
    let mut project = two_rows();
    set_speed(&mut project, 1, 5);
    let mut app = App::with_project(project);

    change_speed(&mut app, 0x1, 0x8);
    assert!(status_line(&app).starts_with("! "));

    app.apply(Action::Cancel);
    assert!(!status_line(&app).contains("CHANGED"));
}

#[test]
fn no_phrase_on_show_means_no_speed_to_start_from() {
    let mut app = App::new();

    app.apply(Action::ChangeSpeed);

    assert!(status_line(&app).contains("NO PHRASE HERE"));
    assert_eq!(app.key_context(), KeyContext::ReferenceValue);
}

#[test]
fn esc_changes_nothing_and_ends_the_selection() {
    let mut app = App::with_project(two_rows());
    let before = app.project().clone();

    apply(
        &mut app,
        &[
            SELECT,
            Action::ChangeSpeed,
            Action::HexDigit(2),
            Action::Cancel,
        ],
    );

    assert!(app.project() == &before);
    assert!(!status_line(&app).contains("NEW SPEED"));
    change_speed(&mut app, 0x2, 0x0);
    assert_eq!(speed(&app, 2), 40, "the whole Song is in scope again");
}

#[test]
fn enter_with_nothing_typed_keeps_asking() {
    let mut app = App::with_project(two_rows());
    let before = app.project().clone();

    apply(&mut app, &[Action::ChangeSpeed, Action::Confirm]);

    assert!(app.project() == &before);
    assert!(status_line(&app).contains("SPEED MUST BE 01-FF"));
}

#[test]
fn speed_zero_is_refused() {
    let mut app = App::with_project(two_rows());
    let before = app.project().clone();

    change_speed(&mut app, 0x0, 0x0);

    assert!(app.project() == &before);
    assert!(status_line(&app).contains("SPEED MUST BE 01-FF"));
}

#[test]
fn one_digit_is_a_whole_speed() {
    let mut app = App::with_project(two_rows());

    apply(
        &mut app,
        &[Action::ChangeSpeed, Action::HexDigit(8), Action::Confirm],
    );

    assert_eq!(speed(&app, 0), 8);
}

#[test]
fn a_third_digit_is_ignored_and_backspace_takes_one_back() {
    let mut app = App::with_project(two_rows());

    apply(
        &mut app,
        &[
            Action::ChangeSpeed,
            Action::HexDigit(1),
            Action::HexDigit(2),
            Action::HexDigit(3),
            Action::Backspace,
            Action::HexDigit(8),
            Action::Confirm,
        ],
    );

    assert_eq!(speed(&app, 0), 0x18);
}

#[test]
fn other_keys_wait_for_the_answer() {
    let mut app = App::with_project(two_rows());

    apply(&mut app, &[Action::ChangeSpeed, Action::Clone, DOWN]);

    assert_eq!(app.project().song_cell(0, 0), Some(ChainId::new(0)));
    assert!(status_line(&app).contains("PHRASE 00 SPEED"));
}

#[test]
fn change_speed_is_one_undo_step() {
    let mut app = App::with_project(two_rows());
    let before = app.project().clone();

    change_speed(&mut app, 0x2, 0x0);
    let after = app.project().clone();
    assert!(after != before);

    app.apply(Action::Undo);
    assert!(app.project() == &before);
    app.apply(Action::Redo);
    assert!(app.project() == &after);
}

#[test]
fn change_speed_opens_from_its_key_and_takes_typed_digits() {
    let keymap = Keymap::pc_tracker();
    let mut app = App::with_project(two_rows());
    let press = |key, modifiers| KeyPress::new(key, modifiers);

    let open = press(Key::Char('t'), Modifiers::CTRL_SHIFT);
    app.apply(keymap.action_in(open, app.key_context()).unwrap());
    assert_eq!(app.key_context(), KeyContext::SpeedEntry);

    let keys = [
        Key::Char('2'),
        Key::Char('a'),
        Key::Backspace,
        Key::Char('0'),
    ];
    for key in keys {
        let action = keymap.action_in(press(key, Modifiers::NONE), app.key_context());
        app.apply(action.unwrap());
    }
    let enter = press(Key::Enter, Modifiers::NONE);
    app.apply(keymap.action_in(enter, app.key_context()).unwrap());

    assert_eq!(speed(&app, 0), 0x20);
}

#[test]
fn the_player_hears_a_change_made_during_playback() {
    let mut project = two_rows();
    *project.phrase_mut(PhraseId::new(0)).step_mut(0) = Step::new(24, 0, 5, 0).unwrap();
    let mut app = App::with_project(project);
    app.apply(Action::PlayFocusedPane);
    app.take_audio_requests();

    change_speed(&mut app, 0x2, 0x0);

    let requests = app.take_audio_requests();
    let heard = requests.iter().any(|request| {
        matches!(request, AudioRequest::UpdateProject(project) if **project == *app.project())
    });
    assert!(heard);
}
