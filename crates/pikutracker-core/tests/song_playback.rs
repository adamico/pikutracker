mod support;

use pikutracker_core::app::{Action, App, Direction};
use pikutracker_core::player::{AudioRequest, Player};
use pikutracker_core::project::{ChainEntry, ChainId, PhraseId, Project, SongRowMarks};
use pikutracker_core::synth::{SAMPLE_RATE, SAMPLES_PER_TICK};
use support::status_line;

const DOWN: Action = Action::Move(Direction::Down);
const SPACE: Action = Action::PlayFocusedPane;
const SHIFT_SPACE: Action = Action::PlaySongFromCursor;
const SHIFT_TAB: Action = Action::FocusPrevious;

const SPEED: i32 = 8;
const STEP_SAMPLES: usize = SPEED as usize * SAMPLES_PER_TICK as usize;
const CROSSFADE_SAMPLES: usize = 300;

const A1: Option<i32> = Some(21);
const A2: Option<i32> = Some(33);
const A3: Option<i32> = Some(45);
const A4: Option<i32> = Some(57);
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

fn place(
    project: &mut Project,
    row: usize,
    track: usize,
    chain: u8,
    entries: &[Option<ChainEntry>],
) {
    let id = ChainId::new(chain);
    for (index, entry) in entries.iter().enumerate() {
        *project.chain_mut(id).entry_mut(index) = *entry;
    }
    project.set_song_cell(row, track, Some(id));
}

fn plain(phrase: PhraseId) -> Option<ChainEntry> {
    Some(ChainEntry::new(phrase))
}

fn looping_from(project: &mut Project, phrase: PhraseId, start: i32) -> PhraseId {
    let phrase_ref = project.phrase_mut(phrase);
    let end = phrase_ref.length() as i32;
    phrase_ref.set_loop(start, end);
    phrase
}

fn transposed(phrase: PhraseId, semitones: i32) -> Option<ChainEntry> {
    let mut entry = ChainEntry::new(phrase);
    entry.set_transpose(semitones);
    Some(entry)
}

fn render(app: &mut App, player: &mut Player, steps: usize) -> Vec<f32> {
    for request in app.take_audio_requests() {
        player.handle(request);
    }
    player.render(steps * STEP_SAMPLES)
}

