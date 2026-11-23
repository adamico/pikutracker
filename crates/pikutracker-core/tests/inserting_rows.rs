mod support;

use pikutracker_core::app::{Action, App, Direction};
use pikutracker_core::project::{
    CHAIN_ENTRIES, ChainEntry, ChainId, MAX_STEPS, PhraseId, Project, SONG_ROWS, Step, TRACKS,
};
use support::{apply, status_line};

const DOWN: Action = Action::Move(Direction::Down);
const UP: Action = Action::Move(Direction::Up);
const SHIFT_TAB: Action = Action::FocusPrevious;

fn song_cell(app: &App, row: usize, track: usize) -> Option<u8> {
    app.project().song_cell(row, track).map(ChainId::number)
}

fn entry_phrase(app: &App, index: usize) -> Option<u8> {
    app.project()
        .chain(ChainId::new(0))
        .entry(index)
        .map(|entry| entry.phrase().number())
}

fn pitch(app: &App, index: usize) -> u8 {
    app.project().phrase(PhraseId::new(0)).step(index).pitch()
}

fn song_rows() -> Project {
    let mut project = Project::new();
    for row in 0..3 {
        project.set_song_cell(row, 0, Some(ChainId::new(0x10 + row as u8)));
        project.set_song_cell(row, 1, Some(ChainId::new(0x20 + row as u8)));
    }
    project
}

fn full_chain() -> Project {
    let mut project = Project::new();
    project.set_song_cell(0, 0, Some(ChainId::new(0)));
    for index in 0..CHAIN_ENTRIES {
        *project.chain_mut(ChainId::new(0)).entry_mut(index) =
            Some(ChainEntry::new(PhraseId::new(index as u8)));
    }
    project
}

fn short_phrase() -> Project {
    let mut project = Project::new();
    project.set_song_cell(0, 0, Some(ChainId::new(0)));
    *project.chain_mut(ChainId::new(0)).entry_mut(0) = Some(ChainEntry::new(PhraseId::new(0)));
    let phrase = project.phrase_mut(PhraseId::new(0));
    phrase.set_length(4);
    for index in 0..MAX_STEPS {
        *phrase.step_mut(index) = Step::new(index as u8, 1, 5, 0).unwrap();
    }
    project
}

#[test]
fn insert_opens_an_empty_song_row_across_every_track() {
    let mut app = App::with_project(song_rows());

    apply(&mut app, &[DOWN, Action::InsertRow]);

    assert_eq!(song_cell(&app, 0, 0), Some(0x10));
    assert_eq!(song_cell(&app, 1, 0), None);
    assert_eq!(song_cell(&app, 1, 1), None);
    assert_eq!(song_cell(&app, 2, 0), Some(0x11));
    assert_eq!(song_cell(&app, 2, 1), Some(0x21));
    assert_eq!(song_cell(&app, 3, 1), Some(0x22));
}

#[test]
fn delete_closes_a_song_row_across_every_track() {
    let mut app = App::with_project(song_rows());

    apply(&mut app, &[DOWN, Action::DeleteRow]);

    assert_eq!(song_cell(&app, 0, 1), Some(0x20));
    assert_eq!(song_cell(&app, 1, 0), Some(0x12));
    assert_eq!(song_cell(&app, 1, 1), Some(0x22));
    assert_eq!(song_cell(&app, 2, 0), None);
}

#[test]
fn the_last_song_row_falls_off_on_insert_and_comes_back_empty_on_delete() {
    let mut project = Project::new();
    project.set_song_cell(SONG_ROWS - 1, 0, Some(ChainId::new(7)));
    let mut app = App::with_project(project.clone());

    app.apply(Action::InsertRow);
    assert_eq!(song_cell(&app, SONG_ROWS - 1, 0), None);

    let mut app = App::with_project(project);

    app.apply(Action::DeleteRow);
    assert_eq!(song_cell(&app, SONG_ROWS - 2, 0), Some(7));
    assert_eq!(song_cell(&app, SONG_ROWS - 1, 0), None);
}

