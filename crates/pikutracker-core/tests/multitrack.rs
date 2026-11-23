mod support;

use pikutracker_core::app::layout;
use pikutracker_core::app::{Action, App, Direction};
use pikutracker_core::player::Player;
use pikutracker_core::project::{
    ChainEntry, ChainId, PhraseId, Project, SongRowEnding, SongRowMarks,
};
use pikutracker_core::synth::SAMPLES_PER_TICK;
use support::{apply, pane_text, status_line};

const TITLE_LINE: usize = 0;
const HEADING_LINE: usize = 1;
const FIRST_LINE: usize = 2;
const CENTRE_LINE: usize = 21;

const DOWN: Action = Action::Move(Direction::Down);
const TAB: Action = Action::FocusNext;
const PLAY_SONG: Action = Action::PlaySongFromCursor;
const FOLLOW: Action = Action::ToggleFollow;

const C2: i32 = 24;
const D2: i32 = 26;
const E2: i32 = 28;
const F2: i32 = 29;
const G2: i32 = 31;

fn write(project: &mut Project, id: u8, speed: i32, pitches: &[i32]) -> PhraseId {
    let id = PhraseId::new(id);
    let phrase = project.phrase_mut(id);
    phrase.set_length(pitches.len() as i32);
    phrase.set_speed(speed);
    for (index, &pitch) in pitches.iter().enumerate() {
        let step = phrase.step_mut(index);
        step.set_pitch(pitch);
        step.set_instrument(2);
        step.set_volume(5);
    }
    id
}

fn place(project: &mut Project, song_row: usize, track: usize, chain: u8, entries: &[ChainEntry]) {
    let id = ChainId::new(chain);
    for (index, entry) in entries.iter().enumerate() {
        *project.chain_mut(id).entry_mut(index) = Some(*entry);
    }
    project.set_song_cell(song_row, track, Some(id));
}

fn line(app: &App, line: usize) -> String {
    pane_text(app, layout::RIGHT_SLOT, line)
}

fn lines(app: &App, count: usize) -> Vec<String> {
    (FIRST_LINE..FIRST_LINE + count)
        .map(|index| line(app, index))
        .collect()
}

#[test]
fn the_multitrack_heads_its_columns_track_1_to_track_4() {
    let app = App::new();

    assert_eq!(line(&app, TITLE_LINE), "MULTITRACK  ROW 00");
    assert_eq!(
        line(&app, HEADING_LINE),
        "    TRACK 1     TRACK 2     TRACK 3     TRACK 4"
    );
}

#[test]
fn a_track_shows_its_steps_in_time_order() {
    let mut project = Project::new();
    let phrase = write(&mut project, 1, 8, &[C2, D2, E2]);
    place(&mut project, 0, 0, 0, &[ChainEntry::new(phrase)]);

    let app = App::with_project(project);

    assert_eq!(
        lines(&app, 4),
        ["00  C-2 2 5 0", "    D-2 2 5 0", "    E-2 2 5 0", ""]
    );
}

#[test]
fn steps_side_by_side_start_together_across_speeds_and_a_slow_step_has_a_tail() {
    let mut project = Project::new();
    let fast = write(&mut project, 1, 8, &[C2, D2, E2, F2]);
    let slow = write(&mut project, 2, 16, &[C2, E2]);
    place(&mut project, 0, 0, 0, &[ChainEntry::new(fast)]);
    place(&mut project, 0, 1, 1, &[ChainEntry::new(slow)]);

    let app = App::with_project(project);

    assert_eq!(
        lines(&app, 5),
        [
            "00  C-2 2 5 0   C-2 2 5 0",
            "    D-2 2 5 0   |",
            "    E-2 2 5 0   E-2 2 5 0",
            "    F-2 2 5 0   |",
            "",
        ]
    );
}

#[test]
fn a_slow_track_alone_takes_no_blank_rows() {
    let mut project = Project::new();
    let slow = write(&mut project, 2, 32, &[C2, E2]);
    place(&mut project, 0, 0, 0, &[ChainEntry::new(slow)]);

    let app = App::with_project(project);

    assert_eq!(lines(&app, 3), ["00  C-2 2 5 0", "    E-2 2 5 0", ""]);
}

