mod support;

use pikutracker_core::app::{Action, App, Direction};
use pikutracker_core::player::{AudioRequest, Player, Resampler};
use pikutracker_core::synth::{SAMPLE_RATE, SAMPLES_PER_TICK};
use support::{apply, status_line};

const UP: Action = Action::Move(Direction::Up);
const DOWN: Action = Action::Move(Direction::Down);
const LEFT: Action = Action::Move(Direction::Left);
const RIGHT: Action = Action::Move(Direction::Right);
const SPACE: Action = Action::PlayFocusedPane;
const A_440: Action = Action::PianoKey(9);
const A_880: Action = Action::PianoKey(21);
const SPEED: usize = 8;
const STEP_SAMPLES: usize = SPEED * SAMPLES_PER_TICK as usize;
const CROSSFADE_SAMPLES: usize = 300;

fn phrase_app() -> App {
    let mut app = App::new();
    apply(
        &mut app,
        &[
            Action::HexDigit(0),
            Action::FocusNext,
            Action::HexDigit(0),
            Action::FocusNext,
            RIGHT,
            Action::HexDigit(0),
            LEFT,
            UP,
            UP,
            UP,
            UP,
            UP,
            Action::HexDigit(0),
            Action::HexDigit(8),
            RIGHT,
            Action::HexDigit(0),
            Action::HexDigit(4),
            LEFT,
            DOWN,
            DOWN,
            DOWN,
            DOWN,
            DOWN,
        ],
    );
    app
}

fn render(app: &mut App, player: &mut Player, samples: usize) -> Vec<f32> {
    for request in app.take_audio_requests() {
        player.handle(request);
    }
    player.render(samples)
}

fn frequency_at(samples: &[f32], rate: u32) -> f32 {
    let rising: Vec<usize> = samples
        .windows(2)
        .enumerate()
        .filter(|(_, pair)| pair[0] < 0.0 && pair[1] >= 0.0)
        .map(|(index, _)| index)
        .collect();
    let (first, last) = (rising[0], rising[rising.len() - 1]);
    (rising.len() - 1) as f32 * rate as f32 / (last - first) as f32
}

fn step_frequency(samples: &[f32], step: usize) -> f32 {
    let start = step * STEP_SAMPLES + CROSSFADE_SAMPLES;
    frequency_at(&samples[start..(step + 1) * STEP_SAMPLES], SAMPLE_RATE)
}

fn step_peak(samples: &[f32], step: usize) -> f32 {
    samples[step * STEP_SAMPLES + CROSSFADE_SAMPLES..(step + 1) * STEP_SAMPLES]
        .iter()
        .fold(0.0, |peak, sample| peak.max(sample.abs()))
}

fn assert_near(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= expected * 0.03,
        "expected about {expected}, got {actual}"
    );
}

fn two_note_app() -> App {
    let mut app = phrase_app();
    apply(&mut app, &[A_440, DOWN, A_880, UP]);
    app.take_audio_requests();
    app
}

#[test]
fn space_in_the_phrase_pane_plays_from_the_cursor_and_loops_to_the_first_step() {
    let mut app = two_note_app();
    let mut player = Player::new();

    app.apply(SPACE);
    let samples = render(&mut app, &mut player, STEP_SAMPLES * 5);

    assert_near(step_frequency(&samples, 0), 880.0);
    assert!(step_peak(&samples, 1) < 0.01, "Step 3 is silent");
    assert_near(step_frequency(&samples, 2), 440.0);
    assert!(step_peak(&samples, 3) < 0.01, "Step 1 is silent");
    assert_near(step_frequency(&samples, 4), 880.0);
}

#[test]
fn playing_from_the_last_step_wraps_to_the_first() {
    let mut app = two_note_app();
    apply(&mut app, &[DOWN, DOWN]);
    let mut player = Player::new();

    app.apply(SPACE);
    let samples = render(&mut app, &mut player, STEP_SAMPLES * 4);

    assert_near(step_frequency(&samples, 1), 440.0);
    assert!(step_peak(&samples, 2) < 0.01);
}

#[test]
fn space_stops_the_phrase_at_once() {
    let mut app = two_note_app();
    let mut player = Player::new();
    app.apply(SPACE);
    render(&mut app, &mut player, STEP_SAMPLES / 2);

    app.apply(SPACE);
    let samples = render(&mut app, &mut player, STEP_SAMPLES);

    assert!(samples.iter().all(|sample| *sample == 0.0));
}

#[test]
fn stopping_leaves_the_cursor_where_it_is() {
    let mut app = two_note_app();
    let before = app.grid().text_dump();

    apply(&mut app, &[SPACE, SPACE]);

    assert_eq!(app.grid().text_dump(), before);
}

#[test]
fn space_after_stopping_plays_again() {
    let mut app = two_note_app();

    apply(&mut app, &[SPACE, SPACE, SPACE]);

    let requests = app.take_audio_requests();
    assert_eq!(requests.len(), 3);
    assert!(matches!(requests[1], AudioRequest::Stop));
    assert!(matches!(requests[2], AudioRequest::PlayPhrase { .. }));
}

