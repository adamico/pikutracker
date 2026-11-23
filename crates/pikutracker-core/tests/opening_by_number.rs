mod support;

use pikutracker_core::app::{Action, App, Direction, layout};
use pikutracker_core::player::Player;
use pikutracker_core::project::{ChainEntry, ChainId, PhraseId, Project};
use pikutracker_core::synth::{SAMPLE_RATE, SAMPLES_PER_TICK};
use support::{apply, pane_text};

const UP: Action = Action::Move(Direction::Up);
const DOWN: Action = Action::Move(Direction::Down);
const RIGHT: Action = Action::Move(Direction::Right);
const SHIFT_TAB: Action = Action::FocusPrevious;
const TAB: Action = Action::FocusNext;
const SPACE: Action = Action::PlayFocusedPane;

const PHRASE_HEADER_ROWS: usize = 5;
const FIRST_STEP_LINE: usize = 7;

fn number(value: u8) -> [Action; 2] {
    [Action::HexDigit(value >> 4), Action::HexDigit(value & 0xF)]
}

fn phrase_text(app: &App, line: usize) -> String {
    pane_text(app, layout::PHRASE, line)
}

fn chain_text(app: &App, line: usize) -> String {
    pane_text(app, layout::CHAIN, line)
}

fn song_text(app: &App, line: usize) -> String {
    pane_text(app, layout::SONG, line)
}

fn project_with_a_loose_phrase() -> Project {
    let mut project = Project::new();
    let first = project.phrase_mut(PhraseId::new(1));
    first.step_mut(0).set_pitch(24);
    first.step_mut(0).set_volume(5);
    *project.chain_mut(ChainId::new(0)).entry_mut(0) = Some(ChainEntry::new(PhraseId::new(0)));
    project.set_song_cell(0, 0, Some(ChainId::new(0)));
    let loose = project.phrase_mut(PhraseId::new(5));
    loose.step_mut(0).set_pitch(33);
    loose.step_mut(0).set_volume(7);
    project
}

fn project_with_a_loose_chain() -> Project {
    let mut project = project_with_a_loose_phrase();
    let loose = project.chain_mut(ChainId::new(3));
    *loose.entry_mut(0) = Some(ChainEntry::new(PhraseId::new(5)));
    *loose.entry_mut(1) = Some(ChainEntry::new(PhraseId::new(1)));
    project
}

fn to_the_chain_title() -> Vec<Action> {
    vec![SHIFT_TAB, UP]
}

fn to_the_phrase_title() -> Vec<Action> {
    [&[SHIFT_TAB, SHIFT_TAB][..], &[UP; PHRASE_HEADER_ROWS + 1]].concat()
}

#[test]
fn typing_a_number_in_the_phrase_title_shows_that_phrase_and_changes_no_chain() {
    let project = project_with_a_loose_phrase();
    let mut app = App::with_project(project.clone());

    apply(
        &mut app,
        &[to_the_phrase_title(), number(5).to_vec()].concat(),
    );

    assert_eq!(app.shown_phrase(), Some(PhraseId::new(5)));
    assert!(phrase_text(&app, 0).starts_with("PHRASE[05]"));
    assert_eq!(phrase_text(&app, FIRST_STEP_LINE), "00 A-2  0    7   0");
    assert_eq!(app.project(), &project);
}

#[test]
fn typing_a_number_in_the_chain_title_shows_that_chain_and_changes_no_song_row() {
    let project = project_with_a_loose_chain();
    let mut app = App::with_project(project.clone());

    apply(
        &mut app,
        &[to_the_chain_title(), number(3).to_vec()].concat(),
    );

    assert_eq!(app.shown_chain(), Some(ChainId::new(3)));
    assert_eq!(chain_text(&app, 0), "CHAIN[03]");
    assert_eq!(chain_text(&app, 2), "00 05     +00");
    assert_eq!(chain_text(&app, 3), "01 01     +00");
    assert_eq!(song_text(&app, 2), "00  00 -- -- --");
    assert_eq!(app.project(), &project);
}

#[test]
fn the_phrase_pane_shows_the_phrase_under_the_chain_cursor_of_a_chain_opened_by_number() {
    let mut app = App::with_project(project_with_a_loose_chain());

    apply(
        &mut app,
        &[to_the_chain_title(), number(3).to_vec()].concat(),
    );
    assert_eq!(app.shown_phrase(), Some(PhraseId::new(5)));

    apply(&mut app, &[DOWN, DOWN]);
    assert_eq!(app.shown_phrase(), Some(PhraseId::new(1)));
    assert!(phrase_text(&app, 0).starts_with("PHRASE 01"));
}

#[test]
fn moving_the_chain_cursor_shows_the_phrase_under_it_again() {
    let mut app = App::with_project(project_with_a_loose_phrase());
    apply(
        &mut app,
        &[to_the_phrase_title(), number(5).to_vec()].concat(),
    );

    apply(&mut app, &[TAB, DOWN]);
    assert_eq!(app.shown_phrase(), None);

    app.apply(UP);
    assert_eq!(app.shown_phrase(), Some(PhraseId::new(0)));
}

#[test]
fn moving_the_song_cursor_shows_the_chain_under_it_again() {
    let mut app = App::with_project(project_with_a_loose_chain());
    apply(
        &mut app,
        &[to_the_chain_title(), number(3).to_vec()].concat(),
    );

    apply(&mut app, &[TAB, RIGHT]);
    assert_eq!(app.shown_chain(), None);

    app.apply(Action::Move(Direction::Left));
    assert_eq!(app.shown_chain(), Some(ChainId::new(0)));
    assert_eq!(app.shown_phrase(), Some(PhraseId::new(0)));
}

