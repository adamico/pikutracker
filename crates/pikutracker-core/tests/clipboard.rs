mod support;

use pikutracker_core::app::{Action, App, Direction, DirectoryEntry, FileRequest, FileResponse};
use pikutracker_core::project::{ChainEntry, ChainId, PhraseId, Project, Step};
use pikutracker_core::project_file;
use std::path::PathBuf;
use support::{apply, status_line};

const UP: Action = Action::Move(Direction::Up);
const DOWN: Action = Action::Move(Direction::Down);
const RIGHT: Action = Action::Move(Direction::Right);
const TAB: Action = Action::FocusNext;
const BACK: Action = Action::FocusPrevious;
const SELECT: Action = Action::StartSelection;

fn song_cell(app: &App, row: usize, track: usize) -> Option<u8> {
    app.project().song_cell(row, track).map(ChainId::number)
}

fn step(app: &App, index: usize) -> Step {
    app.project().phrase(PhraseId::new(0)).step(index)
}

fn entry(app: &App, index: usize) -> Option<ChainEntry> {
    app.project().chain(ChainId::new(0)).entry(index)
}

fn moves(direction: Action, count: usize) -> Vec<Action> {
    vec![direction; count]
}

fn song_block() -> Project {
    let mut project = Project::new();
    for (row, track, chain) in [(0, 0, 0x10), (1, 0, 0x20), (0, 1, 0x30), (1, 1, 0x40)] {
        project.set_song_cell(row, track, Some(ChainId::new(chain)));
    }
    project
}

fn chain_music() -> Project {
    let mut project = Project::new();
    let chain = ChainId::new(0);
    project.set_song_cell(0, 0, Some(chain));
    let mut first = ChainEntry::new(PhraseId::new(5));
    first.set_transpose(3);
    *project.chain_mut(chain).entry_mut(0) = Some(first);
    *project.chain_mut(chain).entry_mut(1) = Some(ChainEntry::new(PhraseId::new(6)));
    project
}

fn phrase_music() -> Project {
    let mut project = Project::new();
    let chain = ChainId::new(0);
    project.set_song_cell(0, 0, Some(chain));
    *project.chain_mut(chain).entry_mut(0) = Some(ChainEntry::new(PhraseId::new(0)));
    let phrase = project.phrase_mut(PhraseId::new(0));
    phrase.set_length(8);
    for index in 0..4 {
        *phrase.step_mut(index) = Step::new(24 + index as u8, 2, 5, 1).unwrap();
    }
    project
}

#[test]
fn a_song_block_pastes_from_the_cursor() {
    let mut app = App::with_project(song_block());
    apply(&mut app, &[SELECT, DOWN, RIGHT, Action::Copy]);
    apply(&mut app, &moves(DOWN, 3));

    app.apply(Action::Paste);

    assert_eq!(song_cell(&app, 4, 1), Some(0x10));
    assert_eq!(song_cell(&app, 5, 1), Some(0x20));
    assert_eq!(song_cell(&app, 4, 2), Some(0x30));
    assert_eq!(song_cell(&app, 5, 2), Some(0x40));
}

#[test]
fn a_whole_song_block_keeps_its_shape_and_its_references() {
    let mut app = App::with_project(song_block());
    apply(
        &mut app,
        &[
            SELECT,
            DOWN,
            RIGHT,
            Action::Copy,
            UP,
            Action::Move(Direction::Left),
        ],
    );
    apply(&mut app, &moves(DOWN, 5));

    app.apply(Action::Paste);

    assert_eq!(song_cell(&app, 5, 0), Some(0x10));
    assert_eq!(song_cell(&app, 6, 0), Some(0x20));
    assert_eq!(song_cell(&app, 5, 1), Some(0x30));
    assert_eq!(song_cell(&app, 6, 1), Some(0x40));
}

#[test]
fn copy_without_a_selection_takes_the_cell_under_the_cursor() {
    let mut app = App::with_project(song_block());
    apply(&mut app, &[Action::Copy, DOWN, DOWN, Action::Paste]);

    assert_eq!(song_cell(&app, 2, 0), Some(0x10));
    assert_eq!(song_cell(&app, 3, 0), None);
}

