use pikutracker_core::app::{Action, App, layout};
use pikutracker_core::player::Player;
use pikutracker_core::project::{ChainEntry, ChainId, PhraseId, Project};
use pikutracker_core::screen::grid::{CELL_WIDTH, ProgressBar};
use pikutracker_core::screen::palette::Colour;
use pikutracker_core::synth::SAMPLES_PER_TICK;

const PLAY_SONG: Action = Action::PlaySongFromCursor;
const SPACE: Action = Action::PlayFocusedPane;

const SPEED: i32 = 8;
const STEP_SAMPLES: usize = SPEED as usize * SAMPLES_PER_TICK as usize;

const FIRST_SONG_ROW: usize = layout::SONG.row + 2;
const FIRST_TRACK_COLUMN: usize = layout::SONG.column + 4;
const TRACK_WIDTH: usize = 3;
const BAR_PIXELS: usize = 2 * CELL_WIDTH;

const SHORT: usize = 0;
const LONG: usize = 1;
const LOOPING: usize = 2;
const EMPTY_ENTRY: usize = 3;

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
    let short = write(&mut project, 1, 2);
    let long = write(&mut project, 2, 4);
    let looping = write(&mut project, 3, 2);
    project.phrase_mut(looping).set_loop(0, 2);
    *project.chain_mut(ChainId::new(0)).entry_mut(0) = Some(ChainEntry::new(short));
    *project.chain_mut(ChainId::new(1)).entry_mut(0) = Some(ChainEntry::new(long));
    *project.chain_mut(ChainId::new(2)).entry_mut(0) = Some(ChainEntry::new(looping));
    *project.chain_mut(ChainId::new(3)).entry_mut(1) = Some(ChainEntry::new(short));
    for track in 0..4 {
        project.set_song_cell(0, track, Some(ChainId::new(track as u8)));
    }
    project.set_song_cell(1, 0, Some(ChainId::new(0)));
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

fn bar(app: &App, song_row: usize, track: usize) -> Option<(Colour, usize)> {
    let grid = app.grid();
    let column = FIRST_TRACK_COLUMN + track * TRACK_WIDTH;
    let cells = [column, column + 1].map(|x| grid.cell(x, FIRST_SONG_ROW + song_row).bar);
    let colour = cells.iter().flatten().map(|bar| bar.colour).next()?;
    let pixels = cells
        .iter()
        .flatten()
        .map(|ProgressBar { pixels, .. }| usize::from(*pixels))
        .sum();
    Some((colour, pixels))
}

fn playing(steps_played: usize, length: usize) -> Option<(Colour, usize)> {
    Some((Colour::DarkGreen, BAR_PIXELS * steps_played / length))
}

const WAITING: Option<(Colour, usize)> = Some((Colour::DarkGrey, BAR_PIXELS));

fn chain_number(app: &App, song_row: usize, track: usize) -> String {
    let column = FIRST_TRACK_COLUMN + track * TRACK_WIDTH;
    let line = app
        .grid()
        .text_dump()
        .lines()
        .nth(FIRST_SONG_ROW + song_row)
        .unwrap()
        .to_owned();
    line.chars().skip(column).take(2).collect()
}

#[test]
fn bars_fill_with_the_steps_played_at_each_phrases_own_rate() {
    let (app, _) = played(&[PLAY_SONG], 0);

    assert_eq!(bar(&app, 0, SHORT), playing(1, 2));
    assert_eq!(bar(&app, 0, LONG), playing(1, 4));
}

#[test]
fn a_track_whose_phrase_has_ended_keeps_a_full_dimmed_bar_while_it_waits() {
    let (app, _) = played(&[PLAY_SONG], 2);

    assert_eq!(bar(&app, 0, SHORT), WAITING);
    assert_eq!(bar(&app, 0, LONG), playing(3, 4));
}

#[test]
fn a_looping_phrase_bar_starts_again_on_each_pass() {
    let (mut app, mut player) = played(&[PLAY_SONG], 1);
    assert_eq!(bar(&app, 0, LOOPING), playing(2, 2));

    play_on(&mut app, &mut player, 1);

    assert_eq!(bar(&app, 0, LOOPING), playing(1, 2));
}

#[test]
fn an_empty_entry_shows_no_bar() {
    let (app, _) = played(&[PLAY_SONG], 1);

    assert_eq!(bar(&app, 0, EMPTY_ENTRY), None);
}

#[test]
fn a_muted_track_shows_its_bar_the_same_way() {
    let (app, _) = played(&[Action::Mute { track_number: 2 }, PLAY_SONG], 0);

    assert_eq!(bar(&app, 0, LONG), playing(1, 4));
}

#[test]
fn only_the_song_row_at_the_playhead_shows_bars() {
    let (app, _) = played(&[PLAY_SONG], 1);

    assert_eq!(bar(&app, 1, SHORT), None);
}

#[test]
fn the_bars_go_away_when_playback_stops() {
    let (mut app, mut player) = played(&[PLAY_SONG], 1);

    app.apply(SPACE);
    play_on(&mut app, &mut player, 0);

    assert_eq!(bar(&app, 0, SHORT), None);
    assert_eq!(bar(&app, 0, LONG), None);
}

#[test]
fn the_chain_number_stays_readable_under_the_bar() {
    let (app, _) = played(&[PLAY_SONG], 1);

    assert_eq!(chain_number(&app, 0, LONG), "01");
    let column = FIRST_TRACK_COLUMN + LONG * TRACK_WIDTH;
    let style = app.grid().cell(column, FIRST_SONG_ROW).style;
    assert_eq!(style.fg, Colour::White);
}