#[test]
fn an_entrys_transpose_is_applied_to_its_pitches() {
    let mut project = Project::new();
    let phrase = write(&mut project, 1, 8, &[C2, D2]);
    let mut entry = ChainEntry::new(phrase);
    entry.set_transpose(12);
    place(&mut project, 0, 0, 0, &[entry]);

    let app = App::with_project(project);

    assert_eq!(lines(&app, 2), ["00  C-3 2 5 0", "    D-3 2 5 0"]);
}

#[test]
fn a_phrase_loop_repeats_until_its_chain_row_ends_and_steps_past_its_length_are_left_out() {
    let mut project = Project::new();
    let timing = write(&mut project, 1, 8, &[C2, C2, C2, C2, C2]);
    let looping = write(&mut project, 2, 8, &[C2, D2, E2, G2]);
    project.phrase_mut(looping).set_loop(1, 3);
    place(&mut project, 0, 0, 0, &[ChainEntry::new(timing)]);
    place(&mut project, 0, 1, 1, &[ChainEntry::new(looping)]);

    let app = App::with_project(project);

    let looped: Vec<String> = lines(&app, 6)
        .iter()
        .map(|line| line.chars().skip(16).collect())
        .collect();
    assert_eq!(
        looped,
        [
            "C-2 2 5 0",
            "D-2 2 5 0",
            "E-2 2 5 0",
            "D-2 2 5 0",
            "E-2 2 5 0",
            ""
        ]
    );
}

#[test]
fn empty_chain_entries_and_missing_chains_show_as_silence() {
    let mut project = Project::new();
    let two = write(&mut project, 1, 8, &[C2, D2]);
    let one = write(&mut project, 2, 8, &[E2]);
    place(
        &mut project,
        0,
        0,
        0,
        &[ChainEntry::new(two), ChainEntry::new(two)],
    );
    place(&mut project, 0, 1, 1, &[ChainEntry::new(one)]);

    let app = App::with_project(project);

    assert_eq!(
        lines(&app, 5),
        [
            "00  C-2 2 5 0   E-2 2 5 0",
            "    D-2 2 5 0",
            "    C-2 2 5 0",
            "    D-2 2 5 0",
            "",
        ]
    );
}

#[test]
fn the_gutter_numbers_each_song_row_with_its_marks_and_scrolls_on_through_the_song() {
    let mut project = Project::new();
    let phrase = write(&mut project, 1, 8, &[C2]);
    place(&mut project, 0, 0, 0, &[ChainEntry::new(phrase)]);
    project.set_song_cell(1, 0, Some(ChainId::new(0)));
    project.set_song_cell(3, 0, Some(ChainId::new(0)));
    let mut marks = SongRowMarks::default();
    marks.toggle_loop_start();
    project.set_song_row_marks(0, marks);
    let mut marks = SongRowMarks::default();
    marks.toggle_ending(SongRowEnding::LoopBack);
    project.set_song_row_marks(1, marks);

    let app = App::with_project(project);

    assert_eq!(
        lines(&app, 5),
        ["00> C-2 2 5 0", "01< C-2 2 5 0", "02", "03  C-2 2 5 0", ""]
    );
}

#[test]
fn an_empty_song_row_gets_a_gutter_line_with_its_mark_that_the_cursor_passes_over() {
    let mut project = Project::new();
    let phrase = write(&mut project, 1, 8, &[C2]);
    place(&mut project, 0, 0, 0, &[ChainEntry::new(phrase)]);
    project.set_song_cell(2, 0, Some(ChainId::new(0)));
    let mut marks = SongRowMarks::default();
    marks.toggle_ending(SongRowEnding::Stop);
    project.set_song_row_marks(1, marks);
    let mut app = App::with_project(project);
    apply(&mut app, &TO_MULTITRACK);

    app.apply(DOWN);

    assert_eq!(
        lines(&app, 4),
        ["00  C-2 2 5 0", "01|", "02 [C-2]2 5 0", ""]
    );
}

fn long_song() -> Project {
    let mut project = Project::new();
    let phrase = write(&mut project, 1, 1, &[C2; 32]);
    place(&mut project, 0, 0, 0, &[ChainEntry::new(phrase)]);
    for song_row in 1..4 {
        project.set_song_cell(song_row, 0, Some(ChainId::new(0)));
    }
    project
}

