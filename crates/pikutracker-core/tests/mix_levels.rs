use pikutracker_core::app::{Action, App};
use pikutracker_core::player::Player;
use pikutracker_core::project::{ChainEntry, ChainId, PhraseId, Project};
use pikutracker_core::synth::SAMPLES_PER_TICK;

const PLAY_SONG: Action = Action::PlaySongFromCursor;
const SPACE: Action = Action::PlayFocusedPane;

const SPEED: i32 = 8;
const STEP_SAMPLES: usize = SPEED as usize * SAMPLES_PER_TICK as usize;
const CROSSFADE_SAMPLES: usize = 300;
const TRIANGLE: i32 = 0;
const SQUARE: i32 = 3;
const RIGHT_SLOT_COLUMN: usize = 83;
const FIRST_SCOPE_ROW: usize = 1;

fn write(project: &mut Project, id: u8, instrument: i32, volume: i32, steps: usize) -> PhraseId {
    let id = PhraseId::new(id);
    let phrase = project.phrase_mut(id);
    phrase.set_length(steps as i32);
    phrase.set_speed(SPEED);
    for index in 0..steps {
        let step = phrase.step_mut(index);
        step.set_pitch(33);
        step.set_instrument(instrument);
        step.set_volume(volume);
    }
    id
}

fn place(project: &mut Project, row: usize, track: usize, phrase: PhraseId) {
    let id = ChainId::new((row * 4 + track) as u8);
    *project.chain_mut(id).entry_mut(0) = Some(ChainEntry::new(phrase));
    project.set_song_cell(row, track, Some(id));
}

fn unison(instrument: i32, volume: i32, tracks: usize, steps: usize) -> Project {
    let mut project = Project::new();
    let phrase = write(&mut project, 1, instrument, volume, steps);
    for track in 0..tracks {
        place(&mut project, 0, track, phrase);
    }
    project
}

fn hear(project: Project, samples: usize) -> Vec<f32> {
    let mut app = App::with_project(project);
    let mut player = Player::new();
    app.apply(PLAY_SONG);
    for request in app.take_audio_requests() {
        player.handle(request);
    }
    player.render(samples)
}

fn peak(samples: &[f32]) -> f32 {
    samples
        .iter()
        .fold(0.0, |peak, sample| peak.max(sample.abs()))
}

#[test]
fn a_mix_below_the_knee_plays_as_a_plain_sum() {
    let one = hear(unison(SQUARE, 7, 1, 8), STEP_SAMPLES * 4);
    let three = hear(unison(SQUARE, 7, 3, 8), STEP_SAMPLES * 4);

    for (single, mixed) in one.iter().zip(&three) {
        assert!((mixed - single * 3.0).abs() < 1e-3, "{mixed} vs {single}");
    }
}

#[test]
fn a_loud_four_track_mix_never_leaves_the_range() {
    let samples = hear(unison(TRIANGLE, 7, 4, 8), STEP_SAMPLES * 4);

    assert!(peak(&samples) <= 1.0);
}

#[test]
fn the_level_comes_back_after_a_loud_passage() {
    let mut project = unison(TRIANGLE, 7, 4, 4);
    let quiet = write(&mut project, 2, TRIANGLE, 5, 8);
    place(&mut project, 1, 0, quiet);

    let samples = hear(project, STEP_SAMPLES * 12);

    let last_step = &samples[STEP_SAMPLES * 11 + CROSSFADE_SAMPLES..];
    let triangle_peak_at_volume_5 = 0.5 * 5.0 / 7.0;
    assert!(
        (peak(last_step) - triangle_peak_at_volume_5).abs() < 0.02,
        "{}",
        peak(last_step)
    );
}

fn scope_marks(app: &App) -> usize {
    app.grid()
        .text_dump()
        .lines()
        .skip(FIRST_SCOPE_ROW)
        .take(36)
        .map(|line| {
            line.chars()
                .skip(RIGHT_SLOT_COLUMN)
                .filter(|ch| *ch == '*')
                .count()
        })
        .sum()
}

fn play_and_report(app: &mut App, player: &mut Player, samples: usize) {
    for request in app.take_audio_requests() {
        player.handle(request);
    }
    player.render(samples);
    app.show_playback(&player.report());
}

#[test]
fn the_scope_is_blank_while_stopped() {
    let app = App::with_project(unison(SQUARE, 5, 1, 8));

    assert_eq!(scope_marks(&app), 0);
}

#[test]
fn the_scope_draws_the_mix_being_heard() {
    let mut app = App::with_project(unison(SQUARE, 5, 1, 8));
    let mut player = Player::new();

    app.apply(PLAY_SONG);
    play_and_report(&mut app, &mut player, STEP_SAMPLES);

    assert_eq!(scope_marks(&app), 23, "one mark in every column");
}

#[test]
fn a_muted_track_does_not_reach_the_scope() {
    let mut app = App::with_project(unison(SQUARE, 5, 1, 8));
    let mut player = Player::new();

    app.apply(Action::Mute { track_number: 1 });
    app.apply(PLAY_SONG);
    play_and_report(&mut app, &mut player, STEP_SAMPLES);

    assert_eq!(scope_marks(&app), 0);
}

#[test]
fn the_scope_clears_when_playback_stops() {
    let mut app = App::with_project(unison(SQUARE, 5, 1, 8));
    let mut player = Player::new();
    app.apply(PLAY_SONG);
    play_and_report(&mut app, &mut player, STEP_SAMPLES);

    app.apply(SPACE);
    play_and_report(&mut app, &mut player, 1);

    assert_eq!(scope_marks(&app), 0);
}
