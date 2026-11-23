mod support;

use pikutracker_core::app::{Action, App, Direction};
use pikutracker_core::keymap::{Key, KeyPress, Keymap, Modifiers, named_actions};
use pikutracker_core::project::{ChainId, Project, SongRowEnding, SongRowMarks};
use support::apply;

const DOWN: Action = Action::Move(Direction::Down);
const FIRST_ROW_LINE: usize = 2;

fn marks(app: &App, row: usize) -> SongRowMarks {
    app.project().song_row_marks(row)
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
    assert_eq!(marks(&app, 1).ending, Some(SongRowEnding::LoopBack));
    app.apply(Action::ToggleLoopBack);
    assert_eq!(marks(&app, 1).ending, None);

    app.apply(Action::ToggleStop);
    assert_eq!(marks(&app, 1).ending, Some(SongRowEnding::Stop));
    app.apply(Action::ToggleStop);
    assert_eq!(marks(&app, 1).ending, None);
}

#[test]
fn setting_loop_back_clears_stop_and_setting_stop_clears_loop_back() {
    let mut app = App::new();

    apply(&mut app, &[Action::ToggleStop, Action::ToggleLoopBack]);
    assert_eq!(marks(&app, 0).ending, Some(SongRowEnding::LoopBack));

    app.apply(Action::ToggleStop);
    assert_eq!(marks(&app, 0).ending, Some(SongRowEnding::Stop));
}

#[test]
fn loop_start_shares_a_row_with_loop_back_or_stop() {
    let mut app = App::new();

    apply(&mut app, &[Action::ToggleLoopStart, Action::ToggleLoopBack]);
    assert_eq!(
        marks(&app, 0),
        SongRowMarks {
            loop_start: true,
            ending: Some(SongRowEnding::LoopBack),
        }
    );

    app.apply(Action::ToggleStop);
    assert_eq!(
        marks(&app, 0),
        SongRowMarks {
            loop_start: true,
            ending: Some(SongRowEnding::Stop),
        }
    );
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
    assert_eq!(marks(&app, 0).ending, Some(SongRowEnding::Stop));
    app.apply(Action::Undo);
    assert_eq!(marks(&app, 0).ending, None);
    assert!(marks(&app, 0).loop_start);
    app.apply(Action::Undo);
    assert_eq!(marks(&app, 0), SongRowMarks::default());

    app.apply(Action::Redo);
    assert!(marks(&app, 0).loop_start);
}

fn song_line(app: &App, line: usize) -> String {
    let dump = app.grid().text_dump();
    let text: String = dump.lines().nth(line).unwrap().chars().take(7).collect();
    text.trim_end().to_owned()
}

#[test]
fn the_song_pane_shows_each_rows_marks_after_its_number() {
    let mut project = Project::new();
    let both = |ending| SongRowMarks {
        loop_start: true,
        ending: Some(ending),
    };
    project.set_song_row_marks(
        1,
        SongRowMarks {
            loop_start: true,
            ending: None,
        },
    );
    project.set_song_row_marks(
        2,
        SongRowMarks {
            loop_start: false,
            ending: Some(SongRowEnding::LoopBack),
        },
    );
    project.set_song_row_marks(
        3,
        SongRowMarks {
            loop_start: false,
            ending: Some(SongRowEnding::Stop),
        },
    );
    project.set_song_row_marks(4, both(SongRowEnding::LoopBack));
    project.set_song_row_marks(5, both(SongRowEnding::Stop));
    let app = App::with_project(project);

    let rows: Vec<String> = (FIRST_ROW_LINE..FIRST_ROW_LINE + 6)
        .map(|line| song_line(&app, line))
        .collect();

    assert_eq!(
        rows,
        ["00 [--]", "01> --", "02< --", "03| --", "04= --", "05/ --"]
    );
}

fn marked_song() -> Project {
    let mut project = Project::new();
    for row in 0..3 {
        project.set_song_cell(row, 0, Some(ChainId::new(row as u8)));
    }
    project.set_song_row_marks(
        1,
        SongRowMarks {
            loop_start: true,
            ending: Some(SongRowEnding::LoopBack),
        },
    );
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
