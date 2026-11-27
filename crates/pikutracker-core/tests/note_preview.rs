mod support;

use pikutracker_core::app::{Action, App, Direction};
use pikutracker_core::player::{AudioRequest, Player};
use pikutracker_core::project::{ChainEntry, ChainId, PhraseId, Project};
use pikutracker_core::synth::{SAMPLE_RATE, SAMPLES_PER_TICK};
use support::apply;

const RIGHT: Action = Action::Move(Direction::Right);
const DOWN: Action = Action::Move(Direction::Down);
const SHIFT_TAB: Action = Action::FocusPrevious;
const SPACE: Action = Action::PlayFocusedPane;
const A_440: Action = Action::PianoKey(9);
const NUDGE_UP: Action = Action::Nudge {
    up: true,
    coarse: false,
};
const OCTAVE_UP: Action = Action::Nudge {
    up: true,
    coarse: true,
};

const SPEED: i32 = 8;
const STEP_SAMPLES: usize = SPEED as usize * SAMPLES_PER_TICK as usize;
const CROSSFADE_SAMPLES: usize = 300;

fn phrase_app(transpose: i32) -> App {
    let mut project = Project::new();
    let id = PhraseId::new(1);
    let phrase = project.phrase_mut(id);
    phrase.set_speed(SPEED);
    let step = phrase.step_mut(0);
    step.set_pitch(33);
    step.set_volume(5);
    let mut entry = ChainEntry::new(id);
    entry.set_transpose(transpose);
    *project.chain_mut(ChainId::new(0)).entry_mut(0) = Some(entry);
    project.set_song_cell(0, 0, Some(ChainId::new(0)));
    let mut app = App::with_project(project);
    apply(&mut app, &[SHIFT_TAB, SHIFT_TAB]);
    app
}

fn render(app: &mut App, player: &mut Player, samples: usize) -> Vec<f32> {
    support::play_along(app, player, samples)
}

fn frequency(samples: &[f32]) -> f32 {
    let rising: Vec<usize> = samples
        .windows(2)
        .enumerate()
        .filter(|(_, pair)| pair[0] < 0.0 && pair[1] >= 0.0)
        .map(|(index, _)| index)
        .collect();
    assert!(rising.len() > 1, "no tone");
    let (first, last) = (rising[0], rising[rising.len() - 1]);
    (rising.len() - 1) as f32 * SAMPLE_RATE as f32 / (last - first) as f32
}

fn peak(samples: &[f32]) -> f32 {
    samples
        .iter()
        .fold(0.0, |peak, sample| peak.max(sample.abs()))
}

fn assert_near(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= expected * 0.03,
        "expected about {expected}, got {actual}"
    );
}

fn previews(app: &mut App) -> usize {
    app.take_audio_requests()
        .iter()
        .filter(|request| matches!(request, AudioRequest::PreviewStep { .. }))
        .count()
}

#[test]
fn entering_a_note_while_stopped_previews_it() {
    let mut app = phrase_app(0);
    let mut player = Player::new();

    app.apply(A_440);
    let samples = render(&mut app, &mut player, STEP_SAMPLES);

    assert_near(frequency(&samples[CROSSFADE_SAMPLES..]), 440.0);
}

#[test]
fn the_preview_uses_the_chain_entrys_transpose() {
    let mut app = phrase_app(12);
    let mut player = Player::new();

    app.apply(A_440);
    let samples = render(&mut app, &mut player, STEP_SAMPLES);

    assert_near(frequency(&samples[CROSSFADE_SAMPLES..]), 880.0);
}

#[test]
fn changing_a_steps_pitch_previews_the_new_note() {
    let mut app = phrase_app(0);
    let mut player = Player::new();

    app.apply(OCTAVE_UP);
    let samples = render(&mut app, &mut player, STEP_SAMPLES);

    assert_near(frequency(&samples[CROSSFADE_SAMPLES..]), 880.0);
}

#[test]
fn changing_a_steps_instrument_previews_the_note() {
    let mut app = phrase_app(0);
    apply(&mut app, &[RIGHT]);

    app.apply(NUDGE_UP);

    assert_eq!(previews(&mut app), 1);
}

#[test]
fn changing_a_steps_volume_or_effect_previews_nothing() {
    let mut app = phrase_app(0);

    apply(&mut app, &[RIGHT, RIGHT, NUDGE_UP, RIGHT, NUDGE_UP]);

    assert_eq!(previews(&mut app), 0);
}

#[test]
fn a_nudge_that_changes_nothing_previews_nothing() {
    let mut app = phrase_app(0);
    apply(&mut app, &[RIGHT]);
    app.take_audio_requests();

    app.apply(Action::Nudge {
        up: false,
        coarse: false,
    });

    assert_eq!(previews(&mut app), 0);
}

#[test]
fn a_silent_step_previews_nothing() {
    let mut app = phrase_app(0);

    apply(&mut app, &[DOWN, NUDGE_UP]);

    assert_eq!(previews(&mut app), 0);
}

#[test]
fn nothing_is_previewed_during_playback() {
    let mut app = phrase_app(0);
    app.apply(SPACE);
    app.take_audio_requests();

    app.apply(A_440);

    assert_eq!(previews(&mut app), 0);
}

#[test]
fn the_preview_sounds_for_as_long_as_the_key_is_held() {
    let mut app = phrase_app(0);
    let mut player = Player::new();
    app.apply(A_440);

    let samples = render(&mut app, &mut player, STEP_SAMPLES * 4);

    assert!(peak(&samples[STEP_SAMPLES * 3..]) > 0.1, "still held");
}

#[test]
fn the_preview_stops_when_the_key_is_released() {
    let mut app = phrase_app(0);
    let mut player = Player::new();
    app.apply(A_440);
    render(&mut app, &mut player, STEP_SAMPLES * 2);

    app.key_released();
    let samples = render(&mut app, &mut player, STEP_SAMPLES);

    assert!(peak(&samples[CROSSFADE_SAMPLES..]) < 0.01);
}

#[test]
fn a_quick_key_press_still_sounds_for_one_step() {
    let mut app = phrase_app(0);
    let mut player = Player::new();
    app.apply(A_440);

    app.key_released();
    let held = render(&mut app, &mut player, STEP_SAMPLES);
    let after = player.render(STEP_SAMPLES);

    assert!(
        peak(&held[STEP_SAMPLES - 100..]) > 0.1,
        "sounds to the Step's end"
    );
    assert!(peak(&after[CROSSFADE_SAMPLES..]) < 0.01);
}

#[test]
fn a_muted_track_does_not_silence_the_preview() {
    let mut app = phrase_app(0);
    let mut player = Player::new();
    app.apply(Action::Mute { track_number: 1 });

    app.apply(A_440);
    let samples = render(&mut app, &mut player, STEP_SAMPLES);

    assert_near(frequency(&samples[CROSSFADE_SAMPLES..]), 440.0);
}

#[test]
fn a_preview_is_not_a_play() {
    let mut app = phrase_app(0);
    let mut player = Player::new();
    app.apply(A_440);
    render(&mut app, &mut player, STEP_SAMPLES);

    app.apply(SPACE);

    assert!(matches!(
        app.take_audio_requests().as_slice(),
        [AudioRequest::PlayPhrase { .. }]
    ));
    assert_eq!(player.finished_play(), None);
}
