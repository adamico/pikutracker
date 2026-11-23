mod support;

use pikutracker_core::app::layout::{self, Area};
use pikutracker_core::app::{Action, App, Direction};
use pikutracker_core::player::Player;
use pikutracker_core::project::{ChainEntry, ChainId, PhraseId, Project};
use pikutracker_core::synth::SAMPLES_PER_TICK;
use support::{pane_text, status_line};

const DOWN: Action = Action::Move(Direction::Down);
const SHIFT_TAB: Action = Action::FocusPrevious;
const SPACE: Action = Action::PlayFocusedPane;
const PLAY_SONG: Action = Action::PlaySongFromCursor;
const FOLLOW: Action = Action::ToggleFollow;

const SPEED: i32 = 8;
const STEP_SAMPLES: usize = SPEED as usize * SAMPLES_PER_TICK as usize;

const FIRST_SONG_ROW: usize = 2;
const FIRST_CHAIN_ROW: usize = 2;
const FIRST_STEP_ROW: usize = 7;

fn write(project: &mut Project, id: u8, steps: usize) -> PhraseId {
    let id = PhraseId::new(id);
    let phrase = project.phrase_mut(id);
    phrase.set_length(steps as i32);
    phrase.set_speed(SPEED);
    for index in 0..steps {
        let step = phrase.step_mut(index);
        step.set_pitch(33);
        step.set_volume(5);
    }
    id
}

fn song() -> Project {
    let mut project = Project::new();
    let first = write(&mut project, 1, 2);
    let second = write(&mut project, 2, 2);
    let third = write(&mut project, 3, 4);
    let opening = project.chain_mut(ChainId::new(0));
    *opening.entry_mut(0) = Some(ChainEntry::new(first));
    *opening.entry_mut(1) = Some(ChainEntry::new(second));
    *project.chain_mut(ChainId::new(1)).entry_mut(0) = Some(ChainEntry::new(third));
    project.set_song_cell(0, 0, Some(ChainId::new(0)));
    project.set_song_cell(1, 0, Some(ChainId::new(1)));
    project
}

fn played(actions: &[Action], steps: usize) -> (App, Player) {
    let mut app = App::with_project(song());
    let mut player = Player::new();
    for action in actions {
        app.apply(*action);
    }
    play_on(&mut app, &mut player, steps);
    (app, player)
}

fn play_on(app: &mut App, player: &mut Player, steps: usize) {
    for request in app.take_audio_requests() {
        player.handle(request);
    }
    player.render(steps * STEP_SAMPLES + 1);
    app.show_playback(&player.report());
}

fn label(app: &App, area: Area, row: usize) -> String {
    pane_text(app, area, row).chars().take(2).collect()
}

#[test]
fn the_song_pane_marks_the_row_being_played() {
    let (app, _) = played(&[PLAY_SONG], 4);

    assert_eq!(label(&app, layout::SONG, FIRST_SONG_ROW), "00");
    assert_eq!(label(&app, layout::SONG, FIRST_SONG_ROW + 1), ">1");
}

#[test]
fn the_chain_pane_marks_the_entry_being_played() {
    let (app, _) = played(&[PLAY_SONG], 2);

    assert_eq!(label(&app, layout::CHAIN, FIRST_CHAIN_ROW), "00");
    assert_eq!(label(&app, layout::CHAIN, FIRST_CHAIN_ROW + 1), ">1");
}

#[test]
fn the_phrase_pane_marks_the_step_being_played() {
    let (app, _) = played(&[PLAY_SONG], 1);

    assert_eq!(label(&app, layout::PHRASE, FIRST_STEP_ROW), "00");
    assert_eq!(label(&app, layout::PHRASE, FIRST_STEP_ROW + 1), ">1");
}

#[test]
fn the_marker_is_separate_from_the_cursor() {
    let (app, _) = played(
        &[
            DOWN,
            SHIFT_TAB,
            SHIFT_TAB,
            DOWN,
            Action::Move(Direction::Up),
            PLAY_SONG,
        ],
        0,
    );

    assert_eq!(label(&app, layout::SONG, FIRST_SONG_ROW + 1), ">1");
    assert!(
        pane_text(&app, layout::PHRASE, FIRST_STEP_ROW).contains('['),
        "cursor stays put"
    );
}

