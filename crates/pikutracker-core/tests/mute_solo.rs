mod support;

use pikutracker_core::app::{Action, App, layout};
use pikutracker_core::player::Player;
use pikutracker_core::project::{ChainEntry, ChainId, PhraseId, Project};
use pikutracker_core::screen::palette::Colour;
use pikutracker_core::synth::{SAMPLE_RATE, SAMPLES_PER_TICK};
use support::pane_text;

const PLAY_SONG: Action = Action::PlaySongFromCursor;
const TAB: Action = Action::FocusNext;
const SPACE: Action = Action::PlayFocusedPane;

const SPEED: i32 = 8;
const STEP_SAMPLES: usize = SPEED as usize * SAMPLES_PER_TICK as usize;
const CROSSFADE_SAMPLES: usize = 300;

const A2: i32 = 33;
const A3: i32 = 45;
const TRACK_HEADER_ROW: usize = 1;

fn write(project: &mut Project, id: u8, pitch: i32, steps: usize) -> PhraseId {
    let id = PhraseId::new(id);
    let phrase = project.phrase_mut(id);
    phrase.set_length(steps as i32);
    phrase.set_speed(SPEED);
    for index in 0..steps {
        let step = phrase.step_mut(index);
        step.set_pitch(pitch);
        step.set_volume(5);
    }
    id
}

fn place(project: &mut Project, row: usize, track: usize, chain: u8, phrase: PhraseId) {
    let id = ChainId::new(chain);
    *project.chain_mut(id).entry_mut(0) = Some(ChainEntry::new(phrase));
    project.set_song_cell(row, track, Some(id));
}

fn two_tracks() -> Project {
    let mut project = Project::new();
    let low = write(&mut project, 1, A2, 2);
    let high = write(&mut project, 2, A3, 4);
    let next = write(&mut project, 3, A2, 2);
    place(&mut project, 0, 0, 0, low);
    place(&mut project, 0, 1, 1, high);
    place(&mut project, 1, 0, 2, next);
    project
}

fn hear(project: Project, actions: &[Action], steps: usize) -> Vec<f32> {
    let mut app = App::with_project(project);
    for action in actions {
        app.apply(*action);
    }
    let mut player = Player::new();
    for request in app.take_audio_requests() {
        player.handle(request);
    }
    player.render(steps * STEP_SAMPLES)
}

fn step_peak(samples: &[f32], step: usize) -> f32 {
    samples[step * STEP_SAMPLES + CROSSFADE_SAMPLES..(step + 1) * STEP_SAMPLES]
        .iter()
        .fold(0.0, |peak, sample| peak.max(sample.abs()))
}

fn step_frequency(samples: &[f32], step: usize) -> f32 {
    let window = &samples[step * STEP_SAMPLES + CROSSFADE_SAMPLES..(step + 1) * STEP_SAMPLES];
    let rising: Vec<usize> = window
        .windows(2)
        .enumerate()
        .filter(|(_, pair)| pair[0] < 0.0 && pair[1] >= 0.0)
        .map(|(index, _)| index)
        .collect();
    let (first, last) = (rising[0], rising[rising.len() - 1]);
    (rising.len() - 1) as f32 * SAMPLE_RATE as f32 / (last - first) as f32
}

fn assert_near(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= expected * 0.03,
        "expected about {expected}, got {actual}"
    );
}

fn headers(app: &App) -> String {
    pane_text(app, layout::SONG, TRACK_HEADER_ROW)
        .trim_start()
        .to_owned()
}

#[test]
fn a_muted_track_is_silent() {
    let samples = hear(
        two_tracks(),
        &[Action::Mute { track_number: 2 }, PLAY_SONG],
        4,
    );

    assert_near(step_frequency(&samples, 0), 440.0);
    assert!(step_peak(&samples, 2) < 0.01, "only Track 2 plays here");
}

#[test]
fn a_muted_track_still_sets_the_chain_rows_length() {
    let samples = hear(
        two_tracks(),
        &[Action::Mute { track_number: 2 }, PLAY_SONG],
        5,
    );

    assert!(step_peak(&samples, 3) < 0.01, "the Chain row is still on");
    assert_near(step_frequency(&samples, 4), 440.0);
}

