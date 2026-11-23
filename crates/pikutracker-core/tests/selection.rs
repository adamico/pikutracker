mod support;

use pikutracker_core::app::{Action, App, Direction, layout};
use pikutracker_core::project::{ChainEntry, ChainId, PhraseId, Project, Step};
use pikutracker_core::screen::palette::Colour;
use support::apply;

const UP: Action = Action::Move(Direction::Up);
const DOWN: Action = Action::Move(Direction::Down);
const RIGHT: Action = Action::Move(Direction::Right);
const TAB: Action = Action::FocusNext;
const SELECT: Action = Action::StartSelection;
const SELECT_UP: Action = Action::Select(Direction::Up);
const SELECT_DOWN: Action = Action::Select(Direction::Down);
const SELECT_LEFT: Action = Action::Select(Direction::Left);
const SELECT_RIGHT: Action = Action::Select(Direction::Right);
const NUDGE_UP: Action = Action::Nudge {
    up: true,
    coarse: false,
};
const NUDGE_DOWN: Action = Action::Nudge {
    up: false,
    coarse: false,
};
const OCTAVE_UP: Action = Action::Nudge {
    up: true,
    coarse: true,
};

const SELECTED: Colour = Colour::Lavender;
const SONG_FIRST_LINE: usize = layout::SONG.row + 2;
const SONG_FIRST_COLUMN: usize = layout::SONG.column + 4;
const SONG_TRACK_WIDTH: usize = 3;

fn song_cell(app: &App, row: usize, track: usize) -> Option<u8> {
    app.project().song_cell(row, track).map(ChainId::number)
}

fn step(app: &App, index: usize) -> Step {
    app.project().phrase(PhraseId::new(0)).step(index)
}

fn entry(app: &App, index: usize) -> Option<ChainEntry> {
    app.project().chain(ChainId::new(0)).entry(index)
}

fn song_cell_background(app: &App, row: usize, track: usize) -> Colour {
    let column = SONG_FIRST_COLUMN + track * SONG_TRACK_WIDTH;
    app.grid().cell(column, SONG_FIRST_LINE + row).style.bg
}

fn song_block() -> Project {
    let mut project = Project::new();
    for (row, track, chain) in [(0, 0, 0x10), (1, 0, 0x20), (0, 1, 0x30), (1, 1, 0x40)] {
        project.set_song_cell(row, track, Some(ChainId::new(chain)));
    }
    project
}

fn music() -> Project {
    let mut project = Project::new();
    let chain = ChainId::new(0);
    project.set_song_cell(0, 0, Some(chain));
    for index in 0..3 {
        *project.chain_mut(chain).entry_mut(index) = Some(ChainEntry::new(PhraseId::new(0)));
    }
    let phrase = project.phrase_mut(PhraseId::new(0));
    for index in 0..4 {
        *phrase.step_mut(index) = Step::new(24 + index as u8, 2, 5, 1).unwrap();
    }
    project
}

#[test]
fn nudging_a_selection_raises_every_cell_in_it() {
    let mut app = App::with_project(song_block());
    apply(&mut app, &[SELECT_DOWN, SELECT_RIGHT, NUDGE_UP]);

    assert_eq!(song_cell(&app, 0, 0), Some(0x11));
    assert_eq!(song_cell(&app, 1, 0), Some(0x21));
    assert_eq!(song_cell(&app, 0, 1), Some(0x31));
    assert_eq!(song_cell(&app, 1, 1), Some(0x41));
}

#[test]
fn a_selection_can_be_dragged_up_and_left_of_where_it_started() {
    let mut app = App::with_project(song_block());
    apply(&mut app, &[DOWN, RIGHT, SELECT_UP, SELECT_LEFT, NUDGE_UP]);

    assert_eq!(song_cell(&app, 0, 0), Some(0x11));
    assert_eq!(song_cell(&app, 1, 1), Some(0x41));
}

#[test]
fn nudging_a_selection_leaves_empty_song_cells_empty() {
    let mut app = App::with_project(song_block());
    apply(&mut app, &[SELECT_DOWN, SELECT_DOWN, NUDGE_UP]);

    assert_eq!(song_cell(&app, 2, 0), None);
}

#[test]
fn nudging_a_selection_is_one_undo_step() {
    let start = song_block();
    let mut app = App::with_project(start.clone());
    apply(&mut app, &[SELECT_DOWN, SELECT_RIGHT, NUDGE_UP]);

    app.apply(Action::Undo);

    assert!(*app.project() == start);
}

#[test]
fn each_nudge_of_a_selection_is_its_own_undo_step() {
    let mut app = App::with_project(song_block());
    apply(&mut app, &[SELECT_DOWN, NUDGE_UP, NUDGE_UP]);

    app.apply(Action::Undo);

    assert_eq!(song_cell(&app, 0, 0), Some(0x11));
}