fn play_on(app: &mut App, player: &mut Player, ticks: usize) {
    for request in app.take_audio_requests() {
        player.handle(request);
    }
    player.render(ticks * SAMPLES_PER_TICK as usize + 1);
    app.show_playback(&player.report());
}

#[test]
fn with_follow_on_it_tracks_the_playhead_and_its_title_shows_the_playheads_song_row() {
    let mut app = App::with_project(long_song());
    let mut player = Player::new();
    apply(&mut app, &[FOLLOW, PLAY_SONG]);

    play_on(&mut app, &mut player, 2 * 32 + 5);

    assert_eq!(line(&app, TITLE_LINE), "MULTITRACK  ROW 02");
    assert_eq!(line(&app, FIRST_LINE + CENTRE_LINE), ">   C-2 2 5 0");
    assert_eq!(line(&app, FIRST_LINE + CENTRE_LINE - 5), "02  C-2 2 5 0");
}

#[test]
fn the_playhead_is_marked_on_the_pass_of_a_phrase_loop_that_is_playing() {
    let mut project = Project::new();
    let timing = write(&mut project, 1, 1, &[C2; 8]);
    let looping = write(&mut project, 2, 1, &[D2, E2]);
    project.phrase_mut(looping).set_loop(0, 2);
    place(&mut project, 0, 0, 0, &[ChainEntry::new(timing)]);
    place(&mut project, 0, 1, 1, &[ChainEntry::new(looping)]);
    let mut app = App::with_project(project);
    let mut player = Player::new();
    apply(&mut app, &[PLAY_SONG]);

    play_on(&mut app, &mut player, 5);

    let marked: Vec<usize> = (0..8)
        .filter(|&offset| line(&app, FIRST_LINE + offset).starts_with('>'))
        .collect();
    assert_eq!(marked, [5]);
}

#[test]
fn with_follow_off_it_stays_where_it_is_while_the_song_plays() {
    let mut app = App::with_project(long_song());
    let mut player = Player::new();
    apply(&mut app, &[PLAY_SONG]);

    play_on(&mut app, &mut player, 2 * 32 + 5);

    assert_eq!(line(&app, TITLE_LINE), "MULTITRACK  ROW 00");
    assert_eq!(line(&app, FIRST_LINE), "00  C-2 2 5 0");
}

#[test]
fn it_scrolls_continuously_through_the_song_and_its_title_shows_the_song_row_in_view() {
    let mut app = App::with_project(long_song());
    apply(&mut app, &[FOLLOW, TAB]);

    app.apply(DOWN);
    assert_eq!(status_line(&app), "FOLLOW OFF");
    apply(&mut app, &[DOWN; 39]);

    assert_eq!(line(&app, TITLE_LINE), "MULTITRACK  ROW 01");
    assert_eq!(line(&app, FIRST_LINE + CENTRE_LINE - 8), "01  C-2 2 5 0");
}

#[test]
fn muted_tracks_are_still_shown() {
    let mut app = App::with_project(long_song());

    app.apply(Action::Mute { track_number: 1 });

    assert_eq!(line(&app, FIRST_LINE), "00  C-2 2 5 0");
}

const UP: Action = Action::Move(Direction::Up);
const LEFT: Action = Action::Move(Direction::Left);
const RIGHT: Action = Action::Move(Direction::Right);
const TO_MULTITRACK: [Action; 1] = [TAB];

fn fast_beside_slow() -> (Project, PhraseId, PhraseId) {
    let mut project = Project::new();
    let fast = write(&mut project, 1, 8, &[C2, D2, E2, F2]);
    let slow = write(&mut project, 2, 16, &[C2, E2]);
    place(&mut project, 0, 0, 0, &[ChainEntry::new(fast)]);
    place(&mut project, 0, 1, 1, &[ChainEntry::new(slow)]);
    (project, fast, slow)
}

#[test]
fn the_focused_multitrack_shows_its_cursor_on_the_first_step_of_track_1() {
    let (project, _, _) = fast_beside_slow();
    let mut app = App::with_project(project);

    apply(&mut app, &TO_MULTITRACK);

    assert_eq!(line(&app, FIRST_LINE), "00 [C-2]2 5 0   C-2 2 5 0");
}

