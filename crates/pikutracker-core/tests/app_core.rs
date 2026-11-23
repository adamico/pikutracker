use pikutracker_core::app::{Action, App, Direction};

const FIRST_SONG_LINE: usize = 2;
const LAST_PANE_LINE: usize = 43;

fn app_after(actions: &[Action]) -> App {
    let mut app = App::new();
    for action in actions {
        app.apply(*action);
    }
    app
}

fn line(app: &App, index: usize) -> String {
    app.grid()
        .text_dump()
        .lines()
        .nth(index)
        .unwrap()
        .to_owned()
}

fn pane_text(app: &App, index: usize, columns: std::ops::Range<usize>) -> String {
    let text: String = line(app, index)
        .chars()
        .chain(std::iter::repeat(' '))
        .skip(columns.start)
        .take(columns.len())
        .collect();
    text.trim_end().to_owned()
}

fn song_text(app: &App, index: usize) -> String {
    pane_text(app, index, 0..28)
}

fn chain_text(app: &App, index: usize) -> String {
    pane_text(app, index, 29..62)
}

fn phrase_text(app: &App, index: usize) -> String {
    pane_text(app, index, 63..82)
}

fn right_slot_text(app: &App, index: usize) -> String {
    pane_text(app, index, 83..106)
}

#[test]
fn a_new_app_shows_the_empty_panes_in_the_agreed_column_budget() {
    let app = App::new();

    assert_eq!(
        app.grid().text_dump(),
        include_str!("snapshots/empty_project.txt")
    );
}

#[test]
fn arrows_move_the_cursor_in_the_song_pane() {
    let app = app_after(&[
        Action::Move(Direction::Down),
        Action::Move(Direction::Right),
    ]);

    assert_eq!(song_text(&app, FIRST_SONG_LINE), "00  -- -- -- --");
    assert_eq!(song_text(&app, FIRST_SONG_LINE + 1), "01  --[--]-- --");
}

#[test]
fn the_cursor_stops_at_the_edges_of_the_song_pane() {
    let mut actions = vec![Action::Move(Direction::Up), Action::Move(Direction::Left)];
    let app = app_after(&actions);
    assert_eq!(song_text(&app, FIRST_SONG_LINE), "00 [--]-- -- --");

    actions.extend([Action::Move(Direction::Right); 20]);
    let app = app_after(&actions);
    assert_eq!(song_text(&app, FIRST_SONG_LINE), "00  -- -- --[--]");
}

#[test]
fn the_song_pane_scrolls_to_keep_the_cursor_visible() {
    let app = app_after(&[Action::Move(Direction::Down); 42]);
    assert_eq!(song_text(&app, FIRST_SONG_LINE), "15  -- -- -- --");
    assert_eq!(song_text(&app, FIRST_SONG_LINE + 21), "2A [--]-- -- --");

    let app = app_after(&[Action::Move(Direction::Down); 300]);
    assert_eq!(song_text(&app, LAST_PANE_LINE), "FF [--]-- -- --");
}

#[test]
fn tab_cycles_focus_through_song_chain_phrase_and_the_right_hand_view() {
    let app = app_after(&[Action::FocusNext]);
    assert_eq!(song_text(&app, 2), "00  -- -- -- --");
    assert_eq!(chain_text(&app, 2), "0[--]---           -");

    let app = app_after(&[Action::FocusNext; 2]);
    assert_eq!(chain_text(&app, 2), "0 -- ---           -");
    assert_eq!(phrase_text(&app, 5), "00[...].. . .");

    let app = app_after(&[Action::FocusNext; 3]);
    assert_eq!(phrase_text(&app, 5), "00 ... .. . .");
    assert_eq!(song_text(&app, 2), "00  -- -- -- --");

    let app = app_after(&[Action::FocusNext; 4]);
    assert_eq!(song_text(&app, 2), "00 [--]-- -- --");
}

#[test]
fn focus_previous_goes_back_from_the_song_pane_to_the_right_hand_view() {
    let app = app_after(&[Action::FocusPrevious]);

    assert_eq!(song_text(&app, 2), "00  -- -- -- --");
    assert_eq!(chain_text(&app, 2), "0 -- ---           -");
    assert_eq!(phrase_text(&app, 5), "00 ... .. . .");
    assert_eq!(right_slot_text(&app, 0), "SCOPE");
}

#[test]
fn arrows_move_the_cursor_inside_the_focused_pane_only() {
    let app = app_after(&[
        Action::FocusNext,
        Action::Move(Direction::Down),
        Action::Move(Direction::Right),
    ]);

    assert_eq!(chain_text(&app, 3), "1 --[---]          -");
    assert_eq!(song_text(&app, 2), "00  -- -- -- --");

    let app = app_after(&[
        Action::FocusNext,
        Action::FocusNext,
        Action::Move(Direction::Down),
        Action::Move(Direction::Right),
        Action::Move(Direction::Right),
        Action::Move(Direction::Right),
    ]);

    assert_eq!(phrase_text(&app, 6), "01 ... .. .[.]");
}

#[test]
fn the_cursor_stops_at_the_edges_of_every_pane() {
    let app = app_after(
        &[
            &[Action::FocusNext][..],
            &[Action::Move(Direction::Down); 30],
            &[Action::Move(Direction::Right); 5],
        ]
        .concat(),
    );
    assert_eq!(chain_text(&app, 17), "F --[---]          -");

    let app = app_after(
        &[
            &[Action::FocusNext, Action::FocusNext][..],
            &[Action::Move(Direction::Down); 40],
            &[Action::Move(Direction::Right); 6],
        ]
        .concat(),
    );
    assert_eq!(phrase_text(&app, 36), "1F ... .. .[.]");
}

#[test]
fn a_pane_keeps_its_cursor_when_focus_comes_back() {
    let app = app_after(&[
        Action::Move(Direction::Down),
        Action::FocusNext,
        Action::FocusPrevious,
    ]);

    assert_eq!(song_text(&app, 3), "01 [--]-- -- --");
}

#[test]
fn the_chain_pane_title_follows_the_song_cursor() {
    let app = app_after(&[
        Action::Move(Direction::Down),
        Action::Move(Direction::Right),
    ]);

    assert_eq!(chain_text(&app, 0), "CHAIN --  T2 ROW 01");
}

#[test]
fn actions_not_built_yet_change_nothing() {
    let app = app_after(&[Action::SuggestClaim]);

    assert_eq!(
        app.grid().text_dump(),
        include_str!("snapshots/empty_project.txt")
    );
}