#[test]
fn paste_overwrites_with_empty_cells_too() {
    let mut app = App::with_project(song_block());
    apply(&mut app, &[DOWN, DOWN, Action::Copy, UP, UP, Action::Paste]);

    assert_eq!(song_cell(&app, 0, 0), None);
}

#[test]
fn paste_is_one_undo_step() {
    let start = song_block();
    let mut app = App::with_project(start.clone());
    apply(&mut app, &[SELECT, DOWN, RIGHT, Action::Copy]);
    apply(&mut app, &moves(DOWN, 4));
    app.apply(Action::Paste);

    app.apply(Action::Undo);

    assert!(*app.project() == start);
}

#[test]
fn cut_copies_and_clears_without_shifting_as_one_step() {
    let start = song_block();
    let mut app = App::with_project(start.clone());
    apply(&mut app, &[SELECT, Action::Cut]);

    assert_eq!(song_cell(&app, 0, 0), None);
    assert_eq!(song_cell(&app, 1, 0), Some(0x20));

    apply(&mut app, &moves(DOWN, 3));
    app.apply(Action::Paste);
    assert_eq!(song_cell(&app, 3, 0), Some(0x10));

    apply(&mut app, &[Action::Undo, Action::Undo]);
    assert!(*app.project() == start);
}

#[test]
fn copy_changes_no_music_and_is_not_an_undo_step() {
    let mut app = App::with_project(song_block());
    apply(&mut app, &[SELECT, DOWN, Action::Copy, Action::Undo]);

    assert_eq!(status_line(&app), "NOTHING TO UNDO");
}

#[test]
fn a_song_paste_past_the_last_row_reports_the_rows_lost() {
    let mut app = App::with_project(song_block());
    apply(&mut app, &[SELECT, DOWN, Action::Copy]);
    apply(&mut app, &moves(DOWN, 254));

    app.apply(Action::Paste);

    assert_eq!(song_cell(&app, 255, 0), Some(0x10));
    assert_eq!(status_line(&app), "! PASTE LOST 1 ROW");
}

#[test]
fn a_song_paste_past_the_last_track_counts_every_clipped_row() {
    let mut app = App::with_project(song_block());
    apply(&mut app, &[SELECT, DOWN, RIGHT, Action::Copy, UP]);
    apply(&mut app, &moves(RIGHT, 6));

    app.apply(Action::Paste);

    assert_eq!(song_cell(&app, 0, 7), Some(0x10));
    assert_eq!(status_line(&app), "! PASTE LOST 2 ROWS");
}

#[test]
fn chain_entries_paste_with_their_transpose() {
    let mut app = App::with_project(chain_music());
    apply(&mut app, &[TAB, SELECT, RIGHT, RIGHT, Action::Copy]);
    apply(&mut app, &moves(DOWN, 4));

    app.apply(Action::Paste);

    assert_eq!(entry(&app, 4), entry(&app, 0));
}

#[test]
fn a_partial_chain_copy_lands_in_its_own_columns() {
    let mut app = App::with_project(chain_music());
    apply(&mut app, &[TAB, RIGHT, Action::Copy, DOWN]);
    app.apply(Action::Move(Direction::Left));

    app.apply(Action::Paste);

    let pasted = entry(&app, 1).unwrap();
    assert_eq!(pasted.transpose(), 3);
    assert_eq!(pasted.phrase(), PhraseId::new(6));
}

#[test]
fn a_chain_paste_past_the_last_entry_reports_the_rows_lost() {
    let mut app = App::with_project(chain_music());
    apply(&mut app, &[TAB, SELECT, DOWN, Action::Copy]);
    apply(&mut app, &moves(DOWN, 14));

    app.apply(Action::Paste);

    assert_eq!(status_line(&app), "! PASTE LOST 1 ROW");
}

#[test]
fn steps_paste_as_values() {
    let mut app = App::with_project(phrase_music());
    apply(
        &mut app,
        &[TAB, TAB, SELECT, DOWN, RIGHT, RIGHT, RIGHT, Action::Copy],
    );
    apply(&mut app, &moves(DOWN, 3));

    app.apply(Action::Paste);

    assert_eq!(step(&app, 4), step(&app, 0));
    assert_eq!(step(&app, 5), step(&app, 1));
}

