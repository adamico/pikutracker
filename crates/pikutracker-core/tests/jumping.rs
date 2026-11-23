use pikutracker_core::app::{Action, App, Direction};
use pikutracker_core::project::{ChainEntry, ChainId, PhraseId, Project};

const TAB: Action = Action::FocusNext;
const UP: Action = Action::Jump(Direction::Up);
const DOWN: Action = Action::Jump(Direction::Down);
const LEFT: Action = Action::Jump(Direction::Left);
const RIGHT: Action = Action::Jump(Direction::Right);

fn project() -> Project {
    let mut project = Project::new();
    for id in 1..=3 {
        let phrase = project.phrase_mut(PhraseId::new(id));
        phrase.step_mut(0).set_pitch(33);
        phrase.step_mut(0).set_volume(5);
    }
    let chain = project.chain_mut(ChainId::new(0));
    *chain.entry_mut(0) = Some(ChainEntry::new(PhraseId::new(1)));
    *chain.entry_mut(3) = Some(ChainEntry::new(PhraseId::new(2)));
    *project.chain_mut(ChainId::new(1)).entry_mut(0) = Some(ChainEntry::new(PhraseId::new(3)));
    *project.chain_mut(ChainId::new(2)).entry_mut(0) = Some(ChainEntry::new(PhraseId::new(3)));
    project.set_song_cell(0, 0, Some(ChainId::new(0)));
    project.set_song_cell(5, 0, Some(ChainId::new(1)));
    project.set_song_cell(0, 1, Some(ChainId::new(2)));
    project
}

fn app(actions: &[Action]) -> App {
    let mut app = App::with_project(project());
    for action in actions {
        app.apply(*action);
    }
    app
}

fn chain_number(app: &App) -> Option<u8> {
    app.shown_chain().map(|id| id.number())
}

fn phrase_number(app: &App) -> Option<u8> {
    app.shown_phrase().map(|id| id.number())
}

#[test]
fn in_the_song_pane_up_and_down_jump_16_rows() {
    let down = app(&[DOWN, DOWN]);
    let back = app(&[DOWN, DOWN, UP]);

    assert!(down.grid().text_dump().contains("20 [--]"));
    assert!(back.grid().text_dump().contains("10 [--]"));
}

#[test]
fn in_the_chain_pane_left_and_right_jump_track() {
    let next = app(&[TAB, RIGHT]);
    let back = app(&[TAB, RIGHT, LEFT]);

    assert_eq!(chain_number(&next), Some(2));
    assert_eq!(chain_number(&back), Some(0));
}

#[test]
fn in_the_chain_pane_up_and_down_jump_to_the_tracks_next_chain() {
    let next = app(&[TAB, DOWN]);
    let past_the_last = app(&[TAB, DOWN, DOWN]);
    let back = app(&[TAB, DOWN, UP]);

    assert_eq!(chain_number(&next), Some(1));
    assert_eq!(
        chain_number(&past_the_last),
        Some(1),
        "it stays on the last"
    );
    assert_eq!(chain_number(&back), Some(0));
}

#[test]
fn in_the_phrase_pane_up_and_down_jump_to_the_chains_next_phrase() {
    let next = app(&[TAB, TAB, DOWN]);
    let back = app(&[TAB, TAB, DOWN, UP]);

    assert_eq!(phrase_number(&next), Some(2));
    assert_eq!(phrase_number(&back), Some(1));
}

#[test]
fn in_the_phrase_pane_left_and_right_jump_track() {
    let next = app(&[TAB, TAB, RIGHT]);

    assert_eq!(phrase_number(&next), Some(3));
    assert!(next.grid().text_dump().contains("PHRASE 03"));
}

#[test]
fn in_the_instrument_view_left_and_right_jump_instrument() {
    let mut app = app(&[TAB, TAB, Action::Move(Direction::Right)]);
    app.apply(Action::HexDigit(8));
    app.apply(Action::Confirm);
    assert!(app.grid().text_dump().contains("INSTRUMENT 8"));

    app.apply(RIGHT);
    assert!(app.grid().text_dump().contains("INSTRUMENT 9"));
    app.apply(LEFT);
    app.apply(LEFT);

    assert!(
        app.grid().text_dump().contains("INSTRUMENT 8"),
        "the Built-in Instruments have no view"
    );
}

#[test]
fn a_jump_by_hand_turns_follow_off() {
    let app = app(&[Action::ToggleFollow, TAB, DOWN]);

    assert!(app.grid().text_dump().contains("FOLLOW OFF"));
}