#[test]
fn nudging_clamps_each_value_at_its_own_limit() {
    let mut project = music();
    *project.phrase_mut(PhraseId::new(0)).step_mut(1) = Step::new(63, 2, 7, 7).unwrap();
    let mut app = App::with_project(project);
    apply(
        &mut app,
        &[
            TAB,
            TAB,
            SELECT_DOWN,
            SELECT_RIGHT,
            SELECT_RIGHT,
            SELECT_RIGHT,
        ],
    );

    app.apply(NUDGE_UP);

    assert_eq!(step(&app, 0), Step::new(25, 3, 6, 2).unwrap());
    assert_eq!(step(&app, 1), Step::new(63, 3, 7, 7).unwrap());
}

#[test]
fn a_coarse_nudge_moves_pitches_by_an_octave() {
    let mut app = App::with_project(music());
    apply(&mut app, &[TAB, TAB, SELECT_DOWN, OCTAVE_UP]);

    assert_eq!(step(&app, 0).pitch(), 36);
    assert_eq!(step(&app, 1).pitch(), 37);
}

#[test]
fn nudging_a_selection_leaves_silent_steps_silent() {
    let mut project = music();
    *project.phrase_mut(PhraseId::new(0)).step_mut(1) = Step::new(30, 2, 0, 0).unwrap();
    let mut app = App::with_project(project);
    apply(&mut app, &[TAB, TAB, RIGHT, RIGHT, SELECT_DOWN, NUDGE_UP]);

    assert_eq!(step(&app, 0).volume(), 6);
    assert_eq!(step(&app, 1), Step::new(30, 2, 0, 0).unwrap());
}

#[test]
fn nudging_a_selection_keeps_custom_instruments() {
    let mut project = music();
    *project.phrase_mut(PhraseId::new(0)).step_mut(0) = Step::new(24, 0x0A, 5, 0).unwrap();
    let mut app = App::with_project(project);
    apply(&mut app, &[TAB, TAB, RIGHT, SELECT_DOWN, NUDGE_UP]);

    assert_eq!(step(&app, 0).instrument(), 0x0A);
    assert_eq!(step(&app, 1).instrument(), 3);
}

#[test]
fn nudging_a_selection_stops_built_in_instruments_at_07() {
    let mut project = music();
    *project.phrase_mut(PhraseId::new(0)).step_mut(0) = Step::new(24, 0x07, 5, 0).unwrap();
    let mut app = App::with_project(project);
    apply(&mut app, &[TAB, TAB, RIGHT, SELECT_DOWN, NUDGE_UP]);

    assert_eq!(step(&app, 0).instrument(), 0x07);
}

#[test]
fn nudging_chain_entries_changes_transposes_and_skips_empty_entries() {
    let mut app = App::with_project(music());
    apply(&mut app, &[TAB, RIGHT, DOWN, DOWN, SELECT_DOWN, NUDGE_DOWN]);

    assert_eq!(entry(&app, 2).unwrap().transpose(), -1);
    assert_eq!(entry(&app, 3), None);
}

#[test]
fn clear_empties_every_selected_song_cell_as_one_step() {
    let start = song_block();
    let mut app = App::with_project(start.clone());
    apply(&mut app, &[SELECT_DOWN, SELECT_RIGHT, Action::Clear]);

    for (row, track) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        assert_eq!(song_cell(&app, row, track), None);
    }

    app.apply(Action::Undo);
    assert!(*app.project() == start);
}

#[test]
fn clear_on_selected_chain_entries_empties_them() {
    let mut app = App::with_project(music());
    apply(&mut app, &[TAB, SELECT_DOWN, Action::Clear]);

    assert_eq!(entry(&app, 0), None);
    assert_eq!(entry(&app, 1), None);
    assert!(entry(&app, 2).is_some());
}

#[test]
fn clear_on_whole_selected_steps_silences_them_and_keeps_their_notes() {
    let mut app = App::with_project(music());
    apply(
        &mut app,
        &[
            TAB,
            TAB,
            SELECT_DOWN,
            SELECT_RIGHT,
            SELECT_RIGHT,
            SELECT_RIGHT,
            Action::Clear,
        ],
    );

    assert_eq!(step(&app, 0), Step::new(24, 2, 0, 0).unwrap());
    assert_eq!(step(&app, 1), Step::new(25, 2, 0, 0).unwrap());
    assert_eq!(step(&app, 2).volume(), 5);
}

#[test]
fn clear_on_the_effect_column_keeps_the_note_sounding() {
    let mut app = App::with_project(music());
    apply(
        &mut app,
        &[TAB, TAB, RIGHT, RIGHT, RIGHT, SELECT_DOWN, Action::Clear],
    );

    assert_eq!(step(&app, 0), Step::new(24, 2, 5, 0).unwrap());
}

#[test]
fn clear_without_a_selection_empties_the_step_cell_under_the_cursor() {
    let mut app = App::with_project(music());
    apply(&mut app, &[TAB, TAB, Action::Clear]);

    assert_eq!(step(&app, 0), Step::new(24, 2, 0, 1).unwrap());
    assert_eq!(step(&app, 1).volume(), 5);
}