#[test]
fn muting_again_brings_the_track_back() {
    let samples = hear(
        two_tracks(),
        &[
            Action::Mute { track_number: 2 },
            Action::Mute { track_number: 2 },
            PLAY_SONG,
        ],
        4,
    );

    assert_near(step_frequency(&samples, 2), 880.0);
}

#[test]
fn a_solo_silences_every_other_track() {
    let samples = hear(
        two_tracks(),
        &[Action::Solo { track_number: 2 }, PLAY_SONG],
        2,
    );

    assert_near(step_frequency(&samples, 0), 880.0);
}

#[test]
fn several_tracks_can_be_soloed_at_once() {
    let both = hear(two_tracks(), &[PLAY_SONG], 2);
    let soloed = hear(
        two_tracks(),
        &[
            Action::Solo { track_number: 1 },
            Action::Solo { track_number: 2 },
            PLAY_SONG,
        ],
        2,
    );

    assert_eq!(both, soloed);
}

#[test]
fn solo_overrides_mute() {
    let samples = hear(
        two_tracks(),
        &[
            Action::Mute { track_number: 2 },
            Action::Solo { track_number: 2 },
            PLAY_SONG,
        ],
        2,
    );

    assert_near(step_frequency(&samples, 0), 880.0);
}

#[test]
fn clearing_brings_back_every_track() {
    let plain = hear(two_tracks(), &[PLAY_SONG], 4);
    let cleared = hear(
        two_tracks(),
        &[
            Action::Mute { track_number: 1 },
            Action::Solo { track_number: 2 },
            Action::ClearMutesAndSolos,
            PLAY_SONG,
        ],
        4,
    );

    assert_eq!(plain, cleared);
}

#[test]
fn a_mute_during_playback_is_heard_without_changing_the_timing() {
    let mut app = App::with_project(two_tracks());
    let mut player = Player::new();
    app.apply(PLAY_SONG);
    for request in app.take_audio_requests() {
        player.handle(request);
    }
    player.render(STEP_SAMPLES);

    app.apply(Action::Mute { track_number: 1 });
    for request in app.take_audio_requests() {
        player.handle(request);
    }
    let samples = player.render(STEP_SAMPLES * 4);

    assert_near(step_frequency(&samples, 0), 880.0);
    assert_near(step_frequency(&samples, 2), 880.0);
    assert!(step_peak(&samples, 3) < 0.01);
}

#[test]
fn the_chain_pane_plays_a_muted_track_all_the_same() {
    let samples = hear(
        two_tracks(),
        &[Action::Mute { track_number: 1 }, TAB, SPACE],
        2,
    );

    assert_near(step_frequency(&samples, 0), 440.0);
}

#[test]
fn the_phrase_pane_plays_a_muted_track_all_the_same() {
    let samples = hear(
        two_tracks(),
        &[Action::Mute { track_number: 1 }, TAB, TAB, SPACE],
        2,
    );

    assert_near(step_frequency(&samples, 0), 440.0);
}

#[test]
fn track_headers_mark_muted_and_soloed_tracks() {
    let mut app = App::with_project(two_tracks());
    assert_eq!(headers(&app), "T1 T2 T3 T4");

    app.apply(Action::Mute { track_number: 2 });
    app.apply(Action::Solo { track_number: 3 });

    assert_eq!(headers(&app), "T1 M2 S3 T4");
}

#[test]
fn a_muted_track_is_dimmed_on_its_header() {
    let mut app = App::with_project(two_tracks());

    app.apply(Action::Mute { track_number: 1 });

    assert_eq!(
        app.grid()
            .cell(layout::SONG.column + 4, layout::SONG.row + TRACK_HEADER_ROW)
            .style
            .fg,
        Colour::DarkGrey
    );
}

#[test]
fn a_track_both_muted_and_soloed_shows_as_soloed() {
    let mut app = App::with_project(two_tracks());

    app.apply(Action::Mute { track_number: 1 });
    app.apply(Action::Solo { track_number: 1 });

    assert_eq!(headers(&app), "S1 T2 T3 T4");
}

#[test]
fn mute_and_solo_do_not_change_the_project() {
    let mut app = App::with_project(two_tracks());
    let before = app.project().clone();

    app.apply(Action::Mute { track_number: 1 });
    app.apply(Action::Solo { track_number: 2 });

    assert_eq!(*app.project(), before);
}