fn hear(project: Project, actions: &[Action], steps: usize) -> Vec<f32> {
    let mut app = App::with_project(project);
    for action in actions {
        app.apply(*action);
    }
    render(&mut app, &mut Player::new(), steps)
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

#[test]
fn chain_rows_play_one_after_another() {
    let mut project = Project::new();
    let low = write(&mut project, 1, &[A2, A2]);
    let high = write(&mut project, 2, &[A3, A3]);
    place(&mut project, 0, 0, 0, &[plain(low), plain(high)]);

    let samples = hear(project, &[SPACE], 4);

    assert_steps(&samples, &[440.0, 440.0, 880.0, 880.0]);
}

#[test]
fn a_chain_row_lasts_exactly_its_ticks_at_183_samples_each() {
    let mut project = Project::new();
    let tone = write(&mut project, 1, &[A2, A2]);
    place(&mut project, 0, 0, 0, &[plain(tone)]);
    let mut app = App::with_project(project);
    let mut player = Player::new();
    app.apply(SPACE);
    render(&mut app, &mut player, 2);
    assert_eq!(player.finished_play(), 0, "still inside the Chain row");

    player.next_sample();

    assert_eq!(player.finished_play(), 1);
}

#[test]
fn tracks_sound_together_as_a_plain_mix() {
    let mut both = Project::new();
    let low = write(&mut both, 1, &[A2, A2]);
    let high = write(&mut both, 2, &[A3, A3]);
    let mut only_low = both.clone();
    let mut only_high = both.clone();
    place(&mut both, 0, 0, 0, &[plain(low)]);
    place(&mut both, 0, 3, 1, &[plain(high)]);
    place(&mut only_low, 0, 0, 0, &[plain(low)]);
    place(&mut only_high, 0, 3, 1, &[plain(high)]);

    let mix = hear(both, &[SPACE], 2);
    let low = hear(only_low, &[SPACE], 2);
    let high = hear(only_high, &[SPACE], 2);

    for (index, sample) in mix.iter().enumerate() {
        assert!((sample - (low[index] + high[index])).abs() < 1e-4);
    }
}

#[test]
fn a_short_phrase_rests_until_the_chain_row_ends() {
    let mut project = Project::new();
    let short = write(&mut project, 1, &[A2, A2]);
    let next = write(&mut project, 2, &[A3, A3]);
    let long_silence = write(&mut project, 3, &[REST; 4]);
    place(&mut project, 0, 0, 0, &[plain(short), plain(next)]);
    place(&mut project, 0, 1, 1, &[plain(long_silence)]);

    let samples = hear(project, &[SPACE], 6);

    assert_steps(&samples, &[440.0, 440.0, 0.0, 0.0, 880.0, 880.0]);
}

#[test]
fn a_phrase_loop_repeats_until_the_chain_row_ends() {
    let mut project = Project::new();
    let riff = write(&mut project, 1, &[A2, A3]);
    let riff = looping_from(&mut project, riff, 0);
    let next = write(&mut project, 2, &[A1, A1]);
    let long_silence = write(&mut project, 3, &[REST; 5]);
    place(&mut project, 0, 0, 0, &[plain(riff), plain(next)]);
    place(&mut project, 0, 1, 1, &[plain(long_silence)]);

    let samples = hear(project, &[SPACE], 7);

    assert_steps(&samples, &[440.0, 880.0, 440.0, 880.0, 440.0, 220.0, 220.0]);
}

#[test]
fn the_steps_before_a_phrase_loop_play_once_as_a_lead_in() {
    let mut project = Project::new();
    let riff = write(&mut project, 1, &[A1, A2, A3]);
    let riff = looping_from(&mut project, riff, 1);
    let next = write(&mut project, 2, &[A4, A4]);
    let long_silence = write(&mut project, 3, &[REST; 6]);
    place(&mut project, 0, 0, 0, &[plain(riff), plain(next)]);
    place(&mut project, 0, 1, 1, &[plain(long_silence)]);

    let samples = hear(project, &[SPACE], 8);

    assert_steps(
        &samples,
        &[220.0, 440.0, 880.0, 440.0, 880.0, 440.0, 1760.0, 1760.0],
    );
}

#[test]
fn an_all_looping_chain_row_plays_its_longest_phrase_once_through() {
    let mut project = Project::new();
    let short = write(&mut project, 1, &[REST, REST]);
    let short = looping_from(&mut project, short, 0);
    let long = write(&mut project, 2, &[A1, A2, A3, A3]);
    let long = looping_from(&mut project, long, 2);
    let next = write(&mut project, 3, &[A4, A4]);
    place(&mut project, 0, 0, 0, &[plain(short)]);
    place(&mut project, 0, 1, 1, &[plain(long), plain(next)]);

    let samples = hear(project, &[SPACE], 6);

    assert_steps(&samples, &[220.0, 440.0, 880.0, 880.0, 1760.0, 1760.0]);
}

#[test]
fn a_chain_row_with_no_entries_takes_no_time() {
    let mut project = Project::new();
    let low = write(&mut project, 1, &[A2]);
    let high = write(&mut project, 2, &[A3]);
    place(&mut project, 0, 0, 0, &[plain(low), None, plain(high)]);

    let samples = hear(project, &[SPACE], 2);

    assert_steps(&samples, &[440.0, 880.0]);
}

#[test]
fn a_transpose_shifts_the_phrase() {
    let mut project = Project::new();
    let tone = write(&mut project, 1, &[A2, A2]);
    place(&mut project, 0, 0, 0, &[transposed(tone, 12)]);

    let samples = hear(project, &[SPACE], 2);

    assert_steps(&samples, &[880.0, 880.0]);
}

#[test]
fn a_step_transposed_past_63_plays_at_its_true_pitch() {
    let mut project = Project::new();
    let tone = write(&mut project, 1, &[Some(63), Some(63)]);
    place(&mut project, 0, 0, 0, &[transposed(tone, 1)]);

    let samples = hear(project, &[SPACE], 2);

    assert_steps(&samples, &[2637.0, 2637.0]);
}

fn three_row_song() -> Project {
    let mut project = Project::new();
    let first = write(&mut project, 1, &[A1]);
    let second = write(&mut project, 2, &[A2]);
    let third = write(&mut project, 3, &[A3]);
    place(&mut project, 0, 0, 0, &[plain(first)]);
    place(&mut project, 1, 0, 1, &[plain(second)]);
    place(&mut project, 2, 0, 2, &[plain(third)]);
    project
}

fn marked(
    mut project: Project,
    row: usize,
    loop_start: bool,
    loop_back: bool,
    stop: bool,
) -> Project {
    let marks = SongRowMarks {
        loop_start,
        loop_back,
        stop,
    };
    project.set_song_row_marks(row, marks);
    project
}

#[test]
fn after_a_loop_back_row_the_song_returns_to_the_loop_start() {
    let project = marked(three_row_song(), 1, true, false, false);
    let project = marked(project, 2, false, true, false);

    let samples = hear(project, &[SPACE], 6);

    assert_steps(&samples, &[220.0, 440.0, 880.0, 440.0, 880.0, 440.0]);
}

#[test]
fn loop_back_returns_to_the_nearest_loop_start_at_or_before_it() {
    let project = marked(three_row_song(), 0, true, false, false);
    let project = marked(project, 1, true, true, false);

    let samples = hear(project, &[SPACE], 4);

    assert_steps(&samples, &[220.0, 440.0, 440.0, 440.0]);
}

#[test]
fn loop_back_without_a_loop_start_returns_to_song_row_00() {
    let project = marked(three_row_song(), 2, false, true, false);

    let samples = hear(project, &[DOWN, SPACE], 4);

    assert_steps(&samples, &[440.0, 880.0, 220.0, 440.0]);
}

#[test]
fn playing_from_below_a_loop_start_still_loops_back_to_it() {
    let project = marked(three_row_song(), 1, true, false, false);
    let project = marked(project, 2, false, true, false);

    let samples = hear(project, &[DOWN, DOWN, SPACE], 3);

    assert_steps(&samples, &[880.0, 440.0, 880.0]);
}

#[test]
fn playback_ends_after_a_stop_row() {
    let project = marked(three_row_song(), 1, false, false, true);

    let samples = hear(project, &[SPACE], 4);

    assert_steps(&samples, &[220.0, 440.0, 0.0, 0.0]);
}

#[test]
fn a_row_with_loop_back_and_stop_ends_playback_as_pico8_does() {
    let project = marked(three_row_song(), 0, true, false, false);
    let project = marked(project, 1, false, true, true);

    let samples = hear(project, &[SPACE], 4);

    assert_steps(&samples, &[220.0, 440.0, 0.0, 0.0]);
}

#[test]
fn playback_ends_at_an_empty_song_row() {
    let mut project = three_row_song();
    project.set_song_cell(1, 0, None);

    let samples = hear(project, &[SPACE], 3);

    assert_steps(&samples, &[220.0, 0.0, 0.0]);
}

#[test]
fn playback_ends_after_the_last_row() {
    let samples = hear(three_row_song(), &[SPACE], 5);

    assert_steps(&samples, &[220.0, 440.0, 880.0, 0.0, 0.0]);
}

#[test]
fn a_loop_back_to_an_empty_row_ends_playback() {
    let mut project = marked(three_row_song(), 2, false, true, false);
    project.set_song_cell(0, 0, None);

    let samples = hear(project, &[DOWN, SPACE], 4);

    assert_steps(&samples, &[440.0, 880.0, 0.0, 0.0]);
}

#[test]
fn space_plays_again_after_the_song_has_stopped_by_itself() {
    let mut app = App::with_project(three_row_song());
    let mut player = Player::new();
    app.apply(SPACE);
    render(&mut app, &mut player, 5);

    app.playback_finished(player.finished_play());
    app.apply(SPACE);

    assert!(matches!(
        app.take_audio_requests().as_slice(),
        [AudioRequest::PlaySong { from_row: 0, .. }]
    ));
}

#[test]
fn a_report_about_an_earlier_play_does_not_stop_the_current_one() {
    let mut app = App::with_project(three_row_song());
    let mut player = Player::new();
    app.apply(SPACE);
    render(&mut app, &mut player, 5);
    let earlier = player.finished_play();

    app.apply(SHIFT_SPACE);
    app.playback_finished(earlier);
    app.take_audio_requests();
    app.apply(SPACE);

    assert!(matches!(
        app.take_audio_requests().as_slice(),
        [AudioRequest::Stop]
    ));
}

#[test]
fn space_in_the_song_pane_plays_from_the_cursors_row() {
    let samples = hear(three_row_song(), &[DOWN, SPACE], 3);

    assert_steps(&samples, &[440.0, 880.0, 0.0]);
}

#[test]
fn shift_space_plays_the_song_from_the_cursors_row_in_any_pane() {
    let samples = hear(
        three_row_song(),
        &[DOWN, SHIFT_TAB, SHIFT_TAB, SHIFT_SPACE],
        2,
    );

    assert_steps(&samples, &[440.0, 880.0]);
}

#[test]
fn shift_space_during_playback_restarts_the_song_with_no_stop_between() {
    let mut app = App::with_project(three_row_song());
    app.apply(SPACE);
    app.take_audio_requests();

    app.apply(DOWN);
    app.apply(SHIFT_SPACE);

    assert!(matches!(
        app.take_audio_requests().as_slice(),
        [AudioRequest::PlaySong { from_row: 1, .. }]
    ));
}

#[test]
fn space_stops_the_song_at_once() {
    let mut app = App::with_project(three_row_song());
    let mut player = Player::new();
    app.apply(SPACE);
    render(&mut app, &mut player, 1);

    app.apply(SPACE);
    let samples = render(&mut app, &mut player, 2);

    assert!(samples.iter().all(|sample| *sample == 0.0));
}

#[test]
fn moving_the_cursor_during_playback_changes_nothing_that_plays() {
    let mut app = App::with_project(three_row_song());
    app.apply(SPACE);
    app.take_audio_requests();

    app.apply(DOWN);
    app.apply(SHIFT_TAB);

    assert!(app.take_audio_requests().is_empty());
}

fn two_track_chain() -> Project {
    let mut project = Project::new();
    let low = write(&mut project, 1, &[A2, A2]);
    let high = write(&mut project, 2, &[A3, A3]);
    let stretch = write(&mut project, 3, &[A4, A4, A4, A4]);
    place(&mut project, 0, 0, 0, &[plain(low), plain(high)]);
    place(&mut project, 0, 1, 1, &[plain(stretch)]);
    project
}

#[test]
fn space_in_the_chain_pane_plays_the_chain_alone_with_chain_row_timing_and_loops() {
    let samples = hear(two_track_chain(), &[SHIFT_TAB, SPACE], 8);

    assert_steps(
        &samples,
        &[440.0, 440.0, 0.0, 0.0, 880.0, 880.0, 440.0, 440.0],
    );
}

#[test]
fn the_chain_plays_from_the_cursors_entry_and_loops_to_the_first() {
    let samples = hear(two_track_chain(), &[SHIFT_TAB, DOWN, SPACE], 4);

    assert_steps(&samples, &[880.0, 880.0, 440.0, 440.0]);
}

#[test]
fn a_phrase_loop_repeats_in_the_chain_pane_as_it_does_in_the_song() {
    let mut project = Project::new();
    let riff = write(&mut project, 1, &[A2, A3]);
    let riff = looping_from(&mut project, riff, 0);
    let stretch = write(&mut project, 2, &[REST; 4]);
    place(&mut project, 0, 0, 0, &[plain(riff)]);
    place(&mut project, 0, 1, 1, &[plain(stretch)]);

    let samples = hear(project, &[SHIFT_TAB, SPACE], 4);

    assert_steps(&samples, &[440.0, 880.0, 440.0, 880.0]);
}

#[test]
fn space_in_the_phrase_pane_plays_the_phrase_alone_with_its_transpose() {
    let mut project = Project::new();
    let riff = write(&mut project, 1, &[A2, A1]);
    let stretch = write(&mut project, 2, &[A4; 8]);
    place(&mut project, 0, 0, 0, &[transposed(riff, 12)]);
    place(&mut project, 0, 1, 1, &[plain(stretch)]);

    let samples = hear(project, &[SHIFT_TAB, SHIFT_TAB, SPACE], 4);

    assert_steps(&samples, &[880.0, 440.0, 880.0, 440.0]);
}

#[test]
fn a_phrase_played_alone_repeats_its_phrase_loop_after_the_lead_in() {
    let mut project = Project::new();
    let riff = write(&mut project, 1, &[A1, A2, A3]);
    let riff = looping_from(&mut project, riff, 1);
    place(&mut project, 0, 0, 0, &[plain(riff)]);

    let samples = hear(project, &[SHIFT_TAB, SHIFT_TAB, SPACE], 6);

    assert_steps(&samples, &[220.0, 440.0, 880.0, 440.0, 880.0, 440.0]);
}

#[test]
fn the_right_hand_view_plays_the_last_focused_pane() {
    let samples = hear(three_row_song(), &[Action::FocusNext, SPACE], 2);

    assert_steps(&samples, &[220.0, 440.0]);
}

fn assert_nothing_to_play(project: Project, actions: &[Action]) {
    let mut app = App::with_project(project);
    for action in actions {
        app.apply(*action);
    }

    assert!(app.take_audio_requests().is_empty());
    assert_eq!(status_line(&app), "NOTHING TO PLAY");
}

#[test]
fn space_on_a_song_row_with_nothing_on_it_says_so() {
    assert_nothing_to_play(Project::new(), &[SPACE]);
    assert_nothing_to_play(three_row_song(), &[DOWN, DOWN, DOWN, SPACE]);
    assert_nothing_to_play(Project::new(), &[SHIFT_SPACE]);
}

#[test]
fn space_in_the_chain_pane_with_no_chain_says_so() {
    assert_nothing_to_play(Project::new(), &[SHIFT_TAB, SPACE]);
}

#[test]
fn space_on_a_chain_with_no_entries_says_so() {
    let mut project = Project::new();
    project.set_song_cell(0, 0, Some(ChainId::new(0)));

    assert_nothing_to_play(project, &[SHIFT_TAB, SPACE]);
}

#[test]
fn space_on_a_chain_of_silent_phrases_says_so() {
    let mut project = Project::new();
    let silence = write(&mut project, 1, &[REST, REST]);
    place(&mut project, 0, 0, 0, &[plain(silence)]);

    assert_nothing_to_play(project, &[SHIFT_TAB, SPACE]);
}

#[test]
fn space_in_the_phrase_pane_with_no_phrase_says_so() {
    assert_nothing_to_play(Project::new(), &[SHIFT_TAB, SHIFT_TAB, SPACE]);
}

#[test]
fn space_on_a_silent_phrase_says_so() {
    let mut project = Project::new();
    let silence = write(&mut project, 1, &[REST, REST]);
    place(&mut project, 0, 0, 0, &[plain(silence)]);

    assert_nothing_to_play(project, &[SHIFT_TAB, SHIFT_TAB, SPACE]);
}