#[test]
fn a_note_goes_into_the_phrase_opened_by_number_and_undo_returns_to_it() {
    let project = project_with_a_loose_phrase();
    let mut app = App::with_project(project.clone());
    apply(
        &mut app,
        &[to_the_phrase_title(), number(5).to_vec()].concat(),
    );

    apply(
        &mut app,
        &[&[DOWN; PHRASE_HEADER_ROWS + 2][..], &[Action::PianoKey(0)]].concat(),
    );
    assert_eq!(app.project().phrase(PhraseId::new(5)).step(1).pitch(), 24);
    assert_eq!(
        app.project().chain(ChainId::new(0)),
        project.chain(ChainId::new(0))
    );

    apply(&mut app, &[Action::FocusNext, DOWN]);
    assert_eq!(app.shown_phrase(), None);

    app.apply(Action::Undo);
    assert_eq!(app.project(), &project);
    assert_eq!(app.shown_phrase(), Some(PhraseId::new(5)));
}

#[test]
fn an_entry_typed_into_a_chain_opened_by_number_goes_into_that_chain() {
    let mut app = App::with_project(project_with_a_loose_chain());
    apply(
        &mut app,
        &[to_the_chain_title(), number(3).to_vec()].concat(),
    );

    apply(&mut app, &[DOWN, DOWN, DOWN]);
    apply(&mut app, &number(7));

    let entry = app.project().chain(ChainId::new(3)).entry(2);
    assert_eq!(entry.map(ChainEntry::phrase), Some(PhraseId::new(7)));
    assert_eq!(app.shown_phrase(), Some(PhraseId::new(7)));
    assert_eq!(app.project().song_cell(0, 0), Some(ChainId::new(0)));
}

#[test]
fn nudging_the_title_number_opens_the_next_phrase() {
    let mut app = App::with_project(project_with_a_loose_phrase());
    apply(
        &mut app,
        &[to_the_phrase_title(), number(4).to_vec()].concat(),
    );

    app.apply(Action::Nudge {
        up: true,
        coarse: false,
    });

    assert_eq!(app.shown_phrase(), Some(PhraseId::new(5)));
}

const SPEED: i32 = 8;
const STEP_SAMPLES: usize = SPEED as usize * SAMPLES_PER_TICK as usize;
const CROSSFADE_SAMPLES: usize = 300;

fn two_step_phrase(project: &mut Project, id: u8, pitch: i32) -> PhraseId {
    let id = PhraseId::new(id);
    let phrase = project.phrase_mut(id);
    phrase.set_length(2);
    phrase.set_speed(SPEED);
    for index in 0..2 {
        phrase.step_mut(index).set_pitch(pitch);
        phrase.step_mut(index).set_volume(5);
    }
    id
}

fn step_frequency(samples: &[f32], step: usize) -> f32 {
    let window = &samples[step * STEP_SAMPLES + CROSSFADE_SAMPLES..(step + 1) * STEP_SAMPLES];
    let rising: Vec<usize> = window
        .windows(2)
        .enumerate()
        .filter(|(_, pair)| pair[0] < 0.0 && pair[1] >= 0.0)
        .map(|(index, _)| index)
        .collect();
    assert!(rising.len() > 1, "Step {step} has no tone");
    let (first, last) = (rising[0], rising[rising.len() - 1]);
    (rising.len() - 1) as f32 * SAMPLE_RATE as f32 / (last - first) as f32
}

#[test]
fn a_chain_opened_by_number_plays_its_entries_in_turn_and_starts_over() {
    let mut project = Project::new();
    let low = two_step_phrase(&mut project, 1, 33);
    let high = two_step_phrase(&mut project, 2, 45);
    let loose = project.chain_mut(ChainId::new(4));
    *loose.entry_mut(0) = Some(ChainEntry::new(low));
    *loose.entry_mut(1) = Some(ChainEntry::new(high));
    let mut app = App::with_project(project);
    apply(
        &mut app,
        &[to_the_chain_title(), number(4).to_vec(), vec![DOWN, SPACE]].concat(),
    );

    let mut player = Player::new();
    for request in app.take_audio_requests() {
        player.handle(request);
    }
    let samples = player.render(6 * STEP_SAMPLES);

    let heard: Vec<f32> = (0..6).map(|step| step_frequency(&samples, step)).collect();
    let expected = [440.0, 440.0, 880.0, 880.0, 440.0, 440.0];
    for (step, (heard, expected)) in heard.iter().zip(expected).enumerate() {
        assert!(
            (heard - expected).abs() <= expected * 0.03,
            "Step {step}: expected about {expected} Hz, got {heard}"
        );
    }
}

#[test]
fn the_chain_pane_marks_the_entry_being_played_in_a_chain_opened_by_number() {
    let mut project = Project::new();
    let low = two_step_phrase(&mut project, 1, 33);
    let high = two_step_phrase(&mut project, 2, 45);
    let loose = project.chain_mut(ChainId::new(4));
    *loose.entry_mut(0) = Some(ChainEntry::new(low));
    *loose.entry_mut(1) = Some(ChainEntry::new(high));
    let mut app = App::with_project(project);
    apply(
        &mut app,
        &[to_the_chain_title(), number(4).to_vec(), vec![DOWN, SPACE]].concat(),
    );

    let mut player = Player::new();
    for request in app.take_audio_requests() {
        player.handle(request);
    }
    player.render(2 * STEP_SAMPLES + 1);
    app.show_playback(&player.report());

    assert!(chain_text(&app, 2).starts_with("00"));
    assert!(chain_text(&app, 3).starts_with(">1"));
}