#[test]
fn a_partial_phrase_copy_lands_in_its_own_columns_wherever_the_cursor_is() {
    let mut project = phrase_music();
    *project.phrase_mut(PhraseId::new(0)).step_mut(5) = Step::new(40, 3, 2, 4).unwrap();
    let mut app = App::with_project(project);
    apply(&mut app, &[TAB, TAB, RIGHT, RIGHT, Action::Copy]);
    apply(&mut app, &moves(Action::Move(Direction::Left), 2));
    apply(&mut app, &moves(DOWN, 5));

    app.apply(Action::Paste);

    assert_eq!(step(&app, 5), Step::new(40, 3, 5, 4).unwrap());
}

#[test]
fn a_silent_step_pasted_over_a_note_silences_it() {
    let mut project = phrase_music();
    *project.phrase_mut(PhraseId::new(0)).step_mut(6) = Step::new(30, 1, 0, 0).unwrap();
    let mut app = App::with_project(project);
    apply(&mut app, &[TAB, TAB]);
    apply(&mut app, &moves(DOWN, 6));
    apply(&mut app, &[Action::Copy]);
    apply(&mut app, &moves(UP, 6));

    app.apply(Action::Paste);

    assert_eq!(step(&app, 0), Step::new(30, 2, 0, 1).unwrap());
}

#[test]
fn a_phrase_paste_stops_at_len_and_never_changes_it() {
    let mut app = App::with_project(phrase_music());
    apply(&mut app, &[TAB, TAB, SELECT]);
    apply(&mut app, &moves(DOWN, 3));
    app.apply(Action::Copy);
    apply(&mut app, &moves(DOWN, 3));

    app.apply(Action::Paste);

    let phrase = app.project().phrase(PhraseId::new(0));
    assert_eq!(phrase.length(), 8);
    assert_eq!(phrase.step(6).pitch(), 24);
    assert_eq!(phrase.step(7).pitch(), 25);
    assert_eq!(phrase.step(8), Step::default());
    assert_eq!(status_line(&app), "! PASTE LOST 2 ROWS");
}

#[test]
fn paste_into_another_kind_of_pane_is_refused_with_a_message() {
    let start = song_block();
    let mut app = App::with_project(start.clone());
    apply(&mut app, &[Action::Copy, TAB]);

    app.apply(Action::Paste);

    assert_eq!(
        status_line(&app),
        "! CANNOT PASTE SONG CELLS INTO THE CHAIN PANE"
    );
    assert!(*app.project() == start);
}

#[test]
fn paste_with_an_empty_clipboard_says_so() {
    let mut app = App::with_project(song_block());

    app.apply(Action::Paste);

    assert_eq!(status_line(&app), "NOTHING TO PASTE");
}

#[test]
fn the_clipboard_lasts_across_focus_changes_and_undo() {
    let mut app = App::with_project(song_block());
    apply(&mut app, &[Action::Copy, TAB, BACK, DOWN, DOWN]);
    app.apply(Action::HexDigit(1));
    app.apply(Action::Undo);

    app.apply(Action::Paste);

    assert_eq!(song_cell(&app, 2, 0), Some(0x10));
}

#[test]
fn the_clipboard_is_emptied_when_another_project_opens() {
    let mut app = App::with_project(song_block());
    app.apply(Action::Copy);

    app.apply(Action::OpenProject);
    app.take_file_requests();
    app.receive_file(FileResponse::Listed {
        directory: PathBuf::from("."),
        entries: vec![DirectoryEntry {
            name: "other.ptrk".to_owned(),
            is_directory: false,
        }],
    });
    apply(&mut app, &[DOWN, Action::Confirm]);
    let Some(FileRequest::Read { path }) = app.take_file_requests().pop() else {
        panic!("expected a read");
    };
    app.receive_file(FileResponse::Read {
        path,
        contents: project_file::write(&song_block()),
    });
    app.apply(Action::Paste);

    assert_eq!(status_line(&app), "NOTHING TO PASTE");
}

#[test]
fn the_clipboard_is_emptied_by_a_new_project() {
    let mut app = App::new();
    apply(&mut app, &[Action::Copy, Action::NewProject, Action::Paste]);

    assert_eq!(status_line(&app), "NOTHING TO PASTE");
}
