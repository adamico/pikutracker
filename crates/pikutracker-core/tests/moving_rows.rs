mod support;

use pikutracker_core::app::{Action, App, Direction, Launch, SettingsContents};
use pikutracker_core::keymap::{Key, KeyPress, Modifiers};
use pikutracker_core::project::{ChainEntry, ChainId, PhraseId, Project, SONG_ROWS, Step};
use std::path::PathBuf;
use std::time::Duration;
use support::apply;

const UP: Action = Action::Move(Direction::Up);
const DOWN: Action = Action::Move(Direction::Down);
const RIGHT: Action = Action::Move(Direction::Right);
const SHIFT_TAB: Action = Action::FocusPrevious;
const SELECT: Action = Action::StartSelection;
const SELECT_DOWN: Action = Action::Select(Direction::Down);
const ROWS_UP: Action = Action::MoveRows { up: true };
const ROWS_DOWN: Action = Action::MoveRows { up: false };

fn song_column(app: &App, track: usize) -> Vec<Option<u8>> {
    (0..5)
        .map(|row| app.project().song_cell(row, track).map(ChainId::number))
        .collect()
}

fn song() -> Project {
    let mut project = Project::new();
    for row in 0..5 {
        project.set_song_cell(row, 0, Some(ChainId::new(0x10 + row as u8)));
        project.set_song_cell(row, 1, Some(ChainId::new(0x20 + row as u8)));
    }
    project
}

#[test]
fn selected_song_rows_move_down_and_the_row_below_takes_their_place() {
    let mut app = App::with_project(song());

    apply(&mut app, &[DOWN, SELECT_DOWN, ROWS_DOWN]);

    assert_eq!(
        song_column(&app, 0),
        [Some(0x10), Some(0x13), Some(0x11), Some(0x12), Some(0x14)]
    );
}

fn entry_phrases(app: &App) -> Vec<Option<u8>> {
    (0..5)
        .map(|index| {
            app.project()
                .chain(ChainId::new(0))
                .entry(index)
                .map(|entry| entry.phrase().number())
        })
        .collect()
}

fn pitches(app: &App) -> Vec<u8> {
    let phrase = app.project().phrase(PhraseId::new(0));
    (0..6).map(|index| phrase.step(index).pitch()).collect()
}

fn chain() -> Project {
    let mut project = Project::new();
    project.set_song_cell(0, 0, Some(ChainId::new(0)));
    for index in 0..5 {
        *project.chain_mut(ChainId::new(0)).entry_mut(index) =
            Some(ChainEntry::new(PhraseId::new(index as u8)));
    }
    project
}

fn phrase_of_length_4() -> Project {
    let mut project = chain();
    let phrase = project.phrase_mut(PhraseId::new(0));
    phrase.set_length(4);
    for index in 0..6 {
        *phrase.step_mut(index) = Step::new(index as u8 + 10, 1, 5, 0).unwrap();
    }
    project
}

fn last_song_row_filled() -> Project {
    let mut project = Project::new();
    project.set_song_cell(SONG_ROWS - 1, 0, Some(ChainId::new(7)));
    project
}

#[test]
fn the_selection_and_cursor_move_with_the_rows_so_each_press_moves_them_again() {
    let mut app = App::with_project(song());

    apply(&mut app, &[SELECT_DOWN, ROWS_DOWN, ROWS_DOWN]);

    assert_eq!(
        song_column(&app, 0),
        [Some(0x12), Some(0x13), Some(0x10), Some(0x11), Some(0x14)]
    );
    apply(&mut app, &[ROWS_UP]);
    assert_eq!(
        song_column(&app, 0),
        [Some(0x12), Some(0x10), Some(0x11), Some(0x13), Some(0x14)]
    );
}

#[test]
fn in_the_song_pane_only_the_selected_tracks_cells_move() {
    let mut app = App::with_project(song());

    apply(&mut app, &[DOWN, DOWN, RIGHT, SELECT, ROWS_UP]);

    assert_eq!(
        song_column(&app, 1),
        [Some(0x20), Some(0x22), Some(0x21), Some(0x23), Some(0x24)]
    );
    assert_eq!(
        song_column(&app, 0),
        [Some(0x10), Some(0x11), Some(0x12), Some(0x13), Some(0x14)]
    );
}

#[test]
fn selected_chain_entries_move_up_and_down() {
    let mut app = App::with_project(chain());

    apply(&mut app, &[SHIFT_TAB, DOWN, DOWN, SELECT_DOWN, ROWS_UP]);
    assert_eq!(
        entry_phrases(&app),
        [Some(0), Some(2), Some(3), Some(1), Some(4)]
    );

    apply(&mut app, &[ROWS_DOWN, ROWS_DOWN]);
    assert_eq!(
        entry_phrases(&app),
        [Some(0), Some(1), Some(4), Some(2), Some(3)]
    );
}

#[test]
fn selected_steps_move_up_and_down_within_len() {
    let mut app = App::with_project(phrase_of_length_4());

    apply(&mut app, &[SHIFT_TAB, SHIFT_TAB, DOWN, SELECT, ROWS_UP]);
    assert_eq!(pitches(&app), [11, 10, 12, 13, 14, 15]);

    apply(&mut app, &[ROWS_DOWN, ROWS_DOWN, ROWS_DOWN]);
    assert_eq!(pitches(&app), [10, 12, 13, 11, 14, 15]);
}

