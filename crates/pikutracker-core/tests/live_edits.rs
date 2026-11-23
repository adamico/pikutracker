mod support;

use pikutracker_core::app::{Action, App, Direction};
use pikutracker_core::player::Player;
use pikutracker_core::project::{
    ChainEntry, ChainId, PhraseId, Project, SongRowEnding, SongRowMarks,
};
use pikutracker_core::synth::{SAMPLE_RATE, SAMPLES_PER_TICK};
use support::apply;

const UP: Action = Action::Move(Direction::Up);
const DOWN: Action = Action::Move(Direction::Down);
const RIGHT: Action = Action::Move(Direction::Right);
const TAB: Action = Action::FocusNext;
const PLAY_SONG: Action = Action::PlaySongFromCursor;
const OCTAVE_UP: Action = Action::Nudge {
    up: true,
    coarse: true,
};

const SPEED: i32 = 8;
const STEP_SAMPLES: usize = SPEED as usize * SAMPLES_PER_TICK as usize;
const CROSSFADE_SAMPLES: usize = 300;

const A1: Option<i32> = Some(21);
const A2: Option<i32> = Some(33);
const A3: Option<i32> = Some(45);
const REST: Option<i32> = None;

fn write(project: &mut Project, id: u8, steps: &[Option<i32>]) -> PhraseId {
    let id = PhraseId::new(id);
    let phrase = project.phrase_mut(id);
    phrase.set_length(steps.len() as i32);
    phrase.set_speed(SPEED);
    for (index, pitch) in steps.iter().enumerate() {
        if let Some(pitch) = pitch {
            let step = phrase.step_mut(index);
            step.set_pitch(*pitch);
            step.set_volume(5);
        }
    }
    id
}

fn place(project: &mut Project, row: usize, track: usize, chain: u8, phrase: PhraseId) {
    let id = ChainId::new(chain);
    *project.chain_mut(id).entry_mut(0) = Some(ChainEntry::new(phrase));
    project.set_song_cell(row, track, Some(id));
}

