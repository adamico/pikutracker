mod support;

use pikutracker_core::app::{Action, App, Direction, layout};
use support::pane_text;

const FIRST_SONG_LINE: usize = 2;
const LAST_SONG_LINE: usize = 24;

fn app_after(actions: &[Action]) -> App {
    let mut app = App::new();
    for action in actions {
        app.apply(*action);
    }
    app
}

fn song_text(app: &App, index: usize) -> String {
    pane_text(app, layout::SONG, index)
}

fn chain_text(app: &App, index: usize) -> String {
    pane_text(app, layout::CHAIN, index)
}

fn phrase_text(app: &App, index: usize) -> String {
    pane_text(app, layout::PHRASE, index)
}

fn right_slot_text(app: &App, index: usize) -> String {
    pane_text(app, layout::RIGHT_SLOT, index)
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
fn the_right_section_shows_the_multitrack_without_the_overview_scope_or_phrases_list() {
    let app = App::new();

    let right_section: Vec<String> = (0..layout::RIGHT_SLOT.rows)
        .map(|line| right_slot_text(&app, line))
        .filter(|text| !text.is_empty())
        .collect();
    assert_eq!(
        right_section,
        [
            "MULTITRACK  ROW 00",
            "    TRACK 1     TRACK 2     TRACK 3     TRACK 4"
        ]
    );
    let dump = app.grid().text_dump();
    for gone in ["OVERVIEW", "SCOPE", "EXPORT TRACKS", "PHRASES"] {
        assert!(!dump.contains(gone), "{gone}");
    }
}

#[test]
fn the_song_pane_sits_under_the_chain_pane_with_23_rows() {
    let app = app_after(&[Action::Move(Direction::Down); 11]);

    assert_eq!(chain_text(&app, 17), "0F --     ---");
    assert_eq!(song_text(&app, 0), "SONG");
    assert_eq!(song_text(&app, FIRST_SONG_LINE), "00  -- -- -- --");
    assert_eq!(song_text(&app, LAST_SONG_LINE), "16  -- -- -- --");
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
    assert_eq!(song_text(&app, FIRST_SONG_LINE), "1F  -- -- -- --");
    assert_eq!(song_text(&app, FIRST_SONG_LINE + 11), "2A [--]-- -- --");

    let app = app_after(&[Action::Move(Direction::Down); 300]);
    assert_eq!(song_text(&app, LAST_SONG_LINE), "FF [--]-- -- --");
}

#[test]
fn tab_goes_through_song_chain_phrase_and_the_multitrack_then_back_to_the_phrase() {
    let app = app_after(&[Action::FocusNext]);
    assert_eq!(song_text(&app, 2), "00  -- -- -- --");
    assert_eq!(chain_text(&app, 2), "00[--]    ---");

    let app = app_after(&[Action::FocusNext; 2]);
    assert_eq!(chain_text(&app, 2), "00 --     ---");
    assert_eq!(phrase_text(&app, 7), "00[...] .    .   .");

    let app = app_after(&[Action::FocusNext; 3]);
    assert_eq!(phrase_text(&app, 7), "00 ...  .    .   .");
    assert_eq!(song_text(&app, 2), "00  -- -- -- --");

    let app = app_after(&[Action::FocusNext; 4]);
    assert_eq!(song_text(&app, 2), "00  -- -- -- --");
    assert_eq!(phrase_text(&app, 7), "00[...] .    .   .");
}

#[test]
fn focus_previous_goes_back_from_the_song_pane_to_the_right_hand_view() {
    let app = app_after(&[Action::FocusPrevious]);

    assert_eq!(song_text(&app, 2), "00  -- -- -- --");
    assert_eq!(chain_text(&app, 2), "00 --     ---");
    assert_eq!(phrase_text(&app, 7), "00 ...  .    .   .");
    assert_eq!(right_slot_text(&app, 0), "MULTITRACK  ROW 00");
}

#[test]
fn arrows_move_the_cursor_inside_the_focused_pane_only() {
    let app = app_after(&[
        Action::FocusNext,
        Action::Move(Direction::Down),
        Action::Move(Direction::Right),
    ]);

    assert_eq!(chain_text(&app, 3), "01 --    [---]");
    assert_eq!(song_text(&app, 2), "00  -- -- -- --");

    let app = app_after(&[
        Action::FocusNext,
        Action::FocusNext,
        Action::Move(Direction::Down),
        Action::Move(Direction::Right),
        Action::Move(Direction::Right),
        Action::Move(Direction::Right),
    ]);

    assert_eq!(phrase_text(&app, 8), "01 ...  .    .  [.]");
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
    assert_eq!(chain_text(&app, 17), "0F --    [---]");

    let app = app_after(
        &[
            &[Action::FocusNext, Action::FocusNext][..],
            &[Action::Move(Direction::Down); 40],
            &[Action::Move(Direction::Right); 6],
        ]
        .concat(),
    );
    assert_eq!(phrase_text(&app, 38), "1F ...  .    .  [.]");
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
fn the_chain_pane_title_names_only_the_chain_under_the_song_cursor() {
    let app = app_after(&[
        Action::Move(Direction::Down),
        Action::Move(Direction::Right),
        Action::HexDigit(0),
        Action::HexDigit(5),
    ]);
    assert_eq!(chain_text(&app, 0), "CHAIN 05");

    let app = app_after(&[Action::Move(Direction::Down)]);
    assert_eq!(chain_text(&app, 0), "CHAIN --");
}

#[test]
fn actions_not_built_yet_change_nothing() {
    let app = app_after(&[Action::SuggestClaim]);

    assert_eq!(
        app.grid().text_dump(),
        include_str!("snapshots/empty_project.txt")
    );
}