#[test]
fn at_either_edge_a_press_changes_nothing_adds_no_undo_entry_and_shows_no_remark() {
    let cases = [
        (song(), vec![SELECT], ROWS_UP),
        (
            last_song_row_filled(),
            [[DOWN; SONG_ROWS - 1].as_slice(), &[SELECT]].concat(),
            ROWS_DOWN,
        ),
        (chain(), vec![SHIFT_TAB, SELECT], ROWS_UP),
        (
            chain(),
            [[SHIFT_TAB].as_slice(), &[DOWN; 15], &[SELECT]].concat(),
            ROWS_DOWN,
        ),
        (
            phrase_of_length_4(),
            vec![SHIFT_TAB, SHIFT_TAB, SELECT],
            ROWS_UP,
        ),
        (
            phrase_of_length_4(),
            vec![SHIFT_TAB, SHIFT_TAB, DOWN, DOWN, DOWN, SELECT],
            ROWS_DOWN,
        ),
    ];
    for (project, moves, press) in cases {
        let mut app = App::with_project(project.clone());
        apply(&mut app, &moves);
        let screen = app.grid().text_dump();

        app.apply(press);

        assert!(app.project() == &project, "{moves:?} {press:?}");
        assert_eq!(app.grid().text_dump(), screen, "{moves:?} {press:?}");
        app.apply(Action::Undo);
        assert!(support::status_line(&app).contains("NOTHING TO UNDO"));
    }
}

#[test]
fn each_press_is_one_undo_entry_in_every_pane() {
    let cases = [
        (song(), vec![DOWN, SELECT_DOWN]),
        (chain(), vec![SHIFT_TAB, DOWN, SELECT_DOWN]),
        (
            phrase_of_length_4(),
            vec![SHIFT_TAB, SHIFT_TAB, DOWN, SELECT],
        ),
    ];
    for (project, moves) in cases {
        let mut app = App::with_project(project.clone());
        apply(&mut app, &moves);

        app.apply(ROWS_DOWN);
        let once = app.project().clone();
        app.apply(ROWS_DOWN);
        assert!(app.project() != &once, "{moves:?}");

        app.apply(Action::Undo);
        assert!(app.project() == &once, "{moves:?}");
        app.apply(Action::Undo);
        assert!(app.project() == &project, "{moves:?}");
        app.apply(Action::Redo);
        assert!(app.project() == &once, "{moves:?}");
    }
}

#[test]
fn without_a_selection_nothing_moves() {
    let mut app = App::with_project(song());

    apply(&mut app, &[DOWN, ROWS_DOWN]);

    assert!(app.project() == &song());
}

fn press(app: &mut App, key: Key, modifiers: Modifiers) {
    let actions: Vec<Action> = app
        .key_down(KeyPress::new(key, modifiers), Duration::from_secs(1))
        .into_iter()
        .chain(app.key_up(key))
        .collect();
    apply(app, &actions);
}

fn chord(app: &mut App, held: Key, key: Key) {
    let actions: Vec<Action> = [
        app.key_down(KeyPress::new(held, Modifiers::NONE), Duration::ZERO),
        app.key_down(
            KeyPress::new(key, Modifiers::NONE),
            Duration::from_millis(10),
        ),
        app.key_up(key),
        app.key_up(held),
    ]
    .into_iter()
    .flatten()
    .collect();
    apply(app, &actions);
}

fn m8_style_song() -> App {
    let mut app = App::new();
    app.launch(Launch {
        config_file: Some(PathBuf::from("/config/pikutracker/config.ron")),
        config: SettingsContents::Bytes(b"(version: 1, preset: M8Style)".to_vec()),
        state_file: None,
        state: SettingsContents::Missing,
        directory: PathBuf::from("/home/user"),
    });
    for row in 0..3 {
        apply(
            &mut app,
            &[Action::HexDigit(1), Action::HexDigit(row), DOWN],
        );
    }
    apply(&mut app, &[UP, UP, UP]);
    app
}

#[test]
fn alt_up_and_down_move_the_selected_rows_in_the_pc_tracker_preset() {
    let mut app = App::with_project(song());
    apply(&mut app, &[DOWN, SELECT]);

    press(&mut app, Key::Down, Modifiers::ALT);
    assert_eq!(
        song_column(&app, 0),
        [Some(0x10), Some(0x12), Some(0x11), Some(0x13), Some(0x14)]
    );

    press(&mut app, Key::Up, Modifiers::ALT);
    assert!(app.project() == &song());
}

#[test]
fn without_a_selection_alt_up_and_down_still_jump() {
    let mut app = App::with_project(song());

    press(&mut app, Key::Down, Modifiers::ALT);

    assert!(app.project() == &song());
    assert!(app.grid().text_dump().contains("10 [--]"));
}

#[test]
fn x_up_and_down_move_the_selected_rows_in_the_m8_style_preset() {
    let mut app = m8_style_song();
    press(&mut app, Key::Char('x'), Modifiers::SHIFT);

    chord(&mut app, Key::Char('x'), Key::Down);
    assert_eq!(
        song_column(&app, 0),
        [Some(0x11), Some(0x10), Some(0x12), None, None]
    );

    chord(&mut app, Key::Char('x'), Key::Up);
    assert_eq!(
        song_column(&app, 0),
        [Some(0x10), Some(0x11), Some(0x12), None, None]
    );
}

#[test]
fn without_a_selection_x_up_and_down_still_jump_in_the_m8_style_preset() {
    let mut app = m8_style_song();

    chord(&mut app, Key::Char('x'), Key::Down);

    assert_eq!(
        song_column(&app, 0),
        [Some(0x10), Some(0x11), Some(0x12), None, None]
    );
    assert!(app.grid().text_dump().contains("10 [--]"));
}
