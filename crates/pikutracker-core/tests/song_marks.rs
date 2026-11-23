mod support;

use pikutracker_core::app::{Action, App, Direction, layout};
use pikutracker_core::keymap::{Key, KeyPress, Keymap, Modifiers, named_actions};
use pikutracker_core::project::{ChainId, Project, SongRowMarks};
use support::apply;

const DOWN: Action = Action::Move(Direction::Down);
const FIRST_ROW_LINE: usize = 2;

fn marks(app: &App, row: usize) -> SongRowMarks {
    app.project().song_row_marks(row)
}

const fn set(loop_start: bool, loop_back: bool, stop: bool) -> SongRowMarks {
    SongRowMarks {
        loop_start,
        loop_back,
        stop,
    }
}

#[test]
fn a_new_song_has_no_marks() {
    let app = App::new();

    assert_eq!(marks(&app, 0), SongRowMarks::default());
}

#[test]
fn loop_start_toggles_on_the_song_row_under_the_cursor() {
    let mut app = App::new();

    apply(&mut app, &[DOWN, DOWN, Action::ToggleLoopStart]);
    assert!(marks(&app, 2).loop_start);
    assert_eq!(marks(&app, 0), SongRowMarks::default());

    app.apply(Action::ToggleLoopStart);
    assert_eq!(marks(&app, 2), SongRowMarks::default());
}

#[test]
fn loop_back_and_stop_toggle_on_the_song_row_under_the_cursor() {
    let mut app = App::new();

    apply(&mut app, &[DOWN, Action::ToggleLoopBack]);
    assert_eq!(marks(&app, 1), set(false, true, false));
    app.apply(Action::ToggleLoopBack);
    assert_eq!(marks(&app, 1), SongRowMarks::default());

    app.apply(Action::ToggleStop);
    assert_eq!(marks(&app, 1), set(false, false, true));
    app.apply(Action::ToggleStop);
    assert_eq!(marks(&app, 1), SongRowMarks::default());
}

#[test]
fn a_row_can_carry_all_three_marks_and_each_toggles_alone() {
    let mut app = App::new();

    apply(
        &mut app,
        &[
            Action::ToggleLoopStart,
            Action::ToggleLoopBack,
            Action::ToggleStop,
        ],
    );
    assert_eq!(marks(&app, 0), set(true, true, true));

    app.apply(Action::ToggleLoopBack);
    assert_eq!(marks(&app, 0), set(true, false, true));
}

#[test]
fn each_toggle_is_one_undoable_edit() {
    let mut app = App::new();
    apply(
        &mut app,
        &[
            Action::ToggleLoopStart,
            Action::ToggleStop,
            Action::ToggleLoopBack,
        ],
    );

    app.apply(Action::Undo);
    assert_eq!(marks(&app, 0), set(true, false, true));
    app.apply(Action::Undo);
    assert_eq!(marks(&app, 0), set(true, false, false));
    app.apply(Action::Undo);
    assert_eq!(marks(&app, 0), SongRowMarks::default());

    app.apply(Action::Redo);
    assert!(marks(&app, 0).loop_start);
}

fn song_line(app: &App, line: usize) -> String {
    let text: String = support::pane_text(app, layout::SONG, line)
        .chars()
        .take(7)
        .collect();
    text.trim_end().to_owned()
}

#[test]
fn the_song_pane_shows_each_rows_marks_after_its_number() {
    let mut project = Project::new();
    project.set_song_row_marks(1, set(true, false, false));
    project.set_song_row_marks(2, set(false, true, false));
    project.set_song_row_marks(3, set(false, false, true));
    project.set_song_row_marks(4, set(true, true, false));
    project.set_song_row_marks(5, set(true, false, true));
    project.set_song_row_marks(6, set(false, true, true));
    project.set_song_row_marks(7, set(true, true, true));
    let app = App::with_project(project);

    let rows: Vec<String> = (FIRST_ROW_LINE..FIRST_ROW_LINE + 8)
        .map(|line| song_line(&app, line))
        .collect();

    assert_eq!(
        rows,
        [
            "00 [--]", "01> --", "02< --", "03| --", "04= --", "05/ --", "06[ --", "07# --"
        ]
    );
}

fn marked_song() -> Project {
    let mut project = Project::new();
    for row in 0..3 {
        project.set_song_cell(row, 0, Some(ChainId::new(row as u8)));
    }
    project.set_song_row_marks(1, set(true, true, false));
    project
}

#[test]
fn insert_row_shifts_the_marks_with_the_rows() {
    let mut app = App::with_project(marked_song());

    apply(&mut app, &[DOWN, Action::InsertRow]);

    assert_eq!(marks(&app, 1), SongRowMarks::default());
    assert_eq!(marks(&app, 2), marked_song().song_row_marks(1));
}

#[test]
fn insert_row_below_the_marks_leaves_them_where_they_are() {
    let mut app = App::with_project(marked_song());

    apply(&mut app, &[DOWN, DOWN, Action::InsertRow]);

    assert_eq!(marks(&app, 1), marked_song().song_row_marks(1));
}

#[test]
fn delete_row_takes_the_marks_of_its_row_and_shifts_the_rest() {
    let mut app = App::with_project(marked_song());

    apply(&mut app, &[Action::DeleteRow]);
    assert_eq!(marks(&app, 0), marked_song().song_row_marks(1));

    app.apply(Action::DeleteRow);
    assert_eq!(marks(&app, 0), SongRowMarks::default());
}

#[test]
fn undoing_a_shift_puts_the_marks_back() {
    let mut app = App::with_project(marked_song());

    apply(&mut app, &[Action::InsertRow, Action::Undo]);

    assert!(app.project() == &marked_song());
}

#[test]
fn moving_song_rows_leaves_the_marks_where_they_are() {
    let mut app = App::with_project(marked_song());

    apply(
        &mut app,
        &[DOWN, Action::StartSelection, Action::MoveRows { up: false }],
    );

    assert_eq!(app.project().song_cell(2, 0), Some(ChainId::new(1)));
    assert_eq!(marks(&app, 1), marked_song().song_row_marks(1));
    assert_eq!(marks(&app, 2), SongRowMarks::default());
}

#[test]
fn the_mark_keys_are_bound_in_both_presets() {
    let pc_tracker = Keymap::pc_tracker();
    let pc = |ch, modifiers| pc_tracker.action_for(KeyPress::new(Key::Char(ch), modifiers));
    assert_eq!(pc('r', Modifiers::CTRL), Some(Action::ToggleLoopStart));
    assert_eq!(pc('r', Modifiers::CTRL_SHIFT), Some(Action::ToggleLoopBack));
    assert_eq!(pc('p', Modifiers::CTRL), Some(Action::ToggleStop));

    let named: Vec<String> = named_actions().into_iter().map(|(name, _)| name).collect();
    for name in ["ToggleLoopStart", "ToggleLoopBack", "ToggleStop"] {
        assert!(named.iter().any(|named| named == name));
    }
    assert!(!named.iter().any(|named| named == "ToggleSongLoopRow"));
}