#[test]
fn moving_the_cursor_or_focus_does_not_change_what_is_playing() {
    let mut app = two_note_app();
    app.apply(SPACE);
    app.take_audio_requests();

    apply(&mut app, &[DOWN, Action::FocusNext, Action::FocusPrevious]);

    assert!(app.take_audio_requests().is_empty());
}

#[test]
fn space_stops_the_phrase_from_any_pane() {
    let mut app = two_note_app();
    apply(&mut app, &[SPACE, Action::FocusPrevious]);
    app.take_audio_requests();

    app.apply(SPACE);

    assert!(matches!(
        app.take_audio_requests().as_slice(),
        [AudioRequest::Stop]
    ));
}

#[test]
fn space_in_the_right_hand_view_plays_the_last_focused_phrase_pane() {
    let mut app = two_note_app();
    app.apply(Action::FocusNext);

    app.apply(SPACE);

    assert!(matches!(
        app.take_audio_requests().as_slice(),
        [AudioRequest::PlayPhrase { from_step: 2, .. }]
    ));
}

#[test]
fn space_on_an_empty_phrase_says_there_is_nothing_to_play() {
    let mut app = phrase_app();

    app.apply(SPACE);

    assert!(app.take_audio_requests().is_empty());
    assert_eq!(status_line(&app), "NOTHING TO PLAY");
}

#[test]
fn a_phrase_silent_inside_len_counts_as_empty() {
    let mut app = phrase_app();
    apply(&mut app, &[DOWN, DOWN, DOWN]);
    apply(
        &mut app,
        &[
            UP,
            UP,
            UP,
            UP,
            UP,
            UP,
            UP,
            UP,
            RIGHT,
            Action::HexDigit(1),
            Action::HexDigit(0),
            LEFT,
            DOWN,
            DOWN,
            DOWN,
            DOWN,
            DOWN,
        ],
    );
    apply(&mut app, &[&[DOWN; 8][..], &[A_440]].concat());
    apply(
        &mut app,
        &[
            &[UP; 14][..],
            &[RIGHT, Action::HexDigit(0), Action::HexDigit(4), LEFT],
            &[DOWN; 5],
        ]
        .concat(),
    );

    app.apply(SPACE);

    assert_eq!(status_line(&app), "NOTHING TO PLAY");
}

#[test]
fn the_message_clears_on_the_next_action() {
    let mut app = phrase_app();
    apply(&mut app, &[SPACE, DOWN]);

    assert!(!status_line(&app).contains("NOTHING TO PLAY"));
}

#[test]
fn the_phrase_filters_reach_the_sound() {
    let reverb = [
        UP,
        UP,
        UP,
        RIGHT,
        Action::HexDigit(2),
        LEFT,
        DOWN,
        DOWN,
        DOWN,
    ];
    let mut dry_app = two_note_app();
    let mut wet_app = two_note_app();
    apply(&mut wet_app, &[UP, UP]);
    apply(&mut wet_app, &reverb);
    apply(&mut wet_app, &[DOWN, DOWN]);

    dry_app.apply(SPACE);
    wet_app.apply(SPACE);
    let dry = render(&mut dry_app, &mut Player::new(), STEP_SAMPLES);
    let wet = render(&mut wet_app, &mut Player::new(), STEP_SAMPLES);

    assert_ne!(dry, wet);
}

#[test]
fn the_step_effect_reaches_the_sound() {
    let mut plain_app = two_note_app();
    let mut fading_app = two_note_app();
    apply(&mut fading_app, &[RIGHT, RIGHT, RIGHT, Action::HexDigit(5)]);

    plain_app.apply(SPACE);
    fading_app.apply(SPACE);
    let plain = render(&mut plain_app, &mut Player::new(), STEP_SAMPLES);
    let fading = render(&mut fading_app, &mut Player::new(), STEP_SAMPLES);

    assert!(step_peak(&plain, 0) > 0.3);
    let tail = &fading[STEP_SAMPLES - 100..];
    assert!(tail.iter().all(|sample| sample.abs() < 0.05));
}

#[test]
fn the_resampler_keeps_the_pitch_at_the_device_rate() {
    let mut app = two_note_app();
    apply(&mut app, &[UP, UP]);
    let mut player = Player::new();
    app.apply(SPACE);
    for request in app.take_audio_requests() {
        player.handle(request);
    }

    let device_rate = 48_000;
    let mut resampler = Resampler::new(device_rate);
    let frames: Vec<f32> = (0..STEP_SAMPLES * 2)
        .map(|_| resampler.next_frame(|| player.next_sample()))
        .collect();

    assert_near(frequency_at(&frames[800..2_800], device_rate), 440.0);
}

#[test]
fn the_resampler_pulls_samples_at_the_synth_rate() {
    let mut pulled = 0;
    let mut resampler = Resampler::new(44_100);

    for _ in 0..44_100 {
        resampler.next_frame(|| {
            pulled += 1;
            0.0
        });
    }

    assert!((22_049..=22_051).contains(&pulled), "pulled {pulled}");
}