#[test]
fn insert_opens_an_empty_chain_entry_and_the_last_entry_falls_off() {
    let mut app = App::with_project(full_chain());

    apply(&mut app, &[SHIFT_TAB, DOWN, Action::InsertRow]);

    assert_eq!(entry_phrase(&app, 0), Some(0));
    assert_eq!(entry_phrase(&app, 1), None);
    assert_eq!(entry_phrase(&app, 2), Some(1));
    assert_eq!(entry_phrase(&app, CHAIN_ENTRIES - 1), Some(14));
}

#[test]
fn delete_closes_a_chain_entry_and_an_empty_one_appears_at_the_end() {
    let mut app = App::with_project(full_chain());

    apply(&mut app, &[SHIFT_TAB, DOWN, Action::DeleteRow]);

    assert_eq!(entry_phrase(&app, 0), Some(0));
    assert_eq!(entry_phrase(&app, 1), Some(2));
    assert_eq!(entry_phrase(&app, CHAIN_ENTRIES - 2), Some(15));
    assert_eq!(entry_phrase(&app, CHAIN_ENTRIES - 1), None);
}

#[test]
fn the_chain_pane_needs_a_chain_to_shift() {
    let mut app = App::new();

    apply(&mut app, &[SHIFT_TAB, Action::InsertRow]);

    assert!(status_line(&app).contains("NO CHAIN HERE"));
}

#[test]
fn insert_shifts_steps_within_len_and_the_last_step_of_len_falls_off() {
    let mut app = App::with_project(short_phrase());

    apply(&mut app, &[SHIFT_TAB, SHIFT_TAB, DOWN, Action::InsertRow]);

    let phrase = app.project().phrase(PhraseId::new(0));
    assert_eq!(pitch(&app, 0), 0);
    assert!(phrase.step(1) == Step::default());
    assert_eq!(pitch(&app, 2), 1);
    assert_eq!(pitch(&app, 3), 2);
    assert_eq!(pitch(&app, 4), 4, "Steps past LEN do not move");
    assert_eq!(phrase.length(), 4);
}

#[test]
fn delete_shifts_steps_within_len_and_an_empty_step_appears_at_the_end_of_len() {
    let mut app = App::with_project(short_phrase());

    apply(&mut app, &[SHIFT_TAB, SHIFT_TAB, DOWN, Action::DeleteRow]);

    let phrase = app.project().phrase(PhraseId::new(0));
    assert_eq!(pitch(&app, 1), 2);
    assert_eq!(pitch(&app, 2), 3);
    assert!(phrase.step(3) == Step::default());
    assert_eq!(pitch(&app, 4), 4, "Steps past LEN do not move");
    assert_eq!(phrase.length(), 4);
}

#[test]
fn the_phrase_header_has_no_step_to_shift() {
    let mut app = App::with_project(short_phrase());
    let before = app.project().clone();

    apply(&mut app, &[SHIFT_TAB, SHIFT_TAB, UP, Action::InsertRow]);

    assert!(app.project() == &before);
    assert!(status_line(&app).contains("NO STEP HERE"));
}

#[test]
fn the_phrase_pane_needs_a_phrase_to_shift() {
    let mut app = App::new();

    apply(&mut app, &[SHIFT_TAB, SHIFT_TAB, Action::DeleteRow]);

    assert!(status_line(&app).contains("NO PHRASE HERE"));
}

#[test]
fn each_insert_or_delete_is_one_undo_step_in_every_pane() {
    let cases = [
        (song_rows(), vec![DOWN]),
        (full_chain(), vec![SHIFT_TAB, DOWN]),
        (short_phrase(), vec![SHIFT_TAB, SHIFT_TAB, DOWN]),
    ];
    for (project, moves) in cases {
        for action in [Action::InsertRow, Action::DeleteRow] {
            let mut app = App::with_project(project.clone());
            apply(&mut app, &moves);

            app.apply(action);
            let after = app.project().clone();
            assert!(after != project);

            app.apply(Action::Undo);
            assert!(app.project() == &project);
            app.apply(Action::Redo);
            assert!(app.project() == &after);
        }
    }
}

#[test]
fn a_shift_moves_every_track_wherever_the_cursor_is() {
    let mut project = Project::new();
    project.set_song_cell(0, TRACKS - 1, Some(ChainId::new(3)));
    let mut app = App::with_project(project);

    app.apply(Action::InsertRow);

    assert_eq!(song_cell(&app, 1, TRACKS - 1), Some(3));
}