#[test]
fn moving_down_and_up_goes_to_the_next_and_previous_step_of_that_track_and_never_to_padding() {
    let (project, _, _) = fast_beside_slow();
    let mut app = App::with_project(project);
    apply(&mut app, &TO_MULTITRACK);

    apply(&mut app, &[RIGHT, DOWN]);
    assert_eq!(line(&app, FIRST_LINE + 1), "    D-2 2 5 0   |");
    assert_eq!(line(&app, FIRST_LINE + 2), "    E-2 2 5 0  [E-2]2 5 0");

    apply(&mut app, &[DOWN, UP]);
    assert_eq!(line(&app, FIRST_LINE), "00  C-2 2 5 0  [C-2]2 5 0");
}

#[test]
fn moving_left_or_right_goes_to_the_step_sounding_at_that_time_on_the_neighbouring_track() {
    let (project, _, _) = fast_beside_slow();
    let mut app = App::with_project(project);
    apply(&mut app, &TO_MULTITRACK);

    apply(&mut app, &[DOWN, DOWN, DOWN, RIGHT]);
    assert_eq!(line(&app, FIRST_LINE + 2), "    E-2 2 5 0  [E-2]2 5 0");

    app.apply(LEFT);
    assert_eq!(line(&app, FIRST_LINE + 2), "   [E-2]2 5 0   E-2 2 5 0");
}

#[test]
fn the_cursor_drives_the_song_row_the_chain_on_that_track_and_the_phrase_and_step() {
    let (mut project, fast, slow) = fast_beside_slow();
    place(
        &mut project,
        1,
        1,
        2,
        &[ChainEntry::new(fast), ChainEntry::new(slow)],
    );
    let mut app = App::with_project(project);
    apply(&mut app, &TO_MULTITRACK);

    apply(&mut app, &[RIGHT, DOWN, DOWN, DOWN, DOWN, DOWN, DOWN, DOWN]);

    assert_eq!(app.shown_chain_id(), Some(ChainId::new(2)));
    assert_eq!(app.shown_phrase_id(), Some(slow));
    app.apply(Action::FocusNext);
    app.apply(Action::FocusNext);
    assert_eq!(pane_text(&app, layout::CHAIN, 3), "01[02]    +00");
    app.apply(Action::FocusNext);
    assert_eq!(pane_text(&app, layout::SONG, 3), "01  --[02]-- --");
}

#[test]
fn moving_left_or_right_passes_over_tracks_with_no_step_sounding_and_stays_when_none_is_left() {
    let mut project = Project::new();
    let fast = write(&mut project, 1, 8, &[C2, D2]);
    let slow = write(&mut project, 2, 16, &[E2]);
    place(&mut project, 0, 0, 0, &[ChainEntry::new(fast)]);
    place(&mut project, 0, 2, 1, &[ChainEntry::new(slow)]);
    let mut app = App::with_project(project);
    apply(&mut app, &TO_MULTITRACK);

    apply(&mut app, &[DOWN, RIGHT]);
    assert_eq!(
        line(&app, FIRST_LINE),
        "00  C-2 2 5 0              [E-2]2 5 0"
    );

    app.apply(RIGHT);
    assert_eq!(
        line(&app, FIRST_LINE),
        "00  C-2 2 5 0              [E-2]2 5 0"
    );
}

#[test]
fn tab_focuses_the_phrase_pane_on_the_step_under_the_cursor() {
    let (project, _, slow) = fast_beside_slow();
    let mut app = App::with_project(project);
    apply(&mut app, &TO_MULTITRACK);

    apply(&mut app, &[RIGHT, DOWN, TAB, Action::PianoKey(0)]);

    assert_eq!(app.project().phrase(slow).step(1).pitch(), C2 as u8);
}

#[test]
fn moving_the_cursor_left_or_right_by_hand_turns_follow_off() {
    let (project, _, _) = fast_beside_slow();
    let mut app = App::with_project(project);
    apply(&mut app, &TO_MULTITRACK);
    app.apply(FOLLOW);

    app.apply(RIGHT);

    assert_eq!(status_line(&app), "FOLLOW OFF");
}