fn render(app: &mut App, player: &mut Player, steps: usize) -> Vec<f32> {
    for request in app.take_audio_requests() {
        player.handle(request);
    }
    player.render(steps * STEP_SAMPLES)
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

fn step_peak(samples: &[f32], step: usize) -> f32 {
    samples[step * STEP_SAMPLES + CROSSFADE_SAMPLES..(step + 1) * STEP_SAMPLES]
        .iter()
        .fold(0.0, |peak, sample| peak.max(sample.abs()))
}

fn assert_steps(samples: &[f32], expected: &[f32]) {
    for (step, frequency) in expected.iter().enumerate() {
        if *frequency == 0.0 {
            let peak = step_peak(samples, step);
            assert!(peak < 0.01, "Step {step} should be silent, peak {peak}");
        } else {
            let actual = step_frequency(samples, step);
            assert!(
                (actual - frequency).abs() <= frequency * 0.03,
                "Step {step}: expected about {frequency} Hz, got {actual}"
            );
        }
    }
}

fn four_steps_then_a3() -> Project {
    let mut project = Project::new();
    let first = write(&mut project, 1, &[A2, A2, A2, A2]);
    let second = write(&mut project, 2, &[A3, A3]);
    place(&mut project, 0, 0, 0, first);
    place(&mut project, 1, 0, 1, second);
    project
}

fn playing_in_phrase_pane(project: Project) -> App {
    let mut app = App::with_project(project);
    apply(&mut app, &[TAB, TAB, PLAY_SONG]);
    app
}

#[test]
fn an_edited_step_is_heard_when_the_playhead_reaches_it() {
    let mut app = playing_in_phrase_pane(four_steps_then_a3());
    let mut player = Player::new();
    let mut samples = render(&mut app, &mut player, 1);

    apply(&mut app, &[DOWN, DOWN, OCTAVE_UP]);
    samples.extend(render(&mut app, &mut player, 3));

    assert_steps(&samples, &[440.0, 440.0, 880.0, 440.0]);
}

#[test]
fn an_undo_during_playback_is_heard_when_the_playhead_reaches_it() {
    let mut app = playing_in_phrase_pane(four_steps_then_a3());
    let mut player = Player::new();
    apply(&mut app, &[DOWN, DOWN, OCTAVE_UP]);
    let mut samples = render(&mut app, &mut player, 1);

    apply(&mut app, &[Action::Undo]);
    samples.extend(render(&mut app, &mut player, 3));

    assert_steps(&samples, &[440.0, 440.0, 440.0, 440.0]);
}

#[test]
fn a_redo_during_playback_is_heard_when_the_playhead_reaches_it() {
    let mut app = playing_in_phrase_pane(four_steps_then_a3());
    let mut player = Player::new();
    apply(&mut app, &[DOWN, DOWN, OCTAVE_UP, Action::Undo]);
    let mut samples = render(&mut app, &mut player, 1);

    apply(&mut app, &[Action::Redo]);
    samples.extend(render(&mut app, &mut player, 3));

    assert_steps(&samples, &[440.0, 440.0, 880.0, 440.0]);
}

#[test]
fn an_edit_to_a_step_already_passed_waits_for_the_next_time_round() {
    let mut project = Project::new();
    let tone = write(&mut project, 1, &[A2, A2]);
    place(&mut project, 0, 0, 0, tone);
    project.set_song_row_marks(
        0,
        SongRowMarks {
            loop_start: false,
            ending: Some(SongRowEnding::LoopBack),
        },
    );
    let mut app = playing_in_phrase_pane(project);
    let mut player = Player::new();
    let mut samples = render(&mut app, &mut player, 1);

    apply(&mut app, &[OCTAVE_UP]);
    samples.extend(render(&mut app, &mut player, 3));

    assert_steps(&samples, &[440.0, 440.0, 880.0, 440.0]);
}

#[test]
fn editing_the_sounding_step_leaves_it_untouched() {
    let mut steady = playing_in_phrase_pane(four_steps_then_a3());
    let mut edited = playing_in_phrase_pane(four_steps_then_a3());
    let (mut steady_player, mut edited_player) = (Player::new(), Player::new());
    for request in steady.take_audio_requests() {
        steady_player.handle(request);
    }
    for request in edited.take_audio_requests() {
        edited_player.handle(request);
    }
    steady_player.render(STEP_SAMPLES / 2);
    edited_player.render(STEP_SAMPLES / 2);

    apply(&mut edited, &[OCTAVE_UP]);
    for request in edited.take_audio_requests() {
        edited_player.handle(request);
    }

    assert_eq!(
        steady_player.render(STEP_SAMPLES / 2),
        edited_player.render(STEP_SAMPLES / 2)
    );
}

#[test]
fn a_speed_change_applies_from_the_next_step() {
    let mut project = Project::new();
    let riff = write(&mut project, 1, &[A2, A3, A1, A1]);
    place(&mut project, 0, 0, 0, riff);
    let mut app = playing_in_phrase_pane(project);
    let mut player = Player::new();
    let mut samples = render(&mut app, &mut player, 0);
    samples.extend(player.render(STEP_SAMPLES / 2));

    apply(
        &mut app,
        &[UP, UP, UP, Action::HexDigit(1), Action::HexDigit(0)],
    );
    samples.extend(render(&mut app, &mut player, 5));

    assert_steps(&samples, &[440.0, 880.0, 880.0, 220.0, 220.0]);
}

#[test]
fn a_phrase_shortened_past_the_playhead_ends_its_entry() {
    let mut app = playing_in_phrase_pane(four_steps_then_a3());
    let mut player = Player::new();
    let mut samples = render(&mut app, &mut player, 2);
    samples.extend(player.render(STEP_SAMPLES / 2));

    apply(
        &mut app,
        &[UP, UP, UP, RIGHT, Action::HexDigit(0), Action::HexDigit(1)],
    );
    let rest = render(&mut app, &mut player, 2);
    samples.extend(rest);

    assert_steps(&samples, &[440.0, 440.0, 440.0, 880.0]);
}

#[test]
fn a_cleared_entry_stays_silent_until_the_next_chain_row() {
    let mut project = Project::new();
    let rests = write(&mut project, 1, &[REST, REST, REST, REST]);
    let tone = write(&mut project, 2, &[A2, A2, A2, A2]);
    let next = write(&mut project, 3, &[A3, A3]);
    place(&mut project, 0, 0, 0, rests);
    place(&mut project, 0, 1, 1, tone);
    place(&mut project, 1, 0, 2, next);
    let mut app = App::with_project(project);
    apply(&mut app, &[RIGHT, TAB, PLAY_SONG]);
    let mut player = Player::new();
    let mut samples = render(&mut app, &mut player, 1);
    samples.extend(player.render(STEP_SAMPLES / 2));

    app.apply(Action::Clear);
    samples.extend(render(&mut app, &mut player, 4));

    assert_steps(&samples, &[440.0, 440.0, 0.0, 0.0, 880.0]);
}

fn three_row_song() -> Project {
    let mut project = Project::new();
    let first = write(&mut project, 1, &[A1]);
    let second = write(&mut project, 2, &[A2]);
    let third = write(&mut project, 3, &[A3]);
    place(&mut project, 0, 0, 0, first);
    place(&mut project, 1, 0, 1, second);
    place(&mut project, 2, 0, 2, third);
    project
}

#[test]
fn an_emptied_song_row_acts_like_reaching_an_empty_row() {
    let mut app = App::with_project(three_row_song());
    app.apply(PLAY_SONG);
    let mut player = Player::new();
    let mut samples = render(&mut app, &mut player, 1);
    samples.extend(player.render(STEP_SAMPLES / 2));

    apply(&mut app, &[DOWN, Action::Clear]);
    samples.extend(render(&mut app, &mut player, 2));

    assert_steps(&samples, &[220.0, 440.0, 0.0]);
}

#[test]
fn a_chain_with_every_entry_cleared_keeps_playing_until_it_has_notes_again() {
    let mut project = Project::new();
    let tone = write(&mut project, 1, &[A2, A2]);
    place(&mut project, 0, 0, 0, tone);
    let mut app = App::with_project(project);
    apply(&mut app, &[TAB, Action::PlayFocusedPane]);
    let mut player = Player::new();
    render(&mut app, &mut player, 1);

    app.apply(Action::Clear);
    let silence = render(&mut app, &mut player, 4);
    assert_eq!(player.finished_play(), 0, "the edit must not stop playback");
    assert!(step_peak(&silence, 3) < 0.01);

    app.apply(Action::HexDigit(1));
    let samples = render(&mut app, &mut player, 2);
    assert_steps(&samples, &[440.0, 440.0]);
}

#[test]
fn a_looping_phrase_shortened_past_the_playhead_starts_its_loop_again() {
    let mut project = Project::new();
    let riff = write(&mut project, 1, &[A2, A2, A3, A3]);
    project.phrase_mut(riff).set_loop(0, 4);
    let stretch = write(&mut project, 2, &[REST; 8]);
    place(&mut project, 0, 0, 0, riff);
    place(&mut project, 0, 1, 1, stretch);
    let mut app = playing_in_phrase_pane(project);
    let mut player = Player::new();
    let mut samples = render(&mut app, &mut player, 2);
    samples.extend(player.render(STEP_SAMPLES / 2));

    apply(
        &mut app,
        &[
            UP,
            UP,
            UP,
            RIGHT,
            RIGHT,
            Action::HexDigit(0),
            Action::HexDigit(2),
        ],
    );
    samples.extend(render(&mut app, &mut player, 3));

    assert_steps(&samples, &[440.0, 440.0, 880.0, 440.0, 440.0]);
}