#[test]
fn a_pane_showing_something_else_has_no_marker() {
    let (app, _) = played(&[PLAY_SONG, DOWN], 1);

    assert_eq!(label(&app, layout::CHAIN, FIRST_CHAIN_ROW), "00");
    assert_eq!(label(&app, layout::PHRASE, FIRST_STEP_ROW + 1), "01");
}

#[test]
fn the_phrase_pane_marks_a_phrase_playing_alone() {
    let (app, _) = played(&[SHIFT_TAB, SHIFT_TAB, SPACE], 1);

    assert_eq!(label(&app, layout::PHRASE, FIRST_STEP_ROW + 1), ">1");
}

#[test]
fn the_marker_goes_away_when_playback_stops() {
    let (mut app, mut player) = played(&[PLAY_SONG], 1);

    app.apply(SPACE);
    play_on(&mut app, &mut player, 0);

    assert_eq!(label(&app, layout::SONG, FIRST_SONG_ROW), "00");
    assert_eq!(label(&app, layout::PHRASE, FIRST_STEP_ROW + 1), "01");
}

#[test]
fn f10_turns_follow_on_and_off_and_says_so() {
    let mut app = App::with_project(song());

    app.apply(FOLLOW);
    assert_eq!(status_line(&app), "FOLLOW ON");

    app.apply(FOLLOW);
    assert_eq!(status_line(&app), "FOLLOW OFF");
}

#[test]
fn the_panes_stay_put_while_follow_is_off() {
    let (app, _) = played(&[PLAY_SONG], 4);

    assert_eq!(app.shown_chain(), Some(ChainId::new(0)));
}

#[test]
fn follow_moves_the_panes_to_the_playhead() {
    let (app, _) = played(&[FOLLOW, PLAY_SONG], 5);

    assert_eq!(app.shown_chain(), Some(ChainId::new(1)));
    assert_eq!(app.shown_phrase(), Some(PhraseId::new(3)));
}

#[test]
fn follow_moves_the_cursor_down_the_chain_and_the_phrase() {
    let (app, _) = played(&[FOLLOW, SHIFT_TAB, SHIFT_TAB, PLAY_SONG], 3);

    assert_eq!(app.shown_phrase(), Some(PhraseId::new(2)));
    assert!(pane_text(&app, layout::PHRASE, FIRST_STEP_ROW + 1).contains('['));
}

#[test]
fn moving_the_cursor_by_hand_turns_follow_off_and_says_so() {
    let (mut app, mut player) = played(&[FOLLOW, PLAY_SONG], 1);

    app.apply(DOWN);
    assert_eq!(status_line(&app), "FOLLOW OFF");

    let row_after_moving = app.shown_chain();
    play_on(&mut app, &mut player, 4);
    assert_eq!(app.shown_chain(), row_after_moving);
}

#[test]
fn moving_the_cursor_says_nothing_when_follow_is_already_off() {
    let mut app = App::with_project(song());

    app.apply(DOWN);

    assert!(!status_line(&app).contains("FOLLOW"));
}

#[test]
fn moving_through_the_log_leaves_follow_on() {
    let (mut app, mut player) = played(&[FOLLOW, Action::ToggleLog, PLAY_SONG], 1);

    app.apply(DOWN);
    assert!(!status_line(&app).contains("FOLLOW"));

    play_on(&mut app, &mut player, 4);
    assert_eq!(app.shown_chain(), Some(ChainId::new(1)));
}

#[test]
fn a_report_says_whether_the_screen_needs_drawing_again() {
    let (mut app, player) = played(&[PLAY_SONG], 1);

    assert!(!app.show_playback(&player.report()));
}

#[test]
fn a_finished_play_is_reported_to_the_app() {
    let mut app = App::with_project(song());
    let mut player = Player::new();
    app.apply(PLAY_SONG);
    play_on(&mut app, &mut player, 9);

    app.apply(SPACE);

    assert!(
        app.take_audio_requests()
            .iter()
            .all(|request| !matches!(request, pikutracker_core::player::AudioRequest::Stop)),
        "Space starts a new play instead of stopping one that has ended"
    );
}