#[test]
fn the_selection_is_drawn_on_the_grid() {
    let mut app = App::with_project(song_block());
    apply(&mut app, &[SELECT_DOWN, SELECT_RIGHT]);

    assert_eq!(song_cell_background(&app, 0, 0), SELECTED);
    assert_eq!(song_cell_background(&app, 1, 0), SELECTED);
    assert_ne!(song_cell_background(&app, 2, 0), SELECTED);
    assert_ne!(song_cell_background(&app, 0, 2), SELECTED);
}

#[test]
fn escape_cancels_the_selection() {
    let mut app = App::with_project(song_block());
    apply(&mut app, &[SELECT_DOWN, Action::Cancel, NUDGE_UP]);

    assert_ne!(song_cell_background(&app, 0, 0), SELECTED);
    assert_eq!(song_cell(&app, 0, 0), Some(0x10));
    assert_eq!(song_cell(&app, 1, 0), Some(0x21));
}

#[test]
fn escape_cancels_the_selection_before_it_dismisses_a_lasting_message() {
    let mut app = App::with_project(song_block());
    app.report(pikutracker_core::app::Report::failure("CANNOT SAVE"));
    app.apply(SELECT);

    app.apply(Action::Cancel);

    let status = app.grid().text_dump().lines().last().unwrap().to_owned();
    assert!(status.contains("CANNOT SAVE"), "{status}");
    assert_ne!(song_cell_background(&app, 0, 0), SELECTED);
}

#[test]
fn changing_focus_drops_the_selection() {
    let mut app = App::with_project(song_block());
    apply(&mut app, &[SELECT_DOWN, TAB, Action::FocusPrevious]);

    assert_ne!(song_cell_background(&app, 0, 0), SELECTED);
    app.apply(NUDGE_UP);
    assert_eq!(song_cell(&app, 0, 0), Some(0x10));
}

#[test]
fn undo_drops_the_selection() {
    let mut app = App::with_project(song_block());
    apply(&mut app, &[SELECT_DOWN, NUDGE_UP, Action::Undo]);

    assert_ne!(song_cell_background(&app, 0, 0), SELECTED);
}

#[test]
fn a_selection_cannot_reach_the_phrase_header() {
    let mut app = App::with_project(music());
    apply(
        &mut app,
        &[TAB, TAB, DOWN, SELECT_UP, SELECT_UP, SELECT_UP, NUDGE_UP],
    );

    assert_eq!(app.project().phrase(PhraseId::new(0)).speed(), 16);
    assert_eq!(step(&app, 0).pitch(), 25);
    assert_eq!(step(&app, 1).pitch(), 26);
}

#[test]
fn a_selection_cannot_start_in_the_phrase_header() {
    let mut app = App::with_project(music());
    apply(&mut app, &[TAB, TAB, UP, SELECT_DOWN, NUDGE_UP]);

    assert_eq!(step(&app, 0).pitch(), 25);
    assert_eq!(step(&app, 1).pitch(), 25);
}

#[test]
fn a_plain_move_drops_the_selection() {
    let mut app = App::with_project(song_block());
    apply(&mut app, &[SELECT_RIGHT, DOWN, NUDGE_UP]);

    assert_ne!(song_cell_background(&app, 0, 0), SELECTED);
    assert_eq!(song_cell(&app, 0, 0), Some(0x10));
    assert_eq!(song_cell(&app, 0, 1), Some(0x30));
    assert_eq!(song_cell(&app, 1, 1), Some(0x41));
}

#[test]
fn a_jump_drops_the_selection() {
    let mut app = App::with_project(song_block());
    apply(
        &mut app,
        &[SELECT_DOWN, Action::Jump(Direction::Down), NUDGE_UP],
    );

    assert_ne!(song_cell_background(&app, 0, 0), SELECTED);
    assert_eq!(song_cell(&app, 0, 0), Some(0x10));
    assert_eq!(song_cell(&app, 1, 0), Some(0x20));
}

#[test]
fn selecting_by_a_jump_takes_in_sixteen_more_song_rows() {
    let mut project = Project::new();
    for row in [0, 16, 17] {
        project.set_song_cell(row, 0, Some(ChainId::new(0x10)));
    }
    let mut app = App::with_project(project);
    apply(&mut app, &[Action::SelectJump(Direction::Down), NUDGE_UP]);

    assert_eq!(song_cell(&app, 0, 0), Some(0x11));
    assert_eq!(song_cell(&app, 16, 0), Some(0x11));
    assert_eq!(song_cell(&app, 17, 0), Some(0x10));
}

#[test]
fn selecting_again_grows_the_selection_from_where_it_started() {
    let mut app = App::with_project(song_block());
    apply(&mut app, &[SELECT_DOWN, SELECT_RIGHT, SELECT_UP, NUDGE_UP]);

    assert_eq!(song_cell(&app, 0, 0), Some(0x11));
    assert_eq!(song_cell(&app, 0, 1), Some(0x31));
    assert_eq!(song_cell(&app, 1, 0), Some(0x20));
    assert_eq!(song_cell(&app, 1, 1), Some(0x40));
}
